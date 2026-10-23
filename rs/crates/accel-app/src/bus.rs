//! 信号总线消费侧（z.plan 修订一新增件，前后端解耦的 app 侧半边）
//! UiState 持后端事实快照；update() 每帧 drain 事件逐一 apply；锁不跨帧

use accel_core::interface::AppInterface;
use accel_core::types::{BackendEvent, TimeInfo, UiPreferences, WeatherData};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};

use accel_core::types::FrontendCommand;

/// 页面标识（导航五页 + 设置浮窗独立；默认时钟页）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageId {
    #[default]
    Clock,
    Countdown,
    World,
    Weather,
    Alarm,
}

/// 导航固定序（PL013.09 五页序）
pub const PAGE_ORDER: [PageId; 5] = [
    PageId::Clock,
    PageId::Countdown,
    PageId::World,
    PageId::Weather,
    PageId::Alarm,
];

/// UI 状态快照（全部字段为后端事实——UI 无状态重建，z.plan 修订一定案）
#[derive(Debug, Clone)]
pub struct UiState {
    /// 当前页
    pub current_page: PageId,
    /// 最新时间快照（Tick 驱动）
    pub time: Option<TimeInfo>,
    /// 当前倍率（RateChanged 驱动）
    pub rate: f64,
    /// 倍率合法区间
    pub rate_bounds: (f64, f64),
    /// 天气快照（city → 数据/失败原因）
    pub weather: HashMap<String, WeatherData>,
    pub weather_errors: HashMap<String, String>,
    /// 闹钟列表快照（AlarmsChanged 全量回推）
    pub alarms: Vec<accel_core::alarm::Alarm>,
    /// 会话偏好（启动时拉取；ThemePref 仅存档——A008 #2 定案仅深色，app 侧零消费）
    pub prefs: UiPreferences,
    /// 设置浮窗开合（.10）
    pub settings_open: bool,
    /// 倒计时目标列表（ConfigSaved 后由 drain 重拉 iface 同步快照）
    pub countdown_targets: Vec<accel_core::settings::CountdownTarget>,
    /// 倒计时加号草稿（页面临时输入态）
    pub countdown_draft_name: String,
    pub countdown_draft_at: String,
    /// 世界时钟常驻城市（ConfigSaved 后由 drain 重拉）
    pub world_pins: Vec<String>,
    /// 选中基准时区（IANA）
    pub base_timezone: Option<String>,
    /// 闹钟自定义添加草稿
    pub alarm_draft_label: String,
    pub alarm_draft_time: String,
    /// 保存回执（A005 #28：ConfigSaved 结果，退出序列据此放行 Close）
    pub save_ack: bool,
}

impl UiState {
    /// 世界常驻城市视图（只读借用）
    pub fn world_pins_view(&self) -> &[String] {
        &self.world_pins
    }
}

impl Default for UiState {
    /// 默认快照（倍率/偏好取 base.json token 单源；time 待首个 Tick 填充）
    fn default() -> Self {
        let base = accel_core::static_config::Static::get();
        UiState {
            current_page: PageId::default(),
            time: None,
            rate: base.base_f64("default_rate"),
            rate_bounds: (base.base_f64("rate_min"), base.base_f64("rate_max")),
            weather: HashMap::new(),
            weather_errors: HashMap::new(),
            alarms: Vec::new(),
            prefs: UiPreferences {
                theme: base.base_str("default_theme").to_string(),
                last_city: base.base_str("default_city").to_string(),
                last_timezone: base.base_str("default_timezone").to_string(),
                countdown_target: String::new(),
            },
            settings_open: false,
            countdown_targets: Vec::new(),
            countdown_draft_name: String::new(),
            countdown_draft_at: String::new(),
            world_pins: Vec::new(),
            base_timezone: None,
            alarm_draft_label: String::new(),
            alarm_draft_time: String::new(),
            save_ack: false,
        }
    }
}

impl UiState {
    /// 后端事件 → 状态快照更新（枚举 match，逐变体消费）
    pub fn apply(&mut self, evt: BackendEvent) {
        match evt {
            BackendEvent::Tick(info) => self.time = Some(info),
            BackendEvent::WeatherUpdated { city, data } => {
                self.weather_errors.remove(&city);
                self.weather.insert(city, data);
            }
            BackendEvent::WeatherFailed { city, reason } => {
                // A005 #13：失败须清除同城旧成功数据，否则页面"红字失败 + 完整数据"并存
                self.weather.remove(&city);
                self.weather_errors.insert(city, reason);
            }
            BackendEvent::AlarmTriggered(alarm) => {
                tracing::info!("闹钟触发: {}（{}）", alarm.label, alarm.time);
            }
            BackendEvent::AlarmsChanged(list) => self.alarms = list,
            BackendEvent::ConfigSaved(ok) => {
                self.save_ack = ok;
            }
            BackendEvent::RateChanged(rate) => self.rate = rate,
            // A005 #19：拒绝事件只记日志——rate 由 drain 侧回拉权威值修正（乐观更新回退）
            BackendEvent::RateRejected(rate) => {
                tracing::warn!("倍率 {rate} 被后端拒绝，UI 将回拉权威值");
            }
        }
    }

