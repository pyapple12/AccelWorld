//! 应用接口层（interface/app_interface.py 的 Rust 对等物）
//! 七域契约 + 命令/事件总线：UI 访问后端的唯一入口；同步查询只读快照，
//! 一切写操作走命令——UI 不直接拿锁（z.plan 修订二·D 所有权模型）

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::alarm::{Alarm, AlarmManager, Clock, RealClock};
use crate::data::{cities, timezones, world_cities};
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
    alarms: Arc<Mutex<AlarmManager>>,
    /// 用户配置文件路径（env ACCELWORLD_CONFIG_FILE 优先，否则 exe 同目录 config/）
    config_path: PathBuf,
    /// 天气在飞任务表（FIX006.9：同城去抖 + Quit 统一 abort；键 = 城市名）
    weather_inflight: Arc<Mutex<WeatherInflight>>,
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
    /// 闹钟随构造自配置装载（Python 由 UI 显式 load_alarm_dicts，Rust 收敛为构造内聚）
    pub fn with_config_path(config_path: PathBuf) -> Self {
        // A005 #18：单次 load 复用（原实现对同一路径 load 两遍——重复磁盘 IO）
        let config = settings::load(&config_path);
        let world = match TimeDilation::new(config.time_dilation_rate) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!(
                    "持久化倍率非法，已回退默认值: {:?}（{e}）",
                    config.time_dilation_rate
                );
                TimeDilation::with_default_rate()
            }
        };
        let mut manager = AlarmManager::new();
        manager.from_value_list(&config.alarms);
        AppInterface {
            world: Arc::new(Mutex::new(world)),
            weather: Arc::new(WeatherClient::new()),
            alarms: Arc::new(Mutex::new(manager)),
            config_path,
            weather_inflight: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// 注入天气客户端构造（httpmock 桩测试用）
    pub fn with_weather(mut self, client: WeatherClient) -> Self {
        self.weather = Arc::new(client);
        self
    }

    /// 默认配置路径：环境变量 ACCELWORLD_CONFIG_FILE 优先（子进程测试隔离，
    /// FIX001.12 同名对等），未设置时 exe 同目录 config/user_config.json
    /// FIX007.15：基准统一 util::paths::exe_dir（与日志同源，不再 CWD/exe 双轨）
    fn default_config_path() -> PathBuf {
        if let Ok(p) = std::env::var("ACCELWORLD_CONFIG_FILE") {
            return PathBuf::from(p);
        }
        // A005 #8：配置相对路径取 base.json 的 user_config token（单源，不再硬编码同值双轨）
        crate::util::paths::exe_dir().join(Static::get().base_str("user_config"))
    }

    // ===== 静态配置域 =====

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
    /// 天气城市候选（FIX006.23：天气域城市表 = data::cities（17 城坐标表），
    /// 与世界时钟域的 world_cities（50 城）职责分离）
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

    /// 切换天气城市（A005 #4 单城模型的城市选择）：城市须在天气城市表内，持久化 last_city
    /// 输入：城市名；输出：是否落盘成功（未知城市拒 false；同名视为已达成 true）
    /// FIX006.22：幂等规则统一为「已存在 = true（不重复落盘）；非法入参 = false」
    pub fn set_weather_city(&self, city: &str) -> bool {
        if cities::get(city).is_none() {
            tracing::warn!("未知天气城市，拒绝设置: {city}");
            return false;
        }
        let mut config = settings::load(&self.config_path);
        if config.last_city == city {
            return true;
        }
        config.last_city = city.to_string();
        settings::save(&self.config_path, &config)
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

    /// 汇总落盘（SaveNow/退出保存路径：整体写回，含闹钟列表）
    pub fn save_config_now(&self) -> bool {
        let mut config = settings::load(&self.config_path);
        config.time_dilation_rate = self.get_rate();
        config.alarms = self.alarms.lock().expect("闹钟锁中毒").to_value_list();
        settings::save(&self.config_path, &config)
    }

    // ===== 闹钟域（PL012.10：CRUD 转发 + 持久化，Python AlarmManager 所有权在接口）=====

    /// 当前闹钟列表（浅拷贝快照）
    pub fn get_alarms(&self) -> Vec<Alarm> {
        self.alarms.lock().expect("闹钟锁中毒").alarms.clone()
    }

    /// 按 ID 查找
    pub fn get_alarm(&self, alarm_id: &str) -> Option<Alarm> {
        self.alarms
            .lock()
            .expect("闹钟锁中毒")
            .get_alarm(alarm_id)
            .cloned()
    }

    /// 闹钟数量上限
    pub fn get_max_alarms() -> usize {
        Static::get().base_i64("max_alarms") as usize
    }

    /// 触发检查（测试/调度共用入口；Python check_alarms 转发）
    pub fn check_alarms(&self, now: chrono::DateTime<Local>) -> Vec<Alarm> {
        self.alarms.lock().expect("闹钟锁中毒").check_alarms(now)
    }

    // ===== 城市域（PL012.10：城市表检索 + 常驻城市持久化）=====

    /// 世界时钟城市表全量名（添加面板候选；世界时钟专用表，天气候选另用 data::cities）
    pub fn get_world_cities() -> Vec<&'static str> {
        world_cities::all_names()
    }

    /// 添加常驻城市（入参与落盘同为城市名——FIX007.2 域统一：原 IANA 存储丢失
    /// 同 tz 多城身份（加入大阪显示东京/加入上海被已常驻的北京吞掉）；城市名即
    /// 身份，走时经 tz_of 映射 IANA；未知城市拒绝）
    /// FIX006.22：幂等规则统一——已存在 = true（补日志非静默）；非法入参 = false
    pub fn add_world_city(&self, name: &str) -> bool {
        if world_cities::by_name(name).is_none() {
            tracing::warn!("未知城市，拒绝添加常驻: {name}");
            return false;
        }
        let mut config = settings::load(&self.config_path);
        if config.world_pins.iter().any(|p| p == name) {
            // FIX006.22：已存在 = 幂等达成（true，不重复落盘）——与 set_weather_city 同语义
            tracing::info!("常驻城市已存在，跳过重复添加: {name}");
            return true;
        }
        config.world_pins.push(name.to_string());
        settings::save(&self.config_path, &config)
    }

    /// 移除常驻城市（入参域与添加侧统一为城市名；不存在记日志返 false——
    /// 移除类无变更可报，与「已存在 = true」互补）
    pub fn remove_world_city(&self, name: &str) -> bool {
        let mut config = settings::load(&self.config_path);
        let before = config.world_pins.len();
        config.world_pins.retain(|p| p != name);
        if config.world_pins.len() == before {
            tracing::info!("常驻城市不存在，移除无变更: {name}");
            return false;
        }
        settings::save(&self.config_path, &config)
    }

    /// 设置基准时区（合法 IANA 且在世界时钟城市表内才收；roundtrip 持久化）
    pub fn set_base_timezone(&self, tz: &str) -> bool {
        if world_cities::all().iter().all(|c| c.tz != tz) {
            tracing::warn!("未知时区标识，拒绝设置: {tz}");
            return false;
        }
        let mut config = settings::load(&self.config_path);
        config.last_timezone = tz.to_string();
        settings::save(&self.config_path, &config)
    }

    // ===== 窗口几何域（UIR.7：位置/大小记忆）=====

    /// 窗口几何存档读取（"x,y,w,h" 形态；未存档 None——启动恢复依据）
    pub fn get_window_geometry(&self) -> Option<String> {
        settings::load(&self.config_path).window_geometry
    }

    /// 窗口几何存档（UI 直接发 SetWindowGeometry；合法数字四元组才收）
    pub fn set_window_geometry(&self, geometry: &str) -> bool {
        if parse_window_geometry(geometry).is_none() {
            tracing::warn!("窗口几何串非法，拒绝存档: {geometry}");
            return false;
        }
        let mut config = settings::load(&self.config_path);
        config.window_geometry = Some(geometry.to_string());
        settings::save(&self.config_path, &config)
    }

    // ===== 倒计时目标域（PL014.06/.08：列表化多目标持久化）=====

    /// 当前倒计时目标列表（同步快照，按 at 升序由页面层排序）
    pub fn get_countdown_targets(&self) -> Vec<crate::settings::CountdownTarget> {
        settings::load(&self.config_path).countdown_targets
    }

    /// 添加倒计时目标（同名拒/空名拒；at 转存储串落盘）
    pub fn add_countdown_target(&self, name: &str, at: &chrono::DateTime<Local>) -> bool {
        if name.is_empty() {
            tracing::warn!("倒计时目标名为空，拒绝添加");
            return false;
        }
        let mut config = settings::load(&self.config_path);
        if config.countdown_targets.iter().any(|t| t.name == name) {
            tracing::warn!("同名倒计时目标已存在，拒绝添加: {name}");
            return false;
        }
        config
            .countdown_targets
            .push(crate::settings::CountdownTarget {
                name: name.to_string(),
                at: at.format("%Y-%m-%d %H:%M:%S").to_string(),
            });
        settings::save(&self.config_path, &config)
    }

    /// 移除倒计时目标（按名；不存在返 false）
    pub fn remove_countdown_target(&self, name: &str) -> bool {
        let mut config = settings::load(&self.config_path);
        let before = config.countdown_targets.len();
        config.countdown_targets.retain(|t| t.name != name);
        if config.countdown_targets.len() == before {
            return false;
        }
        settings::save(&self.config_path, &config)
    }

    // ===== 信号总线（spawn：后台任务主循环 + 闹钟调度）=====

    /// 启动后台任务（真实时钟）：命令分发主循环 + tick 推送 + 闹钟调度
    pub fn spawn(self: Arc<Self>) -> (Sender<FrontendCommand>, Receiver<BackendEvent>) {
        Self::spawn_with_clock(self, Arc::new(RealClock))
    }

    /// 启动后台任务（注入时钟——测试拨钟入口）
    /// 主循环 select 命令分发 / tick 定时推送（周期每轮现读倍率，SetRate 即时生效）；
    /// 闹钟调度独立任务按 alarm_check_ms 轮询：命中发 AlarmTriggered + 一次性自动
    /// 禁用 + AlarmsChanged 回推（PL012.07 编排定案）
    pub fn spawn_with_clock(
        self: Arc<Self>,
        clock: Arc<dyn Clock>,
    ) -> (Sender<FrontendCommand>, Receiver<BackendEvent>) {
        let (cmd_tx, mut cmd_rx) = mpsc::channel(BUS_CAPACITY);
        let (evt_tx, evt_rx) = mpsc::channel(BUS_CAPACITY);
        // 关停信号（watch 保态）：Quit 杀主循环同时唤醒调度任务，杜绝 evt_tx 悬挂
        let (sd_tx, sd_rx) = tokio::sync::watch::channel(false);

        // 闹钟调度任务：固定周期轮询（去重由 manager 同分钟键保证）
        // panic 监控包裹：join Err 记 error 不静默（PL013.12 检验项）
        {
            let itf = Arc::clone(&self);
            let evt_tx = evt_tx.clone();
            let clock = Arc::clone(&clock);
            let mut sd_rx = sd_rx.clone();
            let handle = tokio::spawn(async move {
                let check_ms = Static::get().base_i64("alarm_check_ms").max(1) as u64;
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(check_ms)) => {
                            let now = clock.now();
                            let hits = itf.check_alarms(now);
                            for alarm in hits {
                                // 一次性触发后自动禁用（列表回推供 UI 刷新）
                                let changed = if alarm.is_one_time() {
                                    itf.alarms.lock().expect("闹钟锁中毒").toggle_alarm(&alarm.id)
                                } else {
                                    false
                                };
                                if evt_tx.send(BackendEvent::AlarmTriggered(alarm)).await.is_err() {
                                    return;
                                }
                                if changed {
                                    let _ = evt_tx
                                        .send(BackendEvent::AlarmsChanged(itf.get_alarms()))
                                        .await;
                                }
                            }
                        }
                        _ = sd_rx.changed() => return,
                    }
                }
            });
            tokio::spawn(async move {
                if let Err(e) = handle.await {
                    tracing::error!("闹钟调度任务 panic 中止: {e}");
                }
            });
        }

        // 主循环任务（panic 监控包裹：join Err 记 error 不静默——PL013.12 检验项）
        let main_handle = tokio::spawn(async move {
            loop {
                let period = Duration::from_millis(self.get_tick_interval_ms().max(1) as u64);
                tokio::select! {
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(FrontendCommand::SetRate(rate)) => {
                                if self.set_rate(rate) {
                                    let _ = evt_tx.send(BackendEvent::RateChanged(rate)).await;
                                } else {
                                    // A005 #19：拒绝同样回推（UI 回退乐观更新，不静默偏离）
                                    let _ = evt_tx.send(BackendEvent::RateRejected(rate)).await;
                                }
                            }
                            Some(FrontendCommand::ApplyRate(rate)) => {
                                if self.set_rate(rate) {
                                    let saved = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::RateChanged(rate)).await;
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(saved)).await;
                                } else {
                                    let _ = evt_tx.send(BackendEvent::RateRejected(rate)).await;
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(false)).await;
                                }
                            }
                            Some(FrontendCommand::RefreshWeather { city, force }) => {
                                // FIX006.9：同城在飞去抖（force 强制刷新则取消旧任务重发）；
                                // FIX007.5：替换旧条目时置 superseded 标记——abort 不生效
                                // （旧任务已越过最后 await）时任务尾依标记跳过自摘，不再
                                // 按城市键误删新任务条目；Quit 时统一 abort（不留 evt_tx 克隆）
                                let mut inflight = self
                                    .weather_inflight
                                    .lock()
                                    .expect("天气在飞表锁中毒");
                                if weather_should_skip(&inflight, &city, force) {
                                    tracing::debug!("天气请求在飞，跳过重复触发: {city}");
                                } else {
                                    // 本任务的替换标记：force 重新触发时被置位（身份凭证）
                                    let superseded = Arc::new(std::sync::atomic::AtomicBool::new(false));
                                    inflight_replace(&mut inflight, &city);
                                    let weather = Arc::clone(&self.weather);
                                    let evt_tx = evt_tx.clone();
                                    let inflight_map = Arc::clone(&self.weather_inflight);
                                    let city_task = city.clone();
                                    let superseded_flag = Arc::clone(&superseded);
                                    // FIX006.4/.9 配套可观测性：拉取开始即留痕（冷启动首拉可见）
                                    tracing::info!("天气拉取开始: {city}（force={force}）");
                                    let handle = tokio::spawn(async move {
                                        let result = weather.fetch_by_city(&city_task, force).await;
                                        let evt = match result {
                                            Ok(data) => BackendEvent::WeatherUpdated {
                                                city: city_task.clone(),
                                                data,
                                            },
                                            Err(e) => BackendEvent::WeatherFailed {
                                                city: city_task.clone(),
                                                reason: e.to_string(),
                                            },
                                        };
                                        match &evt {
                                            BackendEvent::WeatherUpdated { city, .. } => {
                                                tracing::info!("天气更新完成: {city}");
                                            }
                                            BackendEvent::WeatherFailed { city, reason } => {
                                                tracing::warn!("天气获取失败: {city}（{reason}）");
                                            }
                                            _ => {}
                                        }
                                        let _ = evt_tx.send(evt).await;
                                        let mut map =
                                            inflight_map.lock().expect("天气在飞表锁中毒");
                                        inflight_self_remove(&mut map, &city_task, &superseded_flag);
                                    });
                                    inflight.insert(city, (handle, superseded));
                                }
                            }
                            Some(FrontendCommand::AddAlarm(draft)) => {
                                // 草稿落库 + 入表 + 回推全量列表（UI 无状态重建）
                                let ok = Alarm::from_draft(draft)
                                    .map(|alarm| self.alarms.lock().expect("闹钟锁中毒").add_alarm(alarm))
                                    .unwrap_or(false);
                                if ok {
                                    let _ = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::AlarmsChanged(self.get_alarms())).await;
                                }
                            }
                            Some(FrontendCommand::RemoveAlarm(id)) => {
                                let removed =
                                    self.alarms.lock().expect("闹钟锁中毒").remove_alarm(&id);
                                if removed {
                                    let _ = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::AlarmsChanged(self.get_alarms())).await;
                                }
                            }
                            Some(FrontendCommand::ReplaceAlarm(alarm)) => {
                                let ok = self.alarms.lock().expect("闹钟锁中毒").replace_alarm(alarm);
                                if ok {
                                    let _ = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::AlarmsChanged(self.get_alarms())).await;
                                }
                            }
                            Some(FrontendCommand::ToggleAlarm(id)) => {
                                let ok = self.alarms.lock().expect("闹钟锁中毒").toggle_alarm(&id);
                                if ok {
                                    let _ = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::AlarmsChanged(self.get_alarms())).await;
                                }
                            }
                            Some(FrontendCommand::SetWeatherCity(city)) => {
                                let ok = self.set_weather_city(&city);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::AddWorldCity(name)) => {
                                let ok = self.add_world_city(&name);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::RemoveWorldCity(name)) => {
                                let ok = self.remove_world_city(&name);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::SetBaseTimezone(tz)) => {
                                let ok = self.set_base_timezone(&tz);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::SetWindowGeometry { geometry }) => {
                                let ok = self.set_window_geometry(&geometry);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::AddCountdownTarget { name, at }) => {
                                let ok = self.add_countdown_target(&name, &at);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::RemoveCountdownTarget(name)) => {
                                let ok = self.remove_countdown_target(&name);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::SetTheme(pref)) => {
                                let theme = match pref {
                                    crate::types::ThemePref::Auto => "auto",
                                    crate::types::ThemePref::Light => "light",
                                    crate::types::ThemePref::Dark => "dark",
                                };
                                let mut config = settings::load(&self.config_path);
                                config.theme = theme.to_string();
                                let ok = settings::save(&self.config_path, &config);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::SaveNow) => {
                                let ok = self.save_config_now();
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::Quit) | None => {
                                // 关停调度任务后主循环退出（evt_tx 全部释放 → 接收端闭合）
                                // FIX006.9：先 abort 在飞天气任务——否则其 evt_tx 克隆会推迟闭合
                                let mut inflight = self
                                    .weather_inflight
                                    .lock()
                                    .expect("天气在飞表锁中毒");
                                inflight_abort_all(&mut inflight);
                                let _ = sd_tx.send(true);
                                break;
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
        // 主循环 panic 监控（join Err 记 error 不静默）
        tokio::spawn(async move {
            if let Err(e) = main_handle.await {
                tracing::error!("命令主循环任务 panic 中止: {e}");
            }
        });
        (cmd_tx, evt_rx)
    }
}

