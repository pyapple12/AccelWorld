//! 应用接口层（interface/app_interface.py 的 Rust 对等物）
//! 七域契约 + 命令/事件总线：UI 访问后端的唯一入口；同步查询只读快照，
//! 一切写操作走命令——UI 不直接拿锁（z.plan 修订二·D 所有权模型）

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::data::{cities, timezones};
use crate::settings;
use crate::static_config::Static;
use crate::time_dilation::TimeDilation;
use crate::types::{BackendEvent, FrontendCommand, TimeInfo, UiPreferences, WeatherData};
use crate::weather::WeatherClient;

// 主题偏好合法取值（PL002.02 三态）
const THEME_CHOICES: [&str; 3] = ["auto", "light", "dark"];

// 信号总线通道容量（命令/事件各一路，背压上限）
const BUS_CAPACITY: usize = 64;

/// 应用接口（app 持有 Arc；内部全 Arc 可跨线程移入 tokio 任务）
pub struct AppInterface {
    world: Arc<Mutex<TimeDilation>>,
    weather: Arc<WeatherClient>,
    /// 用户配置文件路径（env ACCELWORLD_CONFIG_FILE 优先，否则 exe 同目录 config/）
    config_path: PathBuf,
}

impl Default for AppInterface {
    /// 默认构造（委托生产构造：clippy new_without_default）
    fn default() -> Self {
        Self::new()
    }
}

impl AppInterface {
    /// 生产构造：持久化倍率初始化世界（脏值/越界回退默认并记日志，FIX004.2 对等）
    pub fn new() -> Self {
        Self::with_config_path(Self::default_config_path())
    }

