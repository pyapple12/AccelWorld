//! 主窗装配器（ui/main_window.py 的 Rust 对等物；PL017 实心桌面装配）
//! AppHandle + eframe::App 实现：logic（窗口夹取/探针/退出）+ ui（悬浮仪器坞→页总装
//! （抽屉+主仪）→toast）（PL017：设置改为抽屉页，标题去版本，窗口用托盘图标）
//! 玻璃管线/Acrylic/透明窗口整体废弃——标准不透明桌面应用（UIR 定案）

use std::sync::Arc;

use accel_core::interface::AppInterface;
use accel_core::types::FrontendCommand;
use egui::{Context, Pos2, Vec2};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::bus::{Bus, PAGE_ORDER, PageId, UiState};
use crate::pages::Pages;

/// 窗口几何 token 基准（base.json；恢复优先用户存档几何，缺档时用本基准）
fn default_window_rect() -> (f32, f32, f32, f32) {
    let base = accel_core::static_config::Static::get();
    (
        base.base_f64("window_x") as f32,
        base.base_f64("window_y") as f32,
        base.base_f64("window_width") as f32,
        base.base_f64("window_height") as f32,
    )
}

/// 启动几何恢复钳制（纯函数，A009 #6）：存档几何优先，宽高不足最小尺寸时钳到
/// 下限——实证 winit 对 with_inner_size 创建期**不**施加 with_min_inner_size
//（旧 100×100 存档原样创建窄窗，.temp tiny-geo 探针 125px 实证）；坐标原样保留
fn clamped_restore_geometry(
    saved: Option<(f32, f32, f32, f32)>,
    default: (f32, f32, f32, f32),
    min: (f32, f32),
) -> (f32, f32, f32, f32) {
    let (min_w, min_h) = min;
    match saved {
        Some((x, y, w, h)) => (x, y, w.max(min_w), h.max(min_h)),
        None => default,
    }
}

/// 应用句柄（eframe::App 实现）
pub struct AppHandle {
    pub bus: Bus,
    pub state: UiState,
    pub pages: Pages,
    start_time: std::time::Instant,
    /// 探针自动退窗秒数（None=正常模式；到时走退出序列——fps/矩阵探针）
    probe_close_secs: Option<u64>,
    /// 探针隐藏秒数（None=正常模式；到时 Visible(false)——模拟关闭按钮隐藏驻留）
    probe_hide_secs: Option<u64>,
    probe_hide_done: bool,
    /// 探针显示秒数（None=正常模式；到时 Visible(true)+Focus——模拟托盘"显示窗口"恢复）
    probe_show_secs: Option<u64>,
    probe_show_done: bool,
    /// UI 截图探针输出目录（None=正常模式；UIR.2 开发期自检——逐页 PrintWindow 出 PNG）
    ui_shot_dir: Option<std::path::PathBuf>,
    ui_shot_step: usize,
    ui_shot_last: std::time::Instant,
    /// PL016.17 两段式：切页时刻——结算窗内每帧 16ms 提频，窗满才允许出图
    ui_shot_switched: Option<std::time::Instant>,
    /// 标题栏句柄缓存（首帧取到后存档——每帧 DWM 钉色用）
    titlebar_hwnd: Option<isize>,
    /// 启动几何（内尺寸；首帧按当前显示器夹取一次——PL017.01）
    initial_geometry: (f32, f32, f32, f32),
    geometry_clamped: bool,
    /// PL018.01 探针卫生：退出跳过窗口几何存档（避免循环探针覆盖用户存档）
    test_no_save: bool,
    /// PL018.01 期望落点（外框逻辑点）；静置后打印 target/actual/delta
    probe_expect_pos: Option<(f32, f32)>,
    probe_pos_logged: bool,
    /// PL018.01 首帧早期截图毫秒（与 UI_SHOT 目录合用；抓完即退出）
    early_shot_ms: Option<u64>,
    early_shot_done: bool,
    /// PL018.06 首帧门控：隐藏创建，完成 1 帧（或 250ms 看门狗）后再显示
    first_show_done: bool,
    frames_seen: u32,
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
    /// PL018.04 退出自愈：Close 发出时点与次数（None=未发）
    close_sent_at: Option<std::time::Instant>,
    close_attempt: u8,
    /// PL018.04 测试钩子：丢弃前 N 次 Close，验证重发/兜底路径
    test_close_loss: u8,
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

