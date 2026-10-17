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
    /// 探针自动退窗秒数（None=正常模式；到时走退出序列——fps/矩阵探针）
    probe_close_secs: Option<u64>,
    /// 探针隐藏秒数（None=正常模式；到时 Visible(false)——模拟关闭按钮隐藏驻留）
    probe_hide_secs: Option<u64>,
    probe_hide_done: bool,
    /// 探针显示秒数（None=正常模式；到时 Visible(true)+Focus——模拟托盘"显示窗口"恢复）
    probe_show_secs: Option<u64>,
    probe_show_done: bool,
    /// 托盘句柄（构造失败降级 None——无托盘环境行为无损）
    tray: Option<crate::platform::tray::TrayHandles>,
    /// 全局热键句柄（注册失败降级 None）
    hotkeys: Option<crate::platform::hotkey::HotkeyHandles>,
    /// 退出请求（托盘退出/热键退出/探针到时）
    quit_requested: bool,
    /// 退出序列已发（SaveNow+Quit+Close 一次——放行 Close 不再被拦截）
    quit_sent: bool,
    /// 托盘状态推送节流（Tick 更新限频 1s）
    last_status: std::time::Instant,
    /// 帧计数（.13 空闲节奏探针）
    pub repaint_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl eframe::App for AppHandle {
    /// 每帧逻辑（先于 ui；窗口隐藏期也调用）——事件 drain 与视口/命令动作唯一入口
    /// 隐藏期不重绘但 logic 仍驱动：托盘恢复、退出序列、探针时序均须在此（.01 教训）
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.bus.drain(&mut self.state);
        // 托盘/热键事件轮询（命令可能触发 viewport 命令，经 ctx 队列）
        self.poll_platform(ctx);
        // 探针时序（隐藏/显示/退出）+ 退出序列/关闭驻留 + 托盘状态推送
        self.probe_tick(ctx);
        self.exit_or_stay(ctx);
        self.push_tray_status();
        self.repaint_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// 帧绘制：背板 → 导航 → 当前页 → 设置浮窗 → 重绘节奏
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

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

        // 重绘节奏：Tick 间隔（空闲 1s 兜底）——零忙转纪律
        ctx.request_repaint_after(self.state.next_repaint_interval());
    }
}

