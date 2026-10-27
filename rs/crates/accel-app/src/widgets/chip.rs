//! chip 安静徽标（UIR2 子夜星图）：单行「label value」小签——发丝描边卡底
//! 农历/节日等小信息的统一容器；宽度按文案自适应（上限定 240 不撑破布局）

use egui::{Sense, Vec2};

/// chip 高度（layout.chip_height token——UIR2 单行化后 32）
pub fn chip_height() -> f32 {
    crate::theme::layout_i64("chip_height") as f32
}

/// 自适应宽度（纯函数可测）：label + 分隔 + value 行宽 + 内边距 26，钳 [56, 240]
/// （上限 240——最长农历串"丙午年（马年）九月十四玄"实测不撑破，2026-10-23 单行复核）
pub fn chip_width(label_w: f32, value_w: f32) -> f32 {
    (label_w + value_w + 26.0).clamp(56.0, 240.0)
}

/// 绘制一枚 chip（单行：label meta 色 + value 正文色）并返回响应（语义由调用方定义）
/// A008 #7：id 形参已删（玻璃面注册废除后 id 零消费，调用方每帧 format! 空耗）
pub fn show(ui: &mut egui::Ui, label: &str, value: &str) -> egui::Response {
    let label_font = egui::FontId::proportional(crate::theme::font_size("meta"));
    let value_font = egui::FontId::proportional(crate::theme::font_size("body"));
    let ink = crate::theme::ink();
    let label_w = ui
        .painter()
        .layout_no_wrap(label.to_string(), label_font.clone(), ink.muted)
        .rect
        .width();
    let value_w = ui
        .painter()
        .layout_no_wrap(value.to_string(), value_font.clone(), ink.body)
        .rect
        .width();
    let size = Vec2::new(chip_width(label_w, value_w), chip_height());
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let radius = crate::theme::radius_px("sm") as u8;
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
    // PL016.06 small：悬停描边增亮 0.15s（纯信息徽标无操作态，仅可读性反馈）
    let reduce = crate::theme::reduce_motion(ui.ctx());
    let hover_t = if reduce {
        if resp.hovered() { 1.0 } else { 0.0 }
    } else {
        ui.ctx().animate_value_with_time(
            resp.id.with("hover"),
            if resp.hovered() { 1.0 } else { 0.0 },
            crate::theme::motion_ms("hover_ms") / 1000.0,
        )
    };
    if hover_t > 0.003 {
        ui.painter().rect_stroke(
            rect,
            egui::epaint::CornerRadius::same(radius),
            egui::Stroke::new(
                1.0,
                crate::theme::with_alpha(
                    crate::theme::color32_at("selection", 42),
                    (160.0 * hover_t) as u8,
                ),
            ),
            egui::StrokeKind::Inside,
        );
    }
    let center_y = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, center_y),
        egui::Align2::LEFT_CENTER,
        label,
        label_font,
        ink.muted,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 12.0 + label_w + 10.0, center_y),
        egui::Align2::LEFT_CENTER,
        value,
        value_font,
        ink.body,
    );
    resp
}

// ===== src/widgets/chip.rs 说明区 =====
// 输入：chip 文案（label + value）；输出：单行小签（宽度按文案自适应）
// 逻辑步骤：量双段文案宽（meta/body 字号）→ chip_width（+26 内边距与段距，
//   钳 56..240）→ 卡底 + 发丝描边 → label（muted）+ value（body）同行
// 设计理由（UIR2）：旧双行 chip（56px 高）在信息密度上喧宾夺主——单行小签
//   让农历/节日退为注脚层；label/value 双色分行同级信息分层（谁的字段 + 值）
// 异常处理：无 panic 面
// 关联依赖：theme；消费方：pages/clock（农历 chips）、pages/countdown（节日 chips）
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_width_clamped() {
        // 自适应宽：短文案下限 56；中段 = 两段和 + 内边距；长文案上限 240
        assert_eq!(chip_width(10.0, 20.0), 56.0);
        assert_eq!(chip_width(100.0, 90.0), 216.0);
        assert_eq!(chip_width(500.0, 90.0), 240.0);
    }

    #[test]
    fn chip_height_token() {
        // 高度取 layout.chip_height token（UIR2 单行化定值 32）
        assert_eq!(chip_height(), 32.0);
    }
}
