//! 闹钟服务（modules/alarm_service.py 的 Rust 对等物）
//! 数据模型（serde 持久化兼容）/ 预设铃声枚举与参数表 / AlarmManager 状态机 /
//! Clock 抽象（可测性核心——杜绝真睡眠测试）

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::static_config::Static;

// ===== 预设铃声 =====

/// 预设铃声枚举（Python PresetSound 镜像）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresetSound {
    Classic,
    Gentle,
    Beep,
    Chime,
}

impl PresetSound {
    /// 枚举值（持久化字符串，与 Python value 一致）
    pub fn value(self) -> &'static str {
        match self {
            PresetSound::Classic => "classic",
            PresetSound::Gentle => "gentle",
            PresetSound::Beep => "beep",
            PresetSound::Chime => "chime",
        }
    }

    /// 下拉框显示名列表（Python value.title() 顺序生成）
    pub fn display_names() -> Vec<&'static str> {
        vec!["Classic", "Gentle", "Beep", "Chime"]
    }

    /// 大小写不敏感匹配，未命中回退 CLASSIC（Python from_value 对等）
    pub fn from_value(value: &str) -> PresetSound {
        match value.to_lowercase().as_str() {
            "gentle" => PresetSound::Gentle,
            "beep" => PresetSound::Beep,
            "chime" => PresetSound::Chime,
            _ => PresetSound::Classic,
        }
    }

    /// 当前成员显示名
    pub fn display_name(self) -> &'static str {
        Self::display_names()[self.index()]
    }

    /// 枚举序（下拉框索引互转）
    pub fn index(self) -> usize {
        match self {
            PresetSound::Classic => 0,
            PresetSound::Gentle => 1,
            PresetSound::Beep => 2,
            PresetSound::Chime => 3,
        }
    }

    /// 按序号取成员；越界回退 CLASSIC 并告警（2026-09-13 热修复对等）
    pub fn from_index(index: i64) -> PresetSound {
        match index {
            1 => PresetSound::Gentle,
            2 => PresetSound::Beep,
            3 => PresetSound::Chime,
            0 => PresetSound::Classic,
            _ => {
                tracing::warn!("预设铃声序号越界，回退 CLASSIC: {index}");
                PresetSound::Classic
            }
        }
    }
}

/// 预设铃播放参数（频率 Hz, 重复次数, 间隔毫秒）——Python _PRESET_SOUND_CONFIG 导出
pub const PRESET_SOUND_CONFIG: [(PresetSound, i64, i64, i64); 4] = [
    (PresetSound::Classic, 800, 3, 500),
    (PresetSound::Gentle, 600, 2, 800),
    (PresetSound::Beep, 1200, 5, 200),
    (PresetSound::Chime, 1000, 4, 600),
];

/// 每次蜂鸣持续时间（Python play_preset_sound 内字面量）
pub const PRESET_BEEP_DURATION_MS: u64 = 200;

// ===== 时钟抽象（可测性核心，杜绝真睡眠测试）=====

/// 时钟 trait：调度任务与触发判定经此注入（RealClock 生产 / VirtualClock 测试拨钟）
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Local>;
}

/// 真实时钟（chrono Local）
pub struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> DateTime<Local> {
        Local::now()
    }
}

/// 虚拟时钟（测试可拨动；Arc 共享跨任务）
pub struct VirtualClock(pub std::sync::Arc<std::sync::Mutex<DateTime<Local>>>);

impl Clock for VirtualClock {
    fn now(&self) -> DateTime<Local> {
        *self.0.lock().expect("虚拟时钟锁中毒")
    }
}

// ===== 闹钟数据模型 =====

// 预设铃声类型标记（sound_type 字段取值）
const SOUND_TYPE_PRESET: &str = "preset";
const SOUND_VALUE_CLASSIC: &str = "classic";

/// 闹钟实体（Python Alarm.to_dict 字段名/默认值完全一致——持久化兼容硬约束）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Alarm {
    pub label: String,
    /// 触发时分（"HH:MM"）
    pub time: String,
    #[serde(default = "default_preset")]
    pub sound_type: String,
    #[serde(default = "default_classic")]
    pub sound_value: String,
    /// 重复星期（0-6 周一至周日，空 = 一次性）
    #[serde(default)]
    pub repeat_days: Vec<i64>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "gen_id")]
    pub id: String,
    #[serde(default = "now_iso")]
    pub created_at: String,
}

