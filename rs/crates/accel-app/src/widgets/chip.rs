//! chip 纸片徽标（UIR 实心主题：纯矢量小卡——玻璃面注册随 glass 模块删除）
//! 农历/节日等小信息卡的统一容器：label 上 value 下；宽度按文案自适应
//! （上限定 240——长农历串不撑破布局）

use egui::{Sense, Vec2};

/// chip 高度（layout.chip_height token——A009 #14 单值化，原 chip_size 双值槽的
/// w 槽随 chip 宽自适应退化为半死位）
pub fn chip_height() -> f32 {
    crate::theme::layout_i64("chip_height") as f32
}

/// 自适应宽度（纯函数可测）：双行文案取宽者 + 内边距 20，钳 [64, 240]
/// （上限 240——最长农历串"丙午年（马年）九月十四玄"实测溢出 200，2026-10-23）
pub fn chip_width(label_w: f32, value_w: f32) -> f32 {
    (label_w.max(value_w) + 20.0).clamp(64.0, 240.0)
}

/// 绘制一枚 chip（label 上 value 下）并返回响应（语义由调用方定义）
/// A008 #7：id 形参已删（玻璃面注册废除后 id 零消费，调用方每帧 format! 空耗）
pub fn show(ui: &mut egui::Ui, label: &str, value: &str) -> egui::Response {
    let label_font = egui::FontId::proportional(crate::theme::font_size("chip_label"));
    let value_font = egui::FontId::proportional(crate::theme::font_size("chip_value"));
    let label_w = ui
        .painter()
        .layout_no_wrap(
            label.to_string(),
            label_font.clone(),
            crate::theme::ink().dim,
        )
        .rect
        .width();
    let value_w = ui
        .painter()
        .layout_no_wrap(
            value.to_string(),
            value_font.clone(),
            crate::theme::ink().card,
        )
        .rect
        .width();
    let size = Vec2::new(chip_width(label_w, value_w), chip_height());
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let ink = crate::theme::ink();
    let radius = crate::theme::radius_px("md") as u8;
    ui.painter().rect_filled(
        rect,
        egui::epaint::CornerRadius::same(radius),
        crate::theme::card_bg(),
    );
    ui.painter().rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(radius),
        egui::Stroke::new(1.0, crate::theme::border_color()),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.left_top() + Vec2::new(10.0, 13.0),
        egui::Align2::LEFT_TOP,
        label,
        label_font,
        ink.dim,
    );
    ui.painter().text(
        rect.left_bottom() + Vec2::new(10.0, -12.0),
        egui::Align2::LEFT_BOTTOM,
        value,
        value_font,
        ink.card,
    );
    resp
}

// ===== src/widgets/chip.rs 说明区 =====
// 输入：chip 文案；输出：矢量小卡 + 双行文字（宽度按文案自适应）
// 逻辑步骤：量双行文案宽（chip_label/chip_value 字号）→ chip_width（+20 内边距，
//   钳 64..240）→ 卡底 + 描边 → label/value 两行
// 设计理由（UIR 实心主题）：玻璃面注册删除后 chip 退为纯矢量徽标；定宽版实测
//   长农历串（丙午年（马年））溢出卡界——自适应宽是排版正确性的前提，上限防撑破
// 异常处理：无 panic 面
// 关联依赖：theme；消费方：pages/clock（农历 chips）、pages/countdown（节日 chips）
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_width_clamped() {
        // 自适应宽：短文案下限 64；长文案上限 240；中段 +20 内边距
        assert_eq!(chip_width(10.0, 20.0), 64.0);
        assert_eq!(chip_width(100.0, 90.0), 120.0);
        assert_eq!(chip_width(500.0, 90.0), 240.0);
    }

    #[test]
    fn chip_height_token() {
        // 高度取 layout.chip_height token（UIR 定值 56）
        assert_eq!(chip_height(), 56.0);
    }
}
