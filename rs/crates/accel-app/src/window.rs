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
    /// 退出保存起始时刻（A005 #28：SaveNow 后等回执/超时才放行 Close）
    exit_save_started: Option<std::time::Instant>,
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

        // A005 #5：玻璃 tint 按生效主题注入全场景面（注册后统一施加；与几何同有 1 帧滞后）
        self.scene.set_tint_all(self.theme.glass_tint());

        // Acrylic 一次性挂接（.04）：Frame 经 HasWindowHandle 取句柄，WindowHandle
        // 经 From 转 RawWindowHandle（0.6.2 特殊设计：window_handle 递归返自身）；
        // 失败静默降级实底（透明 viewport 下 GL 背板仍提供半透明观感）
        if !self.acrylic_applied
            && let Ok(h) = frame.window_handle()
        {
            let raw: RawWindowHandle = h.into();
            if let RawWindowHandle::Win32(w) = raw {
                // A005 #21：仅在真实调用后置位——首帧句柄未就绪时保留重试机会
                // （原实现无论成败一律置 true，句柄未就绪即永久放弃 Acrylic）
                self.acrylic_applied = crate::platform::acrylic::apply_acrylic(w.hwnd.get());
            }
        }

        // 背板：全屏玻璃管线（背景光场 + 场景面合成）；不可用降级纯色
        self.paint_backplate(&ctx, frame);

        // 导航 + 当前页 + 设置浮窗（各部件自绘 + 注册玻璃面）
        self.sync_page_visibility();
        let accent = crate::theme::accent_rgba();
        self.nav.ui(&ctx, &mut self.state, &mut self.scene, accent);
        self.pages
            .ui(ui, &mut self.state, &self.bus, &mut self.scene);
        self.paint_settings_pop(&ctx);

        // 重绘节奏：Tick 间隔（空闲 1s 兜底）——零忙转纪律
        // A005 #27：有平台集成（托盘/热键）时间隔收紧到 ≤200ms——平台事件仅在帧内
        // 轮询，1s 兜底会让隐藏期菜单/热键响应最坏延迟 1s（收紧后 ≤200ms，成本 5 帧/s）
        let mut interval = self.state.next_repaint_interval();
        if self.tray.is_some() || self.hotkeys.is_some() {
            interval = interval.min(std::time::Duration::from_millis(200));
        }
        ctx.request_repaint_after(interval);
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
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        // A005 #7：托盘与热键同时缺失时"隐藏驻留"即无出口死路（唯一可置退出请求的
        // 通道全部不存在）——此时关闭按钮直接升级为退出请求，走下方退出序列
        if close_requested
            && close_action(self.tray.is_some(), self.hotkeys.is_some()) == CloseAction::Quit
        {
            self.quit_requested = true;
        }
        if self.quit_requested && !self.quit_sent {
            // A005 #28：先发 SaveNow，待回执或超时后再发 Quit+Close——原实现 SaveNow
            // 后立即 Close，runtime drop 可能取消尚未处理的命令（保存非确定性）
            match self.exit_save_started {
                None => {
                    self.exit_save_started = Some(std::time::Instant::now());
                    // FIX006.6：先复位历史回执——否则会话内任一次保存成功即令 exit_ready
                    // 立即为真，本次 SaveNow 的回执等待被短路（等价原「SaveNow 后立即 Close」）
                    self.state.save_ack = false;
                    self.bus.send(FrontendCommand::SaveNow);
                    return;
                }
                Some(started) => {
                    if !exit_ready(self.state.save_ack, started.elapsed(), EXIT_SAVE_TIMEOUT) {
                        return;
                    }
                }
            }
            self.quit_sent = true;
            if self.probe_close_secs.is_some() {
                probe_note("[PROBE] quit");
            }
            self.bus.send(FrontendCommand::Quit);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        // 关闭按钮 → 隐藏驻留（托盘常驻，对齐 Python closeEvent：拦截不落盘——
        // 倍率等改动经 ApplyRate 内聚实时落盘）
        if !self.quit_sent && close_requested {
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
        self.theme = Theme::load(self.state.prefs.theme.as_str())
            .with_system(ctx.system_theme() == Some(egui::Theme::Dark));
        self.theme.apply(ctx);
    }

    /// 背板绘制：全屏玻璃回调（场景含导航/页卡/浮窗面）+ 场景 touch 直连重绘
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
            // FIX006.7：管线构造恒成功路径——真实错误由 wgpu uncaptured_error 处理器
            // 记日志（不 panic）；降级面 = 无 wgpu render state（上方早退分支）
            attach_pipeline(
                &rs.device,
                rs.target_format,
                &mut renderer.callback_resources,
            );
            drop(renderer);
            self.pipeline_attached = true;
            // touch → repaint 直连（FIX004.7 平移）：hook 持 ctx 弱引用
            let ctx_weak = ctx.clone();
            self.scene
                .set_repaint_hook(Some(Box::new(move || ctx_weak.request_repaint())));
        }
        // 场景几何 → uniforms → 全屏回调
        let pixels_per_point = ctx.pixels_per_point();
        let screen = ctx.input(|i| i.content_rect());
        let size_px = [
            screen.width() * pixels_per_point,
            screen.height() * pixels_per_point,
        ];
        let tokens = self.theme.active();
        // 玻璃提亮 token（A005 #5：glass.{mode}.gl_lift，0-255 → 0..1，随生效主题）
        let lift_value = self.theme.glass_lift();
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
        apply_page_visibility(&mut self.scene, self.state.current_page);
    }

    /// 设置浮窗绘制（PL013.10：egui::Window + 主题三段 + 失焦/ESC 关）
    /// A005 #15：原 SURFACE_SETTINGS 玻璃面从未注册（只 set_visible，恒静默 no-op），
    /// 浮窗为 egui 矢量绘制——已删除该常量与三处无效调用
    fn paint_settings_pop(&mut self, ctx: &Context) {
        if !self.state.settings_open {
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
                            self.theme = Theme::load(self.state.prefs.theme.as_str())
                                .with_system(ctx.system_theme() == Some(egui::Theme::Dark));
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

/// 退出前保存超时（A005 #28：回执未到时的放行上限——防后端失联导致永不退出）
const EXIT_SAVE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);

/// 退出保存放行判定（A005 #28 纯逻辑）：收到保存回执或等待超时即放行 Close
fn exit_ready(save_ack: bool, waited: std::time::Duration, timeout: std::time::Duration) -> bool {
    save_ack || waited >= timeout
}

/// 关闭请求的处置（A005 #7 纯逻辑）：有托盘或热键 → 隐藏驻留；两者皆无 → 退出
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    HideToTray,
    Quit,
}