    /// 下一重绘截止：Tick 间隔与空闲兜底取小（空闲 1s 兜底 + Tick 节奏）
    /// 返回 request_repaint_after 的时长（.13 空闲节奏核对的依据）
    /// A008 #19：Tick 未到的兜底值改读 token（原 unwrap_or(100) 与 clock_tick_ms
    /// 同值双源——token 变更即漂移）
    pub fn next_repaint_interval(&self) -> std::time::Duration {
        let tick = self
            .time
            .as_ref()
            .map(|t| t.tick_ms.max(1) as u64)
            .unwrap_or_else(|| {
                accel_core::static_config::Static::get()
                    .base_i64("clock_tick_ms")
                    .max(1) as u64
            });
        std::time::Duration::from_millis(tick.min(1000))
    }
}

/// 平台副作用（A005 #22②：从 UI 帧线程移出——drain 只入队，工作线程执行）
#[derive(Debug, Clone)]
pub enum SideEffect {
    /// 闹钟触发通知（label, HH:MM）
    NotifyAlarm(String, String),
    /// 配置保存失败通知
    NotifySaveFailed,
    /// 铃声（sound_type, sound_value）
    PlaySound(String, String),
}

/// 副作用工作线程（A005 #22②）：std mpsc 无界队列（入队恒成功，UI 帧不阻塞）
/// 可注入 handler 便于单测（生产注入平台实现）
/// FIX007.13：线程创建失败记 error 留痕（原 .ok() 静默——线程缺席时通知与铃声
/// 全部缺席，仅靠 send 侧失败日志信息不足）
fn spawn_side_effect_worker<F>(rx: std::sync::mpsc::Receiver<SideEffect>, handler: F)
where
    F: Fn(SideEffect) + Send + 'static,
{
    if let Err(e) = std::thread::Builder::new()
        .name("accel-side-effects".into())
        .spawn(move || {
            while let Ok(effect) = rx.recv() {
                handler(effect);
            }
        })
    {
        tracing::error!("副作用工作线程创建失败（通知与铃声将缺席）: {e}");
    }
}

/// 生产副作用执行（通知 + 铃声；两者失败皆降级不阻塞）
fn apply_side_effect(effect: SideEffect) {
    match effect {
        SideEffect::NotifyAlarm(label, time) => {
            crate::platform::notify::notify_alarm(&label, &time);
        }
        SideEffect::NotifySaveFailed => {
            crate::platform::notify::notify_save_failed();
        }
        SideEffect::PlaySound(kind, value) => play_alarm_sound(&kind, &value),
    }
}

/// 命令出口（A005 #22①：通道满时不丢命令——入待发队列，下一帧 flush 重试）
struct CmdQueue {
    tx: Sender<FrontendCommand>,
    pending: std::cell::RefCell<std::collections::VecDeque<FrontendCommand>>,
}

// 待发队列上限（防后端长时间停摆导致无界增长；超限丢弃并记 error）
const PENDING_CAP: usize = 256;

impl CmdQueue {
    fn new(tx: Sender<FrontendCommand>) -> CmdQueue {
        CmdQueue {
            tx,
            pending: std::cell::RefCell::new(std::collections::VecDeque::new()),
        }
    }

    /// 尝试直发；满则入待发队列（返回是否已直发）
    fn push(&self, cmd: FrontendCommand) -> bool {
        match self.tx.try_send(cmd) {
            Ok(()) => true,
            Err(tokio::sync::mpsc::error::TrySendError::Full(cmd)) => {
                let mut q = self.pending.borrow_mut();
                if q.len() >= PENDING_CAP {
                    tracing::error!("命令待发队列已满（{PENDING_CAP}），丢弃命令");
                } else {
                    q.push_back(cmd);
                }
                false
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                tracing::error!("命令通道已关闭，丢弃命令");
                false
            }
        }
    }