/// 天气在飞任务表：句柄 + 替换标记（FIX007.5：标记即任务身份凭证——force 替换时
/// 置位，旧任务任务尾依标记跳过自摘，防误删新条目）
type WeatherInflight = std::collections::HashMap<
    String,
    (
        tokio::task::JoinHandle<()>,
        std::sync::Arc<std::sync::atomic::AtomicBool>,
    ),
>;

/// 天气去抖判定（FIX006.9 纯逻辑）：同城在飞且非强制刷新 → 跳过重复触发
fn weather_should_skip(inflight: &WeatherInflight, city: &str, force: bool) -> bool {
    !force && inflight.contains_key(city)
}

/// 窗口几何串解析（纯函数）：`"x,y,w,h"` → 四元组（非数字/缺项 None——启动恢复
/// 回退 token 基准）；旧 Python base64 存档天然解析失败即弃档（一次性迁移）
pub fn parse_window_geometry(text: &str) -> Option<(f32, f32, f32, f32)> {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let nums: Vec<f32> = parts
        .iter()
        .map(|p| p.trim().parse::<f32>().ok())
        .collect::<Option<Vec<_>>>()?;
    Some((nums[0], nums[1], nums[2], nums[3]))
}

/// force 替换旧在飞条目（FIX007.5）：置替换标记 + abort——abort 不生效时标记兜底
fn inflight_replace(map: &mut WeatherInflight, city: &str) {
    if let Some((handle, flag)) = map.remove(city) {
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
        handle.abort();
    }
}

