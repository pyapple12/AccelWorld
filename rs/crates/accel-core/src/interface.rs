//! 应用接口层（interface/app_interface.py 的 Rust 对等物）
//! 七域契约 + 命令/事件总线：UI 访问后端的唯一入口；同步查询只读快照，
//! 一切写操作走命令——UI 不直接拿锁（z.plan 修订二·D 所有权模型）

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::alarm::{Alarm, AlarmManager, Clock, RealClock};
use crate::data::timezones;
use crate::settings;
use crate::static_config::Static;
use crate::time_dilation::TimeDilation;
use crate::types::{BackendEvent, FrontendCommand, TimeInfo, UiPreferences, WeatherData};
use crate::weather::{FetchErr, WeatherClient};
use crate::world_search::{self, WorldCityEntry, WorldSearch};

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
    /// 世界城市搜索缓存（PL028；SQLite 城市主表 291 城 + geocoding 缓存）
    world_search: Arc<WorldSearch>,
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
        // 先加载城市主表（SQLite 快照），settings 的 IANA → 城市名迁移依赖它
        let world_search = Arc::new(WorldSearch::new());
        // A005 #18：单次 load 复用（原实现对同一路径 load 两遍——重复磁盘 IO）
        let mut config = settings::load(&config_path);
        // PL029：天气城市跟随世界页 hero；启动时把持久化镜像 last_city 对齐 hero
        // （base default_city=上海只作无 hero 时的兜底；有 hero 立即被替换）
        if !config.world_hero_city.is_empty() && config.last_city != config.world_hero_city {
            config.last_city = config.world_hero_city.clone();
            if !settings::save(&config_path, &config) {
                tracing::warn!("启动同步天气城市落盘失败: {}", config.last_city);
            }
        }
        // PL020：旧配置可能存小数倍率，装载时四舍五入到整数档再校验
        let stored_rate = config.time_dilation_rate;
        let rate = stored_rate.round();
        if (rate - stored_rate).abs() > 1e-9 {
            tracing::warn!("持久化倍率非整数，已归一: {stored_rate} → {rate}");
        }
        let world = match TimeDilation::new(rate) {
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
        let weather = Arc::new(WeatherClient::new());
        // PL027：启动即应用用户配置的代理策略（false=仅直连；空地址=系统探测）
        weather.set_proxy_policy(config.weather_use_proxy, &config.weather_proxy);
        AppInterface {
            world: Arc::new(Mutex::new(world)),
            weather,
            alarms: Arc::new(Mutex::new(manager)),
            config_path,
            weather_inflight: Arc::new(Mutex::new(std::collections::HashMap::new())),
            world_search,
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

    /// 近期节日快照（未来 count 个，剔除今天及以前；前端只取 DTO）
    pub fn get_upcoming_festivals(
        &self,
        today: chrono::NaiveDate,
        count: usize,
    ) -> Vec<(chrono::NaiveDate, String)> {
        crate::chinese_calendar::upcoming_festivals(today, count)
    }

    /// 下一个春节（默认 hero；后端农历域计算）
    pub fn get_next_spring_festival(&self, today: chrono::NaiveDate) -> chrono::NaiveDate {
        crate::chinese_calendar::next_spring_festival(today)
    }

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

    // A009 #9：get_weather_refresh_interval_ms 已删（app 零调用，缓存 TTL 单源
    // weather_cache_ttl token 由 WeatherClient::new 直读）
    // PL029：get_city_names / set_weather_city 已删——天气城市不再独立选择，
    // 完全跟随世界页 hero（last_city 只作持久化镜像，启动与 hero 变更时同步）

    // ===== 时区域 =====

    /// 时区选项表（显示名, IANA 标识）
    /// A009 #9 登记：生产零调用——保留为 Python 七域契约形态（timezones 表与
    /// 夹具 n_tz 计数锚定；退役须连带夹具重生成，另立决策）
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
        // FIX011.14：合法域判定走 ThemePref::parse 单源（原 THEME_CHOICES 字面量表）
        let theme = match crate::types::ThemePref::parse(&config.theme) {
            Some(p) => p.as_str().to_string(),
            None => base.base_str("default_theme").to_string(),
        };
        UiPreferences {
            theme,
            last_city: config.last_city,
            countdown_target: config.countdown_target,
            world_hero_city: config.world_hero_city,
            weather_use_proxy: config.weather_use_proxy,
            weather_proxy: config.weather_proxy,
        }
    }

    /// 天气代理设置（PL027）：写配置 + 立即更新客户端策略；返回落盘结果
    pub fn set_weather_proxy(&self, enabled: bool, server: &str) -> bool {
        let server = server.trim().to_string();
        let mut config = settings::load(&self.config_path);
        config.weather_use_proxy = enabled;
        config.weather_proxy = server.clone();
        self.weather.set_proxy_policy(enabled, &server);
        settings::save(&self.config_path, &config)
    }

    /// 冷启动磁盘缓存快照（PL027）：只读磁盘，不触发网络
    pub fn get_cached_weather(&self, city: &str) -> Option<(u64, WeatherData)> {
        self.weather.cached_by_city(city)
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

    // A009 #9：get_alarm / get_max_alarms 已删（app 经 AlarmsChanged 全量回推重建，
    // 无单条直查面；上限由 AlarmManager 内部读 token）

    /// 触发检查（测试/调度共用入口；Python check_alarms 转发）
    pub fn check_alarms(&self, now: chrono::DateTime<Local>) -> Vec<Alarm> {
        self.alarms.lock().expect("闹钟锁中毒").check_alarms(now)
    }

    // ===== 城市域（PL012.10：城市表检索 + 常驻城市持久化）=====

    // A009 #9：get_world_cities 已删（app 直取 data::world_cities——A005 #12 只读直取定案）

    /// 添加常驻城市（入参与落盘同为城市名——FIX007.2 域统一：原 IANA 存储丢失
    /// 同 tz 多城身份（加入大阪显示东京/加入上海被已常驻的北京吞掉）；城市名即
    /// 身份，走时经 tz_of 映射 IANA；未知城市拒绝）
    /// FIX006.22：幂等规则统一——已存在 = true（补日志非静默）；非法入参 = false
    pub fn add_world_city(&self, name: &str) -> bool {
        if world_search::by_name(name).is_none() {
            tracing::warn!("未知城市，拒绝添加常驻: {name}");
            return false;
        }
        let mut config = settings::load(&self.config_path);
        if config.world_pins.iter().any(|p| p == name) {
            // FIX006.22：已存在 = 幂等达成（true，不重复落盘）——与 add_world_city 同语义
            tracing::info!("常驻城市已存在，跳过重复添加: {name}");
            return true;
        }
        config.world_pins.push(name.to_string());
        if config.world_hero_city.is_empty() {
            config.world_hero_city = name.to_string();
            config.last_city = name.to_string();
        }
        settings::save(&self.config_path, &config)
    }

    /// 移除常驻城市（入参域与添加侧统一为城市名；不存在记日志返 false——
    /// 移除类无变更可报，与「已存在 = true」互补）
    /// PL028：hero 不能直接移除（必须先 pin 新 hero）
    pub fn remove_world_city(&self, name: &str) -> bool {
        let mut config = settings::load(&self.config_path);
        if config.world_hero_city == name {
            tracing::warn!("hero 城市不能直接移除，先 pin 新城市: {name}");
            return false;
        }
        let before = config.world_pins.len();
        config.world_pins.retain(|p| p != name);
        if config.world_pins.len() == before {
            tracing::info!("常驻城市不存在，移除无变更: {name}");
            return false;
        }
        self.world_search.remove_custom(name);
        settings::save(&self.config_path, &config)
    }

    /// 设置世界时钟 hero（PL028）：必须是已 pin 的城市；同值幂等
    /// PL029：hero 同时是天气城市——last_city 镜像同步落盘
    pub fn set_world_hero(&self, name: &str) -> bool {
        let mut config = settings::load(&self.config_path);
        if !config.world_pins.iter().any(|p| p == name) {
            tracing::warn!("hero 城市不在世界时钟列表，拒绝设置: {name}");
            return false;
        }
        if self.world_search.entry_by_name(name).is_none() {
            tracing::warn!("hero 城市无法解析时区，拒绝设置: {name}");
            return false;
        }
        if config.world_hero_city == name && config.last_city == name {
            return true;
        }
        config.world_hero_city = name.to_string();
        config.last_city = name.to_string();
        settings::save(&self.config_path, &config)
    }

    /// 添加 geocoding 自定义城市（PL028）：落 custom_city + world_pins
    pub fn add_world_city_entry(&self, entry: &WorldCityEntry) -> bool {
        if entry.name.is_empty() || !world_search::valid_timezone(&entry.tz) {
            tracing::warn!("自定义城市名/时区非法，拒绝添加: {}", entry.name);
            return false;
        }
        if !self.world_search.add_custom(entry) {
            return false;
        }
        let mut config = settings::load(&self.config_path);
        if !config.world_pins.iter().any(|p| p == &entry.name) {
            config.world_pins.push(entry.name.clone());
        }
        if config.world_hero_city.is_empty() {
            config.world_hero_city = entry.name.clone();
            config.last_city = entry.name.clone();
        }
        settings::save(&self.config_path, &config)
    }

    /// 世界时钟城市条目（SQLite 城市主表 291 城 + 用户自定义；按当前 pins 顺序）
    pub fn get_world_city_entries(&self) -> Vec<WorldCityEntry> {
        self.get_world_pins()
            .iter()
            .filter_map(|name| self.world_search.entry_by_name(name))
            .collect()
    }

    /// 用户自定义城市快照（PL028；bus 启动/ConfigSaved 后同步）
    pub fn get_world_custom_cities(&self) -> Vec<WorldCityEntry> {
        self.world_search.custom_cities()
    }

    /// 世界时钟搜索：本地 291 城 → SQLite 缓存 → Open-Meteo geocoding
    pub async fn search_world_cities(&self, query: &str) -> Result<Vec<WorldCityEntry>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let min_population =
            crate::static_config::Static::get().base_i64("world_search_min_population");
        let local = self.world_search.local_matches(query);
        if !local.is_empty() {
            return Ok(local);
        }
        let cached = self.world_search.cached(query);
        if let Some(list) = &cached {
            // 旧缓存（缺国家代码/地点类型/人口/英文名，或英文名仍是拼音）不直接用，重新走网络刷新
            let stale = list.iter().any(|e| {
                e.country_code.is_empty()
                    || e.feature_code.is_empty()
                    || e.population <= 0
                    || e.name_en.is_empty()
                    || (!e.name_en.is_empty() && e.name_en == e.pinyin)
            });
            if !stale {
                let mut list = list.clone();
                list.retain(|e| {
                    matches!(e.country_code.as_str(), "CN" | "HK" | "TW" | "MO")
                        && world_search::passes_city_filter(e, min_population)
                });
                if !list.is_empty() {
                    return Ok(list);
                }
            }
        }
        // 国内城市才走 Open-Meteo；境外城市只用本地 241 城表
        let zh_url = world_search::geocode_url(query, "zh");
        let value = match self.weather.fetch_json_url(&zh_url).await {
            Ok(v) => v,
            Err(e) => {
                // 网络失败时回退旧缓存（仍按同规则过滤），至少离线可用
                if let Some(list) = cached {
                    let list: Vec<WorldCityEntry> = list
                        .into_iter()
                        .filter(|c| {
                            matches!(c.country_code.as_str(), "CN" | "HK" | "TW" | "MO")
                                && world_search::passes_city_filter(c, min_population)
                        })
                        .collect();
                    if !list.is_empty() {
                        return Ok(list);
                    }
                }
                return Err(e.to_string());
            }
        };
        let mut results = world_search::parse_geocode(&value);
        results.retain(|e| matches!(e.country_code.as_str(), "CN" | "HK" | "TW" | "MO"));
        if results.is_empty() {
            return Err("未找到匹配城市（境外城市仅支持本地城市表）".to_string());
        }
        results.retain(|e| world_search::passes_city_filter(e, min_population));
        if results.is_empty() {
            return Err(format!("没有符合人口条件的城市（{min_population} 以上）"));
        }
        self.world_search.store_candidates(query, &results);
        Ok(results)
    }

    // ===== 窗口几何域（UIR.7：位置/大小记忆）=====

    /// 窗口几何存档读取（"x,y,w,h" 形态；未存档 None——启动恢复依据）
    pub fn get_window_geometry(&self) -> Option<String> {
        settings::load(&self.config_path).window_geometry
    }

    /// 窗口几何存档（UI 直接发 SetWindowGeometry；合法数字四元组才收，
    /// 存档规范化为 "x,y,w,h" 无空白形态——A008 #3，手编带空格/科学计数不入档）
    pub fn set_window_geometry(&self, geometry: &str) -> bool {
        let Some((x, y, w, h)) = parse_window_geometry(geometry) else {
            tracing::warn!("窗口几何串非法，拒绝存档: {geometry}");
            return false;
        };
        let mut config = settings::load(&self.config_path);
        config.window_geometry = Some(format!("{x:.0},{y:.0},{w:.0},{h:.0}"));
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
                // A009 #8：存储串格式单源 settings::format_date_text（原内联 format 漂移风险）
                at: crate::settings::format_date_text(at.naive_local()),
            });
        settings::save(&self.config_path, &config)
    }

    /// 移除倒计时目标（按名；不存在记日志返 false——与 remove_world_city 同款可观测）
    pub fn remove_countdown_target(&self, name: &str) -> bool {
        let mut config = settings::load(&self.config_path);
        let before = config.countdown_targets.len();
        config.countdown_targets.retain(|t| t.name != name);
        if config.countdown_targets.len() == before {
            // A008 #14：不存在分支补日志（原静默返 false 诊断盲点）
            tracing::info!("倒计时目标不存在，移除无变更: {name}");
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
                                // 一次性触发后自动禁用（FIX012.06：幂等直设不回翻——
                                // 原 toggle 盲翻在两次锁之间可被用户手动关停插队翻回）
                                let changed = if alarm.is_one_time() {
                                    itf.alarms
                                        .lock()
                                        .expect("闹钟锁中毒")
                                        .set_enabled(&alarm.id, false)
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
        let cmd_tx_main = cmd_tx.clone();
        let main_handle = tokio::spawn(async move {
            // A008 #1：tick 改绝对截止时间（sleep 句柄跨循环复用，倍率变化时 reset 重排）——
            // 原写法每轮循环重建 sleep，命令到达快于 tick 周期时（滑杆拖动每帧一发
            // SetRate）sleep 永远到不了期，Tick 被饿死 → 时钟页走秒冻结
            let mut period = self.get_tick_interval_ms().max(1) as u64;
            let mut tick_sleep = std::pin::pin!(tokio::time::sleep(Duration::from_millis(period)));
            loop {
                let now_period = self.get_tick_interval_ms().max(1) as u64;
                if now_period != period {
                    // 倍率变化 → 新节奏即刻重排（保留原「SetRate 后下一轮即新节奏」语义）
                    period = now_period;
                    tick_sleep
                        .as_mut()
                        .reset(tokio::time::Instant::now() + Duration::from_millis(period));
                }
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
                                // FIX010.1：越界拒绝只发 RateRejected（拒绝事实由专事件
                                // 承载）；ConfigSaved(false) 收敛为纯 IO 失败信号
                                if self.set_rate(rate) {
                                    let saved = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::RateChanged(rate)).await;
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(saved)).await;
                                } else {
                                    let _ = evt_tx.send(BackendEvent::RateRejected(rate)).await;
                                }
                            }
                            Some(FrontendCommand::RefreshWeather { city, force }) => {
                                // PL029：天气城市跟随 hero；坐标从 SQLite 城市主表/自定义
                                // 城市条目解析（旧 17 城坐标表已退役）
                                let (lat, lon) = match self
                                    .world_search
                                    .entry_by_name(&city)
                                    .map(|e| (e.lat, e.lon))
                                {
                                    Some(c) => c,
                                    None => {
                                        tracing::warn!("未知天气城市，拒绝拉取: {city}");
                                        let _ = evt_tx
                                            .send(BackendEvent::WeatherFailed {
                                                city,
                                                reason: "未知城市".to_string(),
                                            })
                                            .await;
                                        continue;
                                    }
                                };
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
                                    // 两通道都失败后的自动重试节拍（base.json token，默认 600s）
                                    let retry_delay_s = Static::get()
                                        .base_i64("weather_fail_retry_delay_s")
                                        .max(1) as u64;
                                    let cmd_tx_retry = cmd_tx_main.clone();
                                    // FIX006.4/.9 配套可观测性：拉取开始即留痕（冷启动首拉可见）
                                    tracing::info!("天气拉取开始: {city}（force={force}）");
                                    let handle = tokio::spawn(async move {
                                        let result = weather
                                            .fetch_by_city(&city_task, lat, lon, force)
                                            .await;
                                        // 仅网络类失败（含磁盘缓存回退包裹的网络失败）值得
                                        // 10 分钟后自动重试；解析类错误换时间也不会自愈
                                        let retryable = match &result {
                                            Err(FetchErr::Network(_)) => true,
                                            Err(FetchErr::StaleServed { source, .. }) => {
                                                matches!(source.as_ref(), FetchErr::Network(_))
                                            }
                                            _ => false,
                                        };
                                        // PL016.03（审核 #2）：StaleServed = 失败但磁盘缓存
                                        // 有历史——先发 Updated（旧值上屏）再发 Failed（角标），
                                        // bus 侧不再清旧数据
                                        let evt = match result {
                                            Ok(data) => BackendEvent::WeatherUpdated {
                                                city: city_task.clone(),
                                                data,
                                            },
                                            Err(FetchErr::StaleServed {
                                                data,
                                                age_secs,
                                                source,
                                            }) => {
                                                let _ = evt_tx
                                                    .send(BackendEvent::WeatherUpdated {
                                                        city: city_task.clone(),
                                                        data: *data,
                                                    })
                                                    .await;
                                                BackendEvent::WeatherFailed {
                                                    city: city_task.clone(),
                                                    reason: format!(
                                                        "{source}；正在显示 {} 的缓存，点刷新重试",
                                                        crate::weather::humanize_age(age_secs)
                                                    ),
                                                }
                                            }
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
                                        // UI 已关闭/事件通道断开时不再排自动重试
                                        let delivered = evt_tx.send(evt).await.is_ok();
                                        // 两通道均网络失败：10 分钟后自动再拉（force=false，
                                        // 命中内存/磁盘缓存时零网络；失败会再排下一轮）
                                        if retryable && delivered {
                                            let city_retry = city_task.clone();
                                            tracing::info!(
                                                "天气将在 {retry_delay_s}s 后自动重试: {city_retry}"
                                            );
                                            tokio::spawn(async move {
                                                tokio::time::sleep(Duration::from_secs(
                                                    retry_delay_s,
                                                ))
                                                .await;
                                                let _ = cmd_tx_retry
                                                    .send(FrontendCommand::RefreshWeather {
                                                        city: city_retry,
                                                        force: false,
                                                    })
                                                    .await;
                                            });
                                        }
                                        let mut map =
                                            inflight_map.lock().expect("天气在飞表锁中毒");
                                        inflight_self_remove(&mut map, &city_task, &superseded_flag);
                                    });
                                    inflight.insert(city, (handle, superseded));
                                }
                            }
                            Some(FrontendCommand::AddAlarm(draft)) => {
                                // 草稿落库 + 入表 + 回推全量列表（UI 无状态重建）
                                // A008 #4：失败同样回推现值——添加被拒（时间非法/上限/重复）
                                // 时 UI 不静默偏离，对齐 A005 #19 RateRejected 口径
                                let ok = Alarm::from_draft(draft)
                                    .map(|alarm| self.alarms.lock().expect("闹钟锁中毒").add_alarm(alarm))
                                    .unwrap_or(false);
                                let _ = evt_tx.send(BackendEvent::AlarmsChanged(self.get_alarms())).await;
                                // FIX010.1：仅真写入才回推 ConfigSaved——拒绝事实已由
                                // AlarmsChanged 现值承载，ConfigSaved(false) 收敛为纯 IO
                                // 失败信号（拒绝不再误触发「保存失败」toast，A010 #1）
                                if ok {
                                    let saved = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(saved)).await;
                                }
                            }
                            Some(FrontendCommand::RemoveAlarm(id)) => {
                                let removed =
                                    self.alarms.lock().expect("闹钟锁中毒").remove_alarm(&id);
                                // A008 #4：不存在 id 同样回推现值，UI 不静默；
                                // FIX010.1：拒绝不发 ConfigSaved（同 AddAlarm 口径）
                                let _ = evt_tx.send(BackendEvent::AlarmsChanged(self.get_alarms())).await;
                                if removed {
                                    let saved = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(saved)).await;
                                }
                            }
                            Some(FrontendCommand::ReplaceAlarm(alarm)) => {
                                let ok = self.alarms.lock().expect("闹钟锁中毒").replace_alarm(alarm);
                                let _ = evt_tx.send(BackendEvent::AlarmsChanged(self.get_alarms())).await;
                                // FIX010.1：拒绝不发 ConfigSaved（同 AddAlarm 口径）
                                if ok {
                                    let saved = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(saved)).await;
                                }
                            }
                            Some(FrontendCommand::ToggleAlarm(id)) => {
                                let ok = self.alarms.lock().expect("闹钟锁中毒").toggle_alarm(&id);
                                let _ = evt_tx.send(BackendEvent::AlarmsChanged(self.get_alarms())).await;
                                // FIX010.1：拒绝不发 ConfigSaved（同 AddAlarm 口径）
                                if ok {
                                    let saved = self.save_config_now();
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(saved)).await;
                                }
                            }
                            Some(FrontendCommand::SearchWorldCity(query)) => {
                                let itf = Arc::clone(&self);
                                let evt = evt_tx.clone();
                                tokio::spawn(async move {
                                    let outcome = itf.search_world_cities(&query).await;
                                    let (results, error) = match outcome {
                                        Ok(v) => (v, None),
                                        Err(e) => (Vec::new(), Some(e)),
                                    };
                                    let _ = evt
                                        .send(BackendEvent::WorldCityCandidates {
                                            query,
                                            results,
                                            error,
                                        })
                                        .await;
                                });
                            }
                            Some(FrontendCommand::AddWorldCity(name)) => {
                                // FIX011.01（A011 #1）：拒绝不发 ConfigSaved（同 AddAlarm
                                // 口径）——前置校验后 false 只剩 IO 失败一种语义
                                if world_search::by_name(&name).is_some() {
                                    let ok = self.add_world_city(&name);
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                                } else {
                                    tracing::warn!("未知城市，拒绝添加常驻: {name}");
                                }
                            }
                            Some(FrontendCommand::AddWorldCityEntry(entry)) => {
                                // PL028：geocoding 选中后落 custom_city + pins；非法不发 ConfigSaved
                                if !entry.name.trim().is_empty()
                                    && world_search::valid_timezone(&entry.tz)
                                {
                                    let ok = self.add_world_city_entry(&entry);
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                                } else {
                                    tracing::warn!("自定义城市名/时区非法，拒绝添加");
                                }
                            }
                            Some(FrontendCommand::SetWorldHero(name)) => {
                                // PL028：hero 必须是已 pin 且可解析的城市；拒绝不发 ConfigSaved
                                if self.get_world_pins().iter().any(|p| p == &name)
                                    && self.world_search.entry_by_name(&name).is_some()
                                {
                                    let ok = self.set_world_hero(&name);
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                                } else {
                                    tracing::warn!("hero 城市不在列表或无法解析，拒绝设置: {name}");
                                }
                            }
                            Some(FrontendCommand::RemoveWorldCity(name)) => {
                                // FIX011.01：不存在拒绝不发（同上口径）
                                if self.get_world_pins().iter().any(|p| p == &name) {
                                    let ok = self.remove_world_city(&name);
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                                } else {
                                    tracing::warn!("常驻城市不存在，移除无变更: {name}");
                                }
                            }
                            Some(FrontendCommand::SetWindowGeometry { geometry }) => {
                                // FIX011.01：非法几何串拒绝不发（同上口径）
                                if parse_window_geometry(&geometry).is_some() {
                                    let ok = self.set_window_geometry(&geometry);
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                                } else {
                                    tracing::warn!("窗口几何串非法，拒绝存档: {geometry}");
                                }
                            }
                            Some(FrontendCommand::AddCountdownTarget { name, at }) => {
                                // FIX011.01：空名/同名拒绝不发（同上口径）
                                if !name.is_empty()
                                    && !self
                                        .get_countdown_targets()
                                        .iter()
                                        .any(|t| t.name == name)
                                {
                                    let ok = self.add_countdown_target(&name, &at);
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                                } else {
                                    tracing::warn!("倒计时目标空名或同名，拒绝添加: {name}");
                                }
                            }
                            Some(FrontendCommand::RemoveCountdownTarget(name)) => {
                                // FIX011.01：不存在拒绝不发（同上口径）
                                if self
                                    .get_countdown_targets()
                                    .iter()
                                    .any(|t| t.name == name)
                                {
                                    let ok = self.remove_countdown_target(&name);
                                    let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                                } else {
                                    tracing::warn!("倒计时目标不存在，移除无变更: {name}");
                                }
                            }
                            Some(FrontendCommand::SetTheme(pref)) => {
                                // FIX011.14：存储串 ThemePref::as_str 单源（原手写映射）
                                let mut config = settings::load(&self.config_path);
                                config.theme = pref.as_str().to_string();
                                let ok = settings::save(&self.config_path, &config);
                                let _ = evt_tx.send(BackendEvent::ConfigSaved(ok)).await;
                            }
                            Some(FrontendCommand::SetWeatherProxy { enabled, server }) => {
                                // PL027：保存 + 立即更新代理策略（下一次拉取生效）
                                let ok = self.set_weather_proxy(enabled, &server);
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
                    _ = tick_sleep.as_mut() => {
                        let info = self.get_time_info();
                        if evt_tx.send(BackendEvent::Tick(info)).await.is_err() {
                            // A009 #7：与 Quit 分支同款收尾（abort 在飞 + 关停调度）——
                            // 事件通道先关时不留悬空的调度/在飞任务
                            let mut inflight = self
                                .weather_inflight
                                .lock()
                                .expect("天气在飞表锁中毒");
                            inflight_abort_all(&mut inflight);
                            let _ = sd_tx.send(true);
                            break;
                        }
                        // 周期到点后重排下一轮（截止时间绝对，与命令到达节奏解耦——A008 #1）
                        tick_sleep
                            .as_mut()
                            .reset(tokio::time::Instant::now() + Duration::from_millis(period));
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

/// 窗口几何串解析（纯函数）：`"x,y,w,h"` → 四元组（非数字/缺项/NaN/inf/非正宽高
/// None——启动恢复回退 token 基准）；旧 Python base64 存档天然解析失败即弃档
/// （一次性迁移）。A008 #3：f32::from_str 接受 "NaN"/"inf"，user_config 为用户
/// 可手编外部输入，非法四元组不得喂给窗口几何
pub fn parse_window_geometry(text: &str) -> Option<(f32, f32, f32, f32)> {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let nums: Vec<f32> = parts
        .iter()
        .map(|p| p.trim().parse::<f32>().ok())
        .collect::<Option<Vec<_>>>()?;
    let (x, y, w, h) = (nums[0], nums[1], nums[2], nums[3]);
    let finite = x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite();
    // A009 #6：上限域钳制——天文数字有限值（如 1e38）不落盘不入窗口创建
    //（屏幕坐标/尺寸合理域 1e4 逻辑点，远超任何显示器组合）
    let bounded = x.abs() <= 10_000.0 && y.abs() <= 10_000.0 && w <= 10_000.0 && h <= 10_000.0;
    if !finite || !bounded || w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some((x, y, w, h))
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
// 逻辑步骤：spawn 主循环 select——命令分支（19 变体逐一接线，match 穷尽无兜底——
//   A005 #4 起新增变体不接线即编译失败；UIR.7 扩 SetWindowGeometry 为协议 v1.1；
//   PL027 扩 SetWeatherProxy 为 v1.2；PL029 退役 SetWeatherCity 为 v1.3）
//   与 tick 分支（绝对截止时间，倍率变化比较后 reset 重排）并发
// 设计理由：对齐 Python AppInterface 七域契约方法名（get_time_info/get_rate/
//   set_rate/apply_rate/get_ui_preferences/...；fetch_weather
//   拉取式残留已删——A008 #7，总线 RefreshWeather 为唯一天气入口；A009 #9 再删
//   get_weather_refresh_interval_ms/get_alarm/get_max_alarms/get_world_cities
//   四转发——app 只读直取/全量回推重建；get_timezone_options 登记保留：timezones
//   表与夹具 n_tz 计数锚，退役须连带夹具另立决策），
//   Python 拉取式 + Rust 推送式并存：同步查询供 UI 直拉（Python 习惯对等物），
//   总线供帧循环 drain（z.plan 修订一信号接口）；tick 用绝对截止时间而非
//   每轮重建 sleep（A008 #1：命令快于 tick 周期时 sleep 重建会被饿死，
//   截止时间与命令节奏解耦，倍率变化经比较后 reset 即刻重排）；天气拉取独立
//   tokio 任务（重试/超时最长秒级，不得阻塞命令通道）；两通道（直连/系统代理）
//   均网络失败后按 weather_fail_retry_delay_s 再排一次 force=false 的 RefreshWeather，
//   形成 10 分钟自动重试环（成功即停；解析类错误不排）
// 异常处理：倍率非法回退默认（构造期）/拒绝 false（set/apply）；天气失败降级
//   None + 日志（Python 对等），总线事件则回推 WeatherFailed 供 UI 提示；
//   事件通道关闭（UI 已死）时退出主循环
// FIX006.9：天气拉取任务入「在飞表」（同城去抖，force 取消旧任务重发；任务完成自摘；
//   Quit 统一 abort——不留 evt_tx 克隆，接收端即时闭合）；FIX007.5：自摘按替换标记
//   判定任务身份，旧任务完成不再误删 force 换上的新条目
// FIX006.22：写操作幂等规则统一——「已存在 = true（不重复落盘）」/「非法入参 = false」/
//   「移除类不存在 = false」；PL029：天气城市跟随世界页 hero，17 城天气表退役，
//   坐标从 SQLite city 表/自定义城市条目解析
// 已知偏移：①总线天气任务的 force 清该城缓存键（Python 清全表；z.plan 修订二
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

    /// 测试沙箱目录（RAII：drop 即整目录清理——A008 #15；断言失败不再残留 %TEMP%/accel_iface_*）
    struct SandboxDir(PathBuf);

    impl SandboxDir {
        fn new(tag: &str) -> Self {
            // 每测试独立目录（并行测试同进程不得共用配置路径）；先清同 pid 上次残留
            let dir =
                std::env::temp_dir().join(format!("accel_iface_{tag}_{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("临时目录创建失败");
            Self(dir)
        }

        fn path(&self) -> PathBuf {
            self.0.join("user_config.json")
        }
    }

    impl Drop for SandboxDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn iface(path: &Path) -> AppInterface {
        AppInterface::with_config_path(path.to_path_buf())
    }

    #[test]
    fn three_domain_sync_queries() {
        // 三域拉取走通：时钟快照形态/倍率域
        let sandbox = SandboxDir::new("sync");
        let path = sandbox.path();
        let itf = iface(&path);
        let info = itf.get_time_info();
        assert!(info.custom_time.contains(':'), "加速时间应为 HH:MM:SS 形态");
        assert_eq!(info.day_hours, 48, "默认倍率 2.0 → 48 时日长");
        assert_eq!(itf.get_rate(), 2.0);
        assert_eq!(AppInterface::get_rate_bounds(), (1.0, 365.0));
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
        let sandbox = SandboxDir::new("rate");
        let path = sandbox.path();
        let itf = iface(&path);
        assert!(itf.set_rate(5.0));
        assert_eq!(itf.get_rate(), 5.0);
        assert_eq!(itf.get_tick_interval_ms(), 100);
        // PL020：非整数倍率拒绝（状态不变）
        assert!(!itf.set_rate(5.5));
        assert_eq!(itf.get_rate(), 5.0);
        // 越界拒绝：状态不变
        assert!(!itf.set_rate(366.0));
        assert_eq!(itf.get_rate(), 5.0);
        // 未 apply 前不落盘
        assert_eq!(settings::load(&path).time_dilation_rate, 2.0);
        assert!(itf.apply_rate(3.0));
        assert_eq!(itf.get_rate(), 3.0);
        assert_eq!(settings::load(&path).time_dilation_rate, 3.0);
    }

    #[test]
    fn preferences_and_theme_normalization() {
        // 偏好组装与主题归一化（PL029：城市下拉/17 城表退役，不再断言 get_city_names）
        let sandbox = SandboxDir::new("prefs");
        let path = sandbox.path();
        let config = UserConfig {
            theme: "purple".into(),
            last_city: "上海".into(),
            world_pins: {
                let mut pins = UserConfig::default().world_pins;
                pins.insert(0, "上海".into());
                pins
            },
            world_hero_city: "上海".into(),
            ..UserConfig::default()
        };
        settings::save(&path, &config);
        let itf = iface(&path);
        let prefs = itf.get_ui_preferences();
        assert_eq!(prefs.theme, "auto", "非法主题应归一化回退默认");
        assert_eq!(prefs.last_city, "上海");
        assert_eq!(itf.get_world_pins().len(), 9);
    }

    #[test]
    fn persisted_dirty_rate_falls_back() {
        // 持久化倍率脏值回退默认（FIX004.2 对等：clamp 域外不崩溃）
        let sandbox = SandboxDir::new("dirty");
        let path = sandbox.path();
        let config = UserConfig {
            time_dilation_rate: 366.0,
            ..UserConfig::default()
        };
        settings::save(&path, &config);
        let itf = iface(&path);
        assert_eq!(itf.get_rate(), 2.0, "域外持久化倍率应回退 default_rate");
    }

    #[tokio::test]
    async fn bus_rate_and_quit_flow() {
        // 总线：SetRate → RateChanged；Quit → 循环退出（事件通道关闭）
        let sandbox = SandboxDir::new("busrate");
        let path = sandbox.path();
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
        let sandbox = SandboxDir::new("bustick");
        let path = sandbox.path();
        let itf = Arc::new(iface(&path));
        // 预热农历日缓存：首个 Tick 不背 lunar-rust 初始化耗时（版本门换缓存后尤其重要）
        let _ = itf.get_time_info();
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
        let sandbox = SandboxDir::new("busweather");
        let path = sandbox.path();
        let client = WeatherClient::custom(
            server.base_url(),
            RetrySpec {
                retries: 2,
                delay: Duration::ZERO,
            },
            Duration::from_secs(5),
            Duration::from_secs(3600),
            7,
            24,
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

        let sandbox = SandboxDir::new("vclock");
        let path = sandbox.path();
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
            !itf.alarms
                .lock()
                .expect("闹钟锁中毒")
                .get_alarm("once1")
                .expect("在表")
                .enabled,
            "一次性应已禁用"
        );
        assert!(
            itf.alarms
                .lock()
                .expect("闹钟锁中毒")
                .get_alarm("rep1")
                .expect("在表")
                .enabled,
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
        let sandbox = SandboxDir::new("acrud");
        let path = sandbox.path();
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
        // A009 #5：落盘结果回推 ConfigSaved(true)
        let mut saved_ok = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if matches!(evt_rx.recv().await, Some(BackendEvent::ConfigSaved(true))) {
                saved_ok = true;
                break;
            }
        }
        assert!(saved_ok, "成功添加应回推 ConfigSaved(true)");
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
    async fn bus_weather_proxy_settings_persist() {
        // PL027：开关 + 代理地址落盘，并同步到 UI 偏好快照
        let sandbox = SandboxDir::new("weatherproxy");
        let path = sandbox.path();
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::SetWeatherProxy {
                enabled: false,
                server: "http://127.0.0.1:10808".into(),
            })
            .await
            .expect("发送失败");
        let mut saved = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if matches!(evt_rx.recv().await, Some(BackendEvent::ConfigSaved(true))) {
                saved = true;
                break;
            }
        }
        assert!(saved, "代理设置应落盘成功");
        let config = settings::load(&path);
        assert!(!config.weather_use_proxy);
        assert_eq!(config.weather_proxy, "http://127.0.0.1:10808");
        let prefs = itf.get_ui_preferences();
        assert!(!prefs.weather_use_proxy);
        assert_eq!(prefs.weather_proxy, "http://127.0.0.1:10808");
        cmd_tx.send(FrontendCommand::Quit).await.expect("退出失败");
    }

    #[tokio::test]
    async fn bus_world_search_local_hits_builtin() {
        // PL028：本地 291 城搜索不触发 geocoding（无需网络桩）
        let sandbox = SandboxDir::new("worldsearch");
        let path = sandbox.path();
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::SearchWorldCity("beijing".into()))
            .await
            .expect("发送失败");
        let mut hit = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            match evt_rx.recv().await {
                Some(BackendEvent::WorldCityCandidates { results, .. }) => {
                    hit = results.iter().any(|e| e.name == "北京");
                    break;
                }
                Some(_) => {}
                None => break,
            }
        }
        assert!(hit, "beijing 应在本地命中北京");
        cmd_tx.send(FrontendCommand::Quit).await.expect("退出失败");
    }

    #[tokio::test]
    async fn world_hero_and_custom_city_flow() {
        // PL028：自定义城市落库/入 pins；hero 不可直接删，pin 新 hero 后可删
        let sandbox = SandboxDir::new("worldhero");
        let path = sandbox.path();
        let itf = Arc::new(iface(&path));
        let entry = WorldCityEntry {
            name: "测试城PL028".to_string(),
            name_en: "Test City".to_string(),
            pinyin: "ceshicheng".to_string(),
            initials: "csc".to_string(),
            country: "测试国".to_string(),
            country_code: "TT".to_string(),
            admin1: String::new(),
            feature_code: "PPLC".to_string(),
            population: 1_000_000,
            tz: "Asia/Shanghai".to_string(),
            lat: 31.0,
            lon: 121.0,
        };
        assert!(itf.add_world_city_entry(&entry));
        assert!(itf.get_world_pins().iter().any(|p| p == "测试城PL028"));
        assert!(itf.set_world_hero("测试城PL028"));
        assert_eq!(
            settings::load(&path).world_hero_city,
            "测试城PL028",
            "hero 应落盘"
        );
        assert!(!itf.remove_world_city("测试城PL028"), "hero 不能直接删除");
        assert!(itf.set_world_hero("北京"), "pin 新 hero");
        assert!(itf.remove_world_city("测试城PL028"), "换 hero 后可删");
        assert!(
            itf.get_world_custom_cities()
                .iter()
                .all(|e| e.name != "测试城PL028"),
            "自定义城市记录应删除"
        );
    }

    #[tokio::test]
    async fn bus_city_and_theme_commands() {
        // .10 验收 + FIX007.2 域验收：AddWorldCity 按城市名落盘（大阪独立于东京——
        // 同 tz 多城共存，不再被表序首城吞并）+ 重复添加幂等 + SetWorldHero
        // roundtrip + SetTheme 落盘
        let sandbox = SandboxDir::new("citycmd");
        let path = sandbox.path();
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::AddWorldCity("大阪".into()))
            .await
            .expect("发送失败");
        cmd_tx
            .send(FrontendCommand::SetWorldHero("大阪".into()))
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
        assert_eq!(config.world_hero_city, "大阪", "hero 城市 roundtrip");
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
        assert!(!itf.set_world_hero("Mars/Olympus"));
    }

    #[test]
    fn startup_syncs_weather_city_to_world_hero() {
        // PL029 验收：天气城市跟随 hero——旧档 last_city=上海、hero=东京时，
        // 构造期立即把 last_city 镜像同步为东京并落盘
        let sandbox = SandboxDir::new("wcitysync");
        let path = sandbox.path();
        let config = UserConfig {
            last_city: "上海".into(),
            world_hero_city: "东京".into(),
            world_pins: vec!["北京".into(), "东京".into()],
            ..UserConfig::default()
        };
        settings::save(&path, &config);
        let itf = iface(&path);
        assert_eq!(itf.get_ui_preferences().last_city, "东京");
        assert_eq!(settings::load(&path).last_city, "东京");
    }

    #[test]
    fn set_world_hero_syncs_weather_city() {
        // PL029 验收：hero 变更即天气城市变更（last_city 镜像同步落盘）
        let sandbox = SandboxDir::new("wherosync");
        let path = sandbox.path();
        let itf = iface(&path);
        assert!(itf.add_world_city("上海"));
        assert!(itf.set_world_hero("上海"));
        assert_eq!(settings::load(&path).last_city, "上海");
        assert_eq!(itf.get_ui_preferences().last_city, "上海");
    }

    #[test]
    fn lunar_domain_via_time_info() {
        // .10 验收：农历域经 get_time_info 快照携带（chinese_date/lunar_info 非空且形态正确）
        let sandbox = SandboxDir::new("lunarinfo");
        let path = sandbox.path();
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
        // .10 验收：世界时钟城市表经 data 直取（A009 #9：interface 转发已删——
        // A005 #12 只读直取定案；291 城 SQLite 快照 + 锚点 + 上限 token 锚点）
        let dir = std::env::temp_dir().join(format!("accel_world_domain_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = crate::world_search::WorldSearch::with_path(dir.join("world_cache.sqlite"));
        let names = crate::world_search::all_names();
        assert!(
            names.len() >= 200,
            "世界时钟城市表应扩到 200+（PL028 实测 {}）",
            names.len()
        );
        assert!(names.contains(&"纽约"));
        assert_eq!(
            Static::get().base_i64("max_alarms"),
            10,
            "闹钟上限 token 锚点"
        );
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

    #[test]
    fn window_geometry_parse_rejects_nonfinite_and_nonpositive() {
        // A008 #3 验收：NaN/inf（f32::from_str 可解析）与非正宽高拒绝，
        // 非法四元组不得喂给窗口几何
        assert_eq!(parse_window_geometry("NaN,100,900,500"), None, "NaN 拒");
        assert_eq!(parse_window_geometry("100,inf,900,500"), None, "inf 拒");
        assert_eq!(parse_window_geometry("100,100,0,500"), None, "零宽拒");
        assert_eq!(parse_window_geometry("100,100,900,-5"), None, "负高拒");
        assert_eq!(
            parse_window_geometry("1e39,1e39,1e39,1e39"),
            None,
            "科学计数溢出 f32（→inf）拒"
        );
        // A009 #6：上限域钳制——天文有限值不落盘不入窗口创建
        assert_eq!(parse_window_geometry("1e38,10,900,500"), None, "坐标超域拒");
        assert_eq!(
            parse_window_geometry("100,100,20000,500"),
            None,
            "宽度超域拒"
        );
        assert_eq!(
            parse_window_geometry("-20000,100,900,500"),
            None,
            "坐标负向超域拒"
        );
    }

    #[test]
    fn window_geometry_archive_is_canonical() {
        // A008 #3 验收：存档规范化为无空白 "x,y,w,h"（手编带空格不入原样）；
        // 非法四元组拒存档
        let sandbox = SandboxDir::new("geocanon");
        let path = sandbox.path();
        let itf = iface(&path);
        assert!(itf.set_window_geometry(" 120 , 240 , 960 , 540 "));
        assert_eq!(
            itf.get_window_geometry().as_deref(),
            Some("120,240,960,540"),
            "存档应规范化"
        );
        assert!(!itf.set_window_geometry("NaN,0,0,0"), "非法四元组拒存档");
    }

    #[tokio::test]
    async fn bus_alarm_rejection_pushes_unchanged_list() {
        // A008 #4 验收：非法时间 AddAlarm / 不存在 id RemoveAlarm 失败——
        // 均回推现值列表（UI 不静默偏离，对齐 RateRejected 口径）
        let sandbox = SandboxDir::new("alarmrej");
        let path = sandbox.path();
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::AddAlarm(AlarmDraft {
                label: "非法".into(),
                time: "9:30".into(), // 单 digit 小时即拒（alarm 装载规则）
                sound_type: "preset".into(),
                sound_value: "classic".into(),
                repeat_days: vec![],
                enabled: true,
            }))
            .await
            .expect("发送失败");
        let mut pushed_empty = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(BackendEvent::AlarmsChanged(list)) = evt_rx.recv().await {
                pushed_empty = list.is_empty();
                break;
            }
        }
        assert!(pushed_empty, "失败添加也应回推（空）列表");
        assert!(itf.get_alarms().is_empty(), "非法草稿不得入表");
        // FIX010.1：拒绝路径不再回推 ConfigSaved（拒绝事实由现值列表承载，
        // false 收敛为纯 IO 失败信号——「配置保存失败」toast 误触发根治）
        let drain_deadline =
            tokio::time::Instant::from_std(std::time::Instant::now() + Duration::from_millis(300));
        loop {
            match tokio::time::timeout_at(drain_deadline, evt_rx.recv()).await {
                Ok(Some(BackendEvent::ConfigSaved(_))) => {
                    panic!("拒绝路径不得回推 ConfigSaved");
                }
                Ok(Some(_)) => continue,
                _ => break,
            }
        }
        // 不存在 id 移除同样回推
        cmd_tx
            .send(FrontendCommand::RemoveAlarm("ghost".into()))
            .await
            .expect("发送失败");
        let mut pushed_again = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if matches!(evt_rx.recv().await, Some(BackendEvent::AlarmsChanged(_))) {
                pushed_again = true;
                break;
            }
        }
        assert!(pushed_again, "不存在 id 移除也应回推现值");
    }

    #[tokio::test]
    async fn io_failure_still_pushes_config_saved_false() {
        // FIX010.1 验收：拒绝/IO 分流后，真 IO 失败（配置路径为目录，写入必败）
        // 仍回推 ConfigSaved(false)——false 语义纯化后信号不丢
        let sandbox = SandboxDir::new("iofail");
        let blocked = sandbox.path().parent().expect("沙箱根").join("blocked");
        std::fs::create_dir_all(&blocked).expect("占位目录创建失败");
        let itf = Arc::new(AppInterface::with_config_path(blocked));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        cmd_tx
            .send(FrontendCommand::AddAlarm(AlarmDraft {
                label: "落盘败".into(),
                time: "07:30".into(),
                sound_type: "preset".into(),
                sound_value: "classic".into(),
                repeat_days: vec![],
                enabled: true,
            }))
            .await
            .expect("发送失败");
        let mut pushed = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(BackendEvent::AlarmsChanged(list)) = evt_rx.recv().await {
                pushed = list.len() == 1;
                break;
            }
        }
        assert!(pushed, "合法草稿应入表并回推单元素列表");
        let mut io_failed = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if matches!(evt_rx.recv().await, Some(BackendEvent::ConfigSaved(false))) {
                io_failed = true;
                break;
            }
        }
        assert!(io_failed, "真 IO 失败仍应回推 ConfigSaved(false)");
    }

    #[tokio::test]
    async fn config_command_rejections_do_not_push_config_saved() {
        // A011 #1（FIX011.01）验收：六个配置写命令的拒绝路径不得回推 ConfigSaved
        // ——与闹钟域 FIX010.1 同口径（false 收敛为纯 IO 失败信号，拒绝不再误弹
        // 「配置保存失败」toast）；success 侧保留回推（bus need_reload 依赖）
        let sandbox = SandboxDir::new("cfgreject");
        let itf = Arc::new(AppInterface::with_config_path(sandbox.path()));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(Arc::clone(&itf));
        let rejections = [
            FrontendCommand::AddWorldCity("不存在城".into()),
            FrontendCommand::RemoveWorldCity("幽灵城".into()),
            FrontendCommand::SetWorldHero("不存在城".into()),
            FrontendCommand::SetWindowGeometry {
                geometry: "bad".into(),
            },
            FrontendCommand::AddCountdownTarget {
                name: String::new(),
                at: chrono::Local::now(),
            },
            FrontendCommand::RemoveCountdownTarget("无此目标".into()),
        ];
        for cmd in rejections {
            cmd_tx.send(cmd).await.expect("发送失败");
        }
        let drain_deadline =
            tokio::time::Instant::from_std(std::time::Instant::now() + Duration::from_millis(500));
        loop {
            match tokio::time::timeout_at(drain_deadline, evt_rx.recv()).await {
                Ok(Some(BackendEvent::ConfigSaved(_))) => {
                    panic!("配置命令拒绝路径不得回推 ConfigSaved");
                }
                Ok(Some(_)) => continue,
                _ => break,
            }
        }
        // success 侧保留：合法几何存档仍回推 ConfigSaved(true)
        cmd_tx
            .send(FrontendCommand::SetWindowGeometry {
                geometry: "10,20,900,560".into(),
            })
            .await
            .expect("发送失败");
        let mut saved = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if matches!(evt_rx.recv().await, Some(BackendEvent::ConfigSaved(true))) {
                saved = true;
                break;
            }
        }
        assert!(
            saved,
            "合法写命令仍应回推 ConfigSaved(true)（bus need_reload 依赖）"
        );
    }

    #[tokio::test]
    async fn tick_not_starved_by_command_burst() {
        // A008 #1 验收：命令爆发（模拟滑杆拖动每帧一发 SetRate）期间 Tick 仍按期到达
        let sandbox = SandboxDir::new("tickstarve");
        let path = sandbox.path();
        let itf = Arc::new(iface(&path));
        let (cmd_tx, mut evt_rx) = AppInterface::spawn(itf);
        let sender = tokio::spawn(async move {
            for i in 0..60 {
                let _ = cmd_tx
                    .send(FrontendCommand::SetRate(1.0 + (i % 10) as f64 * 0.5))
                    .await;
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        let mut got_tick = false;
        while std::time::Instant::now() < deadline {
            if matches!(evt_rx.recv().await, Some(BackendEvent::Tick(_))) {
                got_tick = true;
                break;
            }
        }
        assert!(got_tick, "命令爆发期内 Tick 不得被饿死（A008 #1）");
        let _ = sender.await;
    }
}
