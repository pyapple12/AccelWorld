//! 短倍率条（PL017）：360px 居中、6px 轨、1×/20× 端标；绝对几何绘制，
//! 不依赖嵌套布局（原 slider 在全宽/右对齐混排下把旋钮推到端部）。
//! 拖动 SetRate、松手/点击 ApplyRate 恰一次；方向键 ±0.1 即时落盘。

use egui::{Align2, Rect, Sense, Stroke};

use super::slider::{rate_to_ratio, ratio_to_rate};

/// 绘制倍率条并写回 rate；回调语义与 slider 一致
pub fn show(
    ui: &mut egui::Ui,
    min: f64,
    max: f64,
    rate: &mut f64,
    mut on_change: impl FnMut(f64),
    mut on_commit: impl FnMut(f64),
) {
    let ink = crate::theme::ink();
    let width = ui.available_width().clamp(220.0, 360.0);
    let (outer, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 62.0), Sense::hover());
    let inner = Rect::from_center_size(
        egui::pos2(outer.center().x, outer.center().y + 3.0),
        egui::vec2(width, 44.0),
    );
    ui.painter().text(
        egui::pos2(inner.left(), inner.top()),
        Align2::LEFT_TOP,
        "倍率",
        crate::theme::font_medium(crate::theme::font_size("meta")),
        ink.muted,
    );
    ui.painter().text(
        egui::pos2(inner.right(), inner.top()),
        Align2::RIGHT_TOP,
        format!("×{rate:.1}"),
        crate::theme::font_medium(crate::theme::font_size("body")),
        crate::theme::accent_color(),
    );

    let track = Rect::from_min_max(
        egui::pos2(inner.left(), inner.top() + 24.0),
        egui::pos2(inner.right(), inner.top() + 30.0),
    );
    let hit = track.expand2(egui::vec2(0.0, 10.0));
    let resp = ui.interact(hit, egui::Id::unique("rate_strip"), Sense::click_and_drag());
    let mut changed = None;
    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let ratio = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0) as f64;
        let new_rate = ratio_to_rate(ratio, min, max);
        if (new_rate - *rate).abs() > f64::EPSILON {
            *rate = new_rate;
            changed = Some(new_rate);
        }
    }
    if resp.has_focus() {
        let step = ui.input(|i| {
            if i.key_pressed(egui::Key::ArrowLeft) {
                Some(-0.1)
            } else if i.key_pressed(egui::Key::ArrowRight) {
                Some(0.1)
            } else {
                None
            }
        });
        if let Some(d) = step {
            let new_rate =
                ratio_to_rate(rate_to_ratio(*rate, min, max) + d / (max - min), min, max);
            if (new_rate - *rate).abs() > f64::EPSILON {
                *rate = new_rate;
                changed = Some(new_rate);
            }
        }
    }
    if let Some(r) = changed {
        on_change(r);
    }
    if resp.drag_stopped() || resp.clicked() {
        on_commit(*rate);
    }

    let ratio = rate_to_ratio(*rate, min, max) as f32;
    ui.painter().rect_filled(
        track,
        egui::epaint::CornerRadius::same(3),
        crate::theme::border_strong_color(),
    );
    let knob_x = track.left() + track.width() * ratio;
    let fill = Rect::from_min_max(
        track.left_top(),
        egui::pos2(knob_x.max(track.left() + 3.0), track.bottom()),
    );
    ui.painter().rect_filled(
        fill,
        egui::epaint::CornerRadius::same(3),
        crate::theme::accent_color(),
    );
    let knob_t = crate::widgets::motion::anim(
        ui.ctx(),
        egui::Id::unique("rate_knob_hover"),
        if resp.hovered() || resp.dragged() {
            1.0
        } else {
            0.0
        },
        crate::theme::motion_ms("slider_hover_ms"),
    );
    let knob_r = 8.0 + knob_t;
    ui.painter()
        .circle_filled(egui::pos2(knob_x, track.center().y), knob_r, ink.knob);
    ui.painter().circle_stroke(
        egui::pos2(knob_x, track.center().y),
        knob_r,
        Stroke::new(1.0, ink.meta),
    );
    if resp.has_focus() {
        ui.painter().rect_stroke(
            hit.expand(2.0),
            egui::epaint::CornerRadius::same(6),
            Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        egui::pos2(inner.left(), inner.bottom()),
        Align2::LEFT_BOTTOM,
        "1×",
        crate::theme::font_regular(11.0),
        ink.meta,
    );
    ui.painter().text(
        egui::pos2(inner.right(), inner.bottom()),
        Align2::RIGHT_BOTTOM,
        "20×",
        crate::theme::font_regular(11.0),
        ink.meta,
    );
}

// ===== src/widgets/rate_strip.rs 说明区 =====
// 输入：Ui + min/max/rate + change/commit 回调；输出：倍率条（拖拽/点击/方向键）
// 逻辑步骤：固定 360px 居中 → 标签行（倍率/×当前）→ 6px 轨（muted）→ 交互
//   （ratio→rate 0.1 步进，拖动 on_change/松手 on_commit）→ 金色 fill/16px 旋钮
//   → 1×/20× 端标 → 焦点环
// 设计理由（PL017）：原全宽 24px 滑杆是第二视觉体且无端点/说明；嵌套布局在
//   900×500 下把旋钮推至端部。本组件用绝对 Rect 绘制，几何可测、位置确定
// 异常处理：max<=min 时 rate_to_ratio 返回 0；键步长即时落盘
// 关联依赖：slider(rate_to_ratio/ratio_to_rate)/theme；消费方：pages/clock
