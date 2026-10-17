//! 倒计时展示运算（ui/tools/countdown_tools.py 的 Rust 对等物）
//! 目标文本解析（19/16/10 位三格式）+ 剩余时间拆解——纯函数

use chrono::{NaiveDateTime, TimeDelta};

/// 目标文本解析（Python parse_target_text 对等）：
/// 19 位 "YYYY-MM-DD HH:MM:SS" / 16 位 "YYYY-MM-DD HH:MM" / 10 位 "YYYY-MM-DD"
/// （10 位补当日 23:59:59）；非法返回 None
pub fn parse_target_text(text: &str) -> Option<NaiveDateTime> {
    const FMT_FULL: &str = "%Y-%m-%d %H:%M:%S";
    const FMT_MIN: &str = "%Y-%m-%d %H:%M";
    const FMT_DATE: &str = "%Y-%m-%d";
    match text.len() {
        19 => NaiveDateTime::parse_from_str(text, FMT_FULL).ok(),
        16 => NaiveDateTime::parse_from_str(text, FMT_MIN).ok(),
        10 => NaiveDatePart::parse_date_end_of_day(text, FMT_DATE),
        _ => None,
    }
}

// 10 位日期格式的当日 23:59:59 补齐（独立小助手便于测试）
struct NaiveDatePart;

impl NaiveDatePart {
    fn parse_date_end_of_day(text: &str, fmt: &str) -> Option<NaiveDateTime> {
        let d = chrono::NaiveDate::parse_from_str(text, fmt).ok()?;
        d.and_hms_opt(23, 59, 59)
    }
}

/// 剩余时间拆解（Python format_remaining 对等）：
/// 已到点/过期 → ("00天 00:00:00", true)；否则 ("X天 HH:MM:SS", false)
pub fn format_remaining(target: NaiveDateTime, now: NaiveDateTime) -> (String, bool) {
    let remaining = target - now;
    if remaining <= TimeDelta::zero() {
        return ("00天 00:00:00".to_string(), true);
    }
    let days = remaining.num_days();
    let secs = remaining.num_seconds() % 86_400;
    let (hours, rest) = (secs / 3600, secs % 3600);
    let (minutes, seconds) = (rest / 60, rest % 60);
    (
        format!("{days:02}天 {hours:02}:{minutes:02}:{seconds:02}"),
        false,
    )
}

// ===== src/tools/countdown.rs 说明区 =====
// 输入：目标文本 / 目标时刻与当前时刻；输出：解析结果 / (展示文本, 结束态)
// 逻辑步骤：按字节长度分发三格式（10 位补 23:59:59）→ timedelta 拆解
// 设计理由：解析规则单一来源（设置与启动恢复路径共用，FIX001.10 延续）；
//   纯函数零面板依赖（plan#UI2.0 铁律 1 的 Rust 对等）
// 已知偏移：len() 按 UTF-8 字节计（Python 同为 str 长度语义，数字/横线均 ASCII 等价）
// 异常处理：非法格式/越界值 parse 失败返 None，不 panic
// 关联配置：无（纯函数）；消费方：pages/countdown.rs（PL014.08）
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn parse_three_formats() {
        // Python 语义：19/16/10 位三格式 + 10 位补当日 23:59:59
        let full = parse_target_text("2027-02-06 08:00:00").expect("19 位应解析");
        assert_eq!(full.to_string(), "2027-02-06 08:00:00");
        let min = parse_target_text("2027-02-06 08:00").expect("16 位应解析");
        assert_eq!(min.to_string(), "2027-02-06 08:00:00");
        let date = parse_target_text("2027-02-06").expect("10 位应解析");
        assert_eq!(date.to_string(), "2027-02-06 23:59:59");
        // 非法：长度不符 / 越界值
        assert!(parse_target_text("2027-02-06 8:00").is_none());
        assert!(parse_target_text("2027-13-40").is_none());
        assert!(parse_target_text("").is_none());
    }

    #[test]
    fn page_parse_contract_keeps_time_of_day() {
        // A005 #11 回归：页面改用本函数后，带时分的输入必须保留时分
        //（原页内 parse::<NaiveDate> 只收 10 位并把输入强填 23:59:59）
        let t = parse_target_text("2027-02-06 08:30").expect("16 位应解析");
        assert_eq!(t.to_string(), "2027-02-06 08:30:00", "不得改写为 23:59:59");
        let d = parse_target_text("2027-02-06").expect("10 位应解析");
        assert_eq!(d.to_string(), "2027-02-06 23:59:59", "10 位仍补当日末");
    }

    #[test]
    fn remaining_format_and_end_state() {
        // Python 用例语义：正常拆解 + 到点/过期结束态
        let now = NaiveDate::from_ymd_opt(2026, 10, 17)
            .expect("日期合法")
            .and_hms_opt(12, 0, 0)
            .expect("时刻合法");
        let target = NaiveDate::from_ymd_opt(2026, 10, 18)
            .expect("日期合法")
            .and_hms_opt(13, 30, 5)
            .expect("时刻合法");
        let (text, ended) = format_remaining(target, now);
        assert_eq!(text, "01天 01:30:05");
        assert!(!ended);
        // 已到点
        let (text, ended) = format_remaining(now, now);
        assert_eq!(text, "00天 00:00:00");
        assert!(ended);
        // 过期
        let past = now - TimeDelta::hours(2);
        let (text, ended) = format_remaining(past, now);
        assert_eq!(text, "00天 00:00:00");
        assert!(ended);
    }
}