/// 新建闹钟草稿（AddAlarm 命令负载：无 id/created_at，落库时生成）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlarmDraft {
    pub label: String,
    pub time: String,
    pub sound_type: String,
    pub sound_value: String,
    pub repeat_days: Vec<i64>,
    pub enabled: bool,
}

impl Alarm {
    /// 草稿落库（Python 构造路径对等）：生成 id/created_at + 规范化 + 时间校验
    /// 时间非法返回 None（Python 构造抛 ValueError 由调用方跳过的对等形态）
    pub fn from_draft(draft: AlarmDraft) -> Option<Alarm> {
        if parse_alarm_hhmm(&draft.time).is_none() {
            tracing::warn!("闹钟时间格式非法，拒绝创建: {:?}", draft.time);
            return None;
        }
        Some(Alarm {
            label: draft.label,
            time: draft.time,
            sound_type: draft.sound_type,
            sound_value: draft.sound_value,
            repeat_days: draft.repeat_days,
            enabled: draft.enabled,
            id: gen_id(),
            created_at: now_iso(),
        })
    }

    /// lenient 反序列化（Python from_dict tolerant 对等）：返回 None 表示整条拒绝
    /// created_at 键存在但非字符串 → 整条拒绝（FIX003.8 防一次性闹钟"复活"）；
    /// 其余字段类型过滤 + 默认兜底 + 时间校验失败 → None
    pub fn from_json_value(data: &Value) -> Option<Alarm> {
        let obj = data.as_object()?;
        if let Some(ca) = obj.get("created_at").filter(|ca| !ca.is_string()) {
            tracing::warn!("闹钟 created_at 类型非法，整条跳过: {ca:?}");
            return None;
        }
        let label = obj.get("label").and_then(Value::as_str)?.to_string();
        let time = obj.get("time").and_then(Value::as_str)?.to_string();
        // sound_type Literal 两值之外一律回退 preset（Python 类型过滤语义）
        let sound_type = match obj.get("sound_type").and_then(Value::as_str) {
            Some(SOUND_TYPE_PRESET) | Some("custom") => {
                obj["sound_type"].as_str().unwrap().to_string()
            }
            _ => SOUND_TYPE_PRESET.to_string(),
        };
        let sound_value = obj
            .get("sound_value")
            .and_then(Value::as_str)
            .unwrap_or(SOUND_VALUE_CLASSIC)
            .to_string();
        let repeat_days_raw = match obj.get("repeat_days") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => Vec::new(),
        };
        let enabled = obj.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(gen_id);
        let created_at = obj
            .get("created_at")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(now_iso);
        if parse_alarm_hhmm(&time).is_none() {
            tracing::warn!("闹钟时间格式非法，跳过条目: {time:?}");
            return None;
        }
        Some(Alarm {
            label,
            time,
            sound_type,
            sound_value,
            repeat_days: normalize_repeat_days(&repeat_days_raw),
            enabled,
            id,
            created_at,
        })
    }

    /// 序列化（Python to_dict/asdict 对等）
    pub fn to_json_value(&self) -> Value {
        serde_json::to_value(self).expect("Alarm 序列化不可能失败")
    }

    /// 无重复天数即为一次性
    pub fn is_one_time(&self) -> bool {
        self.repeat_days.is_empty()
    }

    /// 触发判定状态机（Python should_trigger_on 逐行对齐）
    /// 顺序：enabled → 时分等值 → repeat_days 空? 一次性路径（created_at 计划日
    /// =创建日或 +1 顺延，数据异常保守不触发）/ 星期命中
    pub fn should_trigger_on(&self, check_time: DateTime<Local>) -> bool {
        if !self.enabled {
            return false;
        }
        let Some((alarm_hour, alarm_minute)) = parse_alarm_hhmm(&self.time) else {
            // 构造期已校验，此处不可达（防御：视同不触发）
            return false;
        };
        if alarm_hour != check_time.hour() || alarm_minute != check_time.minute() {
            return false;
        }
        if self.repeat_days.is_empty() {
            // 一次性：创建时设定时间未过当日为创建当天，已过自动顺延次日（FIX001.20）
            let Some(created) = parse_created_at(&self.created_at) else {
                // created_at 数据异常保守不触发（FIX001.8：防意外每天响）
                return false;
            };
            let mut scheduled_date: NaiveDate = created.date();
            if (alarm_hour, alarm_minute) <= (created.hour(), created.minute()) {
                // 创建时刻已到/过设定时分（同分钟视为已过，防创建后当分钟即响）
                scheduled_date += Duration::days(1);
            }
            return check_time.date_naive() == scheduled_date;
        }
        // 重复闹钟：当前星期（周一=0）命中
        self.repeat_days
            .contains(&(check_time.weekday().num_days_from_monday() as i64))
    }
}

