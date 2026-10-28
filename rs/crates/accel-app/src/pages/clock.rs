//! 时钟页（PL017 主仪重构）：星盘居中 + 盘下 readout + 360px 短倍率条 + 农历 meta。
//! 金针 = 加速时刻 24h 时针（custom_time mod 24），灰针 = 现实 24h，金弧 = 两针差；
//! 读数在盘下，指针不再穿过数字。纯函数 day_fraction/accelerated_hand_fraction/
//! dial_diameter/hero_fit_scale 可测。

use crate::tools::clock::split_lunar_info;
use crate::widgets::digits;
use accel_core::types::FrontendCommand;

/// 从标准日期时间串截取时刻段（"YYYY-MM-DD HH:MM:SS" → "HH:MM:SS"；纯函数可测）
pub fn time_part(standard_datetime: &str) -> &str {
    standard_datetime.get(11..19).unwrap_or("")
}

/// 解析 "H:MM:SS" / "HH:MM:SS"（小时可三位数）；任意异常段回 0
fn parse_hms(text: &str) -> (f32, f32, f32) {
    let mut it = text.split(':');
    let h = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let m = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let s = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    (h, m, s)
}

/// "HH:MM:SS" → 一日已过比例（纯函数）：0 点正上起算；异常形态返 0
pub fn day_fraction(time_hms: &str) -> f32 {
    let (h, m, s) = parse_hms(time_hms);
    ((h * 3600.0 + m * 60.0 + s) / 86400.0).clamp(0.0, 1.0)
}

/// 加速时刻 → 24h 时针比例（纯函数）：`custom_time` 的时对 24 取模，含分秒。
/// 这是双针语义的正确性核心：rate=7.2 时金针每现实日转 7.2 圈，与灰针（1 圈）分开。
pub fn accelerated_hand_fraction(custom_time: &str) -> f32 {
    let (h, m, s) = parse_hms(custom_time);
    ((h % 24.0 + m / 60.0 + s / 3600.0) / 24.0).clamp(0.0, 1.0)
}

/// 加速日序号（1 起；custom_time 小时 ≥24 显示“第 2 天”）——纯函数
pub fn custom_day(custom_time: &str) -> i64 {
    (parse_hms(custom_time).0 as i64) / 24 + 1
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

/// 星盘直径（纯函数）：可用高扣除 readout/rate/meta 后与宽 0.62 取小，钳 [180,560]
pub fn dial_diameter(avail_w: f32, avail_h: f32, reserved_h: f32) -> f32 {
    (avail_h - reserved_h)
        .min(avail_w * 0.62)
        .clamp(180.0, 560.0)
}

/// hero 收号（纯函数）：组合宽超可用宽时按比例缩，下限 0.5
pub fn hero_fit_scale(combined_w: f32, avail_w: f32) -> f32 {
    if combined_w <= avail_w || combined_w <= 0.0 {
        1.0
    } else {
        (avail_w / combined_w).clamp(0.5, 1.0)
    }
}

impl crate::pages::Page for crate::pages::ClockPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let Some(info) = state.time.clone() else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("正在读取时间…")
                        .font(crate::theme::font_regular(crate::theme::font_size(
                            "body_lg",
                        )))
                        .color(crate::theme::ink().muted),
                );
            });
            return;
        };
        let ink = crate::theme::ink();
        // PL017：短窗（<380 可用高）才退化为数字版；宽而矮的 900×500 仍保留星盘
        let compact = ui.available_height() < 380.0;

        // 星点背景（低对比、静态；点击穿透）
        draw_stars(ui);

        let accel = accelerated_hand_fraction(&info.custom_time);
        let real = day_fraction(time_part(&info.standard_datetime));

        if compact {
            let content_h = 12.0 + 20.0 + 8.0 + 76.0 + 8.0 + 62.0 + 4.0 + 40.0;
            ui.add_space(((ui.available_height() - content_h) / 2.0).max(8.0));
            ui.label(
                egui::RichText::new("加速世界")
                    .font(crate::theme::font_semibold(crate::theme::font_size("body")))
                    .color(ink.body),
            );
            ui.add_space(8.0);
            draw_readout(ui, &info, state.rate, true);
            ui.add_space(8.0);
            draw_rate(ui, state, bus);
            ui.add_space(4.0);
            draw_meta(ui, &info);
            return;
        }

        let reserved = 82.0 + 62.0 + 40.0 + 24.0;
        let show_header = ui.available_height() >= 560.0;
        let avail_h = ui.available_height() - if show_header { 52.0 } else { 0.0 };
        if show_header {
            draw_stage_header(ui, &info);
        }
        let dial_d = dial_diameter(ui.available_width(), avail_h, reserved);
        let total = dial_d + 8.0 + 82.0 + 8.0 + 62.0 + 8.0 + 40.0;
        let top_pad = ((avail_h - total) / 2.0).max(0.0);
        ui.add_space(top_pad);
        let center_x = ui.max_rect().center().x;
        let center_y = ui.cursor().top() + dial_d / 2.0;
        draw_dial(ui, egui::pos2(center_x, center_y), dial_d, accel, real);
        ui.allocate_space(egui::vec2(0.0, dial_d + 8.0));
        draw_readout(ui, &info, state.rate, false);
        ui.add_space(8.0);
        draw_rate(ui, state, bus);
        ui.add_space(8.0);
        draw_meta(ui, &info);
    }
}

