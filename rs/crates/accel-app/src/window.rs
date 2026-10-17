//! 主窗装配器（ui/main_window.py 的 Rust 对等物，PL013.01/.11）
//! AppHandle + eframe::App 实现：drain → 导航 → 当前页 → 玻璃背板 → 重绘节奏

use std::sync::Arc;

use accel_core::interface::AppInterface;
use accel_core::types::FrontendCommand;
use egui::{Context, Pos2, Rect, Vec2};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::bus::{Bus, PAGE_ORDER, PageId, UiState};
use crate::glass::painter::{attach_pipeline, build_uniforms, push_paint_callback};
use crate::glass::surface::{GlassScene, GlassSurface};
use crate::nav::NavRail;
use crate::pages::Pages;
use crate::theme::Theme;

/// 玻璃面 id（导航 rail / 页卡 / 浮窗）
pub const SURFACE_NAV: &str = "nav";
pub const SURFACE_PAGE_PREFIX: &str = "page_";
pub const SURFACE_SETTINGS: &str = "settings";

/// 窗口几何 token 基准（base.json；restore 时用户拖拽覆盖）
fn default_window_rect() -> (f32, f32, f32, f32) {
    let base = accel_core::static_config::Static::get();
    (
        base.base_f64("window_x") as f32,
        base.base_f64("window_y") as f32,
        base.base_f64("window_width") as f32,
        base.base_f64("window_height") as f32,
    )
}

