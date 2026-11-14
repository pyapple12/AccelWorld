//! 表单字段（PL021 盒模型）：label 盒 + 固定间距 + 固定高输入框。
//! 输入框底色用 input_bg；中文预编辑覆盖色由 egui selection.bg_fill 决定。

use egui::{Margin, Response, Sense, Stroke, UiBuilder};

use crate::theme;

/// 固定盒字段的三段：label 盒高 / 间距 / 输入框高
fn fixed_parts(ui: &egui::Ui) -> (f32, f32, f32) {
    let label_font = theme::font_medium(theme::font_size("meta"));
    let label_h = super::text_metrics::height(ui.ctx(), &label_font, super::text_ink::InkKind::Cjk);
    let input_h = theme::font_size("meta") + 2.0 * theme::layout_f32("field_input_pad");
    (label_h, theme::layout_f32("field_label_gap"), input_h)
}

/// 固定盒字段总高（label + gap + input）
pub fn fixed_height(ui: &egui::Ui) -> f32 {
    let (label_h, gap, input_h) = fixed_parts(ui);
    label_h + gap + input_h
}

/// 固定盒字段：label 盒 + label_gap + input_h；输入文字在框内垂直居中
pub fn show_fixed(
    ui: &mut egui::Ui,
    label: &str,
    hint: &str,
    text: &mut String,
    width: f32,
) -> Response {
    let ink = theme::ink();
    let label_font = theme::font_medium(theme::font_size("meta"));
    let input_font = theme::font_regular(theme::font_size("meta"));
    let (label_h, gap, input_h) = fixed_parts(ui);
    let total = label_h + gap + input_h;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, total), Sense::hover());

    let label_ink = super::text_ink::measure(ui, label, &label_font, super::text_ink::InkKind::Cjk);
    let label_y = rect.top() + (label_h - label_ink.h) / 2.0 - label_ink.top_pad;
    ui.painter().text(
        egui::pos2(rect.left(), label_y),
        egui::Align2::LEFT_TOP,
        label,
        label_font,
        ink.muted,
    );

    let input_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.top() + label_h + gap),
        egui::vec2(width, input_h),
    );
    let pad_x = theme::layout_f32("field_pad_x");
    let border = theme::layout_f32("field_border");
    let radius = egui::epaint::CornerRadius::same(theme::radius_px("sm") as u8);
    ui.painter()
        .rect_filled(input_rect, radius, theme::input_bg());
    ui.painter().rect_stroke(
        input_rect,
        radius,
        Stroke::new(border, theme::input_border_color()),
        egui::StrokeKind::Inside,
    );

    // 输入文字按自身行高在 16px 框内垂直居中
    let inner = input_rect.shrink2(egui::vec2(pad_x + border, border));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().interact_size.y = 12.0;
    let resp = child.add(
        egui::TextEdit::singleline(text)
            .font(input_font)
            .frame(egui::Frame::new())
            .margin(Margin::ZERO)
            .hint_text(hint)
            .desired_width(inner.width()),
    );
    if resp.has_focus() {
        ui.painter().rect_stroke(
            input_rect.expand(1.0),
            egui::epaint::CornerRadius::same(theme::radius_px("sm") as u8 + 1),
            Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    resp
}

// ===== src/widgets/field.rs 说明区 =====
// 输入：Ui + label/hint/文本/宽度；输出：输入框 Response
// 逻辑步骤：field_height 由 label 行高 + spacing.xs + 输入行高 + field_pad_y×2 计算；
//   show 先按该高度精确分配 rect，再在子 Ui 内放 label 与输入框
// 设计理由（PL021）：旧 field_height=54 是声明值，label 实际行高 19 导致总高 57，
//   两个页脚按 54 计算按钮底距抽屉底；改由同一公式计算，声明值=实际值
// 异常处理：无 panic 面；输入行高下限 1px
// 关联配置：scale.meta/body、spacing.xs、layout.field_pad_x/field_pad_y/field_border