/// 舞台页头（PL017 N5）：产品标/名 + 中文日期；只在可用高 ≥560 时占 52px，
/// 短窗不抢星盘空间（compact 另有 wordmark）。
fn draw_stage_header(ui: &mut egui::Ui, info: &accel_core::types::TimeInfo) {
    let ink = crate::theme::ink();
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), egui::Sense::hover());
    ui.painter().circle_filled(
        egui::pos2(rect.left() + 6.0, rect.center().y - 7.0),
        3.5,
        crate::theme::accent_color(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 18.0, rect.center().y - 12.0),
        egui::Align2::LEFT_CENTER,
        "加速世界",
        crate::theme::font_semibold(14.0),
        ink.strong,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 18.0, rect.center().y + 9.0),
        egui::Align2::LEFT_CENTER,
        "子夜星图",
        crate::theme::font_regular(11.0),
        ink.meta,
    );
    ui.painter().text(
        egui::pos2(rect.right(), rect.center().y - 2.0),
        egui::Align2::RIGHT_CENTER,
        &info.chinese_date,
        crate::theme::font_regular(crate::theme::font_size("meta")),
        ink.muted,
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom()),
            egui::pos2(rect.right(), rect.bottom()),
        ],
        egui::Stroke::new(1.0, crate::theme::border_color()),
    );
}

/// 12 个固定星点（静态、不随机——不引入帧间状态）
fn draw_stars(ui: &egui::Ui) {
    let r = ui.max_rect();
    const STARS: [(f32, f32); 12] = [
        (0.07, 0.18),
        (0.13, 0.72),
        (0.21, 0.36),
        (0.29, 0.86),
        (0.37, 0.12),
        (0.71, 0.14),
        (0.79, 0.62),
        (0.86, 0.31),
        (0.92, 0.78),
        (0.63, 0.88),
        (0.46, 0.06),
        (0.55, 0.94),
    ];
    for (fx, fy) in STARS {
        ui.painter().circle_filled(
            egui::pos2(r.left() + r.width() * fx, r.top() + r.height() * fy),
            1.0,
            crate::theme::star_color(),
        );
    }
}