/// 应用句柄（eframe::App 实现）
pub struct AppHandle {
    pub bus: Bus,
    pub state: UiState,
    pub theme: Theme,
    pub scene: GlassScene,
    pub nav: NavRail,
    pub pages: Pages,
    pub settings_pop: crate::pages::SettingsPop,
    start_time: std::time::Instant,
    pipeline_attached: bool,
    /// Acrylic 挂接一次性标记（.04）
    acrylic_applied: bool,
    /// 探针自动关窗秒数（None=正常模式）
    probe_close_secs: Option<u64>,
    /// 帧计数（.13 空闲节奏探针）
    pub repaint_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl eframe::App for AppHandle {
    /// 每帧逻辑（先于 ui；窗口隐藏期也调用）——事件 drain 唯一入口（锁不跨帧）
    fn logic(&mut self, _ctx: &Context, _frame: &mut eframe::Frame) {
        self.bus.drain(&mut self.state);
        self.repaint_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// 帧绘制：背板 → 导航 → 当前页 → 设置浮窗 → 重绘节奏
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 探针路径：到时自动关窗（fps_probe 静置计数后退出）
        if let Some(secs) = self.probe_close_secs
            && self.start_time.elapsed().as_secs() >= secs
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Acrylic 一次性挂接（.04）：Frame 经 HasWindowHandle 取句柄，WindowHandle
        // 经 From 转 RawWindowHandle（0.6.2 特殊设计：window_handle 递归返自身）；
        // 失败静默降级实底（透明 viewport 下 GL 背板仍提供半透明观感）
        if !self.acrylic_applied {
            if let Ok(h) = frame.window_handle() {
                let raw: RawWindowHandle = h.into();
                if let RawWindowHandle::Win32(w) = raw {
                    self.acrylic_applied = crate::platform::acrylic::apply_acrylic(w.hwnd.get());
                }
            }
            self.acrylic_applied = true; // 无论成败只试一次（失败即环境不支持）
        }

        // 背板：全屏玻璃管线（背景光场 + 场景面合成）；不可用降级纯色
        self.paint_backplate(&ctx, frame);

        // 导航 + 当前页 + 设置浮窗（各部件自绘 + 注册玻璃面）
        self.sync_page_visibility();
        let accent = crate::theme::accent_rgba();
        self.nav.ui(&ctx, &mut self.state, &mut self.scene, accent);
        self.pages.ui(
            ui,
            &mut self.state,
            &self.bus,
            &mut self.scene,
            &mut self.settings_pop,
        );
        self.paint_settings_pop(&ctx);

        // 关闭请求：汇总落盘后放行（eframe 0.37 无 on_close_event——input 检测）
        if ctx.input(|i| i.viewport().close_requested()) {
            self.bus.send(FrontendCommand::SaveNow);
        }

        // 重绘节奏：Tick 间隔（空闲 1s 兜底）——零忙转纪律
        ctx.request_repaint_after(self.state.next_repaint_interval());
    }
}

impl AppHandle {
    /// 背板绘制：全屏玻璃回调（场景含导航/页卡/浮窗面）+ 场景 dirty 时的直连重绘
    fn paint_backplate(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        let Some(rs) = frame.wgpu_render_state() else {
            // 无 wgpu（软件回退等）：纯色背板降级，行为无损
            ctx.set_visuals(if self.theme.is_dark() {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            return;
        };
        if !self.pipeline_attached {
            let mut renderer = rs.renderer.write();
            let ok = attach_pipeline(
                &rs.device,
                rs.target_format,
                &mut renderer.callback_resources,
            );
            drop(renderer);
            if let Err(e) = ok {
                tracing::error!("玻璃管线构造失败，降级纯色背板: {e}");
                self.pipeline_attached = false;
            } else {
                self.pipeline_attached = true;
                // touch → repaint 直连（FIX004.7 平移）：hook 持 ctx 弱引用
                let ctx_weak = ctx.clone();
                self.scene
                    .set_repaint_hook(Some(Box::new(move || ctx_weak.request_repaint())));
            }
        }
        // 场景几何 → uniforms → 全屏回调
        let pixels_per_point = ctx.pixels_per_point();
        let screen = ctx.input(|i| i.content_rect());
        let size_px = [
            screen.width() * pixels_per_point,
            screen.height() * pixels_per_point,
        ];
        let tokens = self.theme.active();
        // glass.gl_lift token（0-255 → 0..1）；三级键经 pointer 直取
        let lift_value = {
            let s = accel_core::static_config::Static::get();
            s.ui.pointer("/glass/gl_lift")
                .and_then(|v| v.as_f64())
                .map(|v| (v / 255.0) as f32)
                .unwrap_or(0.10)
        };
        let uniforms = build_uniforms(
            &self.scene,
            size_px,
            tokens,
            self.start_time.elapsed().as_secs_f32(),
            crate::theme::accent_rgba(),
            lift_value,
        );
        let painter = ctx.layer_painter(egui::LayerId::background());
        push_paint_callback(&painter, uniforms);
    }

    /// 切页可见性：旧页玻璃面隐藏、新页显示（含惰性注册——PL009 僵尸面教训）
    fn sync_page_visibility(&mut self) {
        let current = self.state.current_page;
        for page in PAGE_ORDER {
            let id = format!("{SURFACE_PAGE_PREFIX}{page:?}");
            self.scene.set_visible(&id, page == current);
        }
    }

    /// 设置浮窗绘制（PL013.10：egui::Window + 主题三段 + 失焦/ESC 关）
    fn paint_settings_pop(&mut self, ctx: &Context) {
        if !self.state.settings_open {
            self.scene.set_visible(SURFACE_SETTINGS, false);
            return;
        }
        let mut keep_open = true;
        let settings_window = egui::Window::new("设置")
            .open(&mut keep_open)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::LEFT_BOTTOM, [16.0, -72.0])
            .show(ctx, |ui| {
                ui.label("主题");
                ui.horizontal(|ui| {
                    for (label, pref) in [
                        ("跟随系统", accel_core::types::ThemePref::Auto),
                        ("浅色", accel_core::types::ThemePref::Light),
                        ("深色", accel_core::types::ThemePref::Dark),
                    ] {
                        let selected = self.state.theme_pref() == pref;
                        if ui.selectable_label(selected, label).clicked() {
                            self.bus.send(FrontendCommand::SetTheme(pref));
                            // 乐观更新本地偏好（事件回推前 UI 即响应）
                            self.state.prefs.theme = pref_value(pref).to_string();
                            self.theme = Theme::load(self.state.prefs.theme.as_str());
                            self.theme.apply(ctx);
                        }
                    }
                });
                ui.separator();
                ui.label(format!(
                    "版本 {}",
                    accel_core::static_config::Static::get().version()
                ));
            });
        // 点窗外关闭：主键点击且交互点不在窗体 rect 内（.10 检验项）
        if keep_open && let Some(win) = &settings_window {
            let clicked_outside = ctx.input(|i| {
                i.pointer.primary_clicked()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|pos| !win.response.rect.contains(pos))
            });
            if clicked_outside {
                keep_open = false;
            }
        }
        // ESC 关闭
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            keep_open = false;
        }
        if !keep_open {
            self.state.settings_open = false;
            self.scene.set_visible(SURFACE_SETTINGS, false);
        } else {
            self.scene.set_visible(SURFACE_SETTINGS, true);
        }
    }
}

fn pref_value(pref: accel_core::types::ThemePref) -> &'static str {
    match pref {
        accel_core::types::ThemePref::Auto => "auto",
        accel_core::types::ThemePref::Light => "light",
        accel_core::types::ThemePref::Dark => "dark",
    }
}

/// 注册页卡玻璃面（pages.ui 内每页调用——几何随布局 rect 同步；register 幂等）
pub fn register_page_surface(scene: &mut GlassScene, page: PageId, rect: Rect) {
    let id = format!("{SURFACE_PAGE_PREFIX}{page:?}");
    let mut surface = GlassSurface::new(&id, rect);
    surface.radius = 16.0;
    scene.register(surface);
}

/// tokio runtime 装配（后台任务宿主：interface spawn 的调度/天气任务在此运行）
fn build_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime 构建失败")
}

/// AppHandle 装配 + eframe::run_native（window::run 由 main 调用）
pub fn run() -> Result<(), eframe::Error> {
    run_with_counter(Arc::new(std::sync::atomic::AtomicU64::new(0)), None)
}

