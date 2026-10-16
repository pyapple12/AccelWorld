//! 用户配置（config/settings.py 的 Rust 对等物）
//! UserConfig serde 镜像（字段名与 user_config.json 同 schema）+ lenient 装载 +
//! 原子保存 + 损坏转存；默认值单源于 base.json token（default_factory 对等）

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::static_config::Static;
use crate::util::file_io;

/// 用户配置聚合（Python UserConfig dataclass 镜像）
/// serde 字段名与 Python to_dict 输出完全一致（持久化兼容硬约束）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserConfig {
    pub time_dilation_rate: f64,
    pub theme: String,
    pub last_city: String,
    pub last_timezone: String,
    pub world_pins: Vec<String>,
    pub countdown_target: String,
    pub window_geometry: Option<String>,
    /// 闹钟列表（PL012 落地 Alarm 强类型；本期透传 JSON 形态，兼容既有存档）
    pub alarms: Vec<Value>,
}

impl Default for UserConfig {
    /// 默认值单源：rate/theme/city/timezone/pins 全部现取 base.json token
    /// （Python default_factory 语义对等——配置期求值而非编译期常量）
    fn default() -> Self {
        let base = Static::get();
        UserConfig {
            time_dilation_rate: base.base_f64("default_rate"),
            theme: base.base_str("default_theme").to_string(),
            last_city: base.base_str("default_city").to_string(),
            last_timezone: base.base_str("default_timezone").to_string(),
            world_pins: base
                .base_str_list("default_world_pins")
                .into_iter()
                .map(String::from)
                .collect(),
            countdown_target: String::new(),
            window_geometry: None,
            alarms: Vec::new(),
        }
    }
}

// 装载中间层：全字段 Option<Value>——错型值以 Value 原样进入，逐字段过滤（Python
// dataclass_from_dict 的类型校验过滤对等：类型不符剔除 → 字段默认值兜底）
#[derive(Deserialize)]
struct UserConfigRaw {
    #[serde(default)]
    time_dilation_rate: Option<Value>,
    #[serde(default)]
    theme: Option<Value>,
    #[serde(default)]
    last_city: Option<Value>,
    #[serde(default)]
    last_timezone: Option<Value>,
    #[serde(default)]
    world_pins: Option<Value>,
    #[serde(default)]
    countdown_target: Option<Value>,
    #[serde(default)]
    window_geometry: Option<Value>,
    #[serde(default)]
    alarms: Option<Value>,
}

impl UserConfig {
    /// 从 JSON 值 lenient 构造：未知键忽略、错型剔除、缺省兜底（Python from_dict 对等）
    pub fn from_json_value(v: &Value) -> UserConfig {
        let raw: UserConfigRaw = serde_json::from_value(v.clone())
            .expect("UserConfigRaw 全 Option 字段不可能反序列化失败");
        let d = UserConfig::default();
        UserConfig {
            time_dilation_rate: raw
                .time_dilation_rate
                .and_then(num_f64)
                .unwrap_or(d.time_dilation_rate),
            theme: str_field(raw.theme, d.theme),
            last_city: str_field(raw.last_city, d.last_city),
            last_timezone: str_field(raw.last_timezone, d.last_timezone),
            // world_pins 容器校验 + 元素过滤（非串剔除，FIX003.3 对等）
            world_pins: match raw.world_pins {
                Some(Value::Array(arr)) => arr
                    .into_iter()
                    .filter_map(|p| p.as_str().map(String::from))
                    .collect(),
                _ => d.world_pins,
            },
            countdown_target: str_field(raw.countdown_target, d.countdown_target),
            window_geometry: opt_str_field(raw.window_geometry),
            alarms: match raw.alarms {
                Some(Value::Array(arr)) => arr,
                _ => d.alarms,
            },
        }
    }

    /// 序列化为 JSON 值（Python to_dict/asdict 对等）
    pub fn to_json_value(&self) -> Value {
        serde_json::to_value(self).expect("UserConfig 序列化不可能失败")
    }
}

