//! 天气代码表（data/weather_codes.py 的 Rust 对等物）
//! include_str 嵌入 rs/data/weather_codes.json（码序），WMO 码 → 中文信息检索

use std::sync::OnceLock;

use serde::Deserialize;

// 天气码信息（中文短名/emoji/完整描述，借用自嵌入 JSON）
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct WeatherCodeInfo {
    pub name: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
}

// 未知天气代码兜底（Python UNKNOWN_WEATHER 对等：未知/🌡️/未知天气）
const UNKNOWN_WEATHER: WeatherCodeInfo = WeatherCodeInfo {
    name: "未知",
    icon: "🌡️",
    description: "未知天气",
};

const WEATHER_CODES_JSON: &str = include_str!("../../../../data/weather_codes.json");

// 码表行（平铺字段；不用 serde(flatten)——其与借用 &str 的不变性冲突）
#[derive(Debug, Deserialize)]
pub struct CodeRow {
    code: i64,
    name: &'static str,
    icon: &'static str,
    description: &'static str,
}

static CODES: OnceLock<Vec<CodeRow>> = OnceLock::new();

/// 全表（码升序）
pub fn all() -> &'static [CodeRow] {
    CODES.get_or_init(|| {
        serde_json::from_str(WEATHER_CODES_JSON)
            .expect("weather_codes.json 解析失败（嵌入产物损坏）")
    })
}

/// WMO 码 → 信息（Python WEATHER_CODE_INFO.get(code, UNKNOWN_WEATHER) 对等）
pub fn lookup(code: i64) -> WeatherCodeInfo {
    all()
        .iter()
        .find(|r| r.code == code)
        .map(|r| WeatherCodeInfo {
            name: r.name,
            icon: r.icon,
            description: r.description,
        })
        .unwrap_or(UNKNOWN_WEATHER)
}

/// 未知码兜底信息
pub fn unknown() -> WeatherCodeInfo {
    UNKNOWN_WEATHER
}

#[cfg(test)]
mod tests {
    use super::{all, lookup, unknown};

    #[test]
    fn entry_count_and_anchor() {
        // 条目数（夹具：28 码）与首尾锚定
        assert_eq!(all().len(), 28);
        assert_eq!(all()[0].code, 0);
        assert_eq!(all()[27].code, 99);
    }

    #[test]
    fn lookup_known_and_unknown() {
        // 样例值（夹具锚点：码 0 晴 / 95 雷暴；未知码走兜底）
        let sunny = lookup(0);
        assert_eq!(
            (sunny.name, sunny.icon, sunny.description),
            ("晴", "☀️", "晴朗无云")
        );
        assert_eq!(lookup(95).name, "雷暴");
        assert_eq!(lookup(99).description, "雷暴伴大冰雹");
        assert_eq!(lookup(42).name, "未知");
        assert_eq!(lookup(999).description, "未知天气");
        assert_eq!(unknown().icon, "🌡️");
    }
}

// ===== src/data/weather_codes.rs 说明区 =====
// 输入：rs/data/weather_codes.json（编译期嵌入，28 码升序与 Python WEATHER_CODE_INFO 一致）
// 输出：lookup(code)（未知码回退 UNKNOWN_WEATHER 兜底）/all()/unknown()
// 逻辑步骤：OnceLock 惰性解析 → 码查找（CodeRow 平铺字段——serde flatten 与借用
//   &str 不变性冲突的实证规避）→ WeatherCodeInfo 复制出
// 设计理由：WMO 码 → 中文短名/emoji/描述单表（Python S2 合并决策延续）；
//   未知码回退语义与 Python get(code, UNKNOWN_WEATHER) 对等
// 异常处理：JSON 解析失败 expect 快速失败（嵌入产物损坏级）
// 关联配置：rs/data/weather_codes.json；消费方：weather 解析层（scalar_code_key 查表）
