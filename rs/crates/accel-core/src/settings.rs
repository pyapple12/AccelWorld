//! 用户配置（config/settings.py 的 Rust 对等物）
//! UserConfig serde 镜像（字段名与 user_config.json 同 schema）+ lenient 装载 +
//! 原子保存 + 损坏转存；默认值单源于 base.json token（default_factory 对等）

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::static_config::Static;
use crate::util::file_io;

/// 倒计时目标（PL014.06 新格式：列表化多目标）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CountdownTarget {
    /// 目标名（去重键）
    pub name: String,
    /// 目标时刻（naive 本地形态 "YYYY-MM-DD HH:MM:SS"——与 Python 历史串兼容）
    pub at: String,
}

impl CountdownTarget {
    /// at 串解析（非法 None——装载期剔除依据）
    /// FIX007.10：与自由文本解析同规则，委托 parse_date_text 单源（原逐行重复实现）
    pub fn parse_at(&self) -> Option<NaiveDateTime> {
        parse_date_text(&self.at)
    }
}

/// 目标时刻串解析（三格式对齐 Python parse_target_text：19/16/10 位）
pub fn parse_date_text(text: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M").ok())
        .or_else(|| {
            NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(23, 59, 59))
        })
}

/// 目标时刻 → 存储串（19 位规范形态）
pub fn format_date_text(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

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
    /// 倒计时目标列表（PL014.06 新字段；旧 countdown_target 字段保留读兼容）
    #[serde(default)]
    pub countdown_targets: Vec<CountdownTarget>,
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
            countdown_targets: Vec::new(),
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
    #[serde(default)]
    countdown_targets: Option<Value>,
}

impl UserConfig {
    /// 从 JSON 值 lenient 构造：未知键忽略、错型剔除、缺省兜底（Python from_dict 对等）
    pub fn from_json_value(v: &Value) -> UserConfig {
        // A005 #20：非对象输入（数组/标量/null）直接返默认值——原实现 expect 会 panic
        if !v.is_object() {
            tracing::warn!(
                "用户配置根非对象，回退默认值: {}",
                v.to_string().chars().take(32).collect::<String>()
            );
            return UserConfig::default();
        }
        let raw: UserConfigRaw = serde_json::from_value(v.clone())
            .expect("UserConfigRaw 全 Option 字段 + 前置对象判定，不会失败");
        let d = UserConfig::default();
        // Legacy 倒计时串先取（str_field 会 move 同字段）
        let legacy_target = match &raw.countdown_target {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        };
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
            // 倒计时目标迁移链（PL014.06）：新格式优先 → Legacy 串迁移 → 空
            countdown_targets: match raw.countdown_targets {
                Some(Value::Array(arr)) => {
                    // 新格式装载：name/at 逐项校验（at 解析失败剔除 + 日志）
                    let mut list = Vec::new();
                    for item in arr {
                        let name = item.get("name").and_then(Value::as_str).unwrap_or("");
                        let at = item.get("at").and_then(Value::as_str).unwrap_or("");
                        if name.is_empty() {
                            // FIX010.8：空名（含非对象条目取不到 name）剔除留痕——
                            // 对齐闹钟装载口径，脏数据不再静默丢弃
                            tracing::warn!("倒计时目标名称为空（或条目非对象），剔除");
                            continue;
                        }
                        match parse_date_text(at) {
                            Some(dt) => list.push(CountdownTarget {
                                name: name.to_string(),
                                at: format_date_text(dt),
                            }),
                            None => tracing::warn!("倒计时目标时刻非法，剔除: {name} {at:?}"),
                        }
                    }
                    list
                }
                _ => Vec::new(),
            },
        }
        .migrate_legacy_countdown(legacy_target)
    }

    /// Legacy 迁移（PL014.06）：新格式已有目标则不动；否则旧 countdown_target
    /// 非空串 → 解析（19/16/10 位）→ 列表首项；解析失败丢弃 + 日志
    fn migrate_legacy_countdown(mut self, legacy: Option<String>) -> Self {
        if !self.countdown_targets.is_empty() {
            // 新格式在：旧字段保留原值（幂等迁移语义），列表即权威
            return self;
        }
        let Some(legacy_text) = legacy else {
            return self;
        };
        if legacy_text.is_empty() {
            return self;
        }
        match parse_date_text(&legacy_text) {
            Some(dt) => {
                tracing::info!("倒计时旧格式迁移: {legacy_text:?} → 列表首项");
                self.countdown_targets = vec![CountdownTarget {
                    name: "我的目标".to_string(),
                    at: format_date_text(dt),
                }];
            }
            None => tracing::warn!("倒计时旧格式解析失败，丢弃: {legacy_text:?}"),
        }
        self
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
            let mut config = UserConfig::from_json_value(&v);
            normalize_world_pins(&mut config.world_pins);
            config
        }
        _ => {
            if path.exists() && !BACKUP_DONE.swap(true, Ordering::SeqCst) {
                file_io::backup_corrupted(path);
            }
            UserConfig::default()
        }
    }
}