/// 带外部帧计数器的运行（fps_probe 零忙转断言用；probe_close_secs 到时自动关窗）
pub fn run_with_counter(
    counter: Arc<std::sync::atomic::AtomicU64>,
    probe_close_secs: Option<u64>,
) -> Result<(), eframe::Error> {
    let runtime = Arc::new(build_runtime());
    let iface = Arc::new(AppInterface::new());

    let (x, y, w, h) = default_window_rect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(w, h))
            .with_position(Pos2::new(x, y))
            .with_transparent(true)
            .with_title(format!(
                "加速世界 {}",
                accel_core::static_config::Static::get().version()
            )),
        ..Default::default()
    };

    let rt_for_app = Arc::clone(&runtime);

    let create_app =
        move |cc: &eframe::CreationContext<'_>| -> Result<Box<dyn eframe::App>, String> {
            let prefs = iface.get_ui_preferences();
            let theme = Theme::load(prefs.theme.as_str());
            theme.apply(&cc.egui_ctx);
            install_fonts(&cc.egui_ctx);
            let mut state = UiState {
                rate: iface.get_rate(),
                rate_bounds: AppInterface::get_rate_bounds(),
                prefs,
                ..UiState::default()
            };
            state.time = Some(iface.get_time_info());

            // spawn 需 tokio 上下文——block_in_place + runtime 内完成（双 Arc 进入闭包）
            let bus =
                tokio::task::block_in_place(|| Bus::spawn_on(Arc::clone(&iface), &rt_for_app));

            Ok(Box::new(AppHandle {
                bus,
                state,
                theme,
                scene: GlassScene::new(),
                nav: NavRail::new(),
                pages: Pages::new(),
                settings_pop: crate::pages::SettingsPop::default(),
                start_time: std::time::Instant::now(),
                pipeline_attached: false,
                acrylic_applied: false,
                probe_close_secs,
                repaint_count: counter,
            }))
        };

    let result = eframe::run_native(
        "AccelWorld",
        options,
        Box::new(|cc| {
            create_app(cc).map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })
        }),
    );
    // runtime drop：后台任务全部收束（退出路径零残留）
    drop(runtime);
    result
}

/// 字体注入（PL013.03）：微软雅黑（中文）+ Bahnschrift（数字），默认族雅黑
pub fn install_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    let load = |name: &str| -> Option<Vec<u8>> {
        for dir in [
            "C:/Windows/Fonts",
            "/usr/share/fonts",
            "/System/Library/Fonts",
        ] {
            let candidates: &[&str] = if name == "yahei" {
                &["msyh.ttc", "msyh.ttf", "msyhbd.ttc"]
            } else {
                &["Bahnschrift.ttf", "bahnschrift.ttf"]
            };
            for f in candidates {
                let p = std::path::Path::new(dir).join(f);
                if let Ok(bytes) = std::fs::read(&p) {
                    tracing::info!("字体加载: {}", p.display());
                    return Some(bytes);
                }
            }
        }
        None
    };
    if let Some(bytes) = load("yahei") {
        fonts
            .font_data
            .insert("yahei".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "yahei".into());
    } else {
        tracing::warn!("微软雅黑未找到，中文回退 egui 内置字体（可能 tofu）");
    }
    if let Some(bytes) = load("bahnschrift") {
        fonts.font_data.insert(
            "bahnschrift".into(),
            egui::FontData::from_owned(bytes).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "bahnschrift".into());
    } else {
        tracing::warn!("Bahnschrift 未找到，数字等宽回退内置等宽族");
    }
    ctx.set_fonts(fonts);
}

// ===== src/window.rs 说明区 =====
// 输入：无（装配期读 token/配置）；输出：GUI 主窗进程
// 逻辑步骤：build_runtime → AppInterface::new → NativeOptions（token 几何+透明）
//   → run_native（create_app：theme/fonts/state 装配 → AppHandle）→ update 四步循环
// 设计理由：drain→背板→部件→节奏四步对齐 z.plan A 节主循环骨架；透明 viewport +
//   wgpu alpha 供 Acrylic 透出（.04 在 platform/acrylic.rs 挂接 DWM）；页面玻璃面
//   惰性注册 + 切页 set_visible（PL009 僵尸面教训平移：面随页注册，隐藏页剔除绘制）
// 已知偏移：①run() 内 Bus 装配的占位桥待 .02 联调收口（Bus::spawn 已在 bus.rs，
//   runtime 与 eframe 线程模型衔接在本文件完成）；②几何恢复（用户拖拽记忆）随
//   PL015；③gl_lift 经 pointer 直取（ui.json 无顶层 f64 访问器的三级键）
// 异常处理：管线失败降级纯色背板（行为无损纪律）；字体缺失降级内置（告警）
// 关联配置：window_*/glass.gl_lift/ui.json field.*；消费方：main.rs