/// 星盘：96 刻度 + 四正位 + 双针 + 金弧
fn draw_dial(ui: &egui::Ui, center: egui::Pos2, dial_d: f32, accel: f32, real: f32) {
    let ink = crate::theme::ink();
    let accent = crate::theme::accent_color();
    let idle = crate::theme::tick_idle_color();
    let outer_r = dial_d / 2.0 - 18.0;
    for i in 0..96 {
        let a = i as f32 / 96.0;
        let ang = day_angle(a);
        let hour_tick = i % 4 == 0;
        let len = if hour_tick { 12.0 } else { 7.0 };
        let (x1, y1) = polar(center.x, center.y, outer_r, ang);
        let (x2, y2) = polar(center.x, center.y, outer_r - len, ang);
        ui.painter().line_segment(
            [egui::pos2(x1, y1), egui::pos2(x2, y2)],
            egui::Stroke::new(
                if hour_tick { 2.0 } else { 1.2 },
                if hour_tick { ink.muted } else { idle },
            ),
        );
    }
    let cardinal = egui::FontId::proportional(crate::theme::font_size("meta"));
    for (ang, label) in [(0.0, "00"), (90.0, "06"), (180.0, "12"), (270.0, "18")] {
        let (lx, ly) = polar(center.x, center.y, outer_r + 14.0, ang);
        ui.painter().text(
            egui::pos2(lx, ly),
            egui::Align2::CENTER_CENTER,
            label,
            cardinal.clone(),
            ink.meta,
        );
    }

    // 金弧：两针之间的较短弧（膨胀差）；画在刻度环外侧，避免与刻度混读
    let a0 = day_angle(real);
    let a1 = day_angle(accel);
    let mut delta = (a1 - a0).rem_euclid(360.0);
    let start = if delta <= 180.0 { a0 } else { a1 };
    if delta > 180.0 {
        delta = 360.0 - delta;
    }
    if delta > 0.5 && delta < 359.5 {
        let r = outer_r + 6.0;
        let segs = 32;
        let pts: Vec<egui::Pos2> = (0..=segs)
            .map(|i| {
                let ang = start + delta * (i as f32 / segs as f32);
                let (x, y) = polar(center.x, center.y, r, ang);
                egui::pos2(x, y)
            })
            .collect();
        ui.painter().add(egui::Shape::line(
            pts,
            egui::Stroke::new(1.8, crate::theme::with_alpha(accent, 120)),
        ));
    }

    let hand_r = outer_r - 26.0;
    let (gx, gy) = polar(center.x, center.y, hand_r, a1);
    let (tx, ty) = polar(center.x, center.y, -14.0, a1);
    ui.painter().line_segment(
        [egui::pos2(tx, ty), egui::pos2(gx, gy)],
        egui::Stroke::new(3.0, accent),
    );
    let (rx, ry) = polar(center.x, center.y, hand_r * 0.62, a0);
    let (rtx, rty) = polar(center.x, center.y, -8.0, a0);
    ui.painter().line_segment(
        [egui::pos2(rtx, rty), egui::pos2(rx, ry)],
        egui::Stroke::new(1.8, ink.muted),
    );
    ui.painter().circle_filled(center, 4.0, ink.strong);
}