    /// 帧绘制：悬浮仪器坞 → 页总装 → toast → 重绘节奏（PL017 装配）
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // PL018.06 首帧门控：隐藏创建；完成 1 帧（首帧已 present）或 250ms 兜底后再显示。
        // 看门狗线程负责“隐藏窗口不派发首帧”的死角（见 run_with_counter）。
        if !self.first_show_done
            && (self.frames_seen >= 1
                || self.start_time.elapsed() >= std::time::Duration::from_millis(250))
        {
            self.first_show_done = true;
            tracing::info!(
                "首帧门控显示: frames={} elapsed={:?}",
                self.frames_seen,
                self.start_time.elapsed()
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        // PL017.01：首帧按当前显示器工作区夹取窗口（存档可能来自更大屏幕/旧 DPI）
        self.clamp_window_once(&ctx);
        // PL016.06：减少动效开关写入 ctx data（控件层经 theme::reduce_motion 读取）
        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::unique("accel.reduce_motion"),
                self.state.reduce_motion,
            )
        });

        // 标题栏底色钉制（首帧缓存句柄；每帧幂等——DWM 属性写入廉价，
        // 随主题循环自动换色）
        if self.titlebar_hwnd.is_none()
            && let Ok(h) = frame.window_handle()
            && let RawWindowHandle::Win32(w) = h.into()
        {
            self.titlebar_hwnd = Some(w.hwnd.get());
        }
        if let Some(hwnd) = self.titlebar_hwnd {
            apply_caption_color(hwnd, crate::theme::sidebar_bg());
        }

        // 底部仪器坞（PL017）：居中浮动 pill + 五页标签 + 设置齿轮
        let dock_items = [
            crate::widgets::dock::DockItem {
                page: PageId::Clock,
                icon: sidebar_icon(PageId::Clock),
                label: sidebar_label(PageId::Clock),
            },
            crate::widgets::dock::DockItem {
                page: PageId::Countdown,
                icon: sidebar_icon(PageId::Countdown),
                label: sidebar_label(PageId::Countdown),
            },
            crate::widgets::dock::DockItem {
                page: PageId::World,
                icon: sidebar_icon(PageId::World),
                label: sidebar_label(PageId::World),
            },
            crate::widgets::dock::DockItem {
                page: PageId::Weather,
                icon: sidebar_icon(PageId::Weather),
                label: sidebar_label(PageId::Weather),
            },
            crate::widgets::dock::DockItem {
                page: PageId::Alarm,
                icon: sidebar_icon(PageId::Alarm),
                label: sidebar_label(PageId::Alarm),
            },
        ];
        egui::Panel::bottom("instrument_dock")
            .exact_size(crate::theme::layout_i64("dock_height") as f32)
            .frame(egui::Frame::new().fill(crate::theme::window_bg()))
            .show(ui, |ui| {
                crate::widgets::dock::show(ui, &mut self.state, &dock_items);
            });

        // 页面总装（星盘主仪恒驻 + 非时钟页右侧仪器抽屉）
        self.pages.ui(ui, &mut self.state, &self.bus);

        // 应用内 toast（PL016.07：右上堆叠；存活期自提频，零忙转不破）
        crate::widgets::toast::paint(&ctx, &mut self.state.toasts);

        // PL018.01：落点日志 + 首帧早期截图（仅探针环境；一次性）
        self.probe_extras_tick(&ctx, frame);

        // UI 截图探针（UIR.2：逐页 PrintWindow 出 PNG 后自动退出）
        self.ui_shot_tick(ui.ctx(), frame);

        // 重绘节奏：Tick 间隔（空闲 1s 兜底）——零忙转纪律
        // A005 #27：有平台集成（托盘/热键）时间隔收紧到 ≤200ms——平台事件仅在帧内
        // 轮询，1s 兜底会让隐藏期菜单/热键响应最坏延迟 1s（收紧后 ≤200ms，成本 5 帧/s）
        let mut interval = self.state.next_repaint_interval();
        if self.tray.is_some() || self.hotkeys.is_some() {
            interval = interval.min(std::time::Duration::from_millis(200));
        }
        ctx.request_repaint_after(interval);
        self.frames_seen = self.frames_seen.saturating_add(1);
        // PL018.06：首帧完成后立刻再跑一帧，把“显示”压到 ~1 帧内（不等 tick 间隔）
        if !self.first_show_done {
            ctx.request_repaint();
        }
    }
}

/// 页 → 导航图标（纯函数可测）
fn sidebar_icon(page: PageId) -> crate::widgets::icons::Icon {
    match page {
        PageId::Clock => crate::widgets::icons::Icon::Clock,
        PageId::Countdown => crate::widgets::icons::Icon::Hourglass,
        PageId::World => crate::widgets::icons::Icon::Globe,
        PageId::Weather => crate::widgets::icons::Icon::Cloud,
        PageId::Alarm => crate::widgets::icons::Icon::Bell,
    }
}

/// 页 → 导航文字（纯函数可测）
fn sidebar_label(page: PageId) -> &'static str {
    match page {
        PageId::Clock => "时钟",
        PageId::Countdown => "倒计时",
        PageId::World => "世界",
        PageId::Weather => "天气",
        PageId::Alarm => "闹钟",
    }
}