/// world_pins 域迁移（FIX007.2 双向兼容读）：城市名保持；旧存档 IANA 串映射为
/// 城市名（FIX006.1 期 IANA 存储丢失同 tz 多城身份）；两域皆不收的值原样保留
/// （显示侧回显、走时占位）——迁移幂等，下次落盘即固化
fn normalize_world_pins(pins: &mut [String]) {
    for pin in pins.iter_mut() {
        if crate::data::world_cities::by_name(pin).is_some() {
            continue;
        }
        if let Some(name) = crate::data::world_cities::name_of_tz(pin) {
            *pin = name.to_string();
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
//   不同）；②alarms 字段为 Vec<Value> 透传（A005 #16 更正：原注"PL012 落地 Alarm
//   强类型"已过期——强类型在 AlarmManager 侧，UserConfig 仍以 JSON 形态承载存档）；
//   ③JSON 文本含 NaN/Infinity 字面量时 Python json 容忍并按脏值过滤（不转存），
//   serde_json 判解析失败走损坏转存——两路径终点同为默认值，副作用差异仅 .bak 生成与否
//   （FIX006.13：原重复的 ③「alarms 本期 Vec<Value> 透传」与 ② 自相矛盾，已删并重排）
// 关联依赖：static_config（默认值单源）/ util::file_io（原子写/转存/白名单）
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    /// 测试沙箱目录（RAII：drop 即整目录清理——A008 #15，与 parity.rs 同款模式，
    /// 断言失败不再残留 %TEMP%/accel_settings_*）
    struct SandboxDir(PathBuf);

    impl SandboxDir {
        fn new(tag: &str) -> Self {
            // 每测试独立目录（并行测试同进程不得共用配置路径）；先清同 pid 上次残留
            let dir =
                std::env::temp_dir().join(format!("accel_settings_{tag}_{}", std::process::id()));
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

    #[test]
    fn defaults_match_python_snapshot_shape() {
        // 默认值锚点：rate 2.0 / theme auto / 北京 / last_timezone Asia/Shanghai /
        // 8 pins（FIX007.2：常驻集权威域 = 城市名）/ 空 target
        let d = UserConfig::default();
        assert_eq!(d.time_dilation_rate, 2.0);
        assert_eq!(d.theme, "auto");
        assert_eq!(d.last_city, "北京");
        assert_eq!(d.last_timezone, "Asia/Shanghai");
        assert_eq!(d.world_pins.len(), 8);
        assert_eq!(d.world_pins[0], "北京");
        assert_eq!(d.countdown_target, "");
        assert_eq!(d.window_geometry, None);
        assert!(d.alarms.is_empty());
    }

    #[test]
    fn non_object_input_returns_defaults() {
        // A005 #20 回归：非对象根（数组/标量/null/串）返默认值且不 panic
        for bad in [
            serde_json::json!([1, 2]),
            serde_json::json!(42),
            serde_json::json!("text"),
            Value::Null,
        ] {
            let cfg = UserConfig::from_json_value(&bad);
            assert_eq!(cfg, UserConfig::default(), "非对象输入应回退默认值");
        }
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
        // world_pins 容器合法时仅剔除非串元素（FIX003.3 对等；from_json_value 层
        // 不做域迁移——IANA→城市名的迁移在 load 层，下一测试覆盖）
        let v = serde_json::json!({"world_pins": [1, "Asia/Tokyo", null, 2.5]});
        let cfg = UserConfig::from_json_value(&v);
        assert_eq!(cfg.world_pins, vec!["Asia/Tokyo".to_string()]);
        // 容器整体错型 → 默认 8 pins
        let v2 = serde_json::json!({"world_pins": "notalist"});
        assert_eq!(UserConfig::from_json_value(&v2).world_pins.len(), 8);
    }

    #[test]
    fn world_pins_legacy_iana_migrated_on_load() {
        // FIX007.2 双向兼容读：旧存档 IANA 串装载期迁移为城市名（下次落盘固化）；
        // 两域皆不收的值原样保留（显示回显/走时占位）
        let sandbox = SandboxDir::new("pinmigrate");
        let path = sandbox.path();
        let legacy = serde_json::json!({
            "world_pins": ["Asia/Shanghai", "Asia/Tokyo", "Mars/Olympus"]
        });
        std::fs::write(&path, legacy.to_string()).expect("写旧存档失败");
        let cfg = load(&path);
        assert_eq!(
            cfg.world_pins,
            vec![
                "北京".to_string(),
                "东京".to_string(),
                "Mars/Olympus".to_string()
            ],
            "IANA 应迁移为城市名；未知值原样保留"
        );
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
        let sandbox = SandboxDir::new("roundtrip");
        let path = sandbox.path();
        let cfg = UserConfig {
            time_dilation_rate: 3.5,
            theme: "dark".into(),
            last_city: "上海".into(),
            last_timezone: "Europe/Paris".into(),
            world_pins: vec!["北京".into(), "纽约".into()],
            countdown_target: "春节".into(),
            window_geometry: Some("SGVsbG8=".into()),
            alarms: vec![serde_json::json!({"label": "起床", "time": "07:30"})],
            countdown_targets: Vec::new(),
        };
        assert!(save(&path, &cfg));
        assert_eq!(load(&path), cfg);
    }

    #[test]
    fn countdown_legacy_migration_matrix() {
        // PL014.06 验收：旧格式迁移矩阵（19/16/10 位/空/非法/新格式优先）
        let cfg_from = |payload: Value| {
            let sandbox = SandboxDir::new("cdmatrix");
            let path = sandbox.path();
            let mut base = UserConfig::default().to_json_value();
            for (k, v) in payload.as_object().expect("对象") {
                base[k] = v.clone();
            }
            std::fs::write(&path, serde_json::to_string(&base).expect("序列化")).expect("写入");
            load(&path)
        };
        // 19 位旧串 → 列表首项
        let cfg = cfg_from(serde_json::json!({"countdown_target": "2027-02-06 08:00:00"}));
        assert_eq!(cfg.countdown_targets.len(), 1);
        assert_eq!(cfg.countdown_targets[0].at, "2027-02-06 08:00:00");
        // 10 位旧串 → 补 23:59:59
        let cfg = cfg_from(serde_json::json!({"countdown_target": "2027-02-06"}));
        assert_eq!(cfg.countdown_targets[0].at, "2027-02-06 23:59:59");
        // 空串 → 空列表
        let cfg = cfg_from(serde_json::json!({"countdown_target": ""}));
        assert!(cfg.countdown_targets.is_empty());
        // 非法串 → 丢弃 + 空列表
        let cfg = cfg_from(serde_json::json!({"countdown_target": "不合法"}));
        assert!(cfg.countdown_targets.is_empty());
        // 新格式优先（已有 targets 时旧字段不动列表）
        let cfg = cfg_from(serde_json::json!({
            "countdown_target": "2020-01-01 00:00:00",
            "countdown_targets": [{"name": "新年", "at": "2027-01-01 00:00:00"}]
        }));
        assert_eq!(cfg.countdown_targets.len(), 1);
        assert_eq!(cfg.countdown_targets[0].name, "新年");
        // 新格式脏项剔除：非法 at 剔除 + 空名剔除
        let cfg = cfg_from(serde_json::json!({
            "countdown_targets": [
                {"name": "好", "at": "2027-05-01 10:00"},
                {"name": "坏时刻", "at": "垃圾"},
                {"at": "2027-05-01 10:00"}
            ]
        }));
        assert_eq!(cfg.countdown_targets.len(), 1);
        assert_eq!(cfg.countdown_targets[0].name, "好");
        assert_eq!(
            cfg.countdown_targets[0].at, "2027-05-01 10:00:00",
            "16 位补秒规范化"
        );
        // 保存 → 重读 roundtrip 为新格式（列表持久化）
        let sandbox = SandboxDir::new("cdroundtrip");
        let path = sandbox.path();
        let mut write_back = cfg.clone();
        write_back.countdown_target = String::new();
        assert!(save(&path, &write_back));
        let reloaded = load(&path);
        assert_eq!(reloaded.countdown_targets, write_back.countdown_targets);
    }

    #[test]
    fn corrupt_file_backed_up_and_defaults_restored() {
        // 损坏 → .bak 转存 + 默认值（Python FIX001.2 语义）
        let sandbox = SandboxDir::new("corrupt");
        let path = sandbox.path();
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
