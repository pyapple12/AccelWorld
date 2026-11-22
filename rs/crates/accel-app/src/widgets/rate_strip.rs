//! 短倍率条（PL023 盒子版）：360px 居中、6px 轨、1×/365× 端标、整数 1–365 档；
//! 轨道下附「现实 1 天 = 加速 N 天」量化行。
//! 固定盒：12.5 + 5 + 24 + 5 + 10.0 + 5 + 11.5 = 73.0；拖动 SetRate、松手/点击 ApplyRate，
//! 键盘方向键 400ms 去抖后 ApplyRate（FIX013.34）。

use egui::{Align2, Rect, Sense, Stroke};

use super::slider::{rate_to_ratio, ratio_to_rate};
use super::text_ink::{self, InkKind};

/// 倍率条总高：由字体组合表与 spacing token 计算
pub fn height() -> f32 {
    let cjk = |size: f32| crate::font_profile::get().for_size(size).h;
    let latin = |size: f32| crate::font_profile::get().for_size(size).h_latin;
    let label_row_h =
        cjk(crate::theme::font_size("meta")).max(latin(crate::theme::font_size("body")));
    let tick_row_h = latin(crate::theme::font_size("meta"));
    let quant_row_h = cjk(crate::theme::font_size("meta"));
    label_row_h
        + crate::theme::layout_f32("rate_label_track_gap")
        + crate::theme::layout_f32("rate_track_box")
        + crate::theme::layout_f32("rate_track_ticks_gap")
        + tick_row_h
        + crate::theme::layout_f32("rate_ticks_quant_gap")
        + quant_row_h
}

