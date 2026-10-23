//! 时钟页（UIR 重塑：表盘式布局）——标准时小字 + 加速 hero 等宽大字 +
//! 日进度细条 + 倍率滑杆 + 农历三 chips（A008 #7：玻璃面废除后 chip 无 id）
//! 纯函数 day_progress 可测（分母钳制，FIX006.27 延续）

use crate::tools::clock::split_lunar_info;
use crate::widgets::{chip, digits, slider};
use accel_core::types::FrontendCommand;

/// 日进度（纯函数）：已过小时 / 整数日长，双钳制（日长≤0 或越界返 0..1 域内值）
pub fn day_progress(expanded_hours: f64, remaining_hours: f64) -> f32 {
    if expanded_hours <= 0.0 {
        return 0.0;
    }
    ((expanded_hours - remaining_hours) / expanded_hours).clamp(0.0, 1.0) as f32
}

/// 从标准日期时间串截取时刻段（"YYYY-MM-DD HH:MM:SS" → "HH:MM:SS"；纯函数可测）
pub fn time_part(standard_datetime: &str) -> &str {
    standard_datetime.get(11..19).unwrap_or("")
}

impl crate::pages::Page for crate::pages::ClockPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        // 表盘全部以面板中心为轴绘制（painter 绝对坐标——不依赖 layout 居中耦合）
        let center_x = ui.max_rect().center().x;

        if let Some(info) = &state.time {
            // 中文日期（文本排版走 egui，取居中锚点）
            ui.add_space(20.0);
            let date_font = egui::FontId::proportional(crate::theme::font_size("heading"));
            ui.painter().text(
                egui::pos2(center_x, ui.cursor().top() + 12.0),
                egui::Align2::CENTER_TOP,
                &info.chinese_date,
                date_font,
                ink.muted,
            );
            ui.allocate_space(egui::vec2(0.0, 34.0));

            // 标准时（等宽数字位——走秒不抖）
            let std_font = digits::digit_font(crate::theme::font_size("std_time"));
            digits::draw_centered(
                ui.painter(),
                egui::pos2(
                    center_x,
                    ui.cursor().top() + crate::theme::font_size("std_time") * 0.7,
                ),
                time_part(&info.standard_datetime),
                &std_font,
                ink.soft,
            );
            ui.allocate_space(egui::vec2(0.0, crate::theme::font_size("std_time") * 1.5));
            ui.add_space(12.0);

            // 加速 hero（等宽大字 + ×倍率徽标同行；hero 为绝对主角）
            let hero_font = digits::digit_font(crate::theme::font_size("hero_time"));
            let badge_font = digits::digit_font(crate::theme::font_size("slider_value"));
            let hero_text = info.custom_time.clone();
            let badge = format!("×{:.1}", state.rate);
            let hero_w = digits::text_width(ui.painter(), &hero_text, &hero_font);
            let badge_w = digits::text_width(ui.painter(), &badge, &badge_font);
            let gap = 18.0_f32;
            let hero_y = ui.cursor().top() + crate::theme::font_size("hero_time") * 0.7;
            digits::draw_centered(
                ui.painter(),
                egui::pos2(center_x - (badge_w + gap) / 2.0, hero_y),
                &hero_text,
                &hero_font,
                ink.strong,
            );
            digits::draw_centered(
                ui.painter(),
                egui::pos2(center_x + (hero_w + gap) / 2.0, hero_y + 8.0),
                &badge,
                &badge_font,
                crate::theme::accent_color(),
            );
            ui.allocate_space(egui::vec2(0.0, crate::theme::font_size("hero_time") * 1.5));

            // 日进度细条（表盘余韵：track + accent 填充 + 圆角端头）
            let bar_w = (ui.available_width() * 0.62).min(420.0);
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(center_x - bar_w / 2.0, ui.cursor().top() + 6.0),
                egui::vec2(bar_w, 4.0),
            );
            let progress = day_progress(info.expanded_hours_per_day, info.remaining_hours);
            let track_c = crate::theme::color32_at("ring_track", 70);
            ui.painter()
                .rect_filled(bar_rect, egui::epaint::CornerRadius::same(2), track_c);
            if progress > 0.001 {
                let mut fill = bar_rect;
                fill.max.x = fill.left() + (bar_w * progress).max(4.0);
                ui.painter().rect_filled(
                    fill,
                    egui::epaint::CornerRadius::same(2),
                    crate::theme::accent_color(),
                );
            }
            ui.allocate_space(egui::vec2(0.0, 22.0));
        }

        // 倍率滑杆（拖动 SetRate 实时 / 松手 ApplyRate 落盘）
        let (min, max) = state.rate_bounds;
        let mut rate = state.rate;
        let mut set_rate = None;
        let mut apply_rate = None;
        slider::show(
            ui,
            crate::theme::layout_i64("slider_track") as f32,
            min,
            max,
            &mut rate,
            |r| set_rate = Some(r),
            |r| apply_rate = Some(r),
        );
        if let Some(r) = set_rate {
            bus.send(FrontendCommand::SetRate(r));
        }
        if let Some(r) = apply_rate {
            bus.send(FrontendCommand::ApplyRate(r));
        }
        // 滑杆本地值立即回写（拖动期间 Tick 快照不回刷旧值）
        state.rate = rate;

        // 农历 chips 行（split 三段；矢量纸片——UIR 实心主题）
        if let Some(info) = &state.time {
            let (lunar, phase, god) = split_lunar_info(&info.lunar_info);
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.horizontal(|ui| {
                    chip::show(ui, "农历", &lunar);
                    chip::show(ui, "月相", &phase);
                    chip::show(ui, "财神", &god);
                });
            });
        }
    }
}

// ===== src/pages/clock.rs 说明区 =====
// 输入：UiState（Tick 快照：时间/倍率域）；输出：表盘 hero 装配 + SetRate/ApplyRate
// 逻辑步骤：中文日期 → 标准时（等宽数字位）→ 加速 hero（等宽大字 + ×倍率徽标）→
//   日进度细条（day_progress）→ 倍率滑杆（拖动实时/松手落盘）→ 农历三 chips
// 设计理由（UIR 重塑）：表盘式纵向居中——时钟页是应用的脸面，加速时间必须是
//   视觉主角；等宽数字位根治走秒抖动（y.problems#12）；日进度细条以最低噪音
//   给出"今天过了多少"的仪器读数；倍率徽标紧贴 hero 右下（倍率与时间一体可读）
// 异常处理：time 为 None（首 Tick 前）整块表盘跳过不崩；day_progress 双钳制
// 关联依赖：widgets（digits/slider/chip）/tools::clock；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_progress_clamped() {
        // 双钳制：日长≤0 → 0；负剩余 → 满；正常比例
        assert_eq!(day_progress(0.0, 1.0), 0.0);
        assert_eq!(day_progress(24.0, -1.0), 1.0);
        assert!((day_progress(24.0, 18.0) - 0.25).abs() < 1e-9);
        assert_eq!(day_progress(24.0, 24.0), 0.0);
    }

    #[test]
    fn time_part_extraction() {
        // "YYYY-MM-DD HH:MM:SS" → 时刻段；异常形态回退空串
        assert_eq!(time_part("2026-10-23 12:34:56"), "12:34:56");
        assert_eq!(time_part("bad"), "");
    }
}