/// 任务完成自摘（FIX007.5）：仅未被替换的任务可自摘（原按城市键无条件 remove，
/// 旧任务完成会误删 force 换上的新条目——去抖短暂失效）
fn inflight_self_remove(
    map: &mut WeatherInflight,
    city: &str,
    superseded: &std::sync::atomic::AtomicBool,
) {
    if !superseded.load(std::sync::atomic::Ordering::Relaxed) {
        map.remove(city);
    }
}

/// Quit 收尾：abort 全部在飞任务并清表（不留 evt_tx 克隆，接收端即时闭合）
fn inflight_abort_all(map: &mut WeatherInflight) {
    for (_, (handle, _)) in map.drain() {
        handle.abort();
    }
}

// ===== src/interface.rs 说明区 =====
// 输入：FrontendCommand 流（UI 写意图）；输出：同步查询 DTO 快照 + BackendEvent 流
// 逻辑步骤：spawn 主循环 select——命令分支（16 变体逐一接线，match 穷尽无兜底——
//   A005 #4 起新增变体不接线即编译失败）与 tick 分支（每轮现读倍率定周期，
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
// FIX006.9：天气拉取任务入「在飞表」（同城去抖，force 取消旧任务重发；任务完成自摘；
//   Quit 统一 abort——不留 evt_tx 克隆，接收端即时闭合）；FIX007.5：自摘按替换标记
//   判定任务身份，旧任务完成不再误删 force 换上的新条目
// FIX006.22：写操作幂等规则统一——「已存在 = true（不重复落盘）」/「非法入参 = false」/
//   「移除类不存在 = false」；FIX006.23：天气城市候选 = data::cities（与世界时钟
//   world_cities 两表职责分离）
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
    use std::path::Path;

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
        // 版本经 base.json 单源转出（四段式形态；值本体不在代码内）
        let version = AppInterface::get_version();
        assert_eq!(
            version.split('.').count(),
            4,
            "版本应为四段式，实际 {version}"
        );
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
            Duration::from_secs(3600),
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

    // ===== PL012.07/.10：闹钟调度（VirtualClock）与七域接线 =====

    use crate::alarm::{AlarmDraft, VirtualClock};
    use crate::types::ThemePref;

    #[tokio::test]
    async fn virtual_clock_alarm_triggers_exactly_once() {
        // .07 验收：VirtualClock 拨至触发时刻恰收一次；同分钟重拨不重发；
        // 一次性触发后列表回推 enabled==false（固定日 2026-06-03 周三）
        use chrono::TimeZone;
        let at = |h: u32, m: u32| {
            let naive = chrono::NaiveDate::from_ymd_opt(2026, 6, 3)
                .expect("日期合法")
                .and_hms_opt(h, m, 0)
                .expect("时刻合法");
            Local.from_local_datetime(&naive).single().expect("无歧义")
        };
        let fire = at(9, 0);
        let before = at(8, 59);

        let path = temp_config("vclock");
        let itf = Arc::new(iface(&path));
        // 两条闹钟直入管理器：重复型（周三 [2]）测去重；一次性（created_at 昨日 07:00
        // → 计划日 = 今日）测自动禁用回推
        {
            let mut m = itf.alarms.lock().expect("闹钟锁中毒");
            m.add_alarm(
                crate::alarm::Alarm::from_json_value(&serde_json::json!({
                    "label": "重复", "time": "09:00", "repeat_days": [2],
                    "enabled": true, "id": "rep1", "created_at": "2026-06-01T07:00:00",
                }))
                .expect("装载"),
            );
            m.add_alarm(
                crate::alarm::Alarm::from_json_value(&serde_json::json!({
                    "label": "一次", "time": "09:00", "repeat_days": [],
                    "enabled": true, "id": "once1", "created_at": "2026-06-02T09:30:00",
                }))
                .expect("装载"),
            );
        }
        // 共享虚拟时钟原位拨钟：单次 spawn，调度任务持续读 clock.now()
        let shared = std::sync::Arc::new(std::sync::Mutex::new(before));
        let clock: Arc<dyn crate::alarm::Clock> =
            std::sync::Arc::new(VirtualClock(std::sync::Arc::clone(&shared)));
        let (_cmd, mut evt) = AppInterface::spawn_with_clock(Arc::clone(&itf), clock);

        // 阶段 1：拨至 08:59，一轮调度周期内（alarm_check_ms=1000）无触发
        *shared.lock().expect("拨钟") = before;
        let deadline = std::time::Instant::now() + Duration::from_millis(1300);
        let mut early_fire = 0;
        while std::time::Instant::now() < deadline {
            if matches!(evt.try_recv(), Ok(BackendEvent::AlarmTriggered(_))) {
                early_fire += 1;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(early_fire, 0, "触发前一分钟不应触发");

        // 阶段 2：拨至 09:00 → 两闹钟各恰触发一次 + 一次性禁用回推
        *shared.lock().expect("拨钟") = fire;
        let (mut fired_rep, mut fired_once, mut disabled_push) = (0, 0, false);
        let deadline = std::time::Instant::now() + Duration::from_secs(4);
        while std::time::Instant::now() < deadline {
            match evt.try_recv() {
                Ok(BackendEvent::AlarmTriggered(a)) => {
                    if a.id == "rep1" {
                        fired_rep += 1
                    } else {
                        fired_once += 1
                    }
                }
                Ok(BackendEvent::AlarmsChanged(list)) => {
                    if list.iter().any(|x| x.id == "once1" && !x.enabled) {
                        disabled_push = true;
                    }
                }
                Ok(_) => {}
                Err(_) => {}
            }
            if fired_rep >= 1 && fired_once >= 1 && disabled_push {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(fired_rep, 1, "重复闹钟应恰触发一次");
        assert_eq!(fired_once, 1, "一次性闹钟应恰触发一次");
        assert!(disabled_push, "一次性触发后应回推禁用列表");
        assert!(
            !itf.get_alarm("once1").expect("在表").enabled,
            "一次性应已禁用"
        );
        assert!(
            itf.get_alarm("rep1").expect("在表").enabled,
            "重复闹钟保持启用"
        );

        // 阶段 3：同分钟重拨（08:59 → 09:00）不得重发（去重键含日期+分钟）
        *shared.lock().expect("拨钟") = before;
        tokio::time::sleep(Duration::from_millis(1200)).await;
        *shared.lock().expect("拨钟") = fire;
        let deadline = std::time::Instant::now() + Duration::from_millis(1500);
        let mut refire = 0;
        while std::time::Instant::now() < deadline {
            if matches!(evt.try_recv(), Ok(BackendEvent::AlarmTriggered(_))) {
                refire += 1;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(refire, 0, "同分钟重拨不得重复触发");
    }

    #[tokio::test]
    async fn bus_alarm_crud_and_persistence() {
        // .10 验收：AddAlarm→get_alarms 反查 + RemoveAlarm 回推 + 落盘可复载
        let path = temp_config("acrud");
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::AddAlarm(AlarmDraft {
                label: " CRUD".trim().into(),
                time: "07:30".into(),
                sound_type: "preset".into(),
                sound_value: "classic".into(),
                repeat_days: vec![1, 2],
                enabled: true,
            }))
            .await
            .expect("发送失败");
        let mut listed = None;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(BackendEvent::AlarmsChanged(list)) = evt_rx.recv().await {
                listed = Some(list);
                break;
            }
        }
        let list = listed.expect("应回推列表");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].time, "07:30");
        assert_eq!(itf.get_alarms().len(), 1, "同步反查一致");
        // 落盘复载
        let config = settings::load(&path);
        assert_eq!(config.alarms.len(), 1);
        // 删除回推空列表
        let id = list[0].id.clone();
        cmd_tx
            .send(FrontendCommand::RemoveAlarm(id))
            .await
            .expect("发送失败");
        let mut emptied = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(BackendEvent::AlarmsChanged(l)) = evt_rx.recv().await {
                emptied = l.is_empty();
                break;
            }
        }
        assert!(emptied, "删除后应回推空列表");
    }

    #[tokio::test]
    async fn bus_city_and_theme_commands() {
        // .10 验收 + FIX007.2 域验收：AddWorldCity 按城市名落盘（大阪独立于东京——
        // 同 tz 多城共存，不再被表序首城吞并）+ 重复添加幂等 + SetBaseTimezone
        // roundtrip + SetTheme 落盘
        let path = temp_config("citycmd");
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::AddWorldCity("大阪".into()))
            .await
            .expect("发送失败");
        cmd_tx
            .send(FrontendCommand::SetBaseTimezone("Asia/Tokyo".into()))
            .await
            .expect("发送失败");
        cmd_tx
            .send(FrontendCommand::SetTheme(ThemePref::Dark))
            .await
            .expect("发送失败");
        let mut saved_count = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline && saved_count < 3 {
            if matches!(evt_rx.recv().await, Some(BackendEvent::ConfigSaved(true))) {
                saved_count += 1;
            }
        }
        assert_eq!(saved_count, 3, "三条命令均应 ConfigSaved(true)");
        let config = settings::load(&path);
        assert!(
            config.world_pins.contains(&"大阪".to_string()),
            "大阪应以城市名落盘（FIX007.2 域统一）"
        );
        assert!(
            config.world_pins.contains(&"东京".to_string()),
            "同 tz 的东京身份保留（默认常驻集不受大阪吞并）"
        );
        assert_eq!(config.last_timezone, "Asia/Tokyo", "时区 roundtrip");
        assert_eq!(config.theme, "dark", "主题落盘");
        // 重复添加幂等 true 且不重复落盘（FIX007.2：已存在分支补日志非静默）
        assert!(itf.add_world_city("大阪"), "重复添加应幂等 true");
        assert_eq!(
            itf.get_world_pins()
                .iter()
                .filter(|p| p.as_str() == "大阪")
                .count(),
            1,
            "重复添加不得产生第二条"
        );
        // 非法值拒绝
        assert!(!itf.add_world_city("不存在城"));
        assert!(!itf.set_base_timezone("Mars/Olympus"));
    }

    #[tokio::test]
    async fn set_weather_city_persists_and_rejects_unknown() {
        // A005 #4 验收：SetWeatherCity 持久化 last_city 并回推 ConfigSaved(true)；未知城市拒
        let path = temp_config("wcity");
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::SetWeatherCity("上海".into()))
            .await
            .expect("发送失败");
        let mut saved = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(BackendEvent::ConfigSaved(ok)) = evt_rx.recv().await {
                saved = ok;
                break;
            }
        }
        assert!(saved, "应回推 ConfigSaved(true)");
        assert_eq!(settings::load(&path).last_city, "上海", "last_city 应落盘");
        assert_eq!(itf.get_ui_preferences().last_city, "上海");
        // 未知城市（不在天气城市表）拒绝
        assert!(!itf.set_weather_city("不存在城"));
    }

    #[test]
    fn lunar_domain_via_time_info() {
        // .10 验收：农历域经 get_time_info 快照携带（chinese_date/lunar_info 非空且形态正确）
        let path = temp_config("lunarinfo");
        let itf = iface(&path);
        let info = itf.get_time_info();
        assert!(
            info.chinese_date.contains("星期"),
            "中文日期应含星期：{}",
            info.chinese_date
        );
        assert!(
            info.lunar_info.contains("月相："),
            "农历串应含月相段：{}",
            info.lunar_info
        );
        assert!(info.lunar_info.contains("财神："), "农历串应含财神段");
    }

    #[tokio::test]
    async fn weather_debounce_skips_inflight_same_city() {
        // FIX006.9 验收（纯逻辑）：同城在飞且非强制 → 跳过；强制刷新/异城不受影响
        let mut inflight: WeatherInflight = std::collections::HashMap::new();
        inflight.insert(
            "北京".into(),
            (
                tokio::spawn(async {}),
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            ),
        );
        assert!(
            weather_should_skip(&inflight, "北京", false),
            "同城在飞应跳过"
        );
        assert!(
            !weather_should_skip(&inflight, "北京", true),
            "强制刷新不得跳过"
        );
        assert!(
            !weather_should_skip(&inflight, "上海", false),
            "异城不得跳过"
        );
        inflight.clear();
        assert!(
            !weather_should_skip(&inflight, "北京", false),
            "无在飞任务不得跳过"
        );
    }

    #[tokio::test]
    async fn inflight_self_removal_respects_supersede_flag() {
        // FIX007.5 验收：force 替换出的新条目不被旧任务完成自摘误删
        let mut map: WeatherInflight = std::collections::HashMap::new();
        let (new_handle, new_flag) = (
            tokio::spawn(async {}),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        );
        map.insert(
            "北京".into(),
            (new_handle, std::sync::Arc::clone(&new_flag)),
        );
        // 旧任务已被 force 置替换标记：其任务尾自摘必须跳过（新条目保留）
        let old_flag = std::sync::atomic::AtomicBool::new(true);
        inflight_self_remove(&mut map, "北京", &old_flag);
        assert_eq!(map.len(), 1, "被替换旧任务不得删除新条目");
        // 新任务未被替换：完成即正常自摘
        inflight_self_remove(&mut map, "北京", &new_flag);
        assert!(map.is_empty(), "未被替换任务完成应自摘");
    }

    #[tokio::test]
    async fn inflight_abort_all_finishes_every_task() {
        // FIX007.5 验收：Quit 收尾后全部在飞任务被 abort 且表清空
        let mut map: WeatherInflight = std::collections::HashMap::new();
        let handle = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        let abort_handle = handle.abort_handle();
        map.insert(
            "北京".into(),
            (
                handle,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            ),
        );
        inflight_abort_all(&mut map);
        // abort 异步生效：让出调度直至任务取消完成（上限 1s 防环境挂起）
        for _ in 0..100 {
            if abort_handle.is_finished() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(abort_handle.is_finished(), "在飞任务应被 abort");
        assert!(map.is_empty(), "在飞表应清空");
    }

    #[test]
    fn world_cities_domain_query() {
        // .10 验收：世界时钟城市表经 interface 检索（50 城 + 锚点；FIX007.11 表述校正）
        let names = AppInterface::get_world_cities();
        assert_eq!(names.len(), 50, "世界时钟城市表 50 城（PL012.09 落位实测）");
        assert!(names.contains(&"纽约"));
        assert_eq!(AppInterface::get_max_alarms(), 10);
    }

    #[test]
    fn window_geometry_parse_matrix() {
        // UIR.7 验收：四元组合法/带空白/非法/缺项/旧 base64 弃档
        assert_eq!(
            parse_window_geometry("100,200,900,500"),
            Some((100.0, 200.0, 900.0, 500.0))
        );
        assert_eq!(
            parse_window_geometry(" 1 , 2 , 3 , 4 "),
            Some((1.0, 2.0, 3.0, 4.0)),
            "空格容忍"
        );
        assert_eq!(parse_window_geometry("a,b,c,d"), None, "非数字拒");
        assert_eq!(parse_window_geometry("1,2,3"), None, "缺项拒");
        assert_eq!(parse_window_geometry(""), None, "空串拒");
        assert_eq!(
            parse_window_geometry("SGVsbG8="),
            None,
            "旧 Python base64 存档天然弃档"
        );
    }
}
