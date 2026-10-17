//! 闹钟展示文案（ui/tools/alarm_text.py 的 Rust 对等物）
//! 重复规则文案与铃声文件名文案的单一来源——纯函数

// 星期字符表（重复闹钟显示，0-6 → 一~日；Python _WEEKDAY_CHARS 同表）
const WEEKDAY_CHARS: [&str; 7] = ["一", "二", "三", "四", "五", "六", "日"];

/// 重复规则展示（Python format_repeat_display 逐字符对等）：
/// 空列表 → "一次"；否则 "周" + 逐日字符拼接（[0,1] → "周一二"）
pub fn format_repeat_display(repeat_days: &[i64]) -> String {
    if repeat_days.is_empty() {
        return "一次".to_string();
    }
    let mut s = String::from("周");
    for d in repeat_days {
        if (0..=6).contains(d) {
            s.push_str(WEEKDAY_CHARS[*d as usize]);
        }
    }
    s
}

/// 铃声文件名文案：📁 前缀 + basename 截断 15 字符（Python format_sound_button_name 对等）
pub fn format_sound_button_name(path: &str) -> String {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let cut: String = base.chars().take(15).collect();
    format!("📁 {cut}")
}

// ===== src/tools/alarm_text.rs 说明区 =====
// 输入：重复星期数字列表 / 铃声路径；输出：展示文案（纯函数）
// 逻辑步骤：空 → "一次"；非空 → "周"+逐日映射；basename 取尾段字符截 15
// 设计理由：文案规则单一来源（Python PL001.07 同因）；索引范围由 Alarm 规范化
//   保证 0-6（越界字符跳过防御，与 Python 索引行为差异登记——Python 会 IndexError）
// 异常处理：越界索引跳过不 panic（防御路径，正常数据不触发）
// 关联配置：无（纯函数）；消费方：pages/alarm.rs（PL014.15）
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_display_python_cases() {
        // Python test_ui_tools 四用例逐条移植
        assert_eq!(format_repeat_display(&[]), "一次");
        assert_eq!(format_repeat_display(&[1]), "周二");
        assert_eq!(format_repeat_display(&[6]), "周日");
        assert_eq!(format_repeat_display(&[0, 1, 2]), "周一二三");
        assert_eq!(format_repeat_display(&[0, 6]), "周一日");
    }

    #[test]
    fn sound_button_name_cases() {
        // Python 用例移植：短路径 basename + 长名截 15 字符
        assert_eq!(format_sound_button_name("C:/music/wake.wav"), "📁 wake.wav");
        let long = format!("a{}.wav", "a".repeat(20));
        assert_eq!(
            format_sound_button_name(&long),
            format!("📁 {}", "a".repeat(15))
        );
    }
}