    /// 每帧 flush：按顺序补发待发命令，遇到再次满载即停（保序）
    fn flush(&self) {
        loop {
            let Some(cmd) = self.pending.borrow_mut().pop_front() else {
                return;
            };
            match self.tx.try_send(cmd) {
                Ok(()) => continue,
                Err(tokio::sync::mpsc::error::TrySendError::Full(cmd)) => {
                    self.pending.borrow_mut().push_front(cmd);
                    return;
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    tracing::error!("命令通道已关闭，清空待发队列");
                    self.pending.borrow_mut().clear();
                    return;
                }
            }
        }
    }
}

/// 信号总线：AppHandle 持有（命令发送 + 事件接收 + 后端接口 + 副作用出队）
pub struct Bus {
    pub iface: Arc<AppInterface>,
    cmd: CmdQueue,
    evt_rx: Receiver<BackendEvent>,
    /// 平台副作用出口（通知/铃声由工作线程执行——A005 #22②）
    side_tx: std::sync::mpsc::Sender<SideEffect>,
}

impl Bus {
    /// 经 interface spawn 建立双通道 + 副作用工作线程（要求 tokio 上下文——
    /// eframe 装配期经共享 runtime block_on 进入；window::run 调用）
    pub fn spawn_on(iface: Arc<AppInterface>, runtime: &tokio::runtime::Runtime) -> Bus {
        Self::spawn_on_with(iface, runtime, apply_side_effect)
    }

    /// 注入副作用 handler 的构造（单测用：handler 可替换为收集闭包）
    pub fn spawn_on_with<F>(
        iface: Arc<AppInterface>,
        runtime: &tokio::runtime::Runtime,
        handler: F,
    ) -> Bus
    where
        F: Fn(SideEffect) + Send + 'static,
    {
        let (cmd_tx, evt_rx) = runtime.block_on(async { AppInterface::spawn(Arc::clone(&iface)) });
        let (side_tx, side_rx) = std::sync::mpsc::channel();
        spawn_side_effect_worker(side_rx, handler);
        Bus {
            iface,
            cmd: CmdQueue::new(cmd_tx),
            evt_rx,
            side_tx,
        }
    }

    /// 命令统一出口（页/控件发意图的唯一入口；A005 #22① 满通道入待发队列不丢）
    pub fn send(&self, cmd: FrontendCommand) {
        self.cmd.push(cmd);
    }

    /// 平台副作用的传输出口（A005 #22②：UI 帧线程只入队，工作线程执行）
    fn side_effect(&self, effect: SideEffect) {
        if self.side_tx.send(effect).is_err() {
            tracing::error!("副作用工作线程已退出，丢弃副作用事件");
        }
    }

    /// 每帧 drain：先补发待发命令 → while try_recv 消费全部积压事件 → UiState::apply
    /// ConfigSaved 后重拉倒计时目标同步快照（iface 拉取式通道）
    /// 闹钟触发/保存失败的平台通知与铃声经副作用线程（A005 #22②）
    /// FIX007.11：删除返回值（原「帧计数/节奏探针用」口径失实——唯一调用方
    /// window::logic 直接丢弃；帧计数由 AppHandle.repaint_count 承接）
    pub fn drain(&mut self, state: &mut UiState) {
        self.cmd.flush();
        let mut need_reload = false;
        let mut need_rate_resync = false;
        while let Ok(evt) = self.evt_rx.try_recv() {
            match &evt {
                BackendEvent::AlarmTriggered(alarm) => {
                    // 系统通知 + 铃声经工作线程（失败皆降级不阻塞；触发事实已由
                    // AlarmsChanged/列表状态承载）
                    self.side_effect(SideEffect::NotifyAlarm(
                        alarm.label.clone(),
                        alarm.time.clone(),
                    ));
                    self.side_effect(SideEffect::PlaySound(
                        alarm.sound_type.clone(),
                        alarm.sound_value.clone(),
                    ));
                }
                // FIX010.1：ConfigSaved(false) 已收敛为纯 IO 失败信号（core 侧拒绝
                // 路径不再回推 false）——此处「保存失败」toast 只对真落盘失败触发
                BackendEvent::ConfigSaved(false) => {
                    self.side_effect(SideEffect::NotifySaveFailed);
                }
                // A005 #19：倍率被拒 → 回拉权威值（乐观更新回退）
                BackendEvent::RateRejected(_) => need_rate_resync = true,
                _ => {}
            }
            if matches!(evt, BackendEvent::ConfigSaved(_)) {
                need_reload = true;
            }
            state.apply(evt);
        }
        if need_reload {
            state.countdown_targets = self.iface.get_countdown_targets();
            state.world_pins = self.iface.get_world_pins();
            let prefs = self.iface.get_ui_preferences();
            state.base_timezone = Some(prefs.last_timezone);
            // A005 #4：单城模型下 last_city 由 SetWeatherCity 持久化，重拉同步
            state.prefs.last_city = prefs.last_city;
        }
        if need_rate_resync {
            state.rate = self.iface.get_rate();
        }
    }