/// 数值域过滤：Number（int/float 均收，bool 天然排除；serde_json 数字恒有限）
fn num_f64(v: Value) -> Option<f64> {
    v.as_f64()
}

/// 字符串域过滤：仅 String 收，其余（含 null/数值/布尔）回退默认
fn str_field(v: Option<Value>, default: String) -> String {
    match v {
        Some(Value::String(s)) => s,
        _ => default,
    }
}

/// 可选字符串域过滤：仅 String 收为 Some，其余（含 null）为 None
fn opt_str_field(v: Option<Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) => Some(s),
        _ => None,
    }
}

// 损坏转存节流（Python _backup_done 模块级对等）：同进程仅转存一次，恢复后重置
static BACKUP_DONE: AtomicBool = AtomicBool::new(false);

/// 装载用户配置（Python load_config 对等）
/// 文件不存在/解析失败/结构非对象 → 存在则先转存 .bak，回退默认值
pub fn load(path: &Path) -> UserConfig {
    match file_io::read_json(path) {
        Some(v @ Value::Object(_)) => {
            BACKUP_DONE.store(false, Ordering::SeqCst);
            UserConfig::from_json_value(&v)
        }
        _ => {
            if path.exists() && !BACKUP_DONE.swap(true, Ordering::SeqCst) {
                file_io::backup_corrupted(path);
            }
            UserConfig::default()
        }
    }
}

/// 保存用户配置（Python save_config 对等）：原子写，白名单外/IO 失败记日志返 false
pub fn save(path: &Path, config: &UserConfig) -> bool {
    match file_io::write_json(path, &config.to_json_value()) {
        Ok(()) => true,
        Err(e) => {
            tracing::error!("保存配置失败: {e}");
            false
        }
    }
}

/// 期望的默认配置 JSON（测试/文档锚点用）
pub fn defaults_json() -> Value {
    UserConfig::default().to_json_value()
}

