//! 表单字段（PL017）：label 上置 + 36px 输入框（1px input_border、8px 圆角）+
//! 可选 hint/error。替换“placeholder 当 label”与无边界黑胶囊。

use egui::{Margin, Response, Stroke};

/// 字段整体高度（label 16 + 4 间距 + 34 输入）
pub fn field_height() -> f32 {
    54.0
}

/// 绘制字段并返回输入框 Response；宽度固定由调用方分配。
pub fn show(ui: &mut egui::Ui, label: &str, hint: &str, text: &mut String, width: f32) -> Response {
    let ink = crate::theme::ink();
    ui.allocate_ui_with_layout(
        egui::vec2(width, field_height()),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.label(
                egui::RichText::new(label)
                    .font(crate::theme::font_medium(crate::theme::font_size("meta")))
                    .color(ink.muted),
            );
            ui.add_space(4.0);
            let frame = egui::Frame::new()
                .fill(crate::theme::input_bg())
                .stroke(Stroke::new(1.0, crate::theme::input_border_color()))
                .corner_radius(egui::epaint::CornerRadius::same(8))
                .inner_margin(Margin::symmetric(10, 6));
            let resp = frame
                .show(ui, |ui| {
                    ui.add_sized(
                        [width - 20.0, 22.0],
                        egui::TextEdit::singleline(text)
                            .frame(egui::Frame::new())
                            .hint_text(hint)
                            .desired_width(width - 20.0),
                    )
                })
                .inner;
            if resp.has_focus() {
                ui.painter().rect_stroke(
                    resp.rect.expand(1.0),
                    egui::epaint::CornerRadius::same(9),
                    Stroke::new(1.5, ink.knob),
                    egui::StrokeKind::Inside,
                );
            }
            resp
        },
    )
    .inner
}

// ===== src/widgets/field.rs 说明区 =====
// 输入：label/hint/可变文本/宽度；输出：TextEdit Response
// 逻辑步骤：label 12/500 → 4px → Frame(input_bg + input_border + radius 8) →
//   无边框 TextEdit（22px 行高）→ 聚焦时亮白 1.5px 环
// 设计理由（PL017）：原输入框只有填充色差（1.10:1）且 placeholder 兼任 label；
//   本组件把“字段名”放回结构、把边界做到 ≥3:1（G3），并统一 54px 字段节奏
// 异常处理：无 panic 面；宽度由调用方保证 ≥ 80
// 关联依赖：theme(input_bg/input_border/knob)；消费方：倒计时/闹钟/世界添加行