impl AppHandle {
    /// 首帧窗口几何夹取（PL017.01）：显示器逻辑尺寸 − 安全边距，且不超过
    /// ui.json window_max_*；位置夹到可见区。只做一次，之后尊重用户手动调整。
    fn clamp_window_once(&mut self, ctx: &Context) {
        if self.geometry_clamped {
            return;
        }
        let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
            return;
        };
        let max = crate::theme::window_max_size();
        let (sx, sy, sw, sh) = self.initial_geometry;
        let (x, y, w, h) =
            clamp_geometry_to_monitor(self.initial_geometry, monitor, (520.0, 420.0), max);
        if (w - sw).abs() > 1.0 || (h - sh).abs() > 1.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
        }
        if (x - sx).abs() > 1.0 || (y - sy).abs() > 1.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
        }
        self.geometry_clamped = true;
        tracing::info!(
            "窗口几何夹取: {sw:.0}x{sh:.0}@{sx:.0},{sy:.0} → {w:.0}x{h:.0}@{x:.0},{y:.0}"
        );
    }

    /// PL018.01 探针附加项（一次性）：
    /// ①落点日志——静置 600ms 后打印 target/actual/delta（外框逻辑点），供 #10 基线；
    /// ②首帧早期截图——到点抓一帧落盘后立刻走退出序列（不进入六页矩阵）。
    /// 正常模式两开关均未设时不执行任何工作。
    fn probe_extras_tick(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if let Some((tx, ty)) = self.probe_expect_pos
            && !self.probe_pos_logged
            && self.start_time.elapsed() >= std::time::Duration::from_millis(600)
        {
            self.probe_pos_logged = true;
            match ctx.input(|i| i.viewport().outer_rect) {
                Some(r) => probe_note(&format!(
                    "[PROBE] pos target={tx:.0},{ty:.0} actual={:.0},{:.0} delta={:.0},{:.0} ppp={:.2}",
                    r.min.x,
                    r.min.y,
                    r.min.x - tx,
                    r.min.y - ty,
                    ctx.pixels_per_point()
                )),
                None => probe_note("[PROBE] pos actual=unknown"),
            }
        }
        if let Some(ms) = self.early_shot_ms
            && !self.early_shot_done
            && self.start_time.elapsed() >= std::time::Duration::from_millis(ms)
        {
            self.early_shot_done = true;
            let path = self
                .ui_shot_dir
                .clone()
                .unwrap_or_default()
                .join(format!("early_{ms}.png"));
            let hwnd = match frame.window_handle() {
                Ok(h) => match h.into() {
                    RawWindowHandle::Win32(w) => Some(w.hwnd.get()),
                    _ => None,
                },
                Err(_) => None,
            };
            let ok = hwnd.is_some_and(|h| capture_window_png(h, &path));
            probe_note(&format!(
                "[PROBE] early {ms}ms → {}",
                if ok { "ok" } else { "FAIL" }
            ));
            self.quit_requested = true;
        }
    }
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
        // PL018.04：Close 已发但窗口仍在 → 按 1s/2s/3s 升级（重发→显示后重发→硬退出）
        if self.quit_sent {
            self.escalate_close(ctx);
            return;
        }
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
                    // UIR.7：退出前存档窗口几何（外框矩形 "x,y,w,h"；无外框信息跳过）
                    // FIX011.19：最小化态跳过——Windows 最小化窗口 OS 矩形惯例
                    // (-32000,-32000)，存档将被 parse_window_geometry 越域拒绝
                    // （走「保存失败」误弹链且几何未被存档）
                    let minimized = ctx.input(|i| i.viewport().minimized) == Some(true);
                    let inner = ctx.input(|i| i.viewport().inner_rect);
                    if !minimized
                        && !self.test_no_save
                        && let Some(outer) = ctx.input(|i| i.viewport().outer_rect)
                    {
                        // PL017.01：保存 inner 尺寸（含标题栏的 outer 会把窗口逐次撑大）
                        let (gw, gh) = inner
                            .map(|r| (r.width(), r.height()))
                            .unwrap_or((outer.width(), outer.height()));
                        self.bus.send(FrontendCommand::SetWindowGeometry {
                            geometry: format!(
                                "{:.0},{:.0},{:.0},{:.0}",
                                outer.min.x, outer.min.y, gw, gh
                            ),
                        });
                    }
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
            self.close_sent_at = Some(std::time::Instant::now());
            self.close_attempt = 1;
            if self.probe_close_secs.is_some() {
                probe_note("[PROBE] quit");
            }
            self.bus.send(FrontendCommand::Quit);
            self.send_close(ctx);
            return;
        }
        // 关闭按钮 → 隐藏驻留（托盘常驻，对齐 Python closeEvent：拦截不落盘——
        // 倍率等改动经 ApplyRate 内聚实时落盘）
        if !self.quit_sent && close_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    /// PL018.04 发送 Close（测试钩子可丢弃前 N 次，验证重发/兜底路径）
    fn send_close(&mut self, ctx: &Context) {
        if self.test_close_loss > 0 {
            self.test_close_loss -= 1;
            tracing::warn!("[PROBE] 丢弃一次 Close（剩余 {}）", self.test_close_loss);
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// PL018.04 退出自愈：Close 后窗口仍存活时按 1s/2s/3s 升级——
    /// 重发 → 显示并重发（覆盖隐藏态吞掉 Close）→ 硬退出（SaveNow 回执已到，数据安全）
    fn escalate_close(&mut self, ctx: &Context) {
        let Some(sent) = self.close_sent_at else {
            return;
        };
        // 退出期保活：确保看门狗在窗口不再重绘时仍有帧驱动
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
        match close_step(sent.elapsed(), self.close_attempt) {
            CloseStep::None => {}
            CloseStep::Resend => {
                self.close_attempt += 1;
                tracing::warn!(
                    "退出升级: attempt={} elapsed={:?}",
                    self.close_attempt,
                    sent.elapsed()
                );
                self.send_close(ctx);
            }
            CloseStep::RevealAndResend => {
                self.close_attempt += 1;
                tracing::warn!(
                    "退出升级: attempt={} elapsed={:?}（先显示窗口）",
                    self.close_attempt,
                    sent.elapsed()
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.send_close(ctx);
            }
            CloseStep::ForceExit => {
                tracing::warn!("退出兜底: Close 三次重发仍未生效，强制退出（数据已保存）");
                std::process::exit(0);
            }
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

    /// 探针节拍（PL016.17，开发基建参数非业务 token）：出图最小间隔 / 切页后
    /// 结算窗（≥ 新页 present + 抽屉弹簧走完）/ 结算窗内提频周期
    const UI_SHOT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(1200);
    const UI_SHOT_SETTLE: std::time::Duration = std::time::Duration::from_millis(1600);
    const UI_SHOT_FRAME: std::time::Duration = std::time::Duration::from_millis(16);

    /// UI 截图探针（UIR.2 开发期自检）：两段式逐页 PrintWindow 出 PNG 后自动退出——
    /// step 0-4 五页（每拍完切下一页并起结算窗，step 4 拍完开设置页）、step 5 拍
    /// 设置抽屉（拍完关闭）、step 6 触发退出（不再出图——防覆盖设置帧）。
    /// PL016.17：PrintWindow 在 update 内读到的是上一帧 present，低帧率下切页后
    /// 立即采会拍切换前旧帧（复审实证 00≡01/02≡03 哈希对、04/05 名实错位）——
    /// 切页先进结算窗（16ms 提频驱动新页 present 与抽屉弹簧走完）再采。
    /// 观感终审权仍在用户真机目测；本探针只服务开发迭代（视觉设计不得向测试环境妥协）
    fn ui_shot_tick(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let Some(dir) = self.ui_shot_dir.clone() else {
            return;
        };
        // PL018.01：早期截图模式独占本次运行（抓完即退，不跑六页矩阵）
        if self.early_shot_ms.is_some() {
            return;
        }
        // step len+1 = 退出步（不出图）；其后守卫拦空转
        if self.ui_shot_step > PAGE_ORDER.len() + 1 {
            return;
        }
        // 结算窗：切页后 SETTLE 内每帧 16ms 提频（新页 present + 弹簧走完）
        if let Some(at) = self.ui_shot_switched
            && at.elapsed() < Self::UI_SHOT_SETTLE
        {
            ctx.request_repaint_after(Self::UI_SHOT_FRAME);
            return;
        }
        self.ui_shot_switched = None;
        if self.ui_shot_last.elapsed() < Self::UI_SHOT_INTERVAL {
            return;
        }
        self.ui_shot_last = std::time::Instant::now();
        let step = self.ui_shot_step;

        // 退出步（step 6）：不出图，直接走退出序列
        if step > PAGE_ORDER.len() {
            self.quit_requested = true;
            self.ui_shot_step = step + 1;
            return;
        }

        // 本帧状态出图（tick 在 ui() 末尾——拍到的是当前已绘制状态）
        let name = if step < PAGE_ORDER.len() {
            format!("{:02}_{}", step, page_shot_name(PAGE_ORDER[step]))
        } else {
            "05_settings".to_string()
        };
        let hwnd = match frame.window_handle() {
            Ok(h) => match h.into() {
                RawWindowHandle::Win32(w) => w.hwnd.get(),
                _ => {
                    probe_note("[UI_SHOT] 非 Win32 句柄，探针跳过");
                    self.quit_requested = true;
                    return;
                }
            },
            Err(e) => {
                probe_note(&format!("[UI_SHOT] 句柄获取失败: {e}"));
                self.quit_requested = true;
                return;
            }
        };
        let path = dir.join(format!("ui_{name}.png"));
        let ok = capture_window_png(hwnd, &path);
        probe_note(&format!(
            "[UI_SHOT] {name} → {}",
            if ok { "ok" } else { "FAIL" }
        ));

        // 状态推进：拍完当前页切下一页并起结算窗；五页后开设置页；设置帧拍完关闭
        if step < PAGE_ORDER.len() - 1 {
            self.state.current_page = PAGE_ORDER[step + 1];
        } else {
            self.state.settings_open = step == PAGE_ORDER.len() - 1;
        }
        self.ui_shot_switched = Some(std::time::Instant::now());
        self.ui_shot_step = step + 1;
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
        }
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

/// 退出自愈步进（PL018.04 纯逻辑）：Close 发出后窗口仍在时的升级动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseStep {
    /// 仍在等待窗口关闭
    None,
    /// 重发 Close（命令可能被事件循环丢弃）
    Resend,
    /// 显示窗口后重发（隐藏态可能吞掉 Close）
    RevealAndResend,
    /// 三次重发仍无效 → 硬退出（SaveNow 回执已到，数据安全）
    ForceExit,
}

/// 退出自愈判定（PL018.04）：按「第几次发送 + 已等时长」逐级升级，早到不越级
pub fn close_step(elapsed: std::time::Duration, attempt: u8) -> CloseStep {
    match attempt {
        1 if elapsed >= std::time::Duration::from_secs(1) => CloseStep::Resend,
        2 if elapsed >= std::time::Duration::from_secs(2) => CloseStep::RevealAndResend,
        3 if elapsed >= std::time::Duration::from_secs(3) => CloseStep::ForceExit,
        _ => CloseStep::None,
    }
}

/// PL018.01 解析落点探针参数 "x,y"（容忍空白；非法返回 None——探针不得 panic）
pub fn parse_expect_pos(raw: &str) -> Option<(f32, f32)> {
    let (x, y) = raw.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// 启动几何按显示器夹取（纯函数，PL017.01）：尺寸不超过显示器−边距与 max，
/// 位置夹到可见区；返回 (x,y,w,h)。显示器未知时调用方跳过。
pub fn clamp_geometry_to_monitor(
    saved: (f32, f32, f32, f32),
    monitor: Vec2,
    min: (f32, f32),
    max: Vec2,
) -> (f32, f32, f32, f32) {
    let (sx, sy, sw, sh) = saved;
    let w = sw.min((monitor.x - 48.0).max(min.0)).min(max.x).max(min.0);
    let h = sh.min((monitor.y - 110.0).max(min.1)).min(max.y).max(min.1);
    let x = sx.clamp(24.0, (monitor.x - w - 24.0).max(24.0));
    let y = sy.clamp(24.0, (monitor.y - h - 48.0).max(24.0));
    (x, y, w, h)
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
/// A008 #2：CycleTheme 已删（仅深色定案——热键与浮窗主题切换面整体退役）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformAction {
    ShowWindow,
    HideWindow,
    SaveNow,
    Quit,
}

/// 托盘命令 → 动作（纯函数；只读项/未知 id 已在 tray::command_for 返 None）
fn tray_action(cmd: crate::platform::tray::TrayCommand) -> PlatformAction {
    match cmd {
        crate::platform::tray::TrayCommand::Show => PlatformAction::ShowWindow,
        crate::platform::tray::TrayCommand::Hide => PlatformAction::HideWindow,
        crate::platform::tray::TrayCommand::Quit => PlatformAction::Quit,
    }
}

/// 热键动作 → 动作（纯函数；保存/退出两键一一对应——A008 #2 主题键退役）
fn hotkey_action(kind: crate::platform::hotkey::HotkeyKind) -> PlatformAction {
    match kind {
        crate::platform::hotkey::HotkeyKind::Save => PlatformAction::SaveNow,
        crate::platform::hotkey::HotkeyKind::Quit => PlatformAction::Quit,
    }
}

/// 截图探针文件名段（ASCII 稳妥——文件系统/脚本消费）
fn page_shot_name(page: PageId) -> &'static str {
    match page {
        PageId::Clock => "clock",
        PageId::Countdown => "countdown",
        PageId::World => "world",
        PageId::Weather => "weather",
        PageId::Alarm => "alarm",
    }
}

/// 真窗截图（UIR.2）：PrintWindow(PW_RENDERFULLCONTENT) 采 DWM 合成内容 →
/// GetDIBits 读 BGRA → 转 RGBA → PNG 落盘；任何环节失败返 false 不 panic
#[cfg(windows)]
fn capture_window_png(hwnd: isize, path: &std::path::Path) -> bool {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject,
    };
    // windows-rs 分类 quirks：PrintWindow 归属 Storage::Xps（UIR.2 实测）
    use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    unsafe {
        let hwnd = HWND(hwnd as *mut _);
        let mut rc = RECT::default();
        if GetClientRect(hwnd, &mut rc).is_err() {
            return false;
        }
        let w = (rc.right - rc.left).max(1);
        let h = (rc.bottom - rc.top).max(1);
        let hdc_win = GetDC(Some(hwnd));
        let hdc_mem = CreateCompatibleDC(Some(hdc_win));
        let hbmp = CreateCompatibleBitmap(hdc_win, w, h);
        let old = SelectObject(hdc_mem, hbmp.into());
        // PW_CLIENTONLY(0x1) | PW_RENDERFULLCONTENT(0x2)：只采客户区并取 DWM 合成后内容
        // （PL017.20：原仅 0x2 会把整窗画进客户区尺寸位图，底部 dock 被裁成白带）
        let printed = PrintWindow(hwnd, hdc_mem, PRINT_WINDOW_FLAGS(0x3));
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // top-down 行序
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = DIB_RGB_COLORS.0;
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let got = GetDIBits(
            hdc_mem,
            hbmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(Some(hwnd), hdc_win);
        if !printed.as_bool() || got == 0 {
            return false;
        }
        // BGRA → RGBA（GDI 行序；as_chunks 固定 4 通道步进，clippy 口径）
        for px in buf.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        image::save_buffer(path, &buf, w as u32, h as u32, image::ColorType::Rgba8).is_ok()
    }
}

/// 非 Windows 无截图（探针恒失败——跨平台编译行为无损）
#[cfg(not(windows))]
fn capture_window_png(_hwnd: isize, _path: &std::path::Path) -> bool {
    false
}

/// 标题栏底色钉制（UIR：系统「标题栏强调色」设置会盖掉深浅跟随——
/// DWMWA_CAPTION_COLOR 显式设为侧栏底色，标题栏与界面同体）
/// A008 #12：失败去抖留痕一次（Win10 <22000 不支持该属性恒失败——
/// 原双 `let _ =` 完全静默，标题栏不同色时无从诊断）
#[cfg(windows)]
fn apply_caption_color(hwnd: isize, color: egui::Color32) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DwmSetWindowAttribute,
    };
    static WARNED: AtomicBool = AtomicBool::new(false);
    // COLORREF = 0x00BBGGRR（egui 0.37 预乘存储，实心色 alpha=255 无预乘损耗）
    let colorref =
        (u32::from(color.b()) << 16) | (u32::from(color.g()) << 8) | u32::from(color.r());
    unsafe {
        let hwnd = HWND(hwnd as *mut _);
        let caption = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &colorref as *const u32 as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        // BORDER 同色：CAPTION 设色后底边框区会渲染白条（DWM 副作用，截图实证）
        let border = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            &colorref as *const u32 as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        if (caption.is_err() || border.is_err()) && !WARNED.swap(true, Ordering::Relaxed) {
            tracing::warn!("DWM 标题栏钉色失败（系统或版本不支持，标题栏将保持系统底色）");
        }
    }
}

/// 非 Windows 无标题栏定制
#[cfg(not(windows))]
fn apply_caption_color(_hwnd: isize, _color: egui::Color32) {}

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
    // UI 截图探针（UIR.2）：ACCELWORLD_UI_SHOT=<目录> 逐页 PrintWindow 出 PNG 后自动退出
    let ui_shot_dir = std::env::var("ACCELWORLD_UI_SHOT")
        .ok()
        .map(std::path::PathBuf::from);
    // PL018.01 探针卫生/观测开关：
    //   TEST_NO_SAVE=1 退出不写用户窗口存档；EXPECT_POS="x,y" 打印落点偏差；
    //   EARLY_SHOT_MS=n 到点抓一帧即退（需 UI_SHOT 目录，用于热启动黑窗采样）
    let test_no_save = std::env::var("ACCELWORLD_TEST_NO_SAVE").is_ok_and(|v| v == "1");
    let probe_expect_pos = std::env::var("ACCELWORLD_EXPECT_POS")
        .ok()
        .and_then(|v| parse_expect_pos(&v));
    let early_shot_ms = std::env::var("ACCELWORLD_EARLY_SHOT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|_| ui_shot_dir.is_some());
    // PL018.04 测试钩子：丢弃前 N 次 Close（验证重发/兜底；正常模式为 0）
    let test_close_loss = std::env::var("ACCELWORLD_TEST_CLOSE_LOSS")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(0);
    let runtime = Arc::new(build_runtime());
    let iface = Arc::new(AppInterface::new());

    // 窗口几何恢复（UIR.7）：用户存档 "x,y,w,h" 优先，缺档/非法回退 token 基准；
    // A009 #6：恢复侧自行钳到最小尺寸下限（winit 创建期不施加 min_inner_size 实证）
    // PL017.01：截图探针固定测试几何，不读用户存档；首帧再按当前显示器夹取
    let base = accel_core::static_config::Static::get();
    let (mut x, mut y, mut w, mut h) = clamped_restore_geometry(
        iface
            .get_window_geometry()
            .and_then(|ref g| accel_core::interface::parse_window_geometry(g)),
        default_window_rect(),
        (
            base.base_f64("window_min_width") as f32,
            base.base_f64("window_min_height") as f32,
        ),
    );
    if ui_shot_dir.is_some() {
        let (sw, sh) = std::env::var("ACCELWORLD_UI_SHOT_GEOMETRY")
            .ok()
            .and_then(|v| {
                let (a, b) = v.split_once('x')?;
                Some((a.trim().parse::<f32>().ok()?, b.trim().parse::<f32>().ok()?))
            })
            .unwrap_or((1200.0, 720.0));
        x = 80.0;
        y = 80.0;
        w = sw;
        h = sh;
        // PL018.01：落点探针可指定名义位置（仅探针；配合 TEST_NO_SAVE 不写存档）
        if let Some((px, py)) = std::env::var("ACCELWORLD_TEST_POS")
            .ok()
            .and_then(|v| parse_expect_pos(&v))
        {
            x = px;
            y = py;
        }
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(Vec2::new(w, h))
        .with_position(Pos2::new(x, y))
        // A008 #8：最小尺寸下限入 token（页内减法布局不再塌负宽）
        .with_min_inner_size(Vec2::new(
            base.base_f64("window_min_width") as f32,
            base.base_f64("window_min_height") as f32,
        ))
        // PL017.01：窗口上限；首帧再按显示器工作区夹取
        .with_max_inner_size(crate::theme::window_max_size())
        // PL018.06：隐藏创建，首帧门控后再显示（防 GPU 热释放后的首帧黑窗）
        .with_visible(false)
        .with_title("加速世界");
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/accelworld-tray.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let rt_for_app = Arc::clone(&runtime);

    let create_app =
        move |cc: &eframe::CreationContext<'_>| -> Result<Box<dyn eframe::App>, String> {
            let prefs = iface.get_ui_preferences();
            // A008 #2 定案仅深色：主题偏好不再驱动 Visuals（ThemePref 配置域仅存档）
            crate::theme::apply(&cc.egui_ctx);
            install_fonts(&cc.egui_ctx);
            // PL018.06：首帧门控看门狗——隐藏窗口若平台不派发首帧，250ms 后无条件显示，
            // 避免“等首帧”变成“永不出现”
            let wd_ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(250));
                wd_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                wd_ctx.request_repaint();
            });
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
                pages: Pages::new(),
                start_time: std::time::Instant::now(),
                probe_close_secs,
                probe_hide_secs,
                probe_hide_done: false,
                probe_show_secs,
                probe_show_done: false,
                ui_shot_dir,
                ui_shot_step: 0,
                ui_shot_last: std::time::Instant::now(),
                ui_shot_switched: None,
                titlebar_hwnd: None,
                initial_geometry: (x, y, w, h),
                geometry_clamped: false,
                test_no_save,
                probe_expect_pos,
                probe_pos_logged: false,
                early_shot_ms,
                early_shot_done: false,
                first_show_done: false,
                frames_seen: 0,
                tray,
                hotkeys,
                quit_requested: false,
                quit_sent: false,
                exit_save_started: None,
                close_sent_at: None,
                close_attempt: 0,
                test_close_loss,
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

/// 内嵌 MiSans 子集（PL016.10，审核 #11）：正文字体脱离系统默认性——
/// 编译期嵌入（CJK 基本区全量子集 5.6MB，农历生僻字无 tofu；MiSans 官方
/// SIL/MI 授权免费商用）；font_family token = MiSans 时优先走内嵌，
/// 系统文件仅作 token 自定义时的回退路径
const MISANS_EMBEDDED: &[u8] = include_bytes!("../../../assets/fonts/MiSans-Regular-sub.ttf");
/// 中等字重（500）——次级标题/按钮/标签；PL017 层级（Regular 之上、Semibold 之下）
const MISANS_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/MiSans-Medium-sub.ttf");
/// 半粗字重（600）——页面标题/分区标题/强调；PL017 层级
const MISANS_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/MiSans-Semibold-sub.ttf");

/// 字体注入（PL013.03 + A005 #8 接线）：族名读 ui.json token
/// `font_family`（中文，默认族）/ `font_family_digits`（数字等宽族）
/// FIX011.11：数字族装载与 CJK 回退提取 helper——MiSans/通用分支原各复制一份
pub fn install_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    let s = accel_core::static_config::Static::get();
    let cjk_family = s.ui_str("font_family");
    let digit_family = s.ui_str("font_family_digits");
    // 内嵌优先：token 指向 MiSans 时直接用编译期字节（零系统依赖，跨机一致）
    if cjk_family.replace(' ', "") == "MiSans" {
        fonts.font_data.insert(
            "cjk".into(),
            egui::FontData::from_owned(MISANS_EMBEDDED.to_vec()).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "cjk".into());
        push_cjk_fallback(&mut fonts);
        tracing::info!("中文族字体加载: MiSans（内嵌子集）");
    // 候选名 = 族名去空格派生 + 平台常见文件名兜底（族名与文件名未必同名）
    } else if let Some((path, bytes)) =
        load_font(cjk_family, &["msyh.ttc", "msyh.ttf", "msyhbd.ttc"])
    {
        tracing::info!("中文族字体加载: {path}");
        fonts
            .font_data
            .insert("cjk".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "cjk".into());
        push_cjk_fallback(&mut fonts);
    } else {
        tracing::warn!("{cjk_family} 未找到，中文回退 egui 内置字体（可能 tofu）");
    }
    install_digit_family(&mut fonts, digit_family);
    install_weight_families(&mut fonts);
    install_icon_family(&mut fonts);
    ctx.set_fonts(fonts);
}

