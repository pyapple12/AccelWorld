//! 时钟页（UIR2 子夜星图）：中文日期 + 标准时中字 + 加速 hero 大字 +
//! 星轨刻度日进度（签名元素）+ 倍率滑杆 + 农历三 chips
//! 纯函数 day_progress / tick_h / time_part 可测

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

/// 刻度高度（纯函数，星轨刻度）：整点刻度（每小时第 1 道）高于半点刻度
/// 输入刻度序号；输出像素高——tick 间隔按 hours_per_tick 折算
pub fn tick_height(idx: usize) -> f32 {
    if idx.is_multiple_of(2) { 10.0 } else { 6.0 }
}

impl crate::pages::Page for crate::pages::ClockPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        let sp = |k: &str| crate::theme::spacing(k);
        // 表盘全部以面板中心为轴绘制（painter 绝对坐标——不依赖 layout 居中耦合）
        let center_x = ui.max_rect().center().x;

        if let Some(info) = &state.time {
            // 中文日期（星图页眉）
            ui.add_space(sp("lg"));
            ui.painter().text(
                egui::pos2(center_x, ui.cursor().top() + 12.0),
                egui::Align2::CENTER_TOP,
                &info.chinese_date,
                egui::FontId::proportional(crate::theme::font_size("time_sm")),
                ink.muted,
            );
            ui.allocate_space(egui::vec2(0.0, 36.0));

            // 标准时（等宽数字位——走秒不抖）
            let std_font = digits::digit_font(crate::theme::font_size("time_lg"));
            digits::draw_centered(
                ui.painter(),
                egui::pos2(
                    center_x,
                    ui.cursor().top() + crate::theme::font_size("time_lg") * 0.7,
                ),
                time_part(&info.standard_datetime),
                &std_font,
                ink.muted,
            );
            ui.allocate_space(egui::vec2(0.0, crate::theme::font_size("time_lg") * 1.5));
            ui.add_space(sp("md"));

            // 加速 hero（等宽大字 + ×倍率徽标同行；hero 为绝对主角）
            let hero_font = digits::digit_font(crate::theme::font_size("hero"));
            let badge_font = digits::digit_font(crate::theme::font_size("time_md"));
            let hero_text = info.custom_time.clone();
            let badge = format!("×{:.1}", state.rate);
            let hero_w = digits::text_width(ui.painter(), &hero_text, &hero_font);
            let badge_w = digits::text_width(ui.painter(), &badge, &badge_font);
            let gap = sp("lg");
            let hero_y = ui.cursor().top() + crate::theme::font_size("hero") * 0.7;
            digits::draw_centered(
                ui.painter(),
                egui::pos2(center_x - (badge_w + gap) / 2.0, hero_y),
                &hero_text,
                &hero_font,
                ink.strong,
            );
            digits::draw_centered(
                ui.painter(),
                egui::pos2(center_x + (hero_w + gap) / 2.0, hero_y + 10.0),
                &badge,
                &badge_font,
                crate::theme::accent_color(),
            );
            ui.allocate_space(egui::vec2(0.0, crate::theme::font_size("hero") * 1.5));

            // 星轨刻度日进度（签名元素：48 道刻度的仪器标尺——已过段金色、
            // 未至段弱轨色；整点刻高、半点刻矮，读数如星图刻度环展开）
            let ticks = 48usize;
            let scale_w = (ui.available_width() * 0.62).min(440.0);
            let scale_left = center_x - scale_w / 2.0;
            let scale_y = ui.cursor().top() + 6.0;
            let progress = day_progress(info.expanded_hours_per_day, info.remaining_hours);
            let _ = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::hover());
            for i in 0..ticks {
                let x = scale_left + scale_w * (i as f32 / (ticks - 1) as f32);
                let passed = (i as f32 / (ticks - 1) as f32) <= progress;
                let h = tick_height(i);
                let c = if passed {
                    crate::theme::accent_color()
                } else {
                    crate::theme::track_color()
                };
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(x, scale_y + 10.0 - h),
                        egui::vec2(2.0, h),
                    ),
                    egui::epaint::CornerRadius::same(1),
                    c,
                );
            }
            // 基线（发丝——刻度落其上）
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(scale_left, scale_y + 10.0),
                    egui::vec2(scale_w, 1.0),
                ),
                egui::epaint::CornerRadius::same(0),
                crate::theme::border_color(),
            );
            // 两端读数（0% / 100% 不标——日进度语义自明，刻度即读数）
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

        // 农历 chips 行（split 三段；安静徽标）
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
//   星轨刻度日进度（day_progress × 48 道刻度）→ 倍率滑杆 → 农历三 chips
// 设计理由（UIR2 子夜星图）：星轨刻度是全局签名元素——替代旧 4px 光条（光条是
//   通用默认处理），仪器标尺语义承接"时间膨胀观测仪"的主题；刻度高矮交替
//   （整点高/半点矮）给出无文字的读数节奏；间距全部走 spacing token
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

    #[test]
    fn tick_height_alternates() {
        // 星轨刻度：偶数位（整点）高、奇数位（半点）矮
        assert_eq!(tick_height(0), 10.0);
        assert_eq!(tick_height(1), 6.0);
        assert_eq!(tick_height(48), 10.0);
    }
}
