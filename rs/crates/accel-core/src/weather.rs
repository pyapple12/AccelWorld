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
    let Some(hourly) = root.get("hourly").and_then(Value::as_object) else {
        // A005 #18：hourly 缺失不再连带丢弃 daily——两者独立判定
        return Ok((
            Vec::new(),
            match root.get("daily").and_then(Value::as_object) {
                Some(daily) => extract_daily(daily)?,
                None => Vec::new(),
            },
        ));
    };
    let Some(_) = root.get("daily").and_then(Value::as_object) else {
        return Ok((extract_hourly(hourly)?, Vec::new()));
    };
    let daily = root
        .get("daily")
        .and_then(Value::as_object)
        .expect("前置判定保证存在");
    Ok((extract_hourly(hourly)?, extract_daily(daily)?))
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

/// 字符串系列提取（键缺失/非串元素 → SeriesMissing/对应 Null 变体由调用方语义定）
fn series_strings(
    obj: &serde_json::Map<String, Value>,
    key: &'static str,
) -> Result<Vec<String>, WeatherErr> {
    match obj.get(key) {
        None => Err(WeatherErr::SeriesMissing(key)),
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|v| {
                v.as_str()
                    .map(String::from)
                    .ok_or(WeatherErr::SeriesMissing(key))
            })
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
    // weather_code 任意标量可过（Python 语义）：数值取整查表、布尔按 1/0 查表
    // （Python dict 键 True==1 命中码 1——夹具实证）、字符串等按未知码回退
    let code_key = scalar_code_key(&current["weather_code"]);
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
/// 小数浮点/字符串等返未知哨兵（i64::MIN，不命中任何整键 → 未知）
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
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

/// 天气客户端：按城缓存 TTL + 重试 + force 穿透
/// SSRF 防线（结构性）：base 构造期固定（生产 = https://api.open-meteo.com 常量），
/// 运行期 URL 全部由 base + 已校验坐标派生，用户输入永不进入 host 部分；
/// 禁跟随重定向（reqwest Policy::none，Python no-redirect opener 对等）
pub struct WeatherClient {
    client: reqwest::Client,
    base: String,
    cache: Mutex<HashMap<String, (Instant, WeatherData)>>,
    ttl: Duration,
    retry_spec: RetrySpec,
}

impl Default for WeatherClient {
    /// 默认构造（委托生产构造：clippy new_without_default）
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherClient {
    /// 生产构造：base 固定官方接口，TTL/超时/重试参数全部取 base.json token
    pub fn new() -> Self {
        let base = Static::get();
        let timeout = Duration::from_secs_f64(base.base_f64("weather_timeout_s"));
        let retry_spec = RetrySpec {
            retries: base.base_i64("weather_retries").max(1) as u32,
            delay: Duration::from_secs_f64(base.base_f64("weather_retry_delay")),
        };
        WeatherClient::custom(
            "https://api.open-meteo.com".to_string(),
            retry_spec,
            timeout,
        )
        .with_ttl(Duration::from_secs(
            base.base_i64("weather_cache_ttl") as u64
        ))
    }

    /// 测试构造默认缓存 TTL（FIX006.21：与 base.json weather_cache_ttl 同值单处定义——
    /// 生产 `new()` 读 token 覆盖，测试/定制构造用本常量兜底，避免双源漂移）
    const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(1800);

    /// 测试构造：注入 base/重试规格/超时（httpmock 桩服务用）
    pub fn custom(base: String, retry_spec: RetrySpec, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest Client 构建失败");
        WeatherClient {
            client,
            base,
            cache: Mutex::new(HashMap::new()),
            ttl: Self::DEFAULT_CACHE_TTL,
            retry_spec,
        }
    }

    /// 覆写缓存 TTL（测试用：ZERO 即刻过期 / 长值锁定命中）
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// 城市查询：缓存命中直返（Python get_weather_by_city 对等）；
    /// 未知城市 UnknownCity；force 先清该城缓存穿透
    pub async fn fetch_by_city(&self, city: &str, force: bool) -> Result<WeatherData, FetchErr> {
        if force {
            self.cache.lock().expect("天气缓存锁中毒").remove(city);
        } else if let Some(w) = self.cached(city) {
            return Ok(w);
        }
        let (lat, lon) =
            cities::get(city).ok_or_else(|| FetchErr::UnknownCity(city.to_string()))?;
        let result = self.fetch_by_coords(lat, lon).await?;
        // 仅缓存成功（失败可立即重试，Python 语义）
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(city.to_string(), (Instant::now(), result.clone()));
        }
        Ok(result)
    }

    /// 缓存命中读取：未过 TTL 才有效
    fn cached(&self, city: &str) -> Option<WeatherData> {
        let cache = self.cache.lock().expect("天气缓存锁中毒");
        match cache.get(city) {
            Some((at, w)) if at.elapsed() < self.ttl => Some(w.clone()),
            _ => None,
        }
    }

    /// 经纬度查询：定义域校验 → URL 派生 → 重试网络层 → 解析
    pub async fn fetch_by_coords(&self, lat: f64, lon: f64) -> Result<WeatherData, FetchErr> {
        if !(LAT_RANGE.0..=LAT_RANGE.1).contains(&lat)
            || !(LON_RANGE.0..=LON_RANGE.1).contains(&lon)
        {
            return Err(FetchErr::CoordsOutOfRange { lat, lon });
        }
        let url = format!(
            "{}/v1/forecast?latitude={lat}&longitude={lon}&current={}&timezone=auto\
             &hourly=temperature_2m,precipitation_probability\
             &daily=temperature_2m_max,temperature_2m_min,sunrise,sunset&forecast_days=7\
             &forecast_hours=24",
            self.base,
            REQUIRED_FIELDS.join(","),
        );
        retry(self.retry_spec, || self.fetch_once(&url)).await
    }

    /// 单次请求（供 retry 闭包）：非 2xx/网络错 → Network（可重试）；体解析 → Parse
    async fn fetch_once(&self, url: &str) -> Result<WeatherData, FetchErr> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FetchErr::Network(e.to_string()))?;
        let resp = resp
            .error_for_status()
            .map_err(|e| FetchErr::Network(e.to_string()))?;
        let body: Value = resp
            .json()
            .await
            .map_err(|e| FetchErr::Parse(e.to_string()))?;
        parse_open_meteo(&body).map_err(FetchErr::from)
    }
}