    // 启动期偏好快照（A005 #15：生产零调用已删除——装配期用 `iface.get_ui_preferences()`）
    // FIX006.2：原 cmd_tx()（裸 Sender 出口，绕过 CmdQueue 待发队列）已删除——页面统一经 bus.send
}

/// 闹钟铃声分发（Python play_preset_sound/play_custom_sound 分支对等）：
/// preset → core::audio::play_preset（枚举反解）；custom → play_file（路径直放）
/// 其余 sound_type 回退 preset（alarm.rs 解析层已过滤，此处兜底同语义）
/// A005 #24：custom 路径加扩展名白名单校验（用户配置属外部输入，非法回退 preset）
fn play_alarm_sound(sound_type: &str, sound_value: &str) {
    if sound_type == "custom" && is_allowed_audio_path(sound_value) {
        accel_core::audio::play_file(std::path::Path::new(sound_value));
    } else {
        if sound_type == "custom" {
            tracing::warn!("自定义铃声路径非法（白名单外扩展名），回退预设: {sound_value}");
        }
        accel_core::audio::play_preset(accel_core::alarm::PresetSound::from_value(sound_value));
    }
}

/// 自定义铃声路径白名单（A005 #24：wav/mp3 扩展名 + 非空路径）
fn is_allowed_audio_path(value: &str) -> bool {
    let p = std::path::Path::new(value);
    p.is_absolute()
        && p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("wav") || e.eq_ignore_ascii_case("mp3"))
            .unwrap_or(false)
}

// ===== src/bus.rs 说明区 =====
// 输入：BackendEvent 流（后端事实）+ 页/控件命令意图；输出：UiState 快照 + 命令出口 + 副作用线程
// 逻辑步骤：drain 每帧「先 flush 待发命令 → while try_recv 消费事件 → apply 逐变体更新」
//   → 渲染层只读 state
// 设计理由：z.plan 修订一信号接口的 app 半边——UI 不直碰业务状态（页/控件只拿
//   &UiState 与命令出口）；锁不跨帧（drain 在 logic 首行完成）；Tick 节奏由后端推，
//   app 侧 next_repaint_interval 只做「取小 + 1s 兜底」
// A005 #22 修正：①命令出口满载时入待发队列（下一帧保序补发，帧路径零丢失，上限
//   PENDING_CAP）；②平台副作用（通知/铃声）经 std::thread 工作线程执行——UI 帧只入队，
//   不在渲染线程做 IO（入队恒成功，无界通道）
// 异常处理：send 满载入队/关闭记 error 不 panic；副作用线程退出时记 error 丢弃事件
// 关联依赖：accel-core::interface/types；消费方：window.rs AppHandle
#[cfg(test)]
mod tests {
    use super::*;
    use accel_core::types::TimeInfo;

    fn tick_info(tick_ms: i64) -> TimeInfo {
        TimeInfo {
            standard_datetime: "2026-10-16 12:00:00".into(),
            custom_time: "1:00:00".into(),
            chinese_date: String::new(),
            lunar_info: String::new(),
            dilation_percentage: 200.0,
            expanded_hours_per_day: 48.0,
            remaining_hours: 24.0,
            day_hours: 48,
            tick_ms,
        }
    }

    #[test]
    fn apply_updates_snapshot() {
        // fake 事件驱动快照：Tick/RateChanged/Weather/Alarm 四变体
        let mut state = UiState::default();
        assert!(state.time.is_none());
        state.apply(BackendEvent::Tick(tick_info(50)));
        assert_eq!(state.time.as_ref().expect("应有快照").tick_ms, 50);
        state.apply(BackendEvent::RateChanged(5.5));
        assert_eq!(state.rate, 5.5);
        state.apply(BackendEvent::WeatherFailed {
            city: "北京".into(),
            reason: "网络".into(),
        });
        assert!(state.weather_errors.contains_key("北京"));
        let wd = accel_core::types::WeatherData {
            temperature: 20.0,
            humidity: 50.0,
            wind_speed: 5.0,
            apparent_temperature: 19.0,
            weather_code: 0,
            weather: "晴".into(),
            description: "晴".into(),
            icon: "☀".into(),
            hourly: Vec::new(),
            daily: Vec::new(),
        };
        state.apply(BackendEvent::WeatherUpdated {
            city: "北京".into(),
            data: wd,
        });
        assert!(state.weather.contains_key("北京"));
        assert!(
            !state.weather_errors.contains_key("北京"),
            "成功后失败态应清除"
        );
        state.apply(BackendEvent::AlarmsChanged(vec![]));
        assert!(state.alarms.is_empty());
        // A005 #13 回归：失败事件必须清除同城旧成功数据（否则错误+数据并存）
        state.apply(BackendEvent::WeatherFailed {
            city: "北京".into(),
            reason: "刷新失败".into(),
        });
        assert!(
            !state.weather.contains_key("北京"),
            "失败后同城旧数据应被清除"
        );
        assert!(state.weather_errors.contains_key("北京"));
    }