/// HH:MM / HH:MM:SS 归一化解析（Python _parse_alarm_hhmm 对等：无冒号补 :00）
/// 实测 Python 语义：小时须两位（"9:30" 拒绝）、范围 0-23/0-59、秒与微秒可选
pub fn parse_alarm_hhmm(t: &str) -> Option<(u32, u32)> {
    let s = if t.contains(':') {
        t.to_string()
    } else {
        format!("{t}:00")
    };
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let hour = two_digit_u32(parts[0])?;
    let minute = two_digit_u32(parts[1])?;
    if hour > 23 || minute > 59 {
        return None;
    }
    if let Some(sec_part) = parts.get(2) {
        // 秒段：SS 或 SS.ffffff（仅校验形态，不参与触发比较）
        let (sec, frac) = match sec_part.split_once('.') {
            Some((s2, f)) => (s2, Some(f)),
            None => (*sec_part, None),
        };
        let sec = two_digit_u32(sec)?;
        if sec > 59 {
            return None;
        }
        if frac
            .is_some_and(|f| f.is_empty() || f.len() > 6 || !f.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
    }
    Some((hour, minute))
}

/// 恰两位数字 → u32（"09" → 9；"9"/"009" → None——对齐 fromisoformat 严格位宽）
fn two_digit_u32(s: &str) -> Option<u32> {
    if s.len() != 2 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// created_at ISO 串解析（Python datetime.fromisoformat 对等：naive 本地形态为主，
/// 兼容 RFC3339 带偏移；失败返回 None）
fn parse_created_at(text: &str) -> Option<NaiveDateTime> {
    if let Ok(n) = NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(n);
    }
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|d| d.naive_local())
}

/// repeat_days 规范化（Python __post_init__ 对等）：
/// 布尔剔除（int(True)=1 穿透防御）/ 非整数浮点剔除 / 字符串强转失败剔除 /
/// 越界 [0,6] 剔除 / 去重保序
fn normalize_repeat_days(days: &[Value]) -> Vec<i64> {
    let mut normalized: Vec<i64> = Vec::new();
    for day in days {
        let day_int = match day {
            Value::Bool(_) => continue,
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    i
                } else {
                    let f = n.as_f64().expect("数值变体必可取 f64");
                    if f.fract() != 0.0 || f.abs() > 9_007_199_254_740_992.0 {
                        continue;
                    }
                    f as i64
                }
            }
            Value::String(s) => match s.parse::<i64>() {
                Ok(i) => i,
                Err(_) => continue,
            },
            _ => continue,
        };
        if (0..=6).contains(&day_int) && !normalized.contains(&day_int) {
            normalized.push(day_int);
        }
    }
    normalized
}

fn default_preset() -> String {
    SOUND_TYPE_PRESET.to_string()
}

fn default_classic() -> String {
    SOUND_VALUE_CLASSIC.to_string()
}

fn default_true() -> bool {
    true
}

/// 生成闹钟 id（Python str(uuid.uuid4())[:8] 等价形态：8 位十六进制）
fn gen_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mix = (std::process::id() as u128) ^ nanos.rotate_left(64);
    format!("{mix:08x}")[..8].to_string()
}

/// 创建时刻（Python datetime.now().isoformat() 形态对等：微秒为 0 省略小数段）
fn now_iso() -> String {
    let now = Local::now().naive_local();
    if now.nanosecond() == 0 {
        now.format("%Y-%m-%dT%H:%M:%S").to_string()
    } else {
        now.format("%Y-%m-%dT%H:%M:%S%.6f").to_string()
    }
}