impl AppHandle {
    /// 探针时序（隐藏期仍由 logic 驱动）：隐藏 → 显示 → 退出三段模拟
    /// 隐藏/显示两侧打印同源状态指纹，供"隐藏→显示状态保持"断言比对
    fn probe_tick(&mut self, ctx: &Context) {
        if let Some(secs) = self.probe_hide_secs
            && !self.probe_hide_done
            && self.start_time.elapsed().as_secs() >= secs
        {
            self.probe_hide_done = true;
            probe_note(&format!("[PROBE] hidden {}", self.probe_fingerprint()));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if let Some(secs) = self.probe_show_secs
            && !self.probe_show_done
            && self.start_time.elapsed().as_secs() >= secs
        {
            self.probe_show_done = true;
            probe_note(&format!("[PROBE] shown {}", self.probe_fingerprint()));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if let Some(secs) = self.probe_close_secs
            && self.start_time.elapsed().as_secs() >= secs
        {
            self.quit_requested = true;
        }
    }

    /// 退出序列与关闭驻留（logic 每轮驱动——隐藏期同样生效）
    /// 退出（托盘/热键/探针）→ SaveNow+Quit+Close 一次放行；关闭按钮 → 隐藏驻留
    /// （quit_sent 区分两种 Close 来源：放行的退出 Close 不再被拦截）
    fn exit_or_stay(&mut self, ctx: &Context) {
        if self.quit_requested && !self.quit_sent {
            self.quit_sent = true;
            if self.probe_close_secs.is_some() {
                probe_note("[PROBE] quit");
            }
            self.bus.send(FrontendCommand::SaveNow);
            self.bus.send(FrontendCommand::Quit);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        // 关闭按钮 → 隐藏驻留（托盘常驻，对齐 Python closeEvent：拦截不落盘——
        // 倍率等改动经 ApplyRate 内聚实时落盘）
        if !self.quit_sent && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    /// 托盘状态推送（悬停文本 = 加速时间 + 倍率；节流 1s；隐藏期同样更新）
    fn push_tray_status(&mut self) {
        let Some(tray) = &self.tray else { return };
        if self.last_status.elapsed() < std::time::Duration::from_secs(1) {
            return;
        }
        self.last_status = std::time::Instant::now();
        let tooltip = match &self.state.time {
            Some(t) => format!("加速世界 - {} @ {:.1}x", t.custom_time, self.state.rate),
            None => format!(
                "加速世界 - {}",
                accel_core::static_config::Static::get().version()
            ),
        };
        tray.set_status(self.state.rate, &tooltip);
    }

    /// 探针状态指纹（隐藏/恢复两侧同源打印——"隐藏→显示状态保持"断言数据源）
    fn probe_fingerprint(&self) -> String {
        format!(
            "rate={:.1} page={:?} settings_open={}",
            self.state.rate, self.state.current_page, self.state.settings_open
        )
    }

    /// 托盘/热键事件轮询（logic 每帧调用；事件归一到 PlatformAction 后应用）
    fn poll_platform(&mut self, ctx: &Context) {
        let tray_cmds = self.tray.as_ref().map(|t| t.poll()).unwrap_or_default();
        let hotkey_cmds = self.hotkeys.as_ref().map(|h| h.poll()).unwrap_or_default();
        let mut actions: Vec<PlatformAction> =
            Vec::with_capacity(tray_cmds.len() + hotkey_cmds.len());
        actions.extend(tray_cmds.into_iter().map(tray_action));
        actions.extend(hotkey_cmds.into_iter().map(hotkey_action));
        for action in actions {
            self.apply_platform_action(ctx, action);
        }
    }

    /// 动作应用（托盘/热键/探针的窗口与命令出口唯一汇合点）
    fn apply_platform_action(&mut self, ctx: &Context, action: PlatformAction) {
        match action {
            PlatformAction::ShowWindow => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            PlatformAction::HideWindow => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            PlatformAction::SaveNow => self.bus.send(FrontendCommand::SaveNow),
            PlatformAction::Quit => self.quit_requested = true,
            PlatformAction::CycleTheme => self.cycle_theme(ctx),
        }
    }

    /// 主题三态循环（热键 Ctrl+T；跟随系统→浅色→深色→跟随系统）
    /// 与设置浮窗选择器同一乐观更新路径（SetTheme 回推前 UI 即响应）
    fn cycle_theme(&mut self, ctx: &Context) {
        let next = match self.state.theme_pref() {
            accel_core::types::ThemePref::Auto => accel_core::types::ThemePref::Light,
            accel_core::types::ThemePref::Light => accel_core::types::ThemePref::Dark,
            accel_core::types::ThemePref::Dark => accel_core::types::ThemePref::Auto,
        };
        self.bus.send(FrontendCommand::SetTheme(next));
        self.state.prefs.theme = pref_value(next).to_string();
        self.theme = Theme::load(self.state.prefs.theme.as_str());
        self.theme.apply(ctx);
    }

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

/// 探针标记：stdout（人读）+ tracing 文件日志（脚本断言用，避开 stdout 管道缓冲）
fn probe_note(msg: &str) {
    println!("{msg}");
    tracing::info!("{msg}");
}

/// 平台事件归一后的动作（纯数据；单测可断言 tray/hotkey → 动作映射）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformAction {
    ShowWindow,
    HideWindow,
    SaveNow,
    Quit,
    CycleTheme,
}

/// 托盘命令 → 动作（纯函数；只读项/未知 id 已在 tray::command_for 返 None）
fn tray_action(cmd: crate::platform::tray::TrayCommand) -> PlatformAction {
    match cmd {
        crate::platform::tray::TrayCommand::Show => PlatformAction::ShowWindow,
        crate::platform::tray::TrayCommand::Hide => PlatformAction::HideWindow,
        crate::platform::tray::TrayCommand::Quit => PlatformAction::Quit,
    }
}

/// 热键动作 → 动作（纯函数；保存/退出/主题循环三键一一对应）
fn hotkey_action(kind: crate::platform::hotkey::HotkeyKind) -> PlatformAction {
    match kind {
        crate::platform::hotkey::HotkeyKind::Save => PlatformAction::SaveNow,
        crate::platform::hotkey::HotkeyKind::Quit => PlatformAction::Quit,
        crate::platform::hotkey::HotkeyKind::Theme => PlatformAction::CycleTheme,
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
    // 探针隐藏/显示秒数（ACCELWORLD_HIDE_AFTER / ACCELWORLD_SHOW_AFTER）：
    // 冒烟/矩阵脚本模拟"关闭按钮隐藏驻留 → 托盘显示恢复"全链路
    let probe_hide_secs = std::env::var("ACCELWORLD_HIDE_AFTER")
        .ok()
        .and_then(|v| v.parse().ok());
    let probe_show_secs = std::env::var("ACCELWORLD_SHOW_AFTER")
        .ok()
        .and_then(|v| v.parse().ok());
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

            // 平台集成（PL015.01/.02）：托盘 + 全局热键（主线程创建，winit 泵消息；
            // 失败降级 None 继续运行——无托盘/热键环境行为无损）
            let tray = match crate::platform::tray::TrayHandles::new() {
                Ok(t) => Some(t),
                Err(e) => {
                    tracing::warn!("托盘初始化失败，降级无托盘运行: {e}");
                    None
                }
            };
            let hotkeys = match crate::platform::hotkey::HotkeyHandles::new() {
                Ok(h) => Some(h),
                Err(e) => {
                    tracing::warn!("全局热键注册失败，降级无热键运行: {e}");
                    None
                }
            };

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
                probe_hide_secs,
                probe_hide_done: false,
                probe_show_secs,
                probe_show_done: false,
                tray,
                hotkeys,
                quit_requested: false,
                quit_sent: false,
                last_status: std::time::Instant::now(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::hotkey::HotkeyKind;
    use crate::platform::tray::TrayCommand;

    #[test]
    fn tray_command_maps_to_platform_action() {
        // 菜单事件 → 窗口意图：显示/隐藏/退出三项各自归位（.01 检验项）
        assert_eq!(tray_action(TrayCommand::Show), PlatformAction::ShowWindow);
        assert_eq!(tray_action(TrayCommand::Hide), PlatformAction::HideWindow);
        assert_eq!(tray_action(TrayCommand::Quit), PlatformAction::Quit);
    }

    #[test]
    fn hotkey_maps_to_platform_action() {
        // 热键 → 命令意图：保存/退出/主题循环三键一一对应（.02 检验项）
        assert_eq!(hotkey_action(HotkeyKind::Save), PlatformAction::SaveNow);
        assert_eq!(hotkey_action(HotkeyKind::Quit), PlatformAction::Quit);
        assert_eq!(hotkey_action(HotkeyKind::Theme), PlatformAction::CycleTheme);
    }

    #[test]
    fn hide_show_are_pure_viewport_actions() {
        // 隐藏→显示状态保持：两动作必为纯视口意图（非跨界命令），窗口显示性变化
        // 不触碰 UiState 任何字段——状态快照前后一致
        let state = UiState::default();
        let fingerprint = format!("{state:?}");
        for cmd in [TrayCommand::Hide, TrayCommand::Show] {
            let action = tray_action(cmd);
            assert!(
                matches!(
                    action,
                    PlatformAction::HideWindow | PlatformAction::ShowWindow
                ),
                "隐藏/显示动作必须为纯视口意图，实际 {action:?}"
            );
        }
        assert_eq!(fingerprint, format!("{state:?}"));
    }
}

// ===== src/window.rs 说明区 =====
// 输入：无（装配期读 token/配置）；输出：GUI 主窗进程
// 逻辑步骤：build_runtime → AppInterface::new → NativeOptions（token 几何+透明）
//   → run_native（create_app：theme/fonts/state 装配 + 托盘/热键挂接 → AppHandle）
//   → logic（drain → 平台事件 → 探针时序 → 退出/驻留 → 托盘状态）→ ui（背板→部件→节奏）
// 设计理由：drain→背板→部件→节奏四步对齐 z.plan A 节主循环骨架；透明 viewport +
//   wgpu alpha 供 Acrylic 透出（.04 在 platform/acrylic.rs 挂接 DWM）；页面玻璃面
//   惰性注册 + 切页 set_visible（PL009 僵尸面教训平移：面随页注册，隐藏页剔除绘制）；
//   **隐藏期不重绘但 logic 仍逐轮驱动**——视口命令（托盘恢复/退出/探针）与托盘状态
//   推送一律落在 logic，落在 ui 会在隐藏驻留后失效（.01 真窗实证的返工教训）
// 退出语义（PL015.01 定案，对齐 Python closeEvent）：关闭按钮 → CancelClose+隐藏
//   驻留（不落盘——倍率经 ApplyRate 内聚实时落盘）；托盘退出/热键退出/探针 →
//   quit_requested → SaveNow+Quit+Close 一次放行（quit_sent 区分两种 Close 来源）
// 平台事件归一（PL015.01/.02）：tray::TrayCommand 与 hotkey::HotkeyKind 经纯函数
//   tray_action/hotkey_action 归一到 PlatformAction，再由 apply_platform_action 汇合
//   应用（窗口/命令出口唯一入口——映射可单测，无须真实托盘/热键）
// 探针通路：ACCELWORLD_HIDE_AFTER / ACCELWORLD_SHOW_AFTER / ACCELWORLD_FPS_PROBE
//   模拟"隐藏驻留 → 显示恢复 → 退出"全链路；隐藏与显示两侧打印同源状态指纹，
//   供"隐藏→显示状态保持"断言比对
// 已知偏移：①几何恢复（用户拖拽记忆）未做（PL015 无对应任务，随 UI 雕刻阶段处理）；
//   ②gl_lift 经 pointer 直取（ui.json 无顶层 f64 访问器的三级键）
// 异常处理：管线失败降级纯色背板（行为无损纪律）；字体缺失降级内置（告警）；
//   托盘/热键构造失败降级 None（无系统集成环境行为无损）
// 关联配置：window_*/glass.gl_lift/ui.json field.*；消费方：main.rs