/// 图标族装载（PL019）：Tabler Icons 3.50 子集——独立族名，避免混入正文回退链；
/// 调用点为 `widgets::icons::draw`（字体绘制，24 网格）
fn install_icon_family(fonts: &mut egui::FontDefinitions) {
    fonts.font_data.insert(
        "icons".into(),
        egui::FontData::from_static(crate::widgets::icons::ICON_FONT).into(),
    );
    fonts
        .families
        .insert(egui::FontFamily::Name("icons".into()), vec!["icons".into()]);
    tracing::info!("图标字体加载: Tabler Icons 3.50 子集（29 字形）");
}

/// 中文字重族装载（PL017）：Medium 500 / Semibold 600 与 Regular 同源子集；
/// 两族各以自身为首选、Regular(cjk) 为兜底，缺失时回退默认 proportional
fn install_weight_families(fonts: &mut egui::FontDefinitions) {
    for (name, bytes) in [("medium", MISANS_MEDIUM), ("semibold", MISANS_SEMIBOLD)] {
        fonts.font_data.insert(
            name.into(),
            egui::FontData::from_owned(bytes.to_vec()).into(),
        );
        let family = egui::FontFamily::Name(name.into());
        let list = fonts.families.entry(family).or_default();
        list.insert(0, name.into());
        list.push("cjk".into());
    }
    tracing::info!("中文字重加载: MiSans Medium 500 / Semibold 600（内嵌子集）");
}

