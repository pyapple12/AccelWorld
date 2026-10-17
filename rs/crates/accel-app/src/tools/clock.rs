//! 时钟展示运算（ui/tools/clock_tools.py + date_panel._split_lunar 的 Rust 对等物）
//! 农历串三段拆分 / 进度上界 / Hero 显示格式化——纯函数零 egui 依赖

/// 农历串拆解（Python _split_lunar 逐字符对等）：
/// "主体…月相：X 财神：Y" → (主体, 月相, 财神)；任一标记缺失整体归主体、其余置空
/// （值段不含标记字样——FIX004.3 契约由 modules 产出端保证，此处容错兜底）
pub fn split_lunar_info(text: &str) -> (String, String, String) {
    const PHASE_MARK: &str = "月相：";
    const GOD_MARK: &str = "财神：";
    if text.contains(PHASE_MARK) && text.contains(GOD_MARK) {
        let (lunar, rest) = text.split_once(PHASE_MARK).expect("已验含标记");
        let (phase, god) = rest.split_once(GOD_MARK).expect("已验含标记");
        return (
            lunar.trim().to_string(),
            phase.trim().to_string(),
            god.trim().to_string(),
        );
    }
    (text.trim().to_string(), String::new(), String::new())
}

/// 倍率百分比格式化（"200%"形态；四舍五入取整——UI 展示用非精确值）
pub fn fmt_percent(rate: f64) -> String {
    format!("{:.0}%", rate * 100.0)
}

// ===== src/tools/clock.rs 说明区 =====
// 输入：TimeInfo.lunar_info 串 / rate；输出：展示字符串（纯函数）
// 逻辑步骤：split_lunar_info 双标记 partition → strip 三段；fmt_percent 取整
// 设计理由：date_panel._split_lunar 迁出的 Rust 对等（展示运算出面板，面板只剩
//   控件装配）；标记契约由 core::chinese_calendar 产出端单源，此处只消费容错
// A005 #15：progress_bounds / format_hms 生产零调用已删除（日长与时间串分别由
//   TimeInfo.day_hours / custom_time 直接提供）
// 异常处理：无 panic 面（split_once 前置 contains 校验）
// 关联配置：无（纯函数）；消费方：pages/clock.rs（PL014.05）
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_lunar_roundtrip_and_missing_marks() {
        // Python 用例移植：正常三段 / 标记缺失整体归主体 / FIX004.3 无"拜"回归
        let (lunar, phase, god) =
            split_lunar_info("丙午年（马年）八月十三午时 月相：渐盈凸月 财神：正北方向（坎位）");
        assert_eq!(lunar, "丙午年（马年）八月十三午时");
        assert_eq!(phase, "渐盈凸月");
        assert_eq!(god, "正北方向（坎位）");
        // 标记缺失：整体归主体
        let (lunar, phase, god) = split_lunar_info("纯文本无标记");
        assert_eq!(lunar, "纯文本无标记");
        assert_eq!((phase.as_str(), god.as_str()), ("", ""));
        // 月相段无"拜"（FIX004.3 回归：真实产出串经 split 后 phase 不含"拜"）
        let (_, phase, _) =
            split_lunar_info("乙巳年（蛇年）腊月廿九午时 月相：晓月 财神：东北方向（艮位）");
        assert!(!phase.contains('拜'));
    }

    #[test]
    fn hms_and_percent_formats() {
        assert_eq!(
            crate::tools::clock::fmt_percent(2.0),
            "200%",
            "A005 #15：format_hms 生产零调用已删（时间串由 core::TimeDilation 产出）"
        );
        assert_eq!(fmt_percent(1.1), "110%");
    }
}
