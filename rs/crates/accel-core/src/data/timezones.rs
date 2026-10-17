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

#[cfg(test)]
mod tests {
    use super::all;

    #[test]
    fn entry_count_and_order_anchor() {
        // 条目数与首尾锚定（夹具：8 时区，北京头悉尼尾）
        let all = all();
        assert_eq!(all.len(), 8);
        assert_eq!(all[0].iana, "Asia/Shanghai");
        assert_eq!(all[7].iana, "Australia/Sydney");
    }

    #[test]
    fn display_and_lookup() {
        // 样例值（夹具锚点：巴黎含 DST 双档显示名）
        // A005 #15：by_iana/iana_list 已删——检索改用 all() 的 iana 字段
        let paris = all().iter().find(|t| t.iana == "Europe/Paris");
        assert_eq!(paris.map(|t| t.display), Some("巴黎 (UTC+1/+2)"));
        assert!(all().iter().all(|t| t.iana != "Mars/Olympus"));
    }
}

// ===== src/data/timezones.rs 说明区 =====
// 输入：rs/data/timezones.json（编译期嵌入，8 时区与 Python TIMEZONES 展示序一致）
// 输出：all()/by_iana()/iana_list() 检索（世界时钟下拉与配置校验共用）
// 逻辑步骤：OnceLock 惰性解析 → 线性查找（显示名含 DST 双档标注，E12 修正延续）
// 设计理由：时区表外置复用（Python FIX002.15 同因）；巴黎双档显示名测试锚定
// 异常处理：JSON 解析失败 expect 快速失败（嵌入产物损坏级）
// 关联配置：rs/data/timezones.json；消费方：interface 时区域（get_timezone_options）
