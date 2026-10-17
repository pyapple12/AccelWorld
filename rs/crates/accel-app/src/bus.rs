//! 信号总线消费侧（z.plan 修订一新增件，前后端解耦的 app 侧半边）
//! UiState 持后端事实快照；update() 每帧 drain 事件逐一 apply；锁不跨帧

use accel_core::interface::AppInterface;
use accel_core::types::{BackendEvent, ThemePref, TimeInfo, UiPreferences, WeatherData};
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
    /// 会话偏好（启动时拉取；SetTheme 回推前本地即改——乐观更新）
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
    /// 天气已集城市（多城市切换 chips；含 last_city）
    pub weather_cities: Vec<String>,
    /// 闹钟自定义添加草稿
    pub alarm_draft_label: String,
    pub alarm_draft_time: String,
}

impl UiState {
    /// 世界常驻城市视图（只读借用）
    pub fn world_pins_view(&self) -> &[String] {
        &self.world_pins
    }

    /// 天气已集城市视图（缺省含 last_city——至少一城）
    pub fn weather_cities_view(&self) -> Vec<String> {
        let mut cities = self.weather_cities.clone();
        if !cities.contains(&self.prefs.last_city) {
            cities.push(self.prefs.last_city.clone());
        }
        cities
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
            weather_cities: Vec::new(),
            alarm_draft_label: String::new(),
            alarm_draft_time: String::new(),
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
                self.weather_errors.insert(city, reason);
            }
            BackendEvent::AlarmTriggered(alarm) => {
                tracing::info!("闹钟触发: {}（{}）", alarm.label, alarm.time);
            }
            BackendEvent::AlarmsChanged(list) => self.alarms = list,
            BackendEvent::ConfigSaved(_ok) => {}
            BackendEvent::RateChanged(rate) => self.rate = rate,
        }
    }

    /// 下一重绘截止：Tick 间隔与空闲兜底取小（空闲 1s 兜底 + Tick 节奏）
    /// 返回 request_repaint_after 的时长（.13 空闲节奏核对的依据）
    pub fn next_repaint_interval(&self) -> std::time::Duration {
        let tick = self
            .time
            .as_ref()
            .map(|t| t.tick_ms.max(1) as u64)
            .unwrap_or(100);
        std::time::Duration::from_millis(tick.min(1000))
    }

    /// 当前主题偏好（三态）
    pub fn theme_pref(&self) -> ThemePref {
        match self.prefs.theme.as_str() {
            "light" => ThemePref::Light,
            "dark" => ThemePref::Dark,
            _ => ThemePref::Auto,
        }
    }
}

/// 信号总线：AppHandle 持有（命令发送 + 事件接收 + 后端接口）
pub struct Bus {
    pub iface: Arc<AppInterface>,
    cmd_tx: Sender<FrontendCommand>,
    evt_rx: Receiver<BackendEvent>,
}

impl Bus {
    /// 经 interface spawn 建立双通道（要求 tokio 上下文——eframe 装配期经
    /// 共享 runtime block_on 进入；window::run 调用）
    pub fn spawn_on(iface: Arc<AppInterface>, runtime: &tokio::runtime::Runtime) -> Bus {
        let (cmd_tx, evt_rx) = runtime.block_on(async { AppInterface::spawn(Arc::clone(&iface)) });
        Bus {
            iface,
            cmd_tx,
            evt_rx,
        }
    }

    /// 命令统一出口（页/控件发意图的唯一入口）
    pub fn send(&self, cmd: FrontendCommand) {
        if let Err(e) = self.cmd_tx.try_send(cmd) {
            tracing::error!("命令发送失败（通道满或关闭）: {e}");
        }
    }

    /// 每帧 drain：while try_recv 消费全部积压事件 → UiState::apply
    /// ConfigSaved 后重拉倒计时目标同步快照（iface 拉取式通道）
    /// 返回本轮消费条数（帧计数/节奏探针用）；logic(&mut self) 调用
    pub fn drain(&mut self, state: &mut UiState) -> usize {
        let mut n = 0;
        let mut need_reload = false;
        while let Ok(evt) = self.evt_rx.try_recv() {
            if matches!(evt, BackendEvent::ConfigSaved(_)) {
                need_reload = true;
            }
            state.apply(evt);
            n += 1;
        }
        if need_reload {
            state.countdown_targets = self.iface.get_countdown_targets();
            state.world_pins = self.iface.get_world_pins();
            state.base_timezone = Some(self.iface.get_ui_preferences().last_timezone);
        }
        n
    }

    /// 命令出口克隆（页/子部件需要独立 Sender 时；主出口仍走 send()）
    pub fn cmd_tx(&self) -> Sender<FrontendCommand> {
        self.cmd_tx.clone()
    }

    /// 启动期拉取偏好快照（同步查询，只此一次——后续以事件回推为准）
    pub fn initial_prefs(&self) -> UiPreferences {
        self.iface.get_ui_preferences()
    }
}

// ===== src/bus.rs 说明区 =====
// 输入：BackendEvent 流（后端事实）；输出：UiState 快照 + FrontendCommand 出口
// 逻辑步骤：drain 每帧 while try_recv → apply 逐变体更新 → 渲染层只读 state
// 设计理由：z.plan 修订一信号接口的 app 半边——UI 不直碰业务（页/控件只拿
//   &UiState 与 cmd 发送口），锁不跨帧（drain 在 update 首行完成）；Tick 节奏
//   由后端推（tick_ms 随倍率重排已在 interface 内），app 侧 next_repaint_interval
//   只做「取小 + 1s 兜底」（空闲期 egui 空转防护）
// 异常处理：send 通道满/关记 error 不 panic（UI 假死不应崩进程）
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
    fn theme_pref_mapping() {
        // 三态映射 + 非法回退 auto
        let mut state = UiState::default();
        state.prefs.theme = "dark".into();
        assert_eq!(state.theme_pref(), ThemePref::Dark);
        state.prefs.theme = "light".into();
        assert_eq!(state.theme_pref(), ThemePref::Light);
        state.prefs.theme = "whatever".into();
        assert_eq!(state.theme_pref(), ThemePref::Auto);
    }
}