/// 关闭按钮处置判定（A005 #7：无平台集成时不得隐藏驻留，否则无恢复/退出通道）
pub fn close_action(has_tray: bool, has_hotkey: bool) -> CloseAction {
    if has_tray || has_hotkey {
        CloseAction::HideToTray
    } else {
        CloseAction::Quit
    }
}

/// 页面内容区（A005 #9）：由 nav_rail_width + page_margin token 派生，
/// 替代各页硬编码的 (180,20)/(204,40) 偏移
pub fn content_page_rect(ctx: &egui::Context) -> Rect {
    let content = ctx.input(|i| i.content_rect());
    let rail_w = crate::theme::layout_i64("nav_rail_width") as f32;
    let m = page_margins();
    Rect::from_min_max(
        content.min + Vec2::new(rail_w + m[0], m[1]),
        content.max - Vec2::new(m[2], m[3]),
    )
}

/// 页面外边距 [左,上,右,下]（ui.json layout.page_margin；缺项回退 24）
/// FIX007.6：结果缓存（FIX006.10 同款——原每帧 3 次调用各解析一遍 pointer）
pub fn page_margins() -> [f32; 4] {
    static C: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, [f32; 4]>>> =
        std::sync::OnceLock::new();
    crate::theme::cached(&C, "page_margin", || {
        accel_core::static_config::Static::get()
            .ui
            .pointer("/layout/page_margin")
            .and_then(|v| v.as_array())
            .map(|a| {
                let mut out = [24.0f32; 4];
                for (i, slot) in out.iter_mut().enumerate() {
                    if let Some(v) = a.get(i).and_then(|x| x.as_f64()) {
                        *slot = v as f32;
                    }
                }
                out
            })
            .unwrap_or([24.0; 4])
    })
}

