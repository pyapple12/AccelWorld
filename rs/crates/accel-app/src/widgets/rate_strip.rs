//! 短倍率条（PL020）：360px 居中、6px 轨、1×/365× 端标、整数 1–365 档；
//! 轨道下附「现实 1 天 = 加速 N 天」量化行。
//! 拖动 SetRate、松手/点击 ApplyRate 恰一次；方向键 ±1 即时落盘。

use egui::{Align2, Rect, Sense, Stroke};

use super::slider::{rate_to_ratio, ratio_to_rate};
use super::text_ink::{self, InkKind};

/// 绘制倍率条并写回 rate；高度按断点传入，内部按比例排布
pub fn show(
    ui: &mut egui::Ui,
    min: f64,
    max: f64,
    rate: &mut f64,
    height: f32,
    mut on_change: impl FnMut(f64),
    mut on_commit: impl FnMut(f64),
) {
    let ink = crate::theme::ink();
    let width = ui.available_width().clamp(220.0, 360.0);
    // B18 盒 = 可见内容 bbox：盒顶即「倍率」文字顶，盒底即量化行底
    let s = ((height + 12.0) / 78.0).clamp(0.82, 1.0);
    let (outer, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    let inner_h = height.max(40.0);
    let inner = Rect::from_min_size(
        egui::pos2(outer.center().x - width / 2.0, outer.top()),
        egui::vec2(width, inner_h),
    );
    let track_y = inner.top() + inner_h * 0.36;
    let ticks_y = inner.top() + inner_h * 0.60;
    let label_font = crate::theme::font_medium(crate::theme::font_size("meta") * s);
    let label_ink = text_ink::measure(ui, "倍率", &label_font, InkKind::Cjk);
    let label_h = crate::widgets::text_metrics::height(ui.ctx(), &label_font, InkKind::Cjk);
    ui.painter().text(
        egui::pos2(
            inner.left(),
            label_ink.top_for_bottom(inner.top() + label_h),
        ),
        Align2::LEFT_TOP,
        "倍率",
        label_font,
        ink.muted,
    );
    let rate_font = crate::theme::font_medium(crate::theme::font_size("body") * s);
    let rate_text = format!("×{rate:.0}");
    let rate_ink = text_ink::measure(ui, &rate_text, &rate_font, InkKind::Cjk);
    let rate_h = crate::widgets::text_metrics::height(ui.ctx(), &rate_font, InkKind::Cjk);
    ui.painter().text(
        egui::pos2(inner.right(), rate_ink.top_for_bottom(inner.top() + rate_h)),
        Align2::RIGHT_TOP,
        rate_text,
        rate_font,
        crate::theme::accent_color(),
    );

    let track = Rect::from_min_max(
        egui::pos2(inner.left(), track_y),
        egui::pos2(inner.right(), track_y + 6.0),
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
                Some(-1.0)
            } else if i.key_pressed(egui::Key::ArrowRight) {
                Some(1.0)
            } else {
                None
            }
        });
        if let Some(d) = step {
            let new_rate = (*rate + d).clamp(min, max);
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
    let tick_font = crate::theme::font_regular(11.0 * s);
    let tick_h = crate::widgets::text_metrics::height(ui.ctx(), &tick_font, InkKind::Cjk);
    let tick_bottom = ticks_y + tick_h;
    let tick1_ink = text_ink::measure(ui, "1×", &tick_font, InkKind::Cjk);
    ui.painter().text(
        egui::pos2(inner.left(), tick1_ink.top_for_bottom(tick_bottom)),
        Align2::LEFT_TOP,
        "1×",
        tick_font.clone(),
        ink.meta,
    );
    let tick_max_text = format!("{max:.0}×");
    let tick_max_ink = text_ink::measure(ui, &tick_max_text, &tick_font, InkKind::Cjk);
    ui.painter().text(
        egui::pos2(inner.right(), tick_max_ink.top_for_bottom(tick_bottom)),
        Align2::RIGHT_TOP,
        tick_max_text,
        tick_font,
        ink.meta,
    );
    let quant_font = crate::theme::font_regular(11.0 * s);
    let quant_ink = text_ink::measure(ui, "现实", &quant_font, InkKind::Cjk);
    let year_days = accel_core::static_config::Static::get().base_f64("days_per_year");
    let quant_text = if (*rate - year_days).abs() < 0.5 {
        "现实 1 天 = 加速 1 年".to_string()
    } else {
        format!("现实 1 天 = 加速 {rate:.0} 天")
    };
    ui.painter().text(
        egui::pos2(inner.center().x, quant_ink.top_for_bottom(inner.bottom())),
        Align2::CENTER_TOP,
        quant_text,
        quant_font,
        ink.muted,
    );
}

// ===== src/widgets/rate_strip.rs 说明区 =====
// 输入：Ui + min/max/rate + change/commit 回调；输出：倍率条（拖拽/点击/方向键）
// 逻辑步骤：固定 360px 居中 → 标签行（倍率/×N）→ 6px 轨（muted）→ 交互
//   （比例→整数档，拖动 on_change/松手 on_commit）→ 金色 fill/旋钮
//   → 1×/365× 端标 → 「现实 1 天 = 加速 N 天 / 1 年」量化行 → 焦点环
// 设计理由（PL020）：倍率收敛为 1–365 整数，滑杆只出整数档；量化行把倍率翻译成
//   「现实 1 天 = 加速 N 天」，让 365× 的「天上 1 年」语义可直接读
// 异常处理：max<=min 时 rate_to_ratio 返回 0；键步长即时落盘
// 关联依赖：slider(rate_to_ratio/ratio_to_rate)/theme；消费方：pages/clock
