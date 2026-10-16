//! 闹钟服务（modules/alarm_service.py 的 Rust 对等物）
//! PL011.13 落数据形状（命令/事件枚举负载）；状态机/调度/持久化随 PL012 落地

use serde::{Deserialize, Serialize};

// 预设铃声类型标记（sound_type 字段取值，PL012 落地 PresetSound 强校验）
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
    /// 重复星期（0-6，空 = 一次性）
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

fn default_preset() -> String {
    SOUND_TYPE_PRESET.to_string()
}

fn default_classic() -> String {
    SOUND_VALUE_CLASSIC.to_string()
}

fn default_true() -> bool {
    true
}

/// 生成闹钟 id（进程号+纳秒混合的十六进制；PL012 随强校验一并复核取值域）
fn gen_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("a{:x}{:x}", std::process::id(), nanos)
}

/// 创建时刻（RFC3339 本地时区带偏移）
fn now_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

// ===== src/alarm.rs 说明区 =====
// 输入：serde 反序列化（user_config.json alarms 数组元素）；输出：Alarm/AlarmDraft 实体
// 设计理由：字段名/默认值与 Python to_dict 完全一致（持久化兼容硬约束——旧配置
//   直接可读）；#[serde(default)] 各默认函数对齐 Python dataclass 默认值语义；
//   状态机（触发判定顺序/去重键/过期禁用）与 Clock 抽象随 PL012 落地，本文件
//   仅承载形状使 FrontendCommand/BackendEvent 枚举一次定义到位（协议 v1 定死）
// 异常处理：本期无行为逻辑；PL012 的 normalize() 将补时间 HH:MM 校验 Err 路径
// 关联依赖：serde/chrono；消费方：types（枚举负载）/ interface（PL012 分发）