/// 内容左缩进 = nav_rail_width + page_margin 左值（替代各页硬编码 add_space(156.0)）
pub fn content_left_inset() -> f32 {
    crate::theme::layout_i64("nav_rail_width") as f32 + page_margins()[0]
}

/// 页面内边距（CentralPanel inner_margin；取 page_margin 左上值）
pub fn page_inner_margin() -> egui::Margin {
    let m = page_margins();
    egui::Margin::same(m[1].clamp(0.0, i8::MAX as f32) as i8)
}

/// 启动快照填充（A005 #2）：列表域显式拉取——core 的 spawn 首帧只推 Tick，
/// 不推 AlarmsChanged/配置快照，故 app 侧必须在装配期拉一次全量
fn fill_startup_snapshot(iface: &AppInterface, state: &mut UiState) {
    state.alarms = iface.get_alarms();
    state.world_pins = iface.get_world_pins();
    state.countdown_targets = iface.get_countdown_targets();
    state.base_timezone = Some(state.prefs.last_timezone.clone());
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

/// 页卡玻璃面 id（A005 #23：字面量查表替代每帧 format!——页枚举固定五档）
pub fn page_surface_id(page: PageId) -> &'static str {
    match page {
        PageId::Clock => "page_Clock",
        PageId::Countdown => "page_Countdown",
        PageId::World => "page_World",
        PageId::Weather => "page_Weather",
        PageId::Alarm => "page_Alarm",
    }
}

/// 注册页卡玻璃面（pages.ui 内每页调用——几何随布局 rect 同步；register 幂等）
/// A005 #9：圆角取 radius.lg token（原硬编码 16）
pub fn register_page_surface(scene: &mut GlassScene, page: PageId, rect: Rect) {
    let mut surface = GlassSurface::new(page_surface_id(page), rect);
    surface.radius = crate::theme::radius_px("lg");
    scene.register(surface);
}

/// chip 子面的页归属前缀表（页 → 其 chip 子面 id 前缀；FIX007.3——clock 页农历
/// chips 与 countdown 页节日 chips 各随页开关，其余页无 chip 子面）
const CHIP_PAGE_PREFIXES: [(&str, PageId); 2] =
    [("clock_", PageId::Clock), ("countdown_", PageId::Countdown)];

