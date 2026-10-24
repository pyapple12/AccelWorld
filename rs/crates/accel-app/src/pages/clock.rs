//! 时钟页（UIR4 星盘主仪）：单环双针 24 小时表盘恒驻 home——金长针 = 加速此刻、
//! 灰细针 = 现实此刻，同一刻度盘上两针速差即时间膨胀（钟表的本源语法）；
//! 中心等宽 hero + ×倍率徽标 + 标准时/日期；下方倍率滑杆与农历 chips
//! 纯函数 day_fraction/day_angle/polar/dial_diameter/hero_fit_scale/day_progress 可测

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

/// "HH:MM:SS" → 一日已过比例（纯函数）：0 点正上起算；异常形态返 0
pub fn day_fraction(time_hms: &str) -> f32 {
    let h: f32 = time_hms
        .get(0..2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let m: f32 = time_hms
        .get(3..5)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let s: f32 = time_hms
        .get(6..8)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    ((h * 3600.0 + m * 60.0 + s) / 86400.0).clamp(0.0, 1.0)
}

/// 一日比例 → 表盘角（纯函数）：0 正上、顺时针（0..360）
pub fn day_angle(fraction: f32) -> f32 {
    fraction.clamp(0.0, 1.0) * 360.0
}

/// 表盘极坐标（纯函数）：正上 0° 顺时针 → 平面坐标
pub fn polar(cx: f32, cy: f32, r: f32, angle_deg: f32) -> (f32, f32) {
    let a = angle_deg.to_radians() - std::f32::consts::FRAC_PI_2;
    (cx + r * a.cos(), cy + r * a.sin())
}

/// 星盘直径（纯函数）：高（预留滑杆+chips 96px）与宽 0.9 取小，钳 [200, 440]
pub fn dial_diameter(avail_w: f32, avail_h: f32) -> f32 {
    (avail_h - 96.0).min(avail_w * 0.9).clamp(200.0, 440.0)
}

/// hero 组合宽自适应（纯函数）：组合宽（hero+间距+徽标）超可用宽时按比例
/// 收号，下限 0.55；未超返 1.0
pub fn hero_fit_scale(combined_w: f32, avail_w: f32) -> f32 {
    if combined_w <= avail_w || combined_w <= 0.0 {
        1.0
    } else {
        (avail_w / combined_w).clamp(0.55, 1.0)
    }
}

impl crate::pages::Page for crate::pages::ClockPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        let accent = crate::theme::accent_color();
        let idle = crate::theme::tick_idle_color();

        if let Some(info) = &state.time {
            // —— 星盘主体（painter 绝对坐标；直径随可用域自适应，抽屉开启时缩让）——
            let avail = ui.available_size();
            let dial_d = dial_diameter(avail.x, avail.y);
            let cx = ui.max_rect().center().x;
            let cy = ui.cursor().top() + dial_d / 2.0 + 4.0;
            let outer_r = dial_d / 2.0 - 24.0;

            // 刻度环：24 小时表盘（96 刻度 = 15 分钟一格；整点刻长刻亮）——
            // 纯量尺不含数据（UIR4.6 定案：含义唯一原则——刻度=尺、指针=数据，
            // 废除「已过段填金」（与指针同义重复）；双环亦废（形异义同））
            for i in 0..96 {
                let a = i as f32 / 96.0;
                let ang = day_angle(a);
                let hour_tick = i % 4 == 0;
                let len = if hour_tick { 12.0 } else { 7.0 };
                let (x1, y1) = polar(cx, cy, outer_r, ang);
                let (x2, y2) = polar(cx, cy, outer_r - len, ang);
                ui.painter().line_segment(
                    [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                    egui::Stroke::new(
                        if hour_tick { 2.0 } else { 1.2 },
                        if hour_tick { ink.muted } else { idle },
                    ),
                );
            }
            // 基数标签（00/06/12/18 四正位）
            let cardinal = egui::FontId::proportional(crate::theme::font_size("meta"));
            for (ang, label) in [(0.0, "00"), (90.0, "06"), (180.0, "12"), (270.0, "18")] {
                let (lx, ly) = polar(cx, cy, outer_r + 14.0, ang);
                ui.painter().text(
                    egui::pos2(lx, ly),
                    egui::Align2::CENTER_CENTER,
                    label,
                    cardinal.clone(),
                    ink.meta,
                );
            }

            // 双针（同一表盘两种时间——膨胀即两针速差与角差，钟表的本源语法）：
            // 金色长针 = 加速此刻（快转，倍率高时肉眼可见甩开灰针）；
            // 灰色细针 = 现实此刻（慢转基准）；轴心小毂承载双针
            let acc_frac = day_fraction(&info.custom_time);
            let real_frac = day_fraction(time_part(&info.standard_datetime));
            let hand_r = outer_r - 26.0;
            // 金针（加速）：长针 + 短配重尾
            let (gx, gy) = polar(cx, cy, hand_r, day_angle(acc_frac));
            let (tx, ty) = polar(cx, cy, -14.0, day_angle(acc_frac));
            ui.painter().line_segment(
                [egui::pos2(tx, ty), egui::pos2(gx, gy)],
                egui::Stroke::new(3.0, accent),
            );
            // 灰针（现实）：细短针
            let (rx, ry) = polar(cx, cy, hand_r * 0.62, day_angle(real_frac));
            let (rtx, rty) = polar(cx, cy, -8.0, day_angle(real_frac));
            ui.painter().line_segment(
                [egui::pos2(rtx, rty), egui::pos2(rx, ry)],
                egui::Stroke::new(1.8, ink.muted),
            );
            // 轴毂
            ui.painter()
                .circle_filled(egui::pos2(cx, cy), 4.0, ink.strong);

            // —— 中心读数（等宽 hero + ×倍率徽标；小号标准时与日期居下）——
            let hero_base = crate::theme::font_size("hero");
            let hero_text = info.custom_time.clone();
            let badge = format!("×{:.1}", state.rate);
            let measure = digits::digit_font(hero_base);
            let hero_w0 = digits::text_width(ui.painter(), &hero_text, &measure);
            let badge_w0 = digits::text_width(ui.painter(), &badge, &measure);
            let gap = crate::theme::spacing("md");
            let text_r = hand_r - 34.0;
            let scale = hero_fit_scale((hero_w0 + gap + badge_w0) * 1.0, text_r * 2.0 * 0.92);
            let hero_size = hero_base * scale;
            let hero_font = digits::digit_font(hero_size);
            let badge_font = digits::digit_font(crate::theme::font_size("time_md") * scale);
            let hero_w = hero_w0 * scale;
            let badge_w = badge_w0 * scale;
            let hero_y = cy - hero_size * 0.35;
            digits::draw_centered(
                ui.painter(),
                egui::pos2(cx - (badge_w + gap) / 2.0, hero_y),
                &hero_text,
                &hero_font,
                ink.strong,
            );
            digits::draw_centered(
                ui.painter(),
                egui::pos2(cx + (hero_w + gap) / 2.0, hero_y + 8.0 * scale),
                &badge,
                &badge_font,
                accent,
            );
            // 标准时（等宽数字位）+ 中文日期
            let std_font = digits::digit_font(crate::theme::font_size("time_sm"));
            digits::draw_centered(
                ui.painter(),
                egui::pos2(cx, cy + hero_size * 0.45 + 10.0),
                time_part(&info.standard_datetime),
                &std_font,
                ink.muted,
            );
            ui.painter().text(
                egui::pos2(cx, cy + hero_size * 0.45 + 32.0),
                egui::Align2::CENTER_CENTER,
                &info.chinese_date,
                egui::FontId::proportional(crate::theme::font_size("meta")),
                ink.meta,
            );
            ui.allocate_space(egui::vec2(0.0, dial_d + 8.0));
        } else {
            ui.allocate_space(egui::vec2(0.0, 120.0));
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
// 输入：UiState（Tick 快照：时间/倍率域）；输出：星盘主仪 + SetRate/ApplyRate
// 逻辑步骤：直径自适应（dial_diameter）→ 24 小时刻度环（96 刻度纯量尺，整点刻
//   长刻亮、四正位基数标签）→ 双针（金长针=加速此刻带配重尾、灰细针=现实此刻、
//   轴毂）→ 中心等宽 hero+×徽标（hero_fit_scale 收号）+ 标准时 + 日期 → 滑杆
//   → 农历 chips
// 设计理由（UIR4.6 用户反馈迭代）：双环被否「内外都一样，没有意义」——形异义同
//   确属实；改钟表本源语法：单环双针，金针快转灰针慢转，速差与角差即膨胀读数
//   （20x 时金针肉眼疯转甩开灰针）；含义唯一原则：刻度=尺、指针=数据，废除
//   「已过段填金」（与指针同义重复）；星盘恒驻 home（住与查分离不变）
// 异常处理：time 为 None（首 Tick 前）星盘整块跳过不崩；day_progress/day_fraction
//   双钳制；polar 纯函数无 panic 面
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
    fn day_fraction_cardinals() {
        // 一日比例锚点：0 点 0；正午 0.5；23:59:59 近 1；异常 0
        assert_eq!(day_fraction("00:00:00"), 0.0);
        assert!((day_fraction("12:00:00") - 0.5).abs() < 1e-6);
        assert!(day_fraction("23:59:59") > 0.999);
        assert_eq!(day_fraction(""), 0.0);
    }

    #[test]
    fn day_angle_and_polar_roundtrip() {
        // 角度钳制 + 极坐标四正位（正上/右/下/左）
        assert_eq!(day_angle(0.25), 90.0);
        assert_eq!(day_angle(1.5), 360.0, "越界钳满");
        let (x, y) = polar(100.0, 100.0, 50.0, 0.0);
        assert!((x - 100.0).abs() < 1e-4 && (y - 50.0).abs() < 1e-4, "正上");
        let (x, y) = polar(100.0, 100.0, 50.0, 90.0);
        assert!((x - 150.0).abs() < 1e-4 && (y - 100.0).abs() < 1e-4, "正右");
        let (x, y) = polar(100.0, 100.0, 50.0, 180.0);
        assert!((x - 100.0).abs() < 1e-4 && (y - 150.0).abs() < 1e-4, "正下");
    }

    #[test]
    fn dial_diameter_fits_and_clamps() {
        // 宽约束 / 高约束 / 上下限钳制
        assert_eq!(dial_diameter(400.0, 600.0), 360.0, "宽 0.9 约束");
        assert_eq!(dial_diameter(600.0, 400.0), 304.0, "高扣 96 约束");
        assert_eq!(dial_diameter(1000.0, 1000.0), 440.0, "上限钳");
        assert_eq!(dial_diameter(150.0, 200.0), 200.0, "下限钳");
    }

    #[test]
    fn hero_fit_scale_fits_without_shrinking_below_floor() {
        // 未超宽 1.0 原样；超宽按比例收；极端窄有 0.55 下限
        assert_eq!(hero_fit_scale(300.0, 340.0), 1.0);
        assert!((hero_fit_scale(400.0, 340.0) - 0.85).abs() < 1e-9);
        assert_eq!(hero_fit_scale(500.0, 200.0), 0.55, "极端窄收至下限");
        assert_eq!(hero_fit_scale(0.0, 340.0), 1.0);
    }
}