/// 数字等宽族装载（FIX011.11 单源）：命中则插 Monospace 队首，未命中回退内置
fn install_digit_family(fonts: &mut egui::FontDefinitions, digit_family: &str) {
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
}

/// 数字族内 CJK 回退（UIR）：等宽数字串含「天/周」等汉字——Bahnschrift 无 CJK
/// 字形缺失时渲染 tofu；向 Monospace 族尾部追加 cjk 兜底（内置等宽 Hack 之后，
/// 缺字形时才回落）（FIX011.11 单源：两分支原各复制一份）
fn push_cjk_fallback(fonts: &mut egui::FontDefinitions) {
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("cjk".into());
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

    #[test]
    fn sidebar_icon_label_matrix() {
        // 五页导航图标/文字矩阵（导航语义锚点——替代原 dock 矩阵测试）
        assert_eq!(
            sidebar_icon(PageId::Clock),
            crate::widgets::icons::Icon::Clock
        );
        assert_eq!(
            sidebar_icon(PageId::Countdown),
            crate::widgets::icons::Icon::Hourglass
        );
        assert_eq!(
            sidebar_icon(PageId::World),
            crate::widgets::icons::Icon::Globe
        );
        assert_eq!(
            sidebar_icon(PageId::Weather),
            crate::widgets::icons::Icon::Cloud
        );
        assert_eq!(
            sidebar_icon(PageId::Alarm),
            crate::widgets::icons::Icon::Bell
        );
        assert_eq!(sidebar_label(PageId::Clock), "时钟");
        assert_eq!(sidebar_label(PageId::Alarm), "闹钟");
    }

    #[test]
    fn expect_pos_parses_xy_and_rejects_garbage() {
        // PL018.01：探针参数容错——合法返回坐标，非法/缺项返回 None 不 panic
        assert_eq!(parse_expect_pos("80, 90"), Some((80.0, 90.0)));
        assert_eq!(parse_expect_pos("-12.5,3"), Some((-12.5, 3.0)));
        assert_eq!(parse_expect_pos("80"), None);
        assert_eq!(parse_expect_pos("a,b"), None);
    }

    #[test]
    fn close_step_escalates_without_skipping() {
        // PL018.04：1s 重发 → 2s 显示后重发 → 3s 兜底；早到不越级
        use std::time::Duration;
        assert_eq!(close_step(Duration::from_millis(999), 1), CloseStep::None);
        assert_eq!(close_step(Duration::from_secs(1), 1), CloseStep::Resend);
        assert_eq!(
            close_step(Duration::from_secs(2), 2),
            CloseStep::RevealAndResend
        );
        assert_eq!(close_step(Duration::from_secs(3), 3), CloseStep::ForceExit);
        assert_eq!(close_step(Duration::from_secs(1), 2), CloseStep::None);
        assert_eq!(close_step(Duration::from_secs(2), 3), CloseStep::None);
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
        // 热键 → 命令意图：保存/退出两键一一对应（.02 检验项；A008 #2 主题键退役）
        assert_eq!(hotkey_action(HotkeyKind::Save), PlatformAction::SaveNow);
        assert_eq!(hotkey_action(HotkeyKind::Quit), PlatformAction::Quit);
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
    fn clamp_geometry_keeps_window_in_monitor() {
        // PL017.01：1550×1058 存档在 1536×816 逻辑屏上夹到 ≤1200×820 且位置可见
        let (x, y, w, h) = clamp_geometry_to_monitor(
            (-7.0, -7.0, 1550.0, 1058.0),
            Vec2::new(1536.0, 816.0),
            (520.0, 420.0),
            Vec2::new(1200.0, 820.0),
        );
        assert!(w <= 1200.0 && h <= 820.0, "尺寸未夹取: {w}x{h}");
        assert!(x >= 24.0 && y >= 24.0, "位置越界: {x},{y}");
        assert!(x + w <= 1536.0 - 24.0 + 0.1, "右缘越界: {}", x + w);
        // 正常存档不被改写
        let (x, y, w, h) = clamp_geometry_to_monitor(
            (100.0, 100.0, 900.0, 500.0),
            Vec2::new(1536.0, 816.0),
            (520.0, 420.0),
            Vec2::new(1200.0, 820.0),
        );
        assert_eq!((x, y, w, h), (100.0, 100.0, 900.0, 500.0));
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
        // A009 #20：临时目录 RAII（drop 即清——断言失败不残留 %TEMP%）
        struct RemoveOnDrop(std::path::PathBuf);
        impl Drop for RemoveOnDrop {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let dir = std::env::temp_dir().join(format!("accel_startup_{}", std::process::id()));
        let _cleanup = RemoveOnDrop(dir.clone());
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
    fn restore_geometry_clamped_to_min() {
        // A009 #6 验收：旧小存档钳到最小尺寸；正常存档/缺档不受影响；坐标原样保留
        assert_eq!(
            clamped_restore_geometry(
                Some((50.0, 50.0, 100.0, 80.0)),
                (100.0, 100.0, 900.0, 500.0),
                (520.0, 420.0)
            ),
            (50.0, 50.0, 520.0, 420.0),
            "旧小存档钳到 min"
        );
        assert_eq!(
            clamped_restore_geometry(
                Some((-8.0, -8.0, 1100.0, 700.0)),
                (100.0, 100.0, 900.0, 500.0),
                (520.0, 420.0)
            ),
            (-8.0, -8.0, 1100.0, 700.0),
            "正常存档（含负多屏坐标）原样"
        );
        assert_eq!(
            clamped_restore_geometry(None, (100.0, 100.0, 900.0, 500.0), (520.0, 420.0)),
            (100.0, 100.0, 900.0, 500.0),
            "缺档回退 token 基准"
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
// 逻辑步骤：build_runtime → AppInterface::new → NativeOptions（token 几何 + 最小
//   尺寸下限 + 存档几何优先恢复）→ run_native（create_app：主题 apply/fonts/state
//   装配 + 托盘/热键挂接 → AppHandle）→ logic（drain → 平台事件 → 探针时序 →
//   退出/驻留 → 托盘状态）→ ui（仪器坞 → 页总装（抽屉+主仪） → toast → 设置
//   浮窗 → 截图探针 → 节奏）
// 设计理由：drain→部件→节奏对齐 z.plan A 节主循环骨架（UIR 重塑后背板为 CentralPanel
//   实心 window_bg，玻璃面注册/acrylic/透明 viewport 整体废弃——A008 #6 说明区
//   随实心现状重写）；**隐藏期不重绘但 logic 仍逐轮驱动**——视口命令（托盘恢复/
//   退出/探针）与托盘状态推送一律落在 logic，落在 ui 会在隐藏驻留后失效（.01 真窗
//   实证的返工教训）
// 退出语义（PL015.01 定案，对齐 Python closeEvent）：关闭按钮 → CancelClose+隐藏
//   驻留（不落盘——倍率经 ApplyRate 内聚实时落盘）；托盘退出/热键退出/探针 →
//   quit_requested → SaveNow+Quit+Close 一次放行（quit_sent 区分两种 Close 来源）；
//   退出序列发 SetWindowGeometry 存档外框矩形（UIR.7：位置/大小记忆，启动恢复）
// 平台事件归一（PL015.01/.02）：tray::TrayCommand 与 hotkey::HotkeyKind 经纯函数
//   tray_action/hotkey_action 归一到 PlatformAction，再由 apply_platform_action 汇合
//   应用（窗口/命令出口唯一入口——映射可单测，无须真实托盘/热键；A008 #2 起
//   CycleTheme 退役，PlatformAction 仅 Show/Hide/SaveNow/Quit）
// 标题栏钉色（UIR）：DWMWA_CAPTION_COLOR/BORDER_COLOR 显式设侧栏底色（系统
//   「标题栏强调色」会盖深浅跟随）；A008 #2 定案仅深色恒钉深色；失败去抖留痕一次
//   （A008 #12，Win10 <22000 不支持该属性）
// 探针通路：ACCELWORLD_HIDE_AFTER / ACCELWORLD_SHOW_AFTER / ACCELWORLD_FPS_PROBE /
//   ACCELWORLD_UI_SHOT（UIR.2 真窗逐页 PrintWindow 出 PNG 后自动退出）
// 异常处理：字体缺失降级内置（告警）；托盘/热键构造失败降级 None（无系统集成
//   环境行为无损）；截图采集任何环节失败返 false 走日志（探针专用路径）
// 关联配置：window_*/window_min_*/shortcuts.*/logs_dir（base.json）+ ui.json
//   layout.*/colors.*；消费方：main.rs