    #[test]
    fn repaint_interval_bounds() {
        // 节奏取小：高速 tick 50ms 即用 50；低速 tick 钳 1s 兜底
        let mut state = UiState::default();
        assert_eq!(
            state.next_repaint_interval(),
            std::time::Duration::from_millis(100)
        );
        state.apply(BackendEvent::Tick(tick_info(50)));
        assert_eq!(
            state.next_repaint_interval(),
            std::time::Duration::from_millis(50)
        );
        state.apply(BackendEvent::Tick(tick_info(5000)));
        assert_eq!(
            state.next_repaint_interval(),
            std::time::Duration::from_millis(1000)
        );
    }

    #[test]
    fn custom_ringtone_path_whitelist() {
        // A005 #24 回归：仅 wav/mp3 绝对路径放行；其余（相对路径/其他扩展名/空）拒绝
        assert!(super::is_allowed_audio_path(r"C:\ring\a.mp3"));
        assert!(super::is_allowed_audio_path(r"C:\ring\b.WAV"));
        assert!(!super::is_allowed_audio_path("relative.mp3"));
        assert!(!super::is_allowed_audio_path(r"C:\ring\evil.exe"));
        assert!(!super::is_allowed_audio_path(""));
        assert!(!super::is_allowed_audio_path(r"C:\ring\noext"));
    }

    #[test]
    fn cmd_queue_never_loses_on_full() {
        // A005 #22① 回归：通道容量 1，连发 5 条命令——满载入待发队列，
        // flush 后按序全量到达（帧路径零丢失）
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("测试 runtime");
        let (_tx_keep, mut rx) = tokio::sync::mpsc::channel::<FrontendCommand>(1);
        let queue = CmdQueue::new(_tx_keep);
        for r in [1.0, 2.0, 3.0, 4.0, 5.0] {
            queue.push(FrontendCommand::SetRate(r));
        }
        let mut got = Vec::new();
        // 逐轮 flush + 消费（模拟后端节奏）
        for _ in 0..5 {
            queue.flush();
            if let Ok(FrontendCommand::SetRate(r)) = rx.try_recv() {
                got.push(r);
            }
        }
        rt.block_on(async {});
        assert_eq!(got, vec![1.0, 2.0, 3.0, 4.0, 5.0], "五条命令应保序零丢失");
    }

    #[test]
    fn side_effect_worker_delivers_off_thread() {
        // A005 #22② 回归：副作用经工作线程投递（UI 帧只入队）
        let (tx, rx) = std::sync::mpsc::channel::<SideEffect>();
        let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let sink2 = std::sync::Arc::clone(&sink);
        spawn_side_effect_worker(rx, move |effect| {
            let tag = match effect {
                SideEffect::NotifyAlarm(l, t) => format!("alarm:{l}:{t}"),
                SideEffect::NotifySaveFailed => "save_failed".to_string(),
                SideEffect::PlaySound(k, v) => format!("sound:{k}:{v}"),
            };
            sink2.lock().expect("锁").push(tag);
        });
        tx.send(SideEffect::NotifyAlarm("起床".into(), "07:30".into()))
            .expect("发送");
        tx.send(SideEffect::PlaySound("preset".into(), "classic".into()))
            .expect("发送");
        // 等待工作线程消费（有界轮询）
        for _ in 0..50 {
            if sink.lock().expect("锁").len() == 2 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let got = sink.lock().expect("锁").clone();
        assert_eq!(
            got,
            vec![
                "alarm:起床:07:30".to_string(),
                "sound:preset:classic".to_string()
            ]
        );
    }

    #[test]
    fn repaint_fallback_reads_token() {
        // A008 #19 验收：Tick 未到的兜底间隔取 clock_tick_ms token（单源，钳 1ms）
        let state = UiState::default();
        let expected = accel_core::static_config::Static::get()
            .base_i64("clock_tick_ms")
            .max(1) as u64;
        assert_eq!(
            state.next_repaint_interval(),
            std::time::Duration::from_millis(expected.min(1000))
        );
    }
}