// ===== 闹钟管理器 =====

/// 闹钟管理器（Python AlarmManager 镜像）：上限 token / 同时间同标签去重 /
/// 同分钟触发去重（仅内存，FIX001.21 P3#15 豁免延续）
pub struct AlarmManager {
    pub alarms: Vec<Alarm>,
    max_alarms: usize,
    /// alarm_id → "YYYY-MM-DD HH:MM"（含日期维度，跨天不误判，S8.1）
    last_triggered: HashMap<String, String>,
}

impl AlarmManager {
    /// 空列表启动；上限取 base.max_alarms token
    pub fn new() -> Self {
        AlarmManager {
            alarms: Vec::new(),
            max_alarms: Static::get().base_i64("max_alarms") as usize,
            last_triggered: HashMap::new(),
        }
    }

    /// 上限（get_max_alarms 契约）
    pub fn max_alarms(&self) -> usize {
        self.max_alarms
    }

    /// 添加：上限校验 + 同时间同标签去重（失败记日志返 false）
    pub fn add_alarm(&mut self, alarm: Alarm) -> bool {
        if self.alarms.len() >= self.max_alarms {
            tracing::warn!("已达到最大闹钟数量限制 ({})", self.max_alarms);
            return false;
        }
        if self
            .alarms
            .iter()
            .any(|e| e.time == alarm.time && e.label == alarm.label)
        {
            tracing::warn!("已存在相同时间和标签的闹钟");
            return false;
        }
        self.alarms.push(alarm);
        true
    }

    /// 删除：同时清理触发去重记录
    pub fn remove_alarm(&mut self, alarm_id: &str) -> bool {
        if let Some(pos) = self.alarms.iter().position(|a| a.id == alarm_id) {
            self.alarms.remove(pos);
            self.last_triggered.remove(alarm_id);
            return true;
        }
        false
    }

    /// 按 ID 查找
    pub fn get_alarm(&self, alarm_id: &str) -> Option<&Alarm> {
        self.alarms.iter().find(|a| a.id == alarm_id)
    }

    /// 编辑替换：保留原 ID 原位替换；去重检查排除自身（FIX003.7）
    pub fn replace_alarm(&mut self, alarm: Alarm) -> bool {
        if self
            .alarms
            .iter()
            .any(|e| e.id != alarm.id && e.time == alarm.time && e.label == alarm.label)
        {
            tracing::warn!("已存在相同时间和标签的闹钟");
            return false;
        }
        if let Some(pos) = self.alarms.iter().position(|e| e.id == alarm.id) {
            self.alarms[pos] = alarm;
            return true;
        }
        false
    }

    /// 启用/禁用翻转
    pub fn toggle_alarm(&mut self, alarm_id: &str) -> bool {
        if let Some(pos) = self.alarms.iter().position(|a| a.id == alarm_id) {
            self.alarms[pos].enabled = !self.alarms[pos].enabled;
            return true;
        }
        false
    }

    /// 触发检查（Python check_alarms 对等）：disabled 跳过 → 同分钟去重 →
    /// 状态机命中收集并记录去重键；返回命中克隆列表
    pub fn check_alarms(&mut self, check_time: DateTime<Local>) -> Vec<Alarm> {
        let time_str = check_time.format("%Y-%m-%d %H:%M").to_string();
        let mut triggered = Vec::new();
        for alarm in &self.alarms {
            if !alarm.enabled {
                continue;
            }
            if self.last_triggered.get(&alarm.id).map(String::as_str) == Some(time_str.as_str()) {
                continue;
            }
            if alarm.should_trigger_on(check_time) {
                triggered.push(alarm.clone());
                self.last_triggered
                    .insert(alarm.id.clone(), time_str.clone());
            }
        }
        triggered
    }

    /// 序列化列表（Python to_dict_list）
    pub fn to_value_list(&self) -> Vec<Value> {
        self.alarms.iter().map(Alarm::to_json_value).collect()
    }