// ===== src/weather.rs 说明区 =====
// 解析层（PL011.11）：输入 Open-Meteo JSON 响应（&Value）；输出 WeatherData 或
//   结构化 WeatherErr；判定顺序（根对象 → current → 字段缺失/null → 数值类型 →
//   码表回退）与 Python 逐分支对齐，61 条夹具样本断言决策一致
// 传输层（PL011.12）：WeatherClient 输入城市名/坐标 + force；输出 WeatherData 或
//   FetchErr（UnknownCity/CoordsOutOfRange/Network/Parse）；缓存仅存成功、命中
//   未过 TTL 直返、force 清键穿透、失败可立即重试——Python 缓存语义逐条对齐
// 设计理由：Python 把全部失败降级 None + 日志；Rust 以 Result 保留拒绝原因供
//   传输层/UI 层决策。SSRF 防线结构性强化：base 构造期固定（生产 https 常量），
//   URL 全由 base + 已过定义域校验的坐标派生，用户输入永不进入 host——Python 的
//   运行期 scheme/host 白名单 + DNS 私网阻断针对「URL 可被操纵」场景，Rust 侧
//   该威胁在构造期即消除（偏移登记 z.plan）；重定向禁跟随（Policy::none 对等）
// 异常处理：网络/非 2xx/超时 → Network（唯一可重试变体，对应 Python 白名单）；
//   体解码失败 → Parse 直通（Python JSONDecodeError 不重试对等）；解析结构错 →
//   Parse 直通；坐标越界/未知城市 → 立即拒绝不上抛 panic
// 已知偏移：①weather_code 归一 i64（Python 可持 "0"/True 原样）——展示 parity
//   优先；②serde_json 缩进/NaN 文本层差异（见 settings 说明区）；③SSRF 防线由
//   运行期校验改为构造期固定（防线等价性登记）；④整值浮点码（0.0）自判命中
//   整键（serde as_i64 对浮点底数恒 None）
// 关联配置：weather_cache_ttl/weather_timeout_s/weather_retries/weather_retry_delay
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
    fn parse_legacy_sample_without_hourly_yields_empty_series() {
        // 向后兼容：无 hourly/daily 键（Python 0.6.0.0 快照形态）→ 空序列不拒绝
        let raw = serde_json::json!({"current": {
            "temperature_2m": 1.0, "relative_humidity_2m": 2.0, "weather_code": 0,
            "wind_speed_10m": 3.0, "apparent_temperature": 4.0 }});
        let w = parse_open_meteo(&raw).expect("旧形态应解析");
        assert!(w.hourly.is_empty() && w.daily.is_empty());
    }
}
