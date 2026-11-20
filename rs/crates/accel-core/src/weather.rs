//! 天气服务（modules/weather_service.py 的 Rust 对等物）
//! PL011.11 解析层：Open-Meteo 响应校验 + WeatherData 构造 + 展示文本
//! PL011.12 传输层：WeatherClient（缓存 TTL/重试/白名单/force 穿透）

use serde_json::Value;
use thiserror::Error;

use crate::data::weather_codes;
use crate::types::WeatherData;

// 必要响应字段（与请求 current 参数一致；缺失或 null 判失败，FIX001.14/FIX002.4）
const REQUIRED_FIELDS: [&str; 5] = [
    "temperature_2m",
    "relative_humidity_2m",
    "weather_code",
    "wind_speed_10m",
    "apparent_temperature",
];

// 数值型必要字段（weather_code 允许任意标量；其余须为数值，FIX002.4）
const NUMERIC_FIELDS: [&str; 4] = [
    "temperature_2m",
    "relative_humidity_2m",
    "wind_speed_10m",
    "apparent_temperature",
];

/// 解析层错误（Python 各拒绝路径的结构化对等；Python 侧统一降级 None + 日志）
#[derive(Debug, Error, PartialEq)]
pub enum WeatherErr {
    #[error("天气响应结构异常：根非对象")]
    NotObject,
    #[error("天气响应结构异常：current 缺失或非对象")]
    CurrentNotObject,
    #[error("天气响应缺少必要字段: {0}")]
    MissingField(&'static str),
    #[error("天气响应缺少必要字段（值为 null）: {0}")]
    NullField(&'static str),
    #[error("天气响应数值字段类型非法: {0}")]
    BadType(&'static str),
    #[error("逐小时数据含 null: {0}")]
    HourlyNull(&'static str),
    #[error("逐小时/七日数组缺失或长度不一致: {0}")]
    SeriesMissing(&'static str),
    #[error("七日数据含 null: {0}")]
    DailyNull(&'static str),
    #[error("天气响应字段类型非法（非标量）: {0}")]
    Parse(&'static str),
}

// HourlyPoint/DailyPoint DTO 在 types.rs 单源（weather.rs 只做提取与校验）

/// 逐小时/七日系列提取（同 null/长度纪律）：键缺失或长度不一致 → SeriesMissing，
/// 元素 null → *Null 变体，数值错型 → BadType（与 current 域同拒绝纪律）
fn extract_hourly_daily(
    root: &serde_json::Map<String, Value>,
) -> Result<
    (
        Vec<crate::types::HourlyPoint>,
        Vec<crate::types::DailyPoint>,
    ),
    WeatherErr,
> {
    // 键缺失 → 空序列（向后兼容：Python 0.6.0.0 快照无此键；新 API 恒带参返回）
    // 键存在则全严校验（null/错型/长度不一致 → Err，同 current 域纪律）
    // A008 #9：容器本身错型（hourly 为串/数组等非对象）同样 Err——原写法
    // and_then(as_object) 把错型当缺失静默吞掉，与注释承诺相反
    let hourly = match root.get("hourly") {
        None => None,
        Some(Value::Object(map)) => Some(map),
        Some(_) => return Err(WeatherErr::BadType("hourly")),
    };
    // A005 #18：hourly 缺失不再连带丢弃 daily——两者独立判定
    let daily = match root.get("daily") {
        None => None,
        Some(Value::Object(map)) => Some(map),
        Some(_) => return Err(WeatherErr::BadType("daily")),
    };
    match (hourly, daily) {
        (None, None) => Ok((Vec::new(), Vec::new())),
        (Some(h), None) => Ok((extract_hourly(h)?, Vec::new())),
        (None, Some(d)) => Ok((Vec::new(), extract_daily(d)?)),
        (Some(h), Some(d)) => Ok((extract_hourly(h)?, extract_daily(d)?)),
    }
}

/// 逐小时序列提取（A005 #18：从 extract_hourly_daily 拆出，与 daily 独立判定）
fn extract_hourly(
    hourly: &serde_json::Map<String, Value>,
) -> Result<Vec<crate::types::HourlyPoint>, WeatherErr> {
    let h_times = series_strings(hourly, "time")?;
    let h_temps = series_numbers(hourly, "temperature_2m", SeriesKind::Hourly)?;
    let h_precips = series_numbers(hourly, "precipitation_probability", SeriesKind::Hourly)?;
    if h_times.len() != h_temps.len() || h_times.len() != h_precips.len() {
        return Err(WeatherErr::SeriesMissing("hourly 长度不一致"));
    }
    Ok(h_times
        .iter()
        .zip(h_temps.iter().zip(h_precips.iter()))
        .map(|(t, (temp, p))| crate::types::HourlyPoint {
            time: t.clone(),
            temperature: *temp,
            precip_prob: *p,
        })
        .collect())
}

/// 七日序列提取（同 null/长度纪律；与 hourly 独立判定）
fn extract_daily(
    daily: &serde_json::Map<String, Value>,
) -> Result<Vec<crate::types::DailyPoint>, WeatherErr> {
    let d_dates = series_strings(daily, "time")?;
    let d_max = series_numbers(daily, "temperature_2m_max", SeriesKind::Daily)?;
    let d_min = series_numbers(daily, "temperature_2m_min", SeriesKind::Daily)?;
    let d_sunrise = series_strings(daily, "sunrise")?;
    let d_sunset = series_strings(daily, "sunset")?;
    if d_dates.len() != d_max.len()
        || d_dates.len() != d_min.len()
        || d_dates.len() != d_sunrise.len()
        || d_dates.len() != d_sunset.len()
    {
        return Err(WeatherErr::SeriesMissing("daily 长度不一致"));
    }
    Ok(d_dates
        .iter()
        .enumerate()
        .map(|(i, d)| crate::types::DailyPoint {
            date: d.clone(),
            t_max: d_max[i],
            t_min: d_min[i],
            sunrise: d_sunrise[i].clone(),
            sunset: d_sunset[i].clone(),
        })
        .collect())
}

/// 系列种类（A005 #18：原 `&'static str` 运行期分派改枚举——笔误即编译期暴露）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeriesKind {
    Hourly,
    Daily,
}

/// 字符串系列提取（A011 #18：键缺失/容器错型 → SeriesMissing，元素非串 → BadType
/// ——与 series_numbers 同款错型/缺失分档，本函数不产 Null 变体）
fn series_strings(
    obj: &serde_json::Map<String, Value>,
    key: &'static str,
) -> Result<Vec<String>, WeatherErr> {
    match obj.get(key) {
        None => Err(WeatherErr::SeriesMissing(key)),
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|v| v.as_str().map(String::from).ok_or(WeatherErr::BadType(key)))
            .collect(),
        Some(_) => Err(WeatherErr::SeriesMissing(key)),
    }
}

/// 数值系列提取（null → *Null 变体；错型 → BadType；A005 #18：种类参数改枚举）
fn series_numbers(
    obj: &serde_json::Map<String, Value>,
    key: &'static str,
    kind: SeriesKind,
) -> Result<Vec<f64>, WeatherErr> {
    match obj.get(key) {
        None => Err(WeatherErr::SeriesMissing(key)),
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|v| match v {
                Value::Null => match kind {
                    SeriesKind::Hourly => Err(WeatherErr::HourlyNull(key)),
                    SeriesKind::Daily => Err(WeatherErr::DailyNull(key)),
                },
                Value::Number(_) => v.as_f64().ok_or(WeatherErr::BadType(key)),
                _ => Err(WeatherErr::BadType(key)),
            })
            .collect(),
        Some(_) => Err(WeatherErr::SeriesMissing(key)),
    }
}

/// 解析 Open-Meteo 响应（Python get_weather_by_coords 校验段对等）
/// 判定顺序与 Python 一致：根对象 → current 对象 → 五必要字段（缺失/null）→
/// 四数值字段类型（bool/string/容器拒）→ weather_code 查表（未知回退）
/// PL014.11：追加 hourly/daily 系列解析（同 null/缺字段/长度纪律）
pub fn parse_open_meteo(v: &Value) -> Result<WeatherData, WeatherErr> {
    let root = v.as_object().ok_or(WeatherErr::NotObject)?;
    let current = root
        .get("current")
        .and_then(Value::as_object)
        .ok_or(WeatherErr::CurrentNotObject)?;
    for field in REQUIRED_FIELDS {
        match current.get(field) {
            None => return Err(WeatherErr::MissingField(field)),
            Some(Value::Null) => return Err(WeatherErr::NullField(field)),
            Some(_) => {}
        }
    }
    for field in NUMERIC_FIELDS {
        if !current[field].is_number() {
            return Err(WeatherErr::BadType(field));
        }
    }
    // weather_code 标量纪律（FIX007.16）：布尔/数值查表、字符串按未知码回退
    // （Python dict.get(str) 返 None 同语义）；数组/对象 → Parse（Python
    // dict.get(list) 抛 TypeError 走失败路径——原实现误与字符串同归未知）
    let code_value = &current["weather_code"];
    if !code_value.is_boolean() && !code_value.is_number() && !code_value.is_string() {
        return Err(WeatherErr::Parse("weather_code"));
    }
    let code_key = scalar_code_key(code_value);
    let info = weather_codes::lookup(code_key);
    let (hourly, daily) = extract_hourly_daily(root)?;
    Ok(WeatherData {
        temperature: current["temperature_2m"].as_f64().expect("已过数值校验"),
        humidity: current["relative_humidity_2m"]
            .as_f64()
            .expect("已过数值校验"),
        wind_speed: current["wind_speed_10m"].as_f64().expect("已过数值校验"),
        apparent_temperature: current["apparent_temperature"]
            .as_f64()
            .expect("已过数值校验"),
        weather_code: code_key,
        weather: info.name.to_string(),
        description: info.description.to_string(),
        icon: info.icon.to_string(),
        hourly,
        daily,
    })
}

/// 标量 → 码表查键：布尔按 1/0；数值取整值（整值浮点 0.0 命中码 0——Python dict
/// 查找语义对齐，serde Number::as_i64 对浮点底数恒 None 须自判整值）；
/// 字符串返未知哨兵（i64::MIN，不命中任何整键 → 未知）；数组/对象已在调用方
/// 拒绝（FIX007.16 Parse），不达此处
fn scalar_code_key(v: &Value) -> i64 {
    match v {
        Value::Bool(b) => *b as i64,
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i
            } else {
                let f = n.as_f64().expect("数值变体必可取 f64");
                // 2^53 内整值浮点转 i64 精确；超出或含小数 → 未知哨兵
                if f.fract() == 0.0 && f.abs() <= 9_007_199_254_740_992.0 {
                    f as i64
                } else {
                    i64::MIN
                }
            }
        }
        _ => i64::MIN,
    }
}

/// 空数据失败文案（Python format_weather_info(None) 对等）
pub const WEATHER_FAIL_TEXT: &str = "天气信息获取失败";

/// 展示文本拼装（Python format_weather_info 对等，格式逐字符对齐）
/// A009 #9 登记：生产零调用——parity 契约消费（夹具快照的 Python 文本格式对齐锁），
/// 非死代码
pub fn format_weather_info(weather: Option<&WeatherData>, city_name: &str) -> String {
    let Some(w) = weather else {
        return WEATHER_FAIL_TEXT.to_string();
    };
    let city = if city_name.is_empty() {
        String::new()
    } else {
        format!("{city_name} ")
    };
    format!(
        "{city}{} {} | {:.1}°C | 体感 {:.1}°C | 湿度 {:.0}% | 风力 {:.1}km/h",
        w.icon, w.weather, w.temperature, w.apparent_temperature, w.humidity, w.wind_speed
    )
}

// ===== 传输层（PL011.12）：WeatherClient =====

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::data::cities;
use crate::static_config::Static;
use crate::util::retry::{IsRetryable, RetrySpec, retry};

/// 传输层错误（Python None 降级前的结构化原因）
#[derive(Debug, Error)]
pub enum FetchErr {
    #[error("未知城市: {0}")]
    UnknownCity(String),
    #[error("经纬度越界: lat={lat}, lon={lon}")]
    CoordsOutOfRange { lat: f64, lon: f64 },
    #[error("网络请求失败: {0}")]
    Network(String),
    #[error("响应解析失败: {0}")]
    Parse(String),
    /// PL016.03（审核 #2）：拉取失败但磁盘缓存有历史数据——陈旧数据随错误
    /// 一并上抛，UI 保留旧值 +「N 前缓存」角标（不再整页空白，重启亦可恢复）
    /// data 装箱：FetchErr 体积受 clippy result_large_err 纪律约束
    #[error("拉取失败，显示 {} 前缓存", humanize_age(*age_secs))]
    StaleServed {
        data: Box<WeatherData>,
        age_secs: u64,
        #[source]
        source: Box<FetchErr>,
    },
}

impl From<WeatherErr> for FetchErr {
    fn from(e: WeatherErr) -> Self {
        FetchErr::Parse(e.to_string())
    }
}

impl IsRetryable for FetchErr {
    fn is_retryable(&self) -> bool {
        // 仅网络类重试（Python _NETWORK_ERRORS 白名单对等；解析类直通）
        matches!(self, FetchErr::Network(_))
    }
}

// 经纬度定义域（地理常量，请求前校验防非法参数拼入 URL，V0.4.7.0 对等）
const LAT_RANGE: (f64, f64) = (-90.0, 90.0);
const LON_RANGE: (f64, f64) = (-180.0, 180.0);

/// 磁盘缓存年龄上限秒（FIX012.02：token 单源——原 30 天字面量游离于同域
/// weather_* token 体系外；下钳 1 防零/负配置损坏致缓存永不失效）
fn disk_cache_max_age_secs() -> u64 {
    Static::get()
        .base_i64("weather_disk_cache_max_age_s")
        .max(1) as u64
}

/// 缓存年龄话术（纯函数可测，PL016.03 审核文案纪律：说清事实不含糊）
pub fn humanize_age(age_secs: u64) -> String {
    if age_secs < 60 {
        format!("{age_secs} 秒前")
    } else if age_secs < 3600 {
        format!("{} 分钟前", age_secs / 60)
    } else if age_secs < 24 * 3600 {
        format!("{} 小时前", age_secs / 3600)
    } else {
        format!("{} 天前", age_secs / (24 * 3600))
    }
}

/// 系统代理通道（运行期可换：每次拉取前重读策略，开关代理无需重启）
struct ProxyChannel {
    url: Option<String>,
    client: Option<reqwest::Client>,
}

/// 天气代理策略（PL027）：enabled=false 仅直连；server 空=跟随系统代理
#[derive(Debug, Clone, PartialEq, Eq)]
struct WeatherProxyPolicy {
    enabled: bool,
    server: String,
}

/// 天气客户端：按城缓存 TTL + 重试 + force 穿透 + 直连/代理双通道
/// SSRF 防线（结构性）：base 构造期固定（生产 = https://api.open-meteo.com 常量），
/// 运行期 URL 全部由 base + 已校验坐标派生，用户输入永不进入 host 部分；
/// 禁跟随重定向（reqwest Policy::none，Python no-redirect opener 对等）
pub struct WeatherClient {
    /// 直连通道（显式 no_proxy；先尝试）
    direct: reqwest::Client,
    /// 代理通道（探测/配置存在才构建；每次拉取前热更新）
    proxy: Mutex<ProxyChannel>,
    /// 代理策略（用户设置；运行期可改）
    proxy_policy: Mutex<WeatherProxyPolicy>,
    /// 是否自动跟随策略（生产 true；测试注入 false，避免测试被本机代理劫持）
    proxy_auto: bool,
    base: String,
    cache: Mutex<HashMap<String, (Instant, WeatherData)>>,
    ttl: Duration,
    retry_spec: RetrySpec,
    timeout: Duration,
    connect_timeout: Duration,
    /// 采样参数（A009 #10 token 化：原 URL 字面量 forecast_days=7/forecast_hours=24）
    forecast_days: i64,
    forecast_hours: i64,
    /// PL016.03 磁盘缓存目录（None 禁用——测试隔离；生产 = exe 同目录 weather_cache/）
    disk_cache_dir: Option<PathBuf>,
}

impl Default for WeatherClient {
    /// 默认构造（委托生产构造：clippy new_without_default）
    fn default() -> Self {
        Self::new()
    }
}

/// 构造 reqwest 客户端：统一总超时/连接超时/禁重定向；proxy=None 时显式禁用代理
fn http_client(
    total: Duration,
    connect: Duration,
    proxy: Option<&str>,
) -> reqwest::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(total)
        .connect_timeout(connect)
        .redirect(reqwest::redirect::Policy::none());
    builder = match proxy {
        Some(url) => builder.proxy(reqwest::Proxy::all(url)?),
        None => builder.no_proxy(),
    };
    builder.build()
}

/// 环境变量代理（HTTPS 优先，其次 ALL；与 reqwest 系统匹配器同序）
fn env_proxy() -> Option<String> {
    ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
        .iter()
        .find_map(|key| std::env::var(key).ok())
        .and_then(|raw| normalize_proxy(&raw))
}

/// 代理串归一：Windows 可能是 `host:port` 或 `http=host:port;https=host:port`；
/// 只接受 http/https（socks 需额外特性，本客户端不启用）
fn normalize_proxy(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let candidate = if raw.contains('=') {
        let pick = |want: &str| {
            raw.split(';')
                .filter_map(|part| part.split_once('='))
                .find_map(|(key, value)| {
                    (key.trim().eq_ignore_ascii_case(want)).then(|| value.trim())
                })
        };
        pick("https").or_else(|| pick("http"))?
    } else {
        raw
    };
    if candidate.is_empty() {
        return None;
    }
    if let Some((scheme, _)) = candidate.split_once("://") {
        if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
            return None;
        }
        Some(candidate.to_string())
    } else {
        Some(format!("http://{candidate}"))
    }
}