    /// 装载（Python from_dict_list 对等）：空条目/解析失败过滤 + 超限截断 +
    /// 过期一次性自动禁用（FIX004.14/FIX004.15）
    pub fn from_value_list(&mut self, data: &[Value]) {
        let mut loaded: Vec<Alarm> = Vec::new();
        for item in data {
            // Python `if not item`：null 与空对象跳过（零告警）
            let is_empty = match item {
                Value::Null => true,
                Value::Object(o) => o.is_empty(),
                _ => false,
            };
            if is_empty {
                continue;
            }
            match Alarm::from_json_value(item) {
                Some(alarm) => loaded.push(alarm),
                None => tracing::warn!("跳过无法解析的闹钟条目: {item:?}"),
            }
        }
        if loaded.len() > self.max_alarms {
            tracing::warn!(
                "闹钟数量超限，已截断: {} → {}",
                loaded.len(),
                self.max_alarms
            );
            loaded.truncate(self.max_alarms);
        }
        // 过期一次性禁用：越过计划触发日的死条目禁用保留供查看（FIX004.14）
        let now = Local::now();
        for alarm in &mut loaded {
            let expired = alarm.enabled
                && alarm.is_one_time()
                && !alarm.should_trigger_on(now)
                && parse_created_at(&alarm.created_at)
                    .is_some_and(|created| created.date() < now.date_naive());
            if expired {
                alarm.enabled = false;
                let planned = parse_created_at(&alarm.created_at)
                    .map(|c| c.date().to_string())
                    .unwrap_or_default();
                tracing::warn!(
                    "一次性闹钟已过期（计划日 {planned}），自动禁用: {}",
                    alarm.label
                );
            }
        }
        self.alarms = loaded;
    }
}

impl Default for AlarmManager {
    fn default() -> Self {
        Self::new()
    }
}

