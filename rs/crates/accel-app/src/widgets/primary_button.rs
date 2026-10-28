//! 主操作按钮（PL019）：88×32 实底按钮——ink 底 + window 字，hover 提亮、
//! 按压内缩 0.5px；金不给按钮（金=数据/当前值）。

use egui::{Align2, Response, Sense, Stroke};

/// 绘制主操作按钮并返回 Response（点击/键盘由调用方处理）
pub fn show(ui: &mut egui::Ui, label: &str, width: f32) -> Response {
    let ink = crate::theme::ink();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 32.0), Sense::click());
    let hovered = resp.hovered();
    let pressed = resp.is_pointer_button_down_on();
    let fill = if pressed {
        ink.body
    } else if hovered {
        ink.knob
    } else {
        ink.strong
    };
    let r = if pressed { rect.shrink(0.5) } else { rect };
    ui.painter()
        .rect_filled(r, egui::epaint::CornerRadius::same(8), fill);
    ui.painter().text(
        r.center(),
        Align2::CENTER_CENTER,
        label,
        crate::theme::font_semibold(crate::theme::font_size("body")),
        crate::theme::window_bg(),
    );
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            egui::epaint::CornerRadius::same(9),
            Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ===== src/widgets/primary_button.rs 说明区 =====
// 输入：Ui + 文案 + 宽度；输出：整块 Response
// 逻辑步骤：88×32 命中 → ink 实底（hover 提亮/按压内缩）→ window 字 14/600 → 焦点环
// 设计理由（PL019）：页脚原「+」孤立且基线不齐；主操作用文字按钮，与字段同高
// 异常处理：无 panic 面；宽度由调用方给（默认 88）
// 关联依赖：theme(ink/window)；消费方：pages/countdown、pages/alarm
