//! 倍率映射（原胶囊滑杆的纯函数部分；绘制/交互已由 rate_strip 取代，FIX013.19）。
//! `ratio_to_rate` / `rate_to_ratio` 仍被 rate_strip 复用。

/// 滑杆值映射（纯函数）：轨道内指针比例 → 倍率值（整数四舍五入，域钳制）
pub fn ratio_to_rate(ratio: f64, min: f64, max: f64) -> f64 {
    let clamped = ratio.clamp(0.0, 1.0);
    (min + clamped * (max - min)).round().clamp(min, max)
}

/// 倍率 → 轨道内比例（渲染用；反向同式）
pub fn rate_to_ratio(rate: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        return 0.0;
    }
    ((rate - min) / (max - min)).clamp(0.0, 1.0)
}

// ===== src/widgets/slider.rs 说明区 =====
// 输入：比例/倍率 + 域上下限；输出：钳制后的倍率或 0..1 比例
// 逻辑步骤：ratio_to_rate 先钳比例 → 线性映射 → 整数四舍五入 → 域钳制；
//   rate_to_ratio 先处理 max<=min 退化 → 线性反算 → 钳 0..1
// 设计理由：FIX013.19 删除 show/draw/fill_geometry/knob_center_y（rate_strip 已
//   取代绘制与交互），仅保留两页共用的值映射；整数档语义与 PL020 一致
// 异常处理：max<=min 返 0 比例；非有限输入由调用方保证，无 panic 面
// 关联配置：倍率域 rate_min/rate_max（调用方传入）；消费方：widgets/rate_strip
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_rate_mapping_step_and_clamp() {
        // PL020：整数档四舍五入 + 域钳制
        assert_eq!(ratio_to_rate(0.0, 1.0, 20.0), 1.0);
        assert_eq!(ratio_to_rate(1.0, 1.0, 20.0), 20.0);
        assert_eq!(ratio_to_rate(0.5, 1.0, 20.0), 11.0, "半程 → 10.5 取整 11");
        assert_eq!(ratio_to_rate(2.0, 1.0, 20.0), 20.0, "越界钳满");
        assert_eq!(ratio_to_rate(-1.0, 1.0, 20.0), 1.0, "越界钳最小");
        assert_eq!(
            rate_to_ratio(5.5, 1.0, 20.0),
            4.5 / 19.0,
            "比例=(rate-min)/(max-min)"
        );
        // 往返一致性（整数档）
        let r = ratio_to_rate(rate_to_ratio(7.3, 1.0, 20.0), 1.0, 20.0);
        assert_eq!(r, 7.0);
    }
}