// ===== src/settings.rs 说明区 =====
// 输入：user_config.json 路径 + JSON 值；输出：UserConfig（字段级 lenient 装载）
// 逻辑步骤：read_json → 对象判定 → UserConfigRaw（Option<Value> 中间层）→ 逐字段
//   类型过滤（数值/字符串/数组容器）→ 缺省与错型兜底 defaults；损坏（含非对象）→
//   转存 .bak（进程内节流一次，恢复重置）→ 默认值
// 设计理由：Python dataclass_from_dict 的「白名单过滤+默认兜底」无法用 serde 派生
//   直接表达（serde 错型即整体失败），Option<Value> 中间层逐字段复刻其语义且保持
//   字段名单单源；默认值经 Default 现取 Static（default_factory 对等，token 改即生效）
// 异常处理：装载全容错不 panic（脏配置是用户数据不是编程错误）；保存失败降级 false；
//   转存失败仅日志（对齐 Python logger.error 不外抛）
// 已知偏移（登记 z.plan）：①serde_json 缩进 2 vs Python indent=4（值级等价，字节级
//   不同）；②JSON 文本含 NaN/Infinity 字面量时 Python json 容忍并按脏值过滤（不
//   转存），serde_json 判解析失败走损坏转存——两路径终点同为默认值，副作用差异仅
//   .bak 生成与否；③alarms 本期 Vec<Value> 透传，PL012 落地 Alarm 强类型
// 关联依赖：static_config（默认值单源）/ util::file_io（原子写/转存/白名单）
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    fn temp_cfg(tag: &str) -> PathBuf {
        // 每测试独立目录（并行测试同进程不得共用配置路径）
        let dir = std::env::temp_dir().join(format!("accel_settings_{tag}_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("临时目录创建失败");
        dir.join("user_config.json")
    }

    #[test]
    fn defaults_match_python_snapshot_shape() {
        // 默认值锚点：rate 2.0 / theme auto / 北京 / Asia/Shanghai / 8 pins / 空 target
        let d = UserConfig::default();
        assert_eq!(d.time_dilation_rate, 2.0);
        assert_eq!(d.theme, "auto");
        assert_eq!(d.last_city, "北京");
        assert_eq!(d.last_timezone, "Asia/Shanghai");
        assert_eq!(d.world_pins.len(), 8);
        assert_eq!(d.world_pins[0], "Asia/Shanghai");
        assert_eq!(d.countdown_target, "");
        assert_eq!(d.window_geometry, None);
        assert!(d.alarms.is_empty());
    }

    #[test]
    fn dirty_field_filtering() {
        // 逐字段脏值 → 默认兜底（对齐夹具 dirty_fields 语义样本）
        let d = UserConfig::default();
        for (field, bad) in [
            ("time_dilation_rate", json!("abc")),
            ("time_dilation_rate", json!(true)),
            ("time_dilation_rate", json!(null)),
            ("time_dilation_rate", json!([1.0])),
            ("theme", json!(123)),
            ("theme", json!(null)),
            ("countdown_target", json!(42)),
            ("window_geometry", json!(std::f64::consts::PI)),
            ("alarms", json!("nope")),
        ] {
            let v = serde_json::json!({ field: bad });
            let cfg = UserConfig::from_json_value(&v);
            match field {
                "time_dilation_rate" => assert_eq!(cfg.time_dilation_rate, d.time_dilation_rate),
                "theme" => assert_eq!(cfg.theme, d.theme),
                "countdown_target" => assert_eq!(cfg.countdown_target, ""),
                "window_geometry" => assert_eq!(cfg.window_geometry, None),
                "alarms" => assert!(cfg.alarms.is_empty()),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn world_pins_element_filter() {
        // world_pins 容器合法时仅剔除非串元素（FIX003.3 对等）
        let v = serde_json::json!({"world_pins": [1, "Asia/Tokyo", null, 2.5]});
        let cfg = UserConfig::from_json_value(&v);
        assert_eq!(cfg.world_pins, vec!["Asia/Tokyo".to_string()]);
        // 容器整体错型 → 默认 8 pins
        let v2 = serde_json::json!({"world_pins": "notalist"});
        assert_eq!(UserConfig::from_json_value(&v2).world_pins.len(), 8);
    }

    #[test]
    fn int_rate_accepted_as_float() {
        // JSON 整数倍率收为 f64（Python float 字段收 int 对等）
        let v = serde_json::json!({"time_dilation_rate": 3});
        assert_eq!(UserConfig::from_json_value(&v).time_dilation_rate, 3.0);
    }

    #[test]
    fn save_load_roundtrip_local() {
        // 本地往返：保存 → 重读等价（原子写 + lenient 装载链路）
        let path = temp_cfg("roundtrip");
        let cfg = UserConfig {
            time_dilation_rate: 3.5,
            theme: "dark".into(),
            last_city: "上海".into(),
            last_timezone: "Europe/Paris".into(),
            world_pins: vec!["Asia/Shanghai".into(), "America/New_York".into()],
            countdown_target: "春节".into(),
            window_geometry: Some("SGVsbG8=".into()),
            alarms: vec![serde_json::json!({"label": "起床", "time": "07:30"})],
        };
        assert!(save(&path, &cfg));
        assert_eq!(load(&path), cfg);
    }

    #[test]
    fn corrupt_file_backed_up_and_defaults_restored() {
        // 损坏 → .bak 转存 + 默认值（Python FIX001.2 语义）
        let path = temp_cfg("corrupt");
        fs::write(&path, "{ this is not json !!!").expect("写入失败");
        let cfg = load(&path);
        assert_eq!(cfg, UserConfig::default());
        let bak = path.with_file_name("user_config.json.bak");
        assert!(bak.exists());
        assert_eq!(
            fs::read_to_string(&bak).expect("bak 应可读"),
            "{ this is not json !!!"
        );
    }

    #[test]
    fn missing_file_returns_defaults() {
        // 不存在 → 默认值且不转存
        let dir = std::env::temp_dir().join(format!("accel_settings_ghost_{}", std::process::id()));
        let path = dir.join("ghost.json");
        assert_eq!(load(&path), UserConfig::default());
        assert!(!path.with_file_name("ghost.json.bak").exists());
    }
}