/// 绘制倍率条并写回 rate；内部四个固定盒，不随断点缩放
pub fn show(
    ui: &mut egui::Ui,
    min: f64,
    max: f64,
    rate: &mut f64,
    mut on_change: impl FnMut(f64),
    mut on_commit: impl FnMut(f64),
) {
    let ink = crate::theme::ink();
    let width_max = crate::theme::layout_f32("rate_width_max");
    let width = ui.available_width().clamp(220.0, width_max);
    let track_box_h = crate::theme::layout_f32("rate_track_box");
    let track_thickness = crate::theme::layout_f32("rate_track");
    let label_track_gap = crate::theme::layout_f32("rate_label_track_gap");
    let track_ticks_gap = crate::theme::layout_f32("rate_track_ticks_gap");
    let ticks_quant_gap = crate::theme::layout_f32("rate_ticks_quant_gap");
    let cjk = |size: f32| crate::font_profile::get().for_size(size).h;
    let latin = |size: f32| crate::font_profile::get().for_size(size).h_latin;
    let tick_row_h = latin(crate::theme::font_size("meta"));
    let total_h = height();
    let (outer, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), total_h), Sense::hover());
    let inner = Rect::from_min_size(
        egui::pos2(outer.center().x - width / 2.0, outer.top()),
        egui::vec2(width, total_h),
    );
    let label_row_h =
        cjk(crate::theme::font_size("meta")).max(latin(crate::theme::font_size("body")));
    let label_row_bottom = inner.top() + label_row_h;
    let label_font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let label_ink = text_ink::measure(ui, "倍率", &label_font, InkKind::Cjk);
    crate::layout_probe::text(
        ui,
        "rate.label",
        "倍率",
        &label_font,
        InkKind::Cjk,
        egui::pos2(inner.left(), label_ink.top_for_bottom(label_row_bottom)),
        Align2::LEFT_TOP,
    );
    crate::text_probe::boxed_line(
        ui,
        "rate.label",
        "倍率",
        &label_font,
        InkKind::Cjk,
        egui::pos2(inner.left(), label_ink.top_for_bottom(label_row_bottom)),
        Align2::LEFT_TOP,
        label_row_bottom,
    );
    ui.painter().text(
        egui::pos2(inner.left(), label_ink.top_for_bottom(label_row_bottom)),
        Align2::LEFT_TOP,
        "倍率",
        label_font,
        ink.muted,
    );
    let rate_font = crate::theme::font_medium(crate::theme::font_size("body"));
    let rate_text = format!("×{rate:.0}");
    let rate_ink = text_ink::measure(ui, &rate_text, &rate_font, InkKind::Digits);
    crate::layout_probe::text(
        ui,
        "rate.value",
        &rate_text,
        &rate_font,
        InkKind::Digits,
        egui::pos2(inner.right(), rate_ink.top_for_bottom(label_row_bottom)),
        Align2::RIGHT_TOP,
    );
    crate::text_probe::boxed_line(
        ui,
        "rate.value",
        &rate_text,
        &rate_font,
        InkKind::Digits,
        egui::pos2(inner.right(), rate_ink.top_for_bottom(label_row_bottom)),
        Align2::RIGHT_TOP,
        label_row_bottom,
    );
    ui.painter().text(
        egui::pos2(inner.right(), rate_ink.top_for_bottom(label_row_bottom)),
        Align2::RIGHT_TOP,
        rate_text,
        rate_font,
        crate::theme::accent_color(),
    );

    let track_box_top = label_row_bottom + label_track_gap;
    let track_y = track_box_top + (track_box_h - track_thickness) / 2.0;
    let track = Rect::from_min_max(
        egui::pos2(inner.left(), track_y),
        egui::pos2(inner.right(), track_y + track_thickness),
    );
    crate::layout_probe::rect(ui, "rate.track", track);
    let hit = track.expand2(egui::vec2(0.0, 10.0));
    let resp = ui.interact(hit, egui::Id::unique("rate_strip"), Sense::click_and_drag());
    let commit_id = resp.id.with("kbd_commit");
    let mut changed = None;
    let mut key_stepped = false;
    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        ui.memory_mut(|m| m.data.remove::<f64>(commit_id));
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
            // 步长 token 单源（FIX013.18）；倍率域只收整数档
            let step = accel_core::static_config::Static::get()
                .base_f64("rate_step")
                .max(1.0);
            let new_rate = (*rate + d * step).round().clamp(min, max);
            if (new_rate - *rate).abs() > f64::EPSILON {
                *rate = new_rate;
                changed = Some(new_rate);
                key_stepped = true;
            }
        }
    }
    if let Some(r) = changed {
        on_change(r);
    }
    // 键盘步进 400ms 去抖落盘（FIX013.34）；拖动/点击走下方即时提交
    let now = ui.ctx().input(|i| i.time);
    if key_stepped {
        ui.memory_mut(|m| m.data.insert_temp(commit_id, now));
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(400));
    } else if let Some(last_step) = ui.memory(|m| m.data.get_temp::<f64>(commit_id)) {
        if key_commit_due(last_step, now) {
            ui.memory_mut(|m| m.data.remove::<f64>(commit_id));
            on_commit(*rate);
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(400));
        }
    }
    if resp.drag_stopped() || resp.clicked() {
        ui.memory_mut(|m| m.data.remove::<f64>(commit_id));
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
    let tick_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let tick_row_bottom = track_box_top + track_box_h + track_ticks_gap + tick_row_h;
    let tick1_ink = text_ink::measure(ui, "1×", &tick_font, InkKind::Digits);
    crate::layout_probe::text(
        ui,
        "rate.tick1",
        "1×",
        &tick_font,
        InkKind::Digits,
        egui::pos2(inner.left(), tick1_ink.top_for_bottom(tick_row_bottom)),
        Align2::LEFT_TOP,
    );
    crate::text_probe::boxed_line(
        ui,
        "rate.tick1",
        "1×",
        &tick_font,
        InkKind::Digits,
        egui::pos2(inner.left(), tick1_ink.top_for_bottom(tick_row_bottom)),
        Align2::LEFT_TOP,
        tick_row_bottom,
    );
    ui.painter().text(
        egui::pos2(inner.left(), tick1_ink.top_for_bottom(tick_row_bottom)),
        Align2::LEFT_TOP,
        "1×",
        tick_font.clone(),
        ink.meta,
    );
    let tick_max_text = format!("{max:.0}×");
    let tick_max_ink = text_ink::measure(ui, &tick_max_text, &tick_font, InkKind::Digits);
    crate::layout_probe::text(
        ui,
        "rate.tickmax",
        &tick_max_text,
        &tick_font,
        InkKind::Digits,
        egui::pos2(inner.right(), tick_max_ink.top_for_bottom(tick_row_bottom)),
        Align2::RIGHT_TOP,
    );
    crate::text_probe::boxed_line(
        ui,
        "rate.tickmax",
        &tick_max_text,
        &tick_font,
        InkKind::Digits,
        egui::pos2(inner.right(), tick_max_ink.top_for_bottom(tick_row_bottom)),
        Align2::RIGHT_TOP,
        tick_row_bottom,
    );
    ui.painter().text(
        egui::pos2(inner.right(), tick_max_ink.top_for_bottom(tick_row_bottom)),
        Align2::RIGHT_TOP,
        tick_max_text,
        tick_font,
        ink.meta,
    );
    let quant_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let quant_ink = text_ink::measure(ui, "现实", &quant_font, InkKind::Cjk);
    let quant_row_h = cjk(crate::theme::font_size("meta"));
    let quant_row_bottom = tick_row_bottom + ticks_quant_gap + quant_row_h;
    let year_days = accel_core::static_config::Static::get().base_f64("days_per_year");
    let quant_text = if (*rate - year_days).abs() < 0.5 {
        "现实 1 天 = 加速 1 年".to_string()
    } else {
        format!("现实 1 天 = 加速 {rate:.0} 天")
    };
    crate::layout_probe::text(
        ui,
        "rate.quant",
        &quant_text,
        &quant_font,
        InkKind::Cjk,
        egui::pos2(inner.center().x, quant_ink.top_for_bottom(quant_row_bottom)),
        Align2::CENTER_TOP,
    );
    crate::text_probe::boxed_line(
        ui,
        "rate.quant",
        &quant_text,
        &quant_font,
        InkKind::Cjk,
        egui::pos2(inner.center().x, quant_ink.top_for_bottom(quant_row_bottom)),
        Align2::CENTER_TOP,
        quant_row_bottom,
    );
    ui.painter().text(
        egui::pos2(inner.center().x, quant_ink.top_for_bottom(quant_row_bottom)),
        Align2::CENTER_TOP,
        quant_text,
        quant_font,
        ink.muted,
    );
}

