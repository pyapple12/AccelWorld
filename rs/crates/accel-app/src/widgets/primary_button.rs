//! 主操作按钮（PL019）：88×32 实底按钮——ink 底 + window 字，hover 提亮、
//! 按压只降亮度；PL021 改为扁平动效：底色 140ms 过渡 + 底部金线展开，不缩放/不位移。

use egui::{Align2, Response, Sense, Stroke};

/// 绘制主操作按钮并返回 Response（点击/键盘由调用方处理）
pub fn show(ui: &mut egui::Ui, label: &str, width: f32) -> Response {
    let ink = crate::theme::ink();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 32.0), Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    let hover_t = crate::widgets::motion::hover_t(
        ui.ctx(),
        ui.scope_id().with(("primary_button_hover", label)),
        resp.hovered(),
        crate::theme::motion_ms("card_hover_ms"),
    );
    let base = crate::widgets::motion::lerp_color(ink.strong, ink.knob, hover_t);
    let fill = if pressed { ink.body } else { base };
    ui.painter()
        .rect_filled(rect, egui::epaint::CornerRadius::same(8), fill);
    // 扁平 hover 反馈：底部 2px 金线从中心向两侧展开，不缩放、不位移
    if hover_t > 0.01 {
        let line = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.bottom() - 5.0),
            egui::vec2(24.0 * hover_t, 2.0),
        );
        ui.painter().rect_filled(
            line,
            egui::epaint::CornerRadius::same(1),
            crate::theme::accent_color(),
        );
    }
    ui.painter().text(
        rect.center(),
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
// 逻辑步骤：88×32 命中 → 底色 hover 140ms 提亮（按压降到 body）→ 底部金线展开 →
//   window 字 14/600 → 焦点环
// 设计理由（PL019/PL021）：主操作实底与字段同高；扁平界面不做按下缩放/弹跳，
//   hover 只提亮 + 金线，按压只压暗，避免「弹出」感；reduce_motion 直接落位
// 异常处理：无 panic 面；宽度由调用方给（默认 88）
// 关联依赖：theme(ink/window/accent)、widgets::motion；消费方：countdown/alarm