/// 盘下读数：左加速（hero）+ 日序号，右现实 + 倍率；1px 分隔
fn draw_readout(ui: &mut egui::Ui, info: &accel_core::types::TimeInfo, rate: f64, compact: bool) {
    let ink = crate::theme::ink();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), if compact { 76.0 } else { 82.0 }),
        egui::Sense::hover(),
    );
    let half = rect.width() / 2.0;
    let left = egui::Rect::from_min_size(rect.left_top(), egui::vec2(half, rect.height()));
    let right = egui::Rect::from_min_size(
        egui::pos2(rect.left() + half, rect.top()),
        egui::vec2(half, rect.height()),
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + half, rect.top() + 12.0),
            egui::pos2(rect.left() + half, rect.bottom() - 12.0),
        ],
        egui::Stroke::new(1.0, crate::theme::border_color()),
    );

    let label_font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let accent = crate::theme::accent_color();
    ui.painter().text(
        egui::pos2(left.left() + 12.0, left.top() + 4.0),
        egui::Align2::LEFT_TOP,
        "加速",
        label_font.clone(),
        accent,
    );
    let base = if compact { 48.0 } else { 64.0 };
    let measure = digits::digit_font(base);
    let measured = digits::text_width(ui.painter(), &info.custom_time, &measure);
    let scale = hero_fit_scale(measured, half - 28.0);
    let hero_font = digits::digit_font(base * scale);
    digits::draw_centered(
        ui.painter(),
        egui::pos2(left.center().x, left.center().y + 4.0),
        &info.custom_time,
        &hero_font,
        ink.strong,
    );
    let day = custom_day(&info.custom_time);
    if day > 1 {
        ui.painter().text(
            egui::pos2(left.right() - 12.0, left.bottom() - 6.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("第 {day} 天"),
            crate::theme::font_regular(crate::theme::font_size("meta")),
            ink.meta,
        );
    }
    ui.painter().text(
        egui::pos2(right.left() + 12.0, right.top() + 4.0),
        egui::Align2::LEFT_TOP,
        "现实",
        label_font,
        ink.muted,
    );
    let real_font = digits::digit_font(if compact { 20.0 } else { 24.0 });
    digits::draw_centered(
        ui.painter(),
        egui::pos2(right.center().x, right.center().y + 2.0),
        time_part(&info.standard_datetime),
        &real_font,
        ink.body,
    );
    ui.painter().text(
        egui::pos2(right.right() - 12.0, right.bottom() - 6.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("×{rate:.1} 倍率"),
        crate::theme::font_medium(crate::theme::font_size("meta")),
        accent,
    );
}

/// 短倍率条：360px、6px 轨、1×/20× 端标；拖动 SetRate、松手/点击 ApplyRate
fn draw_rate(ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
    let (min, max) = state.rate_bounds;
    let mut rate = state.rate;
    let mut set_rate = None;
    let mut apply_rate = None;
    crate::widgets::rate_strip::show(
        ui,
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
    state.rate = rate;
}

/// 农历/月相/财神：一条 12px meta 行，不再用三个盒子
fn draw_meta(ui: &mut egui::Ui, info: &accel_core::types::TimeInfo) {
    let (lunar, phase, god) = split_lunar_info(&info.lunar_info);
    ui.label(
        egui::RichText::new(format!("农历 {lunar} · 月相 {phase} · 财神 {god}"))
            .font(crate::theme::font_regular(crate::theme::font_size("meta")))
            .color(crate::theme::ink().muted),
    );
}

// ===== src/pages/clock.rs 说明区 =====
// 输入：UiState（Tick 快照/倍率域）；输出：星盘 + readout + 倍率条 + SetRate/ApplyRate
// 逻辑步骤：compact(<760) 数字版 / 完整版：星盘（96 刻度 + 双针 + 金弧 + 中心倍率）
//   → 盘下 readout（加速 64/现实 24/倍率）→ 360px 短倍率条 → 农历 meta
// 设计理由（PL017）：星盘从 440 固定贴顶改为随可用高缩放并垂直居中；数字从盘心
//   移到盘下，指针不再穿字；金针改加速 24h（数学上不再与灰针恒等）；金只给加速
//   数据，倍率条缩到 360/6px，导航与焦点另用中性/亮白
// 异常处理：time 为 None 显示读取中；hand/custom_day 异常回 0；hero_fit 下限 0.5
// 关联依赖：widgets(digits/slider)/tools::clock；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_part_extraction() {
        assert_eq!(time_part("2026-10-23 12:34:56"), "12:34:56");
        assert_eq!(time_part("bad"), "");
    }

    #[test]
    fn day_fraction_cardinals() {
        assert_eq!(day_fraction("00:00:00"), 0.0);
        assert!((day_fraction("12:00:00") - 0.5).abs() < 1e-6);
        assert!(day_fraction("23:59:59") > 0.999);
        assert_eq!(day_fraction(""), 0.0);
    }

    #[test]
    fn accelerated_hand_wraps_24h_and_differs_from_real() {
        // PL017.05 验收：加速 35:30:48 对 24 取模 → 11:30:48；与 04:55:52 角差 >0
        let acc = accelerated_hand_fraction("35:30:48");
        let real = day_fraction("04:55:52");
        assert!((acc - (11.0 + 30.0 / 60.0 + 48.0 / 3600.0) / 24.0).abs() < 1e-6);
        assert!((acc - real).abs() > 0.1, "rate>1 两针必须有角差");
        assert_eq!(accelerated_hand_fraction("24:00:00"), 0.0);
        assert_eq!(accelerated_hand_fraction("bad"), 0.0);
    }

    #[test]
    fn custom_day_counts_24h_wraps() {
        assert_eq!(custom_day("23:59:59"), 1);
        assert_eq!(custom_day("24:00:00"), 2);
        assert_eq!(custom_day("35:30:48"), 2);
        assert_eq!(custom_day("251:38:42"), 11);
    }

    #[test]
    fn day_angle_and_polar_roundtrip() {
        assert_eq!(day_angle(0.25), 90.0);
        assert_eq!(day_angle(1.5), 360.0);
        let (x, y) = polar(100.0, 100.0, 50.0, 0.0);
        assert!((x - 100.0).abs() < 1e-4 && (y - 50.0).abs() < 1e-4);
        let (x, y) = polar(100.0, 100.0, 50.0, 90.0);
        assert!((x - 150.0).abs() < 1e-4 && (y - 100.0).abs() < 1e-4);
    }

    #[test]
    fn dial_diameter_reserves_readout_and_rate() {
        assert_eq!(dial_diameter(400.0, 600.0, 178.0), 248.0);
        assert_eq!(dial_diameter(1000.0, 1000.0, 178.0), 560.0);
        assert_eq!(dial_diameter(150.0, 200.0, 178.0), 180.0);
    }

    #[test]
    fn hero_fit_shrinks_with_floor() {
        assert_eq!(hero_fit_scale(300.0, 340.0), 1.0);
        assert!((hero_fit_scale(400.0, 340.0) - 0.85).abs() < 1e-6);
        assert_eq!(hero_fit_scale(500.0, 100.0), 0.5);
    }
}