/// 代理日志标签：隐藏 userinfo（环境变量代理可能带账号密码）
fn proxy_label(url: &str) -> String {
    match url.split_once('@') {
        Some((_, rest)) => format!("***@{rest}"),
        None => url.to_string(),
    }
}

/// Windows 系统代理：读 HKCU Internet Settings（ProxyEnable + ProxyServer）
#[cfg(windows)]
fn windows_proxy() -> Option<String> {
    let settings = windows_registry::CURRENT_USER
        .open("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .ok()?;
    if settings.get_u32("ProxyEnable").unwrap_or(0) == 0 {
        return None;
    }
    let raw = settings.get_string("ProxyServer").ok()?;
    normalize_proxy(&raw)
}

/// 系统代理探测：环境变量优先，其次 Windows 系统设置；非 Windows 仅环境变量
fn detect_system_proxy() -> Option<String> {
    if let Some(url) = env_proxy() {
        return Some(url);
    }
    #[cfg(windows)]
    {
        if let Some(url) = windows_proxy() {
            return Some(url);
        }
    }
    None
}

/// 客户端构造参数（消除 8 参函数；生产/测试共用）
struct WeatherClientConfig {
    base: String,
    retry_spec: RetrySpec,
    timeout: Duration,
    connect_timeout: Duration,
    ttl: Duration,
    forecast_days: i64,
    forecast_hours: i64,
    proxy_url: Option<String>,
    proxy_auto: bool,
}

impl WeatherClient {
    /// 生产构造：base 固定官方接口，TTL/超时/重试/采样参数全部取 base.json token
    /// （FIX007.9：TTL 单源于 token——原 DEFAULT_CACHE_TTL 字面量双源已删；
    /// A009 #10：采样参数同 token 化）
    /// 网络策略：先直连（no_proxy），直连网络失败且探测到系统代理时改走代理；
    /// 两通道都失败由 interface 层排 10 分钟后的自动重试
    pub fn new() -> Self {
        let base = Static::get();
        let mut client = WeatherClient::from_config(WeatherClientConfig {
            base: "https://api.open-meteo.com".to_string(),
            retry_spec: RetrySpec {
                retries: base.base_i64("weather_retries").max(1) as u32,
                delay: Duration::from_secs_f64(base.base_f64("weather_retry_delay")),
            },
            timeout: Duration::from_secs_f64(base.base_f64("weather_timeout_s")),
            connect_timeout: Duration::from_secs_f64(base.base_f64("weather_connect_timeout_s")),
            ttl: Duration::from_secs(base.base_i64("weather_cache_ttl") as u64),
            forecast_days: base.base_i64("weather_forecast_days"),
            forecast_hours: base.base_i64("weather_forecast_hours"),
            proxy_url: detect_system_proxy(),
            proxy_auto: true,
        });
        // PL016.03：生产启用磁盘缓存（FIX007.15 基准 exe 同目录；目录懒建）
        client.disk_cache_dir = Some(crate::util::paths::exe_dir().join("weather_cache"));
        client
    }

    /// 测试构造：注入 base/重试规格/超时/缓存 TTL/采样参数（httpmock 桩服务用）
    /// FIX007.9：TTL 改由调用方注入（原 DEFAULT_CACHE_TTL 与 token 同值双源漂移风险）
    pub fn custom(
        base: String,
        retry_spec: RetrySpec,
        timeout: Duration,
        ttl: Duration,
        forecast_days: i64,
        forecast_hours: i64,
    ) -> Self {
        Self::from_config(WeatherClientConfig {
            base,
            retry_spec,
            timeout,
            connect_timeout: Duration::from_secs(5),
            ttl,
            forecast_days,
            forecast_hours,
            proxy_url: None,
            proxy_auto: false,
        })
    }

    /// 双通道构造（proxy_url=None 仅直连；无效代理配置降级仅直连并记日志）
    fn from_config(cfg: WeatherClientConfig) -> Self {
        let direct =
            http_client(cfg.timeout, cfg.connect_timeout, None).expect("reqwest Client 构建失败");
        let mut proxy_url = None;
        let mut proxy_client = None;
        if let Some(url) = cfg.proxy_url.as_deref() {
            match http_client(cfg.timeout, cfg.connect_timeout, Some(url)) {
                Ok(client) => {
                    tracing::info!("天气启用系统代理回退: {}", proxy_label(url));
                    proxy_url = Some(url.to_string());
                    proxy_client = Some(client);
                }
                Err(e) => tracing::warn!(
                    "系统代理配置不可用，天气仅直连: {}（{e}）",
                    proxy_label(url)
                ),
            }
        } else if !cfg!(test) {
            tracing::info!("未检测到系统代理，天气仅直连");
        }
        WeatherClient {
            direct,
            proxy: Mutex::new(ProxyChannel {
                url: proxy_url,
                client: proxy_client,
            }),
            proxy_policy: Mutex::new(WeatherProxyPolicy {
                enabled: cfg.proxy_auto,
                server: String::new(),
            }),
            proxy_auto: cfg.proxy_auto,
            base: cfg.base,
            cache: Mutex::new(HashMap::new()),
            ttl: cfg.ttl,
            retry_spec: cfg.retry_spec,
            timeout: cfg.timeout,
            connect_timeout: cfg.connect_timeout,
            forecast_days: cfg.forecast_days,
            forecast_hours: cfg.forecast_hours,
            disk_cache_dir: None,
        }
    }

    /// 覆写缓存 TTL（测试用：ZERO 即刻过期 / 长值锁定命中）
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// 覆写系统代理通道（测试用；空串/非法 URL 保持仅直连）
    pub fn with_proxy(mut self, url: &str) -> Self {
        match http_client(self.timeout, self.connect_timeout, Some(url)) {
            Ok(client) => {
                let channel = self.proxy.get_mut().expect("天气代理锁中毒");
                channel.url = Some(url.to_string());
                channel.client = Some(client);
            }
            Err(e) => tracing::warn!(
                "系统代理配置不可用，保持仅直连: {}（{e}）",
                proxy_label(url)
            ),
        }
        self
    }

    /// 更新代理策略（设置页）：false=仅直连；server 空=跟随系统代理。
    /// 立即重建代理通道，下一次拉取生效（无需重启）
    pub fn set_proxy_policy(&self, enabled: bool, server: &str) {
        {
            let mut policy = self.proxy_policy.lock().expect("天气代理策略锁中毒");
            policy.enabled = enabled;
            policy.server = server.trim().to_string();
        }
        self.refresh_proxy();
    }

    /// 拉取前重读代理策略：目标变化才重建客户端（开关/改地址无需重启程序）。
    /// 生产专用（proxy_auto=true）；测试注入的代理通道不被本机策略覆盖。
    fn refresh_proxy(&self) {
        if !self.proxy_auto {
            return;
        }
        let policy = self
            .proxy_policy
            .lock()
            .expect("天气代理策略锁中毒")
            .clone();
        let mut invalid = false;
        let target = if !policy.enabled {
            None
        } else if policy.server.trim().is_empty() {
            detect_system_proxy()
        } else {
            match normalize_proxy(&policy.server) {
                Some(url) => Some(url),
                None => {
                    invalid = true;
                    None
                }
            }
        };
        let mut channel = self.proxy.lock().expect("天气代理锁中毒");
        if channel.url == target {
            return;
        }
        match target {
            Some(url) => match http_client(self.timeout, self.connect_timeout, Some(&url)) {
                Ok(client) => {
                    tracing::info!("天气代理已更新: {}", proxy_label(&url));
                    channel.url = Some(url);
                    channel.client = Some(client);
                }
                Err(e) => {
                    tracing::warn!("天气代理配置不可用，仅直连: {}（{e}）", proxy_label(&url));
                    channel.url = None;
                    channel.client = None;
                }
            },
            None => {
                if channel.url.is_some() {
                    tracing::info!("天气代理已关闭或不可用，天气改为仅直连");
                }
                channel.url = None;
                channel.client = None;
                if invalid {
                    tracing::warn!("天气代理地址非法，仅直连: {}", policy.server);
                }
            }
        }
    }

    /// 覆写磁盘缓存目录（PL016.03 测试用：沙箱目录隔离，防写 exe 旁）
    pub fn with_disk_cache(mut self, dir: PathBuf) -> Self {
        self.disk_cache_dir = Some(dir);
        self
    }

    /// 城市查询：缓存命中直返（Python get_weather_by_city 对等）；
    /// 未知城市 UnknownCity；force 先清该城缓存穿透
    /// PL016.03（审核 #2）：成功同步落盘缓存；失败回退磁盘陈旧值
    /// （StaleServed 携带数据与年龄上抛——UI 保留旧值 + 角标，重启可恢复）
    pub async fn fetch_by_city(&self, city: &str, force: bool) -> Result<WeatherData, FetchErr> {
        if force {
            self.cache.lock().expect("天气缓存锁中毒").remove(city);
        } else if let Some(w) = self.cached(city) {
            return Ok(w);
        }
        let (lat, lon) =
            cities::get(city).ok_or_else(|| FetchErr::UnknownCity(city.to_string()))?;
        match self.fetch_by_coords(lat, lon).await {
            Ok(data) => {
                // 仅缓存成功（失败可立即重试，Python 语义）
                // A008 #10：锁中毒策略与 force/读路径统一为 expect
                self.cache
                    .lock()
                    .expect("天气缓存锁中毒")
                    .insert(city.to_string(), (Instant::now(), data.clone()));
                self.disk_cache_write(city, &data);
                Ok(data)
            }
            Err(e) => match self.disk_cache_read(city) {
                Some((age_secs, stale)) => {
                    tracing::warn!("天气拉取失败，回退磁盘缓存: {city}（{age_secs}s 前）");
                    Err(FetchErr::StaleServed {
                        data: Box::new(stale),
                        age_secs,
                        source: Box::new(e),
                    })
                }
                None => Err(e),
            },
        }
    }

    /// 磁盘缓存快照（PL027 冷启动首屏）：只读磁盘，不发网络；返回 (年龄秒, 数据)
    pub fn cached_by_city(&self, city: &str) -> Option<(u64, WeatherData)> {
        self.disk_cache_read(city)
    }

    /// 缓存命中读取：未过 TTL 才有效
    fn cached(&self, city: &str) -> Option<WeatherData> {
        let cache = self.cache.lock().expect("天气缓存锁中毒");
        match cache.get(city) {
            Some((at, w)) if at.elapsed() < self.ttl => Some(w.clone()),
            _ => None,
        }
    }

    /// 磁盘缓存落盘（PL016.03，尽力而为）：{"fetched_at": epoch, "data": …}
    /// 写失败仅记日志——磁盘缓存是恢复手段，不构成失败路径
    fn disk_cache_write(&self, city: &str, data: &WeatherData) {
        let Some(dir) = &self.disk_cache_dir else {
            return;
        };
        let Ok(epoch) = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
        else {
            return;
        };
        let payload = serde_json::json!({ "fetched_at": epoch, "data": data });
        let path = dir.join(format!("{}.json", city));
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::warn!("天气磁盘缓存目录创建失败: {}（{e}）", parent.display());
            return;
        }
        match serde_json::to_string(&payload) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, text) {
                    tracing::warn!("天气磁盘缓存写入失败: {}（{e}）", path.display());
                }
            }
            Err(e) => tracing::warn!("天气磁盘缓存序列化失败: {e}"),
        }
    }

    /// 磁盘缓存读取（PL016.03）：返回 (年龄秒, 数据)；缺失/损坏/超龄上限返 None。
    /// 无 TTL 限制（断网回退语义），但 30 天上限防陈旧数据永久兜底
    fn disk_cache_read(&self, city: &str) -> Option<(u64, WeatherData)> {
        let dir = self.disk_cache_dir.as_ref()?;
        let text = std::fs::read_to_string(dir.join(format!("{city}.json"))).ok()?;
        let v: Value = serde_json::from_str(&text).ok()?;
        let fetched_at = v.get("fetched_at")?.as_u64()?;
        let data = serde_json::from_value::<WeatherData>(v.get("data")?.clone()).ok()?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        if now < fetched_at {
            return None; // 时钟回拨防御
        }
        let age = now - fetched_at;
        if age > disk_cache_max_age_secs() {
            tracing::warn!("天气磁盘缓存超龄丢弃: {city}（{age}s）");
            return None;
        }
        Some((age, data))
    }

    /// 经纬度查询：定义域校验 → URL 派生 → 直连 → 直连失败走系统代理 → 解析
    /// 有代理回退时直连只探一次（15s 级）后立即换代理；无代理时直连跑满重试次数
    /// 两通道网络失败合并为短错误，由 interface 层安排 10 分钟后的自动重试
    pub async fn fetch_by_coords(&self, lat: f64, lon: f64) -> Result<WeatherData, FetchErr> {
        if !(LAT_RANGE.0..=LAT_RANGE.1).contains(&lat)
            || !(LON_RANGE.0..=LON_RANGE.1).contains(&lon)
        {
            return Err(FetchErr::CoordsOutOfRange { lat, lon });
        }
        let url = format!(
            "{}/v1/forecast?latitude={lat}&longitude={lon}&current={}&timezone=auto\
             &hourly=temperature_2m,precipitation_probability\
             &daily=temperature_2m_max,temperature_2m_min,sunrise,sunset\
             &forecast_days={}&forecast_hours={}",
            self.base,
            REQUIRED_FIELDS.join(","),
            self.forecast_days,
            self.forecast_hours,
        );
        // 每次拉取前重读系统代理：10 分钟自动重试期间开关代理也能生效（无需重启）
        self.refresh_proxy();
        let proxy_client = self.proxy.lock().expect("天气代理锁中毒").client.clone();
        // 有代理回退时直连只做一次探针（15s 级），失败立即换代理，不把时间耗在
        // 同一条拥塞链路上；无代理时按配置重试次数跑满直连
        let direct_spec = if proxy_client.is_some() {
            RetrySpec {
                retries: 1,
                delay: self.retry_spec.delay,
            }
        } else {
            self.retry_spec
        };
        let direct_err =
            match retry(direct_spec, || Self::fetch_once(&self.direct, &url, "直连")).await {
                Ok(data) => return Ok(data),
                Err(e) => e,
            };
        // 解析类错误也换代理一次机会（被劫持/门户页可能只在直连出现）；代理侧
        // 解析错同样直通，不做 10 分钟自动重试
        let Some(proxy) = proxy_client.as_ref() else {
            return Err(direct_err);
        };
        tracing::warn!("天气直连失败，改走系统代理重试: {direct_err}");
        match retry(self.retry_spec, || {
            Self::fetch_once(proxy, &url, "系统代理")
        })
        .await
        {
            Ok(data) => Ok(data),
            Err(FetchErr::Network(proxy_err)) => {
                tracing::warn!("天气直连与系统代理均失败: 直连={direct_err}; 代理={proxy_err}");
                Err(FetchErr::Network("直连与系统代理均失败".to_string()))
            }
            Err(e) => Err(e),
        }
    }

    /// 单次请求（供 retry 闭包）：非 2xx/网络错 → Network（可重试）；体解析 → Parse
    /// 日志带 source 链（reqwest Display 只给「error sending request for url」，
    /// 底层超时/连接重置需逐层取 source 才可见）
    async fn fetch_once(
        client: &reqwest::Client,
        url: &str,
        via: &str,
    ) -> Result<WeatherData, FetchErr> {
        let resp = client.get(url).send().await.map_err(|e| {
            tracing::warn!("天气{via}请求失败: {}", error_chain(&e));
            FetchErr::Network(e.to_string())
        })?;
        let resp = resp.error_for_status().map_err(|e| {
            tracing::warn!("天气{via}响应状态异常: {}", error_chain(&e));
            FetchErr::Network(e.to_string())
        })?;
        let body: Value = resp.json().await.map_err(|e| {
            tracing::warn!("天气{via}响应解析失败: {}", error_chain(&e));
            FetchErr::Parse(e.to_string())
        })?;
        parse_open_meteo(&body).map_err(FetchErr::from)
    }
}