    /// 注入配置路径构造（测试隔离；配置缺省时以 default_rate 初始化）
    pub fn with_config_path(config_path: PathBuf) -> Self {
        let saved_rate = settings::load(&config_path).time_dilation_rate;
        let world = match TimeDilation::new(saved_rate) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("持久化倍率非法，已回退默认值: {saved_rate:?}（{e}）");
                TimeDilation::with_default_rate()
            }
        };
        AppInterface {
            world: Arc::new(Mutex::new(world)),
            weather: Arc::new(WeatherClient::new()),
            config_path,
        }
    }

    /// 注入天气客户端构造（httpmock 桩测试用）
    pub fn with_weather(mut self, client: WeatherClient) -> Self {
        self.weather = Arc::new(client);
        self
    }

    /// 默认配置路径：环境变量 ACCELWORLD_CONFIG_FILE 优先（子进程测试隔离，
    /// FIX001.12 同名对等），未设置时 exe 同目录 config/user_config.json
    fn default_config_path() -> PathBuf {
        if let Ok(p) = std::env::var("ACCELWORLD_CONFIG_FILE") {
            return PathBuf::from(p);
        }
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        exe_dir.join("config").join("user_config.json")
    }

    // ===== 静态配置域 =====

    /// base.json 静态参数域（版本/倍率范围/周期等 UI 所需键）
    pub fn app_static() -> serde_json::Value {
        Static::get().base.clone()
    }

    /// ui.json 静态样式 token 域（字体/颜色表）
    pub fn ui_static() -> serde_json::Value {
        Static::get().ui.clone()
    }

    /// 版本号单一来源（base.version）
    pub fn get_version() -> String {
        Static::get().version().to_string()
    }

    // ===== 时钟域 =====

    /// 标准/加速时间信息快照（真实时钟；测试用 world 直注 TimeDilation::time_info）
    pub fn get_time_info(&self) -> TimeInfo {
        self.world
            .lock()
            .expect("world 锁中毒")
            .time_info(Local::now())
    }

    /// 刷新周期毫秒（随倍率联动，T001.1）
    pub fn get_tick_interval_ms(&self) -> i64 {
        self.world.lock().expect("world 锁中毒").tick_interval_ms()
    }

    // ===== 倍率域 =====

    /// 当前生效倍率
    pub fn get_rate(&self) -> f64 {
        self.world.lock().expect("world 锁中毒").rate()
    }

    /// 倍率合法区间 (rate_min, rate_max)
    pub fn get_rate_bounds() -> (f64, f64) {
        let base = Static::get();
        (base.base_f64("rate_min"), base.base_f64("rate_max"))
    }

    /// 轻量路径：范围校验 + 世界重建，不落盘（滑杆拖动实时生效，FIX001.23）
    pub fn set_rate(&self, rate: f64) -> bool {
        let Ok(world) = TimeDilation::new(rate) else {
            return false;
        };
        *self.world.lock().expect("world 锁中毒") = world;
        true
    }

    /// 内聚动作：校验 + 重建 + 持久化（越界返回 false 不重建不落盘）
    pub fn apply_rate(&self, rate: f64) -> bool {
        if !self.set_rate(rate) {
            return false;
        }
        let mut config = settings::load(&self.config_path);
        config.time_dilation_rate = self.get_rate();
        settings::save(&self.config_path, &config)
    }

    // ===== 天气域 =====

    /// 城市名有序列表（Python sorted(CITIES.keys()) 对等：排序返回）
    pub fn get_city_names() -> Vec<String> {
        let mut names: Vec<String> = cities::names().into_iter().map(String::from).collect();
        names.sort();
        names
    }

    /// 城市天气查询（缓存/重试内聚；失败返回 None 不外抛——Python 降级语义对等）
    pub async fn fetch_weather(&self, city: &str, force: bool) -> Option<WeatherData> {
        match self.weather.fetch_by_city(city, force).await {
            Ok(w) => Some(w),
            Err(e) => {
                tracing::error!("获取天气信息失败: {city} {e}");
                None
            }
        }
    }

    /// 自动刷新周期 = 缓存 TTL × 1000（单源派生，E15）
    pub fn get_weather_refresh_interval_ms() -> i64 {
        Static::get().base_i64("weather_cache_ttl") * 1000
    }

    // ===== 时区域 =====

    /// 时区选项表（显示名, IANA 标识）
    pub fn get_timezone_options() -> Vec<(&'static str, &'static str)> {
        timezones::all()
            .iter()
            .map(|t| (t.display, t.iana))
            .collect()
    }

    /// 世界时钟常驻城市集（用户配置 world_pins）
    pub fn get_world_pins(&self) -> Vec<String> {
        settings::load(&self.config_path).world_pins
    }

    // ===== 配置偏好域 =====

    /// 组装 UI 会话偏好快照；主题归一化三态，非法值回退默认
    pub fn get_ui_preferences(&self) -> UiPreferences {
        let base = Static::get();
        let config = settings::load(&self.config_path);
        let theme = if THEME_CHOICES.contains(&config.theme.as_str()) {
            config.theme
        } else {
            base.base_str("default_theme").to_string()
        };
        UiPreferences {
            theme,
            last_city: config.last_city,
            last_timezone: config.last_timezone,
            countdown_target: config.countdown_target,
        }
    }

    /// 汇总落盘（SaveNow/退出保存路径：整体写回）
    pub fn save_config_now(&self) -> bool {
        let mut config = settings::load(&self.config_path);
        config.time_dilation_rate = self.get_rate();
        settings::save(&self.config_path, &config)
    }

    // ===== 信号总线（spawn：后台任务主循环）=====

    /// 启动后台任务：返回命令发送端 + 事件接收端
    /// 主循环 select 命令分发 / tick 定时推送；tick 周期每轮现读倍率（SetRate 即时生效）
    pub fn spawn(self: Arc<Self>) -> (Sender<FrontendCommand>, Receiver<BackendEvent>) {
        let (cmd_tx, mut cmd_rx) = mpsc::channel(BUS_CAPACITY);
        let (evt_tx, evt_rx) = mpsc::channel(BUS_CAPACITY);
        tokio::spawn(async move {
            loop {
                let period = Duration::from_millis(self.get_tick_interval_ms().max(1) as u64);
                tokio::select! {
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(FrontendCommand::SetRate(rate)) => {
                                if self.set_rate(rate) {
                                    let _ = evt_tx.send(BackendEvent::RateChanged(rate)).await;
                                }
                            }
                            Some(FrontendCommand::ApplyRate(rate)) => {
                                if self.set_rate(rate) {
                                    let saved = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::RateChanged(rate)).await;
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(saved)).await;
                                }
                            }
                            Some(FrontendCommand::RefreshWeather { city, force }) => {
                                // 独立任务拉取（重试/超时不阻塞主循环），完成回推事件
                                let weather = Arc::clone(&self.weather);
                                let evt_tx = evt_tx.clone();
                                tokio::spawn(async move {
                                    let evt = match weather.fetch_by_city(&city, force).await {
                                        Ok(data) => BackendEvent::WeatherUpdated { city, data },
                                        Err(e) => BackendEvent::WeatherFailed {
                                            city,
                                            reason: e.to_string(),
                                        },
                                    };
                                    let _ = evt_tx.send(evt).await;
                                });
                            }
                            Some(FrontendCommand::SaveNow) => {
                                let ok = self.save_config_now();
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::Quit) | None => break,
                            Some(other) => {
                                // 闹钟/倒计时/世界城市/主题域随 PL012/PL014 接线（协议 v1 已定形状）
                                tracing::warn!("命令待后续 PL 接线: {other:?}");
                            }
                        }
                    }
                    _ = tokio::time::sleep(period) => {
                        let info = self.get_time_info();
                        if evt_tx.send(BackendEvent::Tick(info)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
        (cmd_tx, evt_rx)
    }
}

// ===== src/interface.rs 说明区 =====
// 输入：FrontendCommand 流（UI 写意图）；输出：同步查询 DTO 快照 + BackendEvent 流
// 逻辑步骤：spawn 主循环 select——命令分支（SetRate/ApplyRate/RefreshWeather/
//   SaveNow/Quit 即期处理，其余域 warn 待接线）与 tick 分支（每轮现读倍率定周期，
//   SetRate 后下一轮即新节奏，等价「重排 interval」）并发
// 设计理由：对齐 Python AppInterface 七域契约方法名（get_time_info/get_rate/
//   set_rate/apply_rate/get_city_names/fetch_weather/get_ui_preferences/...），
//   Python 拉取式 + Rust 推送式并存：同步查询供 UI 直拉（Python 习惯对等物），
//   总线供帧循环 drain（z.plan 修订一信号接口）；tick 用 sleep-每轮现读而非
//   固定 interval（tokio interval 周期不可变，重建即重排）；天气拉取独立
//   tokio 任务（重试/超时最长秒级，不得阻塞命令通道）
// 异常处理：倍率非法回退默认（构造期）/拒绝 false（set/apply）；天气失败降级
//   None + 日志（Python 对等），总线事件则回推 WeatherFailed 供 UI 提示；
//   事件通道关闭（UI 已死）时退出主循环
// 已知偏移：①fetch_weather 的 force 清该城缓存键（Python 清全表；z.plan 修订二
//   per-city 定案，多城市语义更正确）；②配置读经每调用 load（Python 有模块级
//   缓存；Rust 由调用频率低 + 原子写保证一致性，缓存层 PL013 视性能引入）
// 关联配置：config_path（env 注入/exe 同目录）；base.json 各 token；消费方：app bus
#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::UserConfig;
    use crate::util::retry::RetrySpec;
    use std::fs;

    fn temp_config(tag: &str) -> PathBuf {
        // 每测试独立目录（并行测试同进程不得共用配置路径）
        let dir = std::env::temp_dir().join(format!("accel_iface_{tag}_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("临时目录创建失败");
        dir.join("user_config.json")
    }

    fn iface(path: &Path) -> AppInterface {
        AppInterface::with_config_path(path.to_path_buf())
    }

    #[test]
    fn three_domain_sync_queries() {
        // 三域拉取走通：时钟快照形态/倍率域/天气派生周期
        let path = temp_config("sync");
        let itf = iface(&path);
        let info = itf.get_time_info();
        assert!(info.custom_time.contains(':'), "加速时间应为 HH:MM:SS 形态");
        assert_eq!(info.day_hours, 48, "默认倍率 2.0 → 48 时日长");
        assert_eq!(itf.get_rate(), 2.0);
        assert_eq!(AppInterface::get_rate_bounds(), (1.0, 20.0));
        assert_eq!(AppInterface::get_weather_refresh_interval_ms(), 1_800_000);
        assert_eq!(AppInterface::get_version(), "0.6.0.0");
    }

    #[test]
    fn rate_set_and_apply_paths() {
        // set_rate 轻量路径（不落盘）与 apply_rate 内聚路径（落盘可复读）
        let path = temp_config("rate");
        let itf = iface(&path);
        assert!(itf.set_rate(5.5));
        assert_eq!(itf.get_rate(), 5.5);
        assert_eq!(itf.get_tick_interval_ms(), 100);
        // 越界拒绝：状态不变
        assert!(!itf.set_rate(99.0));
        assert_eq!(itf.get_rate(), 5.5);
        // 未 apply 前不落盘
        assert_eq!(settings::load(&path).time_dilation_rate, 2.0);
        assert!(itf.apply_rate(3.0));
        assert_eq!(itf.get_rate(), 3.0);
        assert_eq!(settings::load(&path).time_dilation_rate, 3.0);
    }

    #[test]
    fn city_names_sorted_and_preferences() {
        // get_city_names 排序返回；偏好组装与主题归一化
        let path = temp_config("prefs");
        let names = AppInterface::get_city_names();
        assert_eq!(names.len(), 17);
        let mut sorted_check = names.clone();
        sorted_check.sort();
        assert_eq!(names, sorted_check, "城市名应排序返回（Python parity）");
        assert!(names.contains(&"北京".to_string()));

        let config = UserConfig {
            theme: "purple".into(),
            last_city: "上海".into(),
            ..UserConfig::default()
        };
        settings::save(&path, &config);
        let itf = iface(&path);
        let prefs = itf.get_ui_preferences();
        assert_eq!(prefs.theme, "auto", "非法主题应归一化回退默认");
        assert_eq!(prefs.last_city, "上海");
        assert_eq!(itf.get_world_pins().len(), 8);
        let tz = AppInterface::get_timezone_options();
        assert_eq!(tz.len(), 8);
        assert_eq!(tz[0], ("北京 (UTC+8)", "Asia/Shanghai"));
    }

    #[test]
    fn persisted_dirty_rate_falls_back() {
        // 持久化倍率脏值回退默认（FIX004.2 对等：clamp 域外不崩溃）
        let path = temp_config("dirty");
        let config = UserConfig {
            time_dilation_rate: 99.0,
            ..UserConfig::default()
        };
        settings::save(&path, &config);
        let itf = iface(&path);
        assert_eq!(itf.get_rate(), 2.0, "域外持久化倍率应回退 default_rate");
    }

    #[tokio::test]
    async fn bus_rate_and_quit_flow() {
        // 总线：SetRate → RateChanged；Quit → 循环退出（事件通道关闭）
        let path = temp_config("busrate");
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::SetRate(7.0))
            .await
            .expect("发送失败");
        match evt_rx.recv().await {
            Some(BackendEvent::RateChanged(r)) => assert_eq!(r, 7.0),
            other => panic!("首个事件应为 RateChanged: {other:?}"),
        }
        assert_eq!(itf.get_rate(), 7.0);
        cmd_tx.send(FrontendCommand::Quit).await.expect("发送失败");
        // Quit 后循环退出，事件通道随之关闭（先可能收到残余 Tick）
        let mut closed = false;
        for _ in 0..64 {
            if evt_rx.recv().await.is_none() {
                closed = true;
                break;
            }
        }
        assert!(closed, "Quit 后事件通道应关闭");
    }

    #[tokio::test]
    async fn bus_tick_stream_flows() {
        // 总线：tick 推送按倍率节奏到达（默认 2.0 → 100ms 周期，2 秒内必有多个）
        let path = temp_config("bustick");
        let itf = Arc::new(iface(&path));
        let (_cmd_tx, mut evt_rx) = AppInterface::spawn(itf);
        let mut ticks = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline && ticks < 3 {
            if matches!(evt_rx.recv().await, Some(BackendEvent::Tick(_))) {
                ticks += 1;
            }
        }
        assert!(ticks >= 3, "2 秒内应收到 ≥3 个 Tick，实际 {ticks}");
    }

    #[tokio::test]
    async fn bus_weather_event_with_stub() {
        // 总线：RefreshWeather → WeatherUpdated（httpmock 桩 + 注入客户端）
        use httpmock::{Method, MockServer};
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(Method::GET).path("/v1/forecast");
            then.status(200).json_body(serde_json::json!({"current": {
                "temperature_2m": 18.0, "relative_humidity_2m": 40, "weather_code": 0,
                "wind_speed_10m": 5.0, "apparent_temperature": 17.5 }}));
        });
        let path = temp_config("busweather");
        let client = WeatherClient::custom(
            server.base_url(),
            RetrySpec {
                retries: 2,
                delay: Duration::ZERO,
            },
            Duration::from_secs(5),
        );
        let itf = Arc::new(iface(&path).with_weather(client));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(itf);
        cmd_tx
            .send(FrontendCommand::RefreshWeather {
                city: "北京".into(),
                force: false,
            })
            .await
            .expect("发送失败");
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut got = false;
        while std::time::Instant::now() < deadline {
            match evt_rx.recv().await {
                Some(BackendEvent::WeatherUpdated { city, data }) => {
                    assert_eq!(city, "北京");
                    assert_eq!(data.weather, "晴");
                    got = true;
                    break;
                }
                Some(_) => {}
                None => break,
            }
        }
        assert!(got, "应收到 WeatherUpdated");
    }
}