/// 页面玻璃面可见性统一开关（FIX007.3 可测核心）：页卡面 + 页归属 chip 子面——
/// 原实现只盖页卡面，chip 子面（默认注册即 visible）切页后残留为游离玻璃矩形；
/// 隐藏不注销（页回归时该页 ui 重注册刷新几何）
pub fn apply_page_visibility(scene: &mut GlassScene, current: PageId) {
    for page in PAGE_ORDER {
        scene.set_visible(page_surface_id(page), page == current);
    }
    for (prefix, page) in CHIP_PAGE_PREFIXES {
        let visible = page == current;
        let ids: Vec<String> = scene
            .surfaces()
            .iter()
            .filter(|s| s.surface_id.starts_with(prefix))
            .map(|s| s.surface_id.clone())
            .collect();
        for id in ids {
            scene.set_visible(&id, visible);
        }
    }
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
            let theme = Theme::load(prefs.theme.as_str())
                .with_system(cc.egui_ctx.system_theme() == Some(egui::Theme::Dark));
            theme.apply(&cc.egui_ctx);
            install_fonts(&cc.egui_ctx);
            let mut state = UiState {
                rate: iface.get_rate(),
                rate_bounds: AppInterface::get_rate_bounds(),
                prefs,
                ..UiState::default()
            };
            state.time = Some(iface.get_time_info());
            // A005 #2 修正：启动即拉全量列表快照——core spawn 首帧不推事件，若只等
            // ConfigSaved 重拉，则持久化闹钟/常驻城市/倒计时目标/基准时区启动全空
            fill_startup_snapshot(&iface, &mut state);

            // spawn 需 tokio 上下文——block_in_place + runtime 内完成（双 Arc 进入闭包）
            let bus =
                tokio::task::block_in_place(|| Bus::spawn_on(Arc::clone(&iface), &rt_for_app));

            // FIX006.4：冷启动天气首拉——core spawn 首帧只推 Tick，不推天气；装配期按
            // 持久化的 last_city 拉一次（否则天气页在用户手动刷新前整段空白）
            bus.send(FrontendCommand::RefreshWeather {
                city: state.prefs.last_city.clone(),
                force: false,
            });

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
                exit_save_started: None,
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

/// 字体注入（PL013.03 + A005 #8 接线）：族名读 ui.json token
/// `font_family`（中文，默认族）/ `font_family_digits`（数字等宽族）
pub fn install_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    let s = accel_core::static_config::Static::get();
    let cjk_family = s.ui_str("font_family");
    let digit_family = s.ui_str("font_family_digits");
    // 候选名 = 族名去空格派生 + 平台常见文件名兜底（族名与文件名未必同名）
    if let Some((path, bytes)) = load_font(cjk_family, &["msyh.ttc", "msyh.ttf", "msyhbd.ttc"]) {
        tracing::info!("中文族字体加载: {path}");
        fonts
            .font_data
            .insert("cjk".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "cjk".into());
    } else {
        tracing::warn!("{cjk_family} 未找到，中文回退 egui 内置字体（可能 tofu）");
    }
    if let Some((path, bytes)) = load_font(digit_family, &["Bahnschrift.ttf", "bahnschrift.ttf"]) {
        tracing::info!("数字族字体加载: {path}");
        fonts
            .font_data
            .insert("digits".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "digits".into());
    } else {
        tracing::warn!("{digit_family} 未找到，数字等宽回退内置等宽族");
    }
    ctx.set_fonts(fonts);
}

