//! 倒计时展示运算（ui/tools/countdown_tools.py 的 Rust 对等物）
//! 目标文本解析（年 / 年月 / 年月日 / 月日 / 完整时刻）+ 剩余时间拆解——纯函数

use chrono::{Datelike, NaiveDate, NaiveDateTime, TimeDelta};

/// 目标文本解析（盒模型倒计时输入）：
/// "YYYY" / "YYYY-M" / "YYYY-M-D" / "M-D" / "YYYY-M-D HH:MM[:SS]"
/// 纯日期补 00:00；M-D 取下一次发生；非法返回 None
pub fn parse_target_text(text: &str, now: NaiveDateTime) -> Option<NaiveDateTime> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut tokens = text.split_whitespace();
    let date_text = tokens.next()?;
    let time_text = tokens.next();
    if tokens.next().is_some() {
        return None;
    }

    let date_parts: Vec<&str> = date_text.split('-').collect();
    if date_parts.is_empty() || date_parts.len() > 3 {
        return None;
    }
    let nums: Vec<u32> = date_parts
        .iter()
        .map(|p| p.parse::<u32>().ok())
        .collect::<Option<Vec<_>>>()?;

    let (year, month, day) = match (nums.len(), time_text.is_some()) {
        (1, false) if date_parts[0].len() == 4 => (nums[0], 1, 1),
        (2, false) if date_parts[0].len() == 4 => (nums[0], nums[1], 1),
        (2, false) => {
            let year = next_occurrence_year(now, nums[0], nums[1])?;
            (year, nums[0], nums[1])
        }
        (3, _) if date_parts[0].len() == 4 => (nums[0], nums[1], nums[2]),
        _ => return None,
    };
    if !(1..=9999).contains(&year) {
        return None;
    }
    let date = NaiveDate::from_ymd_opt(year as i32, month, day)?;
    let (hour, minute, second) = match time_text {
        Some(t) => parse_time(t)?,
        None => (0, 0, 0),
    };
    date.and_hms_opt(hour, minute, second)
}

/// "HH:MM" / "HH:MM:SS"（1–2 位时分秒）→ (时, 分, 秒)
fn parse_time(text: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = text.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let hour = parts[0].parse::<u32>().ok()?;
    let minute = parts[1].parse::<u32>().ok()?;
    let second = parts
        .get(2)
        .map(|s| s.parse::<u32>().ok())
        .unwrap_or(Some(0))?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some((hour, minute, second))
}

/// 不带年的 M-D：取下一次该日期（今年已过则明年；2-29 找到下一个闰年）
fn next_occurrence_year(now: NaiveDateTime, month: u32, day: u32) -> Option<u32> {
    let start = now.year().max(1) as u32;
    for offset in 0..=8 {
        let year = start + offset;
        if let Some(date) = NaiveDate::from_ymd_opt(year as i32, month, day)
            && let Some(at) = date.and_hms_opt(0, 0, 0)
            && at > now
        {
            return Some(year);
        }
    }
    None
}

/// 剩余时间拆解（Python format_remaining 对等）：
/// 已到点/过期 → ("已到达", true)（PL016.11 文字冗余）；否则 ("X天 HH:MM:SS", false)
pub fn format_remaining(target: NaiveDateTime, now: NaiveDateTime) -> (String, bool) {
    let remaining = target - now;
    if remaining <= TimeDelta::zero() {
        // PL016.11（审核 #14，状态不只靠颜色）：到点态由纯「红色 00:00」升级为
        // 文字「已到达」——朱砂色之外的语义冗余
        return ("已到达".to_string(), true);
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
// 逻辑步骤：拆日期段（年/年月/年月日/月日）→ 月日无年取下一次 → 校验月日
//   → 纯日期补 00:00；带时间解析 HH:MM[:SS] → timedelta 拆解
// 设计理由：解析规则单一来源（页面输入路径）；纯函数零面板依赖
// 已知偏移：月日无年依赖传入 now 选下一次；显式过去的日期由页面校验拒绝
// 异常处理：非法格式/越界值/非法月日 parse 失败返 None，不 panic
// 关联配置：无（纯函数）；消费方：pages/countdown.rs
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn parse_partial_formats() {
        let now = NaiveDate::from_ymd_opt(2026, 11, 14)
            .expect("日期合法")
            .and_hms_opt(12, 0, 0)
            .expect("时刻合法");
        assert_eq!(
            parse_target_text("2027", now)
                .expect("年应解析")
                .to_string(),
            "2027-01-01 00:00:00"
        );
        assert_eq!(
            parse_target_text("2027-2", now)
                .expect("年月应解析")
                .to_string(),
            "2027-02-01 00:00:00"
        );
        assert_eq!(
            parse_target_text("2027-2-3", now)
                .expect("年月日应解析")
                .to_string(),
            "2027-02-03 00:00:00"
        );
        // M-D：今年已过 → 明年
        assert_eq!(
            parse_target_text("3-3", now)
                .expect("月日应解析")
                .to_string(),
            "2027-03-03 00:00:00"
        );
        // M-D：今年未过 → 今年
        assert_eq!(
            parse_target_text("12-1", now)
                .expect("月日应解析")
                .to_string(),
            "2026-12-01 00:00:00"
        );
        // 完整格式保留时分
        assert_eq!(
            parse_target_text("2027-2-6 8:30", now)
                .expect("完整时刻应解析")
                .to_string(),
            "2027-02-06 08:30:00"
        );
        assert!(parse_target_text("2027-13", now).is_none());
        assert!(parse_target_text("2027-2-30", now).is_none());
        assert!(parse_target_text("2-30", now).is_none());
        assert!(parse_target_text("", now).is_none());
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
        // 已到点（PL016.11：文字态替代红色 00:00——非颜色冗余）
        let (text, ended) = format_remaining(now, now);
        assert_eq!(text, "已到达");
        assert!(ended);
        // 过期
        let past = now - TimeDelta::hours(2);
        let (text, ended) = format_remaining(past, now);
        assert_eq!(text, "已到达");
        assert!(ended);
    }
}