/// 错误链展开（日志用）：逐层拼接 Display，保留超时/连接重置等底层原因
fn error_chain(err: &(dyn std::error::Error + 'static)) -> String {
    let mut out = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        out.push_str(" <- ");
        out.push_str(&cause.to_string());
        source = cause.source();
    }
    out
}

// ===== src/weather.rs 说明区 =====
// 解析层（PL011.11）：输入 Open-Meteo JSON 响应（&Value）；输出 WeatherData 或
//   结构化 WeatherErr；判定顺序（根对象 → current → 字段缺失/null → 数值类型 →
//   码表回退）与 Python 逐分支对齐，61 条夹具样本断言决策一致
// 传输层（PL011.12 / PL025）：WeatherClient 输入城市名/坐标 + force；输出 WeatherData 或
//   FetchErr（UnknownCity/CoordsOutOfRange/Network/Parse/StaleServed——A011 订正补记
//   PL016.03 变体：拉取失败但磁盘缓存有历史时携旧值与年龄上抛，UI 保留旧值 + 角标）；
//   内存缓存仅存成功、命中未过 TTL 直返、force 清键穿透、失败可立即重试；磁盘缓存
//   （exe 旁 weather_cache/，30 天上限）成功落盘、失败尽力回退静默降级——
//   Python 缓存语义逐条对齐
// 网络策略（PL025/PL027）：用户设置优先（WeatherProxyPolicy：关=仅直连；
//   开+空=系统探测；开+地址=指定代理）；直连通道显式 no_proxy 先尝试；
//   允许代理时直连只探一次，失败后代理通道按 weather_retries 重试；每次拉取前
//   重读策略（URL 变化才重建 client），开关/改地址无需重启；无代理时直连按
//   weather_retries 跑满。两通道都网络失败合并为短 Network 错误，由 interface 排
//   weather_fail_retry_delay_s（默认 600s）后的自动重试；解析类错误不自动重试
//   （换时间不会改变响应结构）。冷启动由 UI 先读 cached_by_city 磁盘快照上屏，
//   首帧后才发起网络首拉
// 设计理由：Python 把全部失败降级 None + 日志；Rust 以 Result 保留拒绝原因供
//   传输层/UI 层决策。SSRF 防线结构性强化：base 构造期固定（生产 https 常量），
//   URL 全由 base + 已过定义域校验的坐标派生，用户输入永不进入 host——Python 的
//   运行期 scheme/host 白名单 + DNS 私网阻断针对「URL 可被操纵」场景，Rust 侧
//   该威胁在构造期即消除（偏移登记 z.plan）；重定向禁跟随（Policy::none 对等）
// 异常处理：网络/非 2xx/超时 → Network（唯一可重试变体，对应 Python 白名单）；
//   体解码失败 → Parse 直通（Python JSONDecodeError 不重试对等）；解析结构错 →
//   Parse 直通；坐标越界/未知城市 → 立即拒绝不上抛 panic；错误日志逐层展开
//   source 链（reqwest Display 只有「error sending request for url」，底层超时/
//   连接重置须取 source 才可见）
// 已知偏移：①weather_code 归一 i64（Python 可持 "0"/True 原样）——展示 parity
//   优先；②serde_json 缩进/NaN 文本层差异（见 settings 说明区）；③SSRF 防线由
//   运行期校验改为构造期固定（防线等价性登记）；④整值浮点码（0.0）自判命中
//   整键（serde as_i64 对浮点底数恒 None）；⑤系统代理仅支持 http/https
//   （socks 需 reqwest socks 特性，本客户端不启用，配置为 socks 时仅直连）
// 关联配置：weather_cache_ttl/weather_disk_cache_max_age_s（FIX012.02 token 化）/
//   weather_timeout_s/weather_connect_timeout_s/weather_retries/weather_retry_delay/
//   weather_fail_retry_delay_s/weather_forecast_days/weather_forecast_hours
//   （base.json）；城市表 data::cities；重试 util::retry；消费方：interface（.13）
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normal_response_parses() {
        // 正常样本：int/float 混收，码 0 → 晴
        let raw = json!({"current": {
            "temperature_2m": 23.5, "relative_humidity_2m": 62, "weather_code": 0,
            "wind_speed_10m": 10.8, "apparent_temperature": 25.1 }});
        let w = parse_open_meteo(&raw).expect("应解析成功");
        assert_eq!(w.temperature, 23.5);
        assert_eq!(w.humidity, 62.0);
        assert_eq!(w.weather, "晴");
        assert_eq!(w.icon, "☀️");
        assert_eq!(w.description, "晴朗无云");
    }

    #[test]
    fn structural_rejections() {
        // 根非对象 / current 缺失 / current 非对象
        assert_eq!(parse_open_meteo(&json!([])), Err(WeatherErr::NotObject));
        assert_eq!(parse_open_meteo(&json!("x")), Err(WeatherErr::NotObject));
        assert_eq!(
            parse_open_meteo(&json!({"foo": 1})),
            Err(WeatherErr::CurrentNotObject)
        );
        assert_eq!(
            parse_open_meteo(&json!({"current": []})),
            Err(WeatherErr::CurrentNotObject)
        );
    }

    #[test]
    fn field_level_rejections() {
        // 缺失/null/错型（字符串/布尔）逐分支
        let base = json!({"current": {
            "temperature_2m": 1.0, "relative_humidity_2m": 2.0, "weather_code": 3,
            "wind_speed_10m": 4.0, "apparent_temperature": 5.0 }});
        assert_eq!(
            parse_open_meteo(&json!({"current": {}})),
            Err(WeatherErr::MissingField("temperature_2m"))
        );
        let mut null_temp = base.clone();
        null_temp["current"]["temperature_2m"] = Value::Null;
        assert_eq!(
            parse_open_meteo(&null_temp),
            Err(WeatherErr::NullField("temperature_2m"))
        );
        let mut str_wind = base.clone();
        str_wind["current"]["wind_speed_10m"] = json!("4.0");
        assert_eq!(
            parse_open_meteo(&str_wind),
            Err(WeatherErr::BadType("wind_speed_10m"))
        );
        let mut bool_hum = base.clone();
        bool_hum["current"]["relative_humidity_2m"] = json!(true);
        assert_eq!(
            parse_open_meteo(&bool_hum),
            Err(WeatherErr::BadType("relative_humidity_2m"))
        );
    }

    #[test]
    fn series_string_wrong_type_is_badtype() {
        // A011 #18（FIX011.18）：系列字符串元素非串与 current 域同拒绝纪律——
        // BadType（错型）而非 SeriesMissing（缺失），错型/缺失分档
        let raw = json!({"current": {
            "temperature_2m": 1.0, "relative_humidity_2m": 2.0, "weather_code": 0,
            "wind_speed_10m": 4.0, "apparent_temperature": 5.0 },
            "hourly": {"time": [1, 2], "temperature_2m": [1.0, 2.0],
                "precipitation_probability": [1, 2]}});
        assert_eq!(
            parse_open_meteo(&raw),
            Err(WeatherErr::BadType("time")),
            "字符串系列元素非串应报 BadType"
        );
    }

    #[test]
    fn weather_code_scalar_parity() {
        // Python dict 查找语义：bool True 命中码 1、浮点 0.0 命中码 0、字符串/小数回退未知
        let mk = |code: Value| {
            parse_open_meteo(&json!({"current": {
            "temperature_2m": 1.0, "relative_humidity_2m": 2.0, "weather_code": code,
            "wind_speed_10m": 4.0, "apparent_temperature": 5.0 }}))
        };
        assert_eq!(mk(json!(true)).unwrap().weather, "晴");
        assert_eq!(mk(json!(0.0)).unwrap().weather, "晴");
        assert_eq!(mk(json!("0")).unwrap().weather, "未知");
        assert_eq!(mk(json!(42)).unwrap().description, "未知天气");
        assert_eq!(mk(json!(2.5)).unwrap().icon, "🌡️");
    }

    #[test]
    fn format_text_matches_python_shape() {
        // 展示文本形态：城市前缀/一位小数/湿度整数（逐字符对齐交由夹具 parity 测试）
        let raw = json!({"current": {
            "temperature_2m": 23.5, "relative_humidity_2m": 62, "weather_code": 2,
            "wind_speed_10m": 10.8, "apparent_temperature": 25.1 }});
        let w = parse_open_meteo(&raw).unwrap();
        let text = format_weather_info(Some(&w), "北京");
        assert_eq!(
            text,
            "北京 ⛅ 多云 | 23.5°C | 体感 25.1°C | 湿度 62% | 风力 10.8km/h"
        );
        assert_eq!(
            format_weather_info(Some(&w), ""),
            "⛅ 多云 | 23.5°C | 体感 25.1°C | 湿度 62% | 风力 10.8km/h"
        );
        assert_eq!(format_weather_info(None, "北京"), "天气信息获取失败");
    }

    // ===== 传输层五路径（PL011.12，httpmock 桩服务）=====

    use crate::util::retry::RetrySpec;
    use httpmock::{Method, MockServer};

    fn full_body(code: i64) -> serde_json::Value {
        // PL014.11 全量样本：current + hourly（2 点）+ daily（2 点）
        serde_json::json!({
            "current": {
                "temperature_2m": 20.5, "relative_humidity_2m": 55, "weather_code": code,
                "wind_speed_10m": 8.4, "apparent_temperature": 21.0 },
            "hourly": {
                "time": ["2026-10-17T00:00", "2026-10-17T01:00"],
                "temperature_2m": [18.0, 17.5],
                "precipitation_probability": [10, 20] },
            "daily": {
                "time": ["2026-10-17", "2026-10-18"],
                "temperature_2m_max": [24.0, 23.0],
                "temperature_2m_min": [15.0, 14.5],
                "sunrise": ["2026-10-17T06:20", "2026-10-18T06:21"],
                "sunset": ["2026-10-17T17:45", "2026-10-18T17:44"] }
        })
    }

    fn stub(server: &MockServer) -> httpmock::Mock<'_> {
        server.mock(|when, then| {
            when.method(Method::GET)
                .path("/v1/forecast")
                .query_param("latitude", "39.9042")
                .query_param("longitude", "116.4074")
                .query_param("timezone", "auto");
            then.status(200).json_body(full_body(0));
        })
    }

    fn fast_client(base: String) -> WeatherClient {
        WeatherClient::custom(
            base,
            RetrySpec {
                retries: 3,
                delay: Duration::ZERO,
            },
            Duration::from_secs(5),
            Duration::from_secs(3600),
            7,
            24,
        )
    }

    #[tokio::test]
    async fn cache_hit_avoids_second_request() {
        // 路径 1 缓存命中：两次查询仅一次网络请求（长 TTL 锁定）
        let server = MockServer::start();
        let mock = stub(&server);
        let client = fast_client(server.base_url()).with_ttl(Duration::from_secs(3600));
        let w1 = client
            .fetch_by_city("北京", false)
            .await
            .expect("首次应成功");
        let w2 = client
            .fetch_by_city("北京", false)
            .await
            .expect("缓存命中应成功");
        assert_eq!(
            (w1.weather.clone(), w2.weather),
            ("晴".to_string(), "晴".to_string())
        );
        assert_eq!(mock.calls(), 1, "缓存命中不应再发请求");
    }

    #[tokio::test]
    async fn ttl_expiry_refetches() {
        // 路径 2 TTL 过期：ZERO TTL 下两次查询各发一次请求
        let server = MockServer::start();
        let mock = stub(&server);
        let client = fast_client(server.base_url()).with_ttl(Duration::ZERO);
        client
            .fetch_by_city("北京", false)
            .await
            .expect("首次应成功");
        client
            .fetch_by_city("北京", false)
            .await
            .expect("过期后应重新拉取");
        assert_eq!(mock.calls(), 2);
    }

    #[tokio::test]
    async fn retry_exhaustion_surfaces_network_error() {
        // 路径 3 重试上限：恒 500 → 网络错耗尽上抛（调用次数 = retries）
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(Method::GET).path("/v1/forecast");
            then.status(500);
        });
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
        let err = client
            .fetch_by_city("北京", false)
            .await
            .expect_err("应耗尽失败");
        assert!(matches!(err, FetchErr::Network(_)), "应为网络错: {err}");
        assert_eq!(mock.calls(), 2, "重试次数应恰为 retries");
    }

    #[tokio::test]
    async fn force_bypasses_cache() {
        // 路径 4 force 穿透：缓存已热仍强制重拉
        let server = MockServer::start();
        let mock = stub(&server);
        let client = fast_client(server.base_url()).with_ttl(Duration::from_secs(3600));
        client
            .fetch_by_city("北京", false)
            .await
            .expect("首次应成功");
        client
            .fetch_by_city("北京", true)
            .await
            .expect("force 应穿透成功");
        assert_eq!(mock.calls(), 2);
    }

    #[tokio::test]
    async fn input_rejection_without_request() {
        // 路径 5 白名单拒绝（输入侧）：未知城市/坐标越界立即拒绝，零网络请求；
        //   且 URL 派生断言——host 全来自构造期 base，坐标经定义域校验（SSRF 结构性证明）
        let server = MockServer::start();
        let mock = stub(&server);
        let client = fast_client(server.base_url());
        assert!(matches!(
            client.fetch_by_city("不存在城", false).await,
            Err(FetchErr::UnknownCity(_))
        ));
        assert!(matches!(
            client.fetch_by_coords(200.0, 0.0).await,
            Err(FetchErr::CoordsOutOfRange { .. })
        ));
        // 成功路径的 URL 形态：query 恰为派生参数（stub 的 query_param 匹配即证明）
        client.fetch_by_city("北京", false).await.expect("应成功");
        mock.assert();
        assert_eq!(mock.calls(), 1, "拒绝路径零请求 + 成功路径恰一次");
    }

    #[tokio::test]
    async fn fetch_failure_serves_disk_stale() {
        // PL016.03（审核 #2）验收：首拉成功落盘 → 服务转 500 → force 拉取失败
        // 回退磁盘陈旧值（StaleServed 携带数据），重启后（新 client 同目录）同享
        let server = MockServer::start();
        let mut mock = stub(&server);
        let sandbox = std::env::temp_dir().join(format!("accel_wcache_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&sandbox);
        let client = fast_client(server.base_url()).with_disk_cache(sandbox.clone());
        let fresh = client
            .fetch_by_city("北京", false)
            .await
            .expect("首拉应成功");
        // PL027：冷启动首屏只读磁盘快照
        let (age, cached) = client.cached_by_city("北京").expect("磁盘缓存快照应可用");
        assert_eq!(cached.temperature, fresh.temperature);
        assert!(age < 60, "刚写入缓存年龄应很小: {age}s");
        mock.delete();
        server.mock(|when, then| {
            when.method(Method::GET)
                .path("/v1/forecast")
                .query_param("latitude", "39.9042")
                .query_param("longitude", "116.4074")
                .query_param("timezone", "auto");
            then.status(500).body("boom");
        });
        match client.fetch_by_city("北京", true).await {
            Err(FetchErr::StaleServed { data, .. }) => {
                assert_eq!(
                    data.temperature, fresh.temperature,
                    "陈旧值应为上次成功数据"
                );
            }
            other => panic!("应回退磁盘缓存，实际 {other:?}"),
        }
        // 重启等价：新 client 同目录（内存缓存为空 + force）仍可回退
        let client2 = fast_client(server.base_url()).with_disk_cache(sandbox.clone());
        assert!(matches!(
            client2.fetch_by_city("北京", true).await,
            Err(FetchErr::StaleServed { .. })
        ));
        let _ = std::fs::remove_dir_all(&sandbox);
    }

    #[test]
    fn humanize_age_buckets() {
        // PL016.03 话术分桶：秒/分钟/小时/天
        assert_eq!(humanize_age(5), "5 秒前");
        assert_eq!(humanize_age(125), "2 分钟前");
        assert_eq!(humanize_age(3 * 3600), "3 小时前");
        assert_eq!(humanize_age(2 * 24 * 3600), "2 天前");
    }

    // ===== PL014.11 天气扩展（hourly/daily）路径 =====

    #[tokio::test]
    async fn hourly_daily_parsed_and_cached_as_one() {
        // 全字段断言 + 缓存一体（current+hourly+daily 同缓存键，命中不重发）
        let server = MockServer::start();
        let mock = stub(&server);
        let client = fast_client(server.base_url()).with_ttl(Duration::from_secs(3600));
        let w = client.fetch_by_city("北京", false).await.expect("应成功");
        assert_eq!(w.hourly.len(), 2);
        assert_eq!(w.hourly[0].time, "2026-10-17T00:00");
        assert_eq!(w.hourly[0].precip_prob, 10.0);
        assert_eq!(w.daily.len(), 2);
        assert_eq!(w.daily[0].t_max, 24.0);
        assert_eq!(w.daily[0].sunrise, "2026-10-17T06:20");
        client.fetch_by_city("北京", false).await.expect("缓存命中");
        assert_eq!(mock.calls(), 1, "缓存命中不重发");
    }

    #[tokio::test]
    async fn hourly_null_rejects_whole_packet() {
        // hourly 含 null → 整包 Err（HourlyNull）
        let server = MockServer::start();
        let mut body = full_body(0);
        body["hourly"]["temperature_2m"][1] = serde_json::Value::Null;
        server.mock(|when, then| {
            when.method(Method::GET).path("/v1/forecast");
            then.status(200).json_body(body);
        });
        let client = fast_client(server.base_url());
        let err = client
            .fetch_by_city("北京", false)
            .await
            .expect_err("null 应整包拒绝");
        assert!(err.to_string().contains("逐小时数据含 null"), "{err}");
    }

    #[tokio::test]
    async fn daily_missing_sunset_rejects() {
        // daily 缺 sunset 键 → 整包 Err（SeriesMissing）
        let server = MockServer::start();
        let mut body = full_body(0);
        body["daily"]
            .as_object_mut()
            .expect("对象")
            .remove("sunset");
        server.mock(|when, then| {
            when.method(Method::GET).path("/v1/forecast");
            then.status(200).json_body(body);
        });
        let client = fast_client(server.base_url());
        let err = client
            .fetch_by_city("北京", false)
            .await
            .expect_err("缺 sunset 应拒绝");
        assert!(
            err.to_string().contains("长度不一致") || err.to_string().contains("缺失"),
            "{err}"
        );
    }

    #[test]
    fn proxy_policy_switch_controls_channel() {
        // PL027：关闭=仅直连；打开+地址=用该地址；测试只建 client，不发网络
        let client = WeatherClient::new();
        client.set_proxy_policy(false, "http://127.0.0.1:10808");
        assert!(
            client.proxy.lock().expect("代理锁").url.is_none(),
            "关闭代理应清空代理通道"
        );
        client.set_proxy_policy(true, "http://127.0.0.1:9");
        assert_eq!(
            client.proxy.lock().expect("代理锁").url.as_deref(),
            Some("http://127.0.0.1:9"),
            "打开代理应使用配置地址"
        );
    }

    // ===== 天气双通道（直连 → 系统代理）=====

    #[test]
    fn proxy_normalization_handles_windows_forms() {
        // 纯 host:port / 带 scheme / Windows 分协议串（https 优先）/ socks 拒绝
        assert_eq!(
            normalize_proxy("127.0.0.1:10808").as_deref(),
            Some("http://127.0.0.1:10808")
        );
        assert_eq!(
            normalize_proxy("http://127.0.0.1:10808").as_deref(),
            Some("http://127.0.0.1:10808")
        );
        assert_eq!(
            normalize_proxy("http=10.0.0.1:80;https=10.0.0.2:443").as_deref(),
            Some("http://10.0.0.2:443")
        );
        assert_eq!(normalize_proxy("socks=127.0.0.1:1080"), None);
        assert_eq!(normalize_proxy("  "), None);
    }

    /// 单次尝试客户端（双通道测试用：失败不叠加重试时长）
    fn one_shot_client(base: String) -> WeatherClient {
        WeatherClient::custom(
            base,
            RetrySpec {
                retries: 1,
                delay: Duration::ZERO,
            },
            Duration::from_secs(5),
            Duration::from_secs(3600),
            7,
            24,
        )
    }

    #[tokio::test]
    async fn direct_failure_falls_back_to_proxy() {
        // 直连桩恒 500 → 自动改走代理桩，代理 200 命中
        let direct = MockServer::start();
        let proxy = MockServer::start();
        direct.mock(|when, then| {
            when.method(Method::GET).path("/v1/forecast");
            then.status(500);
        });
        let proxy_mock = proxy.mock(|when, then| {
            when.method(Method::GET).path("/v1/forecast");
            then.status(200).json_body(full_body(0));
        });
        let client = one_shot_client(direct.base_url()).with_proxy(&proxy.base_url());
        let data = client
            .fetch_by_coords(39.9042, 116.4074)
            .await
            .expect("代理通道应成功");
        assert_eq!(data.weather, "晴");
        assert_eq!(proxy_mock.calls(), 1, "代理通道应恰被调用一次");
    }

    #[tokio::test]
    async fn both_channels_fail_returns_compact_network_error() {
        // 两通道都 500 → 合并为短 Network（interface 层据此排 10 分钟重试）
        let direct = MockServer::start();
        let proxy = MockServer::start();
        for server in [&direct, &proxy] {
            server.mock(|when, then| {
                when.method(Method::GET).path("/v1/forecast");
                then.status(500);
            });
        }
        let client = one_shot_client(direct.base_url()).with_proxy(&proxy.base_url());
        let err = client
            .fetch_by_coords(39.9042, 116.4074)
            .await
            .expect_err("两通道都应失败");
        assert!(
            matches!(err, FetchErr::Network(ref m) if m.contains("直连与系统代理均失败")),
            "{err}"
        );
    }

    #[test]
    fn parse_legacy_sample_without_hourly_yields_empty_series() {
        // 向后兼容：无 hourly/daily 键（Python 0.6.0.0 快照形态）→ 空序列不拒绝
        let raw = serde_json::json!({"current": {
            "temperature_2m": 1.0, "relative_humidity_2m": 2.0, "weather_code": 0,
            "wind_speed_10m": 3.0, "apparent_temperature": 4.0 }});
        let w = parse_open_meteo(&raw).expect("旧形态应解析");
        assert!(w.hourly.is_empty() && w.daily.is_empty());
    }

    #[test]
    fn series_container_wrong_type_rejects() {
        // A008 #9 验收：hourly/daily 键存在但容器错型（串/数组）→ BadType 拒绝，
        // 不再静默按缺失处理（原 and_then(as_object) 把错型当缺失与注释承诺相反）
        let base = serde_json::json!({"current": {
            "temperature_2m": 1.0, "relative_humidity_2m": 2.0, "weather_code": 0,
            "wind_speed_10m": 3.0, "apparent_temperature": 4.0 }});
        let mut raw = base.clone();
        raw["hourly"] = serde_json::json!("oops");
        assert_eq!(
            parse_open_meteo(&raw).expect_err("hourly 错型应拒绝"),
            WeatherErr::BadType("hourly")
        );
        let mut raw = base;
        raw["daily"] = serde_json::json!([1, 2]);
        assert_eq!(
            parse_open_meteo(&raw).expect_err("daily 错型应拒绝"),
            WeatherErr::BadType("daily")
        );
    }

    #[test]
    fn weather_code_container_yields_parse_error() {
        // FIX007.16：weather_code 为数组/对象 → Parse 错误（Python dict.get(list)
        // 抛 TypeError 走失败路径对齐；原实现误与字符串同归未知）
        let mk = |code: Value| {
            serde_json::json!({"current": {
                "temperature_2m": 1.0, "relative_humidity_2m": 2.0, "weather_code": code,
                "wind_speed_10m": 3.0, "apparent_temperature": 4.0 }})
        };
        for code in [serde_json::json!([0]), serde_json::json!({"c": 0})] {
            let err = parse_open_meteo(&mk(code)).expect_err("容器码应 Parse 错误");
            assert_eq!(err, WeatherErr::Parse("weather_code"));
        }
        // 字符串仍走未知码回退不报错（Python dict.get(str) → None 同语义）
        let w = parse_open_meteo(&mk(serde_json::json!("sunny"))).expect("字符串码应成功");
        assert_eq!(w.weather, "未知");
    }
}
