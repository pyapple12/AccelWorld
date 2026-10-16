//! 时区表（data/timezones.py 的 Rust 对等物）
//! include_str 嵌入 rs/data/timezones.json（显示序一致），强类型检索

use std::sync::OnceLock;

use serde::Deserialize;

// 时区行（显示名 + IANA 标识）
#[derive(Debug, Deserialize)]
pub struct TimezoneEntry {
    pub display: &'static str,
    pub iana: &'static str,
}

const TIMEZONES_JSON: &str = include_str!("../../../../data/timezones.json");

static TIMEZONES: OnceLock<Vec<TimezoneEntry>> = OnceLock::new();

/// 全表（展示序：北京/东京/首尔/伦敦/巴黎/纽约/洛杉矶/悉尼）
pub fn all() -> &'static [TimezoneEntry] {
    TIMEZONES.get_or_init(|| {
        serde_json::from_str(TIMEZONES_JSON).expect("timezones.json 解析失败（嵌入产物损坏）")
    })
}

/// IANA 标识检索（配置 world_pins 校验用）
pub fn by_iana(iana: &str) -> Option<&'static TimezoneEntry> {
    all().iter().find(|t| t.iana == iana)
}

/// 全部 IANA 标识（展示序）
pub fn iana_list() -> Vec<&'static str> {
    all().iter().map(|t| t.iana).collect()
}

#[cfg(test)]
mod tests {
    use super::{all, by_iana, iana_list};

    #[test]
    fn entry_count_and_order_anchor() {
        // 条目数与首尾锚定（夹具：8 时区，北京头悉尼尾）
        let all = all();
        assert_eq!(all.len(), 8);
        assert_eq!(all[0].iana, "Asia/Shanghai");
        assert_eq!(all[7].iana, "Australia/Sydney");
        assert_eq!(iana_list().len(), 8);
    }

    #[test]
    fn display_and_lookup() {
        // 样例值（夹具锚点：巴黎含 DST 双档显示名；未知标识返 None）
        assert_eq!(
            by_iana("Europe/Paris").map(|t| t.display),
            Some("巴黎 (UTC+1/+2)")
        );
        assert!(by_iana("Mars/Olympus").is_none());
    }
}