/// 键盘落盘去抖（纯函数）：最后步进后满 400ms 返回 true
fn key_commit_due(last_step_s: f64, now_s: f64) -> bool {
    now_s - last_step_s >= 0.4
}

// ===== src/widgets/rate_strip.rs 说明区 =====
// 输入：Ui + min/max/rate + change/commit 回调；输出：倍率条（拖拽/点击/方向键）
// 逻辑步骤：固定 360px 居中 → 标签行 12.5（倍率/×N 盒底）→ 5 → 轨道盒 24（6px 轨居中）
//   → 5 → 端标行 10.0（数字）→ 5 → 量化行 11.5（中文）；交互比例→整数档，拖动 on_change/松手 on_commit；
//   键盘方向键按 rate_step 步进，400ms 无新步进后 on_commit
// 设计理由（PL023）：四个固定盒，不随断点缩放；轨/端标/量化行都是盒子；金只做数据
// 异常处理：max<=min 时 rate_to_ratio 返回 0；键步长 token 非法时钳 1
// 关联依赖：slider(rate_to_ratio/ratio_to_rate)/text_ink/theme；消费方：pages/clock

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_commit_waits_400ms() {
        // FIX013.34：0.399s 不提交，0.400s 恰提交
        assert!(!key_commit_due(10.0, 10.399));
        assert!(key_commit_due(10.0, 10.400));
    }

    #[test]
    fn height_matches_fixed_boxes() {
        assert!((height() - 73.0).abs() < 1e-6, "got {}", height());
    }
}