// ===== src/alarm.rs 说明区 =====
// 输入：闹钟 JSON（lenient）/ 草稿 / 注入检查时刻；输出：实体/管理器操作结果/命中列表
// 逻辑步骤（状态机顺序即实现顺序，z.plan 修订二·E）：enabled → HH:MM 等值 →
//   repeat_days 空 ? 一次性（created_at 计划日 = 创建日或 +1 顺延 / 异常保守不触发）
//   : 星期命中；去重键 "YYYY-MM-DD HH:MM" 仅内存（豁免延续）
// 设计理由：字段名/默认值与 Python to_dict 逐字一致（持久化兼容）；created_at
//   特殊拒绝路径（键在非字符串）优先于类型过滤（FIX003.8 防复活）；时间解析
//   实测对齐 fromisoformat（两位小时严格位宽、"9:30" 拒绝、无冒号补 :00 后仍
//   须合法）；gen_id 8 位十六进制对齐 uuid4[:8] 形态；now_iso 微秒为 0 省略段
//   对齐 isoformat；Clock trait 注入使调度与触发全链可测（无真睡眠）
// 异常处理：lenient 装载全路径 None 跳过 + warning；管理器操作布尔返回；
//   parse_alarm_hhmm/parse_created_at 全 Option 无 panic
// 已知偏移（登记 z.plan）：①x.progress 任务文本「"9:30" 补 :00」与 Python 实测
//   不符（"9:30" 即拒绝——单 digit 小时非法），以 Python 为准；②gen_id 用
//   pid×纳秒混合 xor 代替 uuid4（同 8 位形态，无 uuid 依赖）
// 关联配置：max_alarms（base.json token）；消费方：interface（PL012.07/.10）、
//   rodio 播放（PRESET_SOUND_CONFIG/PRESET_BEEP_DURATION_MS，PL012.08）
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn local(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Local> {
        let naive = NaiveDate::from_ymd_opt(y, m, d)
            .expect("日期合法")
            .and_hms_opt(h, mi, 0)
            .expect("时刻合法");
        Local.from_local_datetime(&naive).single().expect("无歧义")
    }

    fn alarm_json(id: &str, label: &str, time: &str, repeat: Value, created: &str) -> Value {
        serde_json::json!({
            "label": label, "time": time, "repeat_days": repeat,
            "enabled": true, "id": id, "created_at": created,
            "sound_type": "preset", "sound_value": "classic",
        })
    }

    #[test]
    fn preset_sound_enum_contract() {
        // 枚举值/显示名/大小写匹配/序号越界回退
        assert_eq!(PresetSound::Classic.value(), "classic");
        assert_eq!(
            PresetSound::display_names(),
            vec!["Classic", "Gentle", "Beep", "Chime"]
        );
        assert_eq!(PresetSound::from_value("GENTLE"), PresetSound::Gentle);
        assert_eq!(PresetSound::from_value("nope"), PresetSound::Classic);
        assert_eq!(PresetSound::Chime.display_name(), "Chime");
        assert_eq!(PresetSound::from_index(3), PresetSound::Chime);
        assert_eq!(PresetSound::from_index(99), PresetSound::Classic);
        // 常量表逐条对 Python 值
        assert_eq!(PRESET_SOUND_CONFIG[0], (PresetSound::Classic, 800, 3, 500));
        assert_eq!(PRESET_SOUND_CONFIG[1], (PresetSound::Gentle, 600, 2, 800));
        assert_eq!(PRESET_SOUND_CONFIG[2], (PresetSound::Beep, 1200, 5, 200));
        assert_eq!(PRESET_SOUND_CONFIG[3], (PresetSound::Chime, 1000, 4, 600));
    }

    #[test]
    fn time_format_semantics() {
        // Python fromisoformat 实测语义（PL012.05 移植基线）
        assert_eq!(parse_alarm_hhmm("09:30"), Some((9, 30)));
        assert_eq!(parse_alarm_hhmm("09:30:45"), Some((9, 30)));
        assert_eq!(parse_alarm_hhmm("09:30:45.5"), Some((9, 30)));
        assert_eq!(parse_alarm_hhmm("9:30"), None, "单位数小时应拒绝");
        assert_eq!(parse_alarm_hhmm("25:00"), None);
        assert_eq!(parse_alarm_hhmm("09:60"), None);
        assert_eq!(parse_alarm_hhmm(""), None);
        // 无冒号补 :00 后仍须两位合法（"0930:00" 非法）
        assert_eq!(parse_alarm_hhmm("0930"), None);
    }

    #[test]
    fn repeat_days_normalization() {
        // 模型类用例移植（Python 实码语义：布尔剔除防 int(True)=1 穿透）：
        // [true]→[]、[1.7]/["1.5"]→[]、[9]→[]、字符串/整数/整值浮点收
        let n = |v: Value| normalize_repeat_days(v.as_array().unwrap());
        assert_eq!(n(serde_json::json!([true])), Vec::<i64>::new(), "布尔剔除");
        assert_eq!(n(serde_json::json!([1.7])), Vec::<i64>::new());
        assert_eq!(n(serde_json::json!(["1.5"])), Vec::<i64>::new());
        assert_eq!(n(serde_json::json!([9])), Vec::<i64>::new());
        assert_eq!(n(serde_json::json!(["2", 3, 4.0])), vec![2, 3, 4]);
        assert_eq!(n(serde_json::json!([1, "1", 1.0])), vec![1], "去重保序");
        assert_eq!(
            n(serde_json::json!([null, [1], {"a":1}])),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn from_json_reject_paths() {
        // created_at 键在但非字符串 → 整条拒绝（FIX003.8）
        let mut v = alarm_json(
            "t1",
            "a",
            "09:30",
            serde_json::json!([]),
            "2026-01-01T08:00:00",
        );
        v["created_at"] = serde_json::json!(42);
        assert!(Alarm::from_json_value(&v).is_none());
        // 时间非法 → 拒绝
        let bad_time = alarm_json(
            "t1",
            "a",
            "25:00",
            serde_json::json!([]),
            "2026-01-01T08:00:00",
        );
        assert!(Alarm::from_json_value(&bad_time).is_none());
        // 缺 label → 拒绝（必填）
        let mut no_label = alarm_json(
            "t1",
            "a",
            "09:30",
            serde_json::json!([]),
            "2026-01-01T08:00:00",
        );
        no_label.as_object_mut().unwrap().remove("label");
        assert!(Alarm::from_json_value(&no_label).is_none());
        // sound_type 非法值回退 preset；repeat_days 混型规范化
        let mut dirty = alarm_json(
            "t1",
            "a",
            "09:30",
            serde_json::json!([true, 9]),
            "2026-01-01T08:00:00",
        );
        dirty["sound_type"] = serde_json::json!("weird");
        dirty["enabled"] = serde_json::json!("yes");
        let a = Alarm::from_json_value(&dirty).expect("应装载");
        assert_eq!(a.sound_type, "preset");
        assert!(a.enabled, "非布尔 enabled 过滤回退默认 true");
        assert_eq!(a.repeat_days, Vec::<i64>::new(), "true 剔除且 9 越界剔除");
        // created_at 缺失 → 默认生成（旧配置兼容）
        let mut no_ca = alarm_json(
            "t1",
            "a",
            "09:30",
            serde_json::json!([]),
            "2026-01-01T08:00:00",
        );
        no_ca.as_object_mut().unwrap().remove("created_at");
        let a2 = Alarm::from_json_value(&no_ca).expect("应装载");
        assert!(!a2.created_at.is_empty());
    }

    #[test]
    fn one_time_cross_day_postponement() {
        // 跨天顺延（FIX001.20）：23:50 建次日 08:00 → 计划日为次日
        let a = Alarm::from_json_value(&alarm_json(
            "o1",
            "早起",
            "08:00",
            serde_json::json!([]),
            "2026-06-01T23:50:00",
        ))
        .expect("装载");
        assert!(
            !a.should_trigger_on(local(2026, 6, 1, 8, 0)),
            "创建当日不应触发"
        );
        assert!(
            a.should_trigger_on(local(2026, 6, 2, 8, 0)),
            "顺延次日应触发"
        );
        assert!(
            !a.should_trigger_on(local(2026, 6, 3, 8, 0)),
            "第三日不再触发"
        );
        // 创建时刻未过设定时分 → 当日触发
        let b = Alarm::from_json_value(&alarm_json(
            "o2",
            "早起",
            "08:00",
            serde_json::json!([]),
            "2026-06-01T07:00:00",
        ))
        .expect("装载");
        assert!(b.should_trigger_on(local(2026, 6, 1, 8, 0)));
        // 同分钟视为已过（防创建后当分钟即响）
        let c = Alarm::from_json_value(&alarm_json(
            "o3",
            "早起",
            "08:00",
            serde_json::json!([]),
            "2026-06-01T08:00:00",
        ))
        .expect("装载");
        assert!(!c.should_trigger_on(local(2026, 6, 1, 8, 0)));
        assert!(c.should_trigger_on(local(2026, 6, 2, 8, 0)));
    }

    #[test]
    fn created_at_invalid_no_revive() {
        // created_at 非法串：保守不触发（FIX001.8 防复活），也不因解析 panic
        let mut v = alarm_json("t1", "a", "09:30", serde_json::json!([]), "not-a-date");
        v["created_at"] = serde_json::json!("not-a-date");
        let a = Alarm::from_json_value(&v).expect("字符串形态应装载");
        assert!(!a.should_trigger_on(local(2026, 6, 1, 9, 30)));
    }

    #[test]
    fn repeat_weekday_hit() {
        // 重复闹钟按星期：2026-06-01 是周一（weekday 0）
        let a = Alarm::from_json_value(&alarm_json(
            "w1",
            "工作",
            "09:30",
            serde_json::json!([0, 2]),
            "2026-05-01T08:00:00",
        ))
        .expect("装载");
        assert!(a.should_trigger_on(local(2026, 6, 1, 9, 30)), "周一命中");
        assert!(!a.should_trigger_on(local(2026, 6, 2, 9, 30)), "周二不命中");
        assert!(a.should_trigger_on(local(2026, 6, 3, 9, 30)), "周三命中");
        assert!(
            !a.should_trigger_on(local(2026, 6, 1, 9, 31)),
            "时分不等不命中"
        );
    }

    #[test]
    fn manager_crud_and_dedup() {
        // 上限/同时间同标签去重/编辑排除自身/删除清去重/翻转
        let mut m = AlarmManager::new();
        assert_eq!(m.max_alarms(), 10);
        let a1 = Alarm::from_json_value(&alarm_json(
            "id1",
            "a",
            "08:00",
            serde_json::json!([]),
            "2026-05-01T07:00:00",
        ))
        .unwrap();
        assert!(m.add_alarm(a1.clone()));
        let dup = Alarm::from_json_value(&alarm_json(
            "id2",
            "a",
            "08:00",
            serde_json::json!([]),
            "2026-05-02T07:00:00",
        ))
        .unwrap();
        assert!(!m.add_alarm(dup), "同时间同标签应拒绝");
        // 编辑成重复拒且列表不变（FIX003.7：replace 去重排除自身）
        let mut edited = a1.clone();
        edited.label = "b".into();
        edited.time = "08:00".into();
        assert!(m.replace_alarm(edited));
        let other = Alarm::from_json_value(&alarm_json(
            "id3",
            "c",
            "09:00",
            serde_json::json!([1]),
            "2026-05-01T07:00:00",
        ))
        .unwrap();
        assert!(m.add_alarm(other.clone()));
        // 重复编辑目标（改成与 other 同时间同标签）应拒绝
        let mut clash = m.get_alarm(&a1.id).unwrap().clone();
        clash.label = "c".into();
        clash.time = "09:00".into();
        assert!(!m.replace_alarm(clash), "编辑成重复条目应拒绝");
        assert_eq!(m.alarms.len(), 2);
        // toggle
        assert!(m.toggle_alarm(&other.id));
        assert!(!m.get_alarm(&other.id).unwrap().enabled);
        // remove 清去重键
        assert!(m.remove_alarm(&other.id));
        assert!(!m.remove_alarm(&other.id));
        assert_eq!(m.alarms.len(), 1);
    }

    #[test]
    fn manager_load_truncates_and_disables_expired() {
        // 装载 11 条截 10（FIX004.15）+ 过期一次性自动禁用（FIX004.14）
        let mut m = AlarmManager::new();
        let mut list = Vec::new();
        for i in 0..11 {
            list.push(alarm_json(
                &format!("id{i}"),
                &format!("闹钟{i}"),
                &format!("0{}:00", i % 10),
                serde_json::json!([]),
                "2025-01-01T08:00:00",
            ));
        }
        m.from_value_list(&list);
        assert_eq!(m.alarms.len(), 10, "超限应截断");
        // 计划日 2025-01-01/02 已过 → 全部禁用
        assert!(m.alarms.iter().all(|a| !a.enabled), "过期一次性应禁用");
        // 空条目/null/垃圾条目过滤
        let mut m2 = AlarmManager::new();
        m2.from_value_list(&[
            Value::Null,
            serde_json::json!({}),
            serde_json::json!("junk"),
            alarm_json(
                "ok1",
                "ok",
                "07:00",
                serde_json::json!([1]),
                "2027-01-01T06:00:00",
            ),
        ]);
        assert_eq!(m2.alarms.len(), 1);
        assert_eq!(m2.alarms[0].label, "ok");
        assert!(m2.alarms[0].enabled, "未过期保持启用");
    }

    #[test]
    fn check_alarms_dedup_same_minute() {
        // 同分钟去重：命中恰一次；跨分钟/跨天可再触发
        let mut m = AlarmManager::new();
        let a = Alarm::from_json_value(&alarm_json(
            "d1",
            "每分",
            "09:00",
            serde_json::json!([0]),
            "2026-05-01T08:00:00",
        ))
        .unwrap();
        m.add_alarm(a);
        let hits1 = m.check_alarms(local(2026, 6, 1, 9, 0));
        let hits2 = m.check_alarms(local(2026, 6, 1, 9, 0));
        assert_eq!(hits1.len(), 1);
        assert!(hits2.is_empty(), "同分钟不应重复触发");
        // 次周同日（周一）可再触发
        let hits3 = m.check_alarms(local(2026, 6, 8, 9, 0));
        assert_eq!(hits3.len(), 1);
    }

    #[test]
    fn draft_creation_and_roundtrip() {
        // 草稿 → 实体（id/created_at 生成 + 规范化）→ JSON 往返
        let draft = AlarmDraft {
            label: "测试".into(),
            time: "07:15".into(),
            sound_type: "preset".into(),
            sound_value: "chime".into(),
            repeat_days: vec![1],
            enabled: true,
        };
        let a = Alarm::from_draft(draft).expect("合法草稿应创建");
        assert!(!a.id.is_empty());
        assert!(!a.created_at.is_empty());
        let v = a.to_json_value();
        let back = Alarm::from_json_value(&v).expect("往返应成功");
        assert_eq!(back, a);
    }
}