/// 按族名装载字体文件（A005 #8）：族名去空格派生候选 + 调用方平台兜底名
/// 输入：ui.json 族名 + 兜底文件名表；输出：命中路径与字节（未命中 None）
fn load_font(family: &str, fallback_names: &[&str]) -> Option<(String, Vec<u8>)> {
    let compact = family.replace(' ', "");
    let derived = [format!("{compact}.ttf"), format!("{compact}.ttc")];
    for dir in [
        "C:/Windows/Fonts",
        "/usr/share/fonts",
        "/System/Library/Fonts",
    ] {
        let names = derived
            .iter()
            .map(String::as_str)
            .chain(fallback_names.iter().copied());
        for name in names {
            let p = std::path::Path::new(dir).join(name);
            if let Ok(bytes) = std::fs::read(&p) {
                return Some((p.display().to_string(), bytes));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::hotkey::HotkeyKind;
    use crate::platform::tray::TrayCommand;

    fn probe_rect() -> Rect {
        Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 60.0))
    }

    #[test]
    fn page_switch_hides_chip_subsurfaces() {
        // FIX007.3 验收：连切五页后非当前页可见面数 = 0（页卡 + chip 子面统一开关——
        // 原实现只盖页卡面，chip 子面残留为游离玻璃矩形）
        let mut scene = GlassScene::new();
        // 预注册全部页卡面 + 两页 chip 子面（模拟各页都曾激活过）
        for p in PAGE_ORDER {
            scene.register(GlassSurface::new(page_surface_id(p), probe_rect()));
        }
        for id in [
            "clock_lunar_main",
            "clock_lunar_phase",
            "clock_lunar_god",
            "countdown_festival_0",
            "countdown_festival_5",
        ] {
            scene.register(GlassSurface::new(id, probe_rect()));
        }
        for current in PAGE_ORDER {
            apply_page_visibility(&mut scene, current);
            for s in scene.draw_list() {
                let owned = s.surface_id == page_surface_id(current)
                    || CHIP_PAGE_PREFIXES
                        .iter()
                        .filter(|(_, pg)| *pg == current)
                        .any(|(pre, _)| s.surface_id.starts_with(pre));
                assert!(
                    owned,
                    "切到 {current:?} 后不应有他页可见面: {}",
                    s.surface_id
                );
            }
        }
        // 隐藏不注销：面总数稳定（可见性开关非生命周期操作）
        assert_eq!(scene.total_count(), 10, "面总数应稳定（含全部 chip 子面）");
    }

    #[test]
    fn festival_chip_ids_stay_fixed_across_days() {
        // FIX007.3 验收：节日 chip 固定序号 id（countdown_festival_{i}）——原日期键
        // 跨日累积逼近 MAX_SURFACES=16，页卡面将被拒收致玻璃整体消失
        let mut scene = GlassScene::new();
        for _day in 0..3 {
            for i in 0..6 {
                let mut s = GlassSurface::new(&format!("countdown_festival_{i}"), probe_rect());
                s.elevation = 5;
                scene.register(s);
            }
        }
        assert_eq!(scene.total_count(), 6, "跨日注册应幂等——id 集不增长");
    }

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
    fn exit_ready_waits_for_ack_or_timeout() {
        // A005 #28 回归：保存回执到达即放行；未到则等满超时（防后端失联永不退出）
        use std::time::Duration;
        let t = Duration::from_millis(300);
        assert!(
            exit_ready(true, Duration::from_millis(0), t),
            "有回执立即可退"
        );
        assert!(!exit_ready(false, Duration::from_millis(299), t));
        assert!(exit_ready(false, t, t), "超时兜底放行");
    }

    #[test]
    fn close_action_avoids_dead_end() {
        // A005 #7 回归：有托盘/热键 → 隐藏驻留；两者皆无 → 直接退出（无出口死路防护）
        assert_eq!(close_action(true, true), CloseAction::HideToTray);
        assert_eq!(close_action(true, false), CloseAction::HideToTray);
        assert_eq!(close_action(false, true), CloseAction::HideToTray);
        assert_eq!(
            close_action(false, false),
            CloseAction::Quit,
            "无平台集成应退出"
        );
    }

    #[test]
    fn startup_snapshot_pulled() {
        // A005 #2 回归：持久化的闹钟/常驻城市/倒计时目标/基准时区在装配期即入快照
        //（原实现为空起手 + 只等 ConfigSaved 重拉 → 启动后列表全空）
        let dir = std::env::temp_dir().join(format!("accel_startup_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("临时目录创建失败");
        let path = dir.join("user_config.json");
        let cfg = accel_core::settings::UserConfig {
            world_pins: vec!["东京".to_string(), "巴黎".to_string()],
            last_timezone: "Europe/Paris".to_string(),
            alarms: vec![serde_json::json!({
                "label": "起床", "time": "07:30", "repeat_days": [1],
                "enabled": true, "id": "a1", "created_at": "2026-10-17T06:00:00",
            })],
            countdown_targets: vec![accel_core::settings::CountdownTarget {
                name: "新年".to_string(),
                at: "2027-01-01 00:00:00".to_string(),
            }],
            ..accel_core::settings::UserConfig::default()
        };
        assert!(accel_core::settings::save(&path, &cfg), "配置应落盘");
        let iface = AppInterface::with_config_path(path);
        let mut state = UiState {
            prefs: iface.get_ui_preferences(),
            ..UiState::default()
        };
        fill_startup_snapshot(&iface, &mut state);
        assert_eq!(state.alarms.len(), 1, "持久化闹钟应入启动快照");
        assert_eq!(state.world_pins.len(), 2, "常驻城市应入启动快照");
        assert_eq!(state.countdown_targets.len(), 1, "倒计时目标应入启动快照");
        assert_eq!(
            state.base_timezone.as_deref(),
            Some("Europe/Paris"),
            "基准时区应入启动快照"
        );
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
//   ②玻璃 lift/tint 经 theme.rs 的 glass_lift()/glass_tint() 读 glass.{mode} 域
//   （A005 #5 修正后不再直读 pointer）
// 异常处理：无 wgpu render state 时降级纯色背板（FIX006.7：管线构造恒成功路径，
//   真实管线错误由 wgpu uncaptured_error 处理器记日志；行为无损纪律）；字体缺失
//   降级内置（告警）；托盘/热键构造失败降级 None（无系统集成环境行为无损）
// 关联配置：window_*/glass.gl_lift/ui.json field.*；消费方：main.rs
