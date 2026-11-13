//! 时钟页（PL020 12h 双半日）：星盘居中 + 盘下 readout + 短倍率条 + 农历 meta。
//! 三针走现实时间（时/分/秒），黄针走加速时刻 12h；AM/PM 胶囊跟随现实半日；
//! 金弧 = 黄针与时针的较短弧。纯函数 hour_fraction/minute_fraction/second_fraction/
//! accelerated_hand_fraction/is_pm/dial_diameter/hero_fit_scale 可测。

use crate::tools::clock::split_lunar_info;
use crate::widgets::digits;
use crate::widgets::text_ink::{self, InkKind};
use accel_core::types::FrontendCommand;
use chrono::Datelike;

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

/// 现实时刻 → 12h 时针比例（纯函数）：时对 12 取模，含分秒；异常形态返 0
pub fn hour_fraction(time_hms: &str) -> f32 {
    let (h, m, s) = parse_hms(time_hms);
    ((h % 12.0 + m / 60.0 + s / 3600.0) / 12.0).clamp(0.0, 1.0)
}

/// 现实时刻 → 分针比例（纯函数）：分 + 秒/60，0..1
pub fn minute_fraction(time_hms: &str) -> f32 {
    let (_, m, s) = parse_hms(time_hms);
    ((m + s / 60.0) / 60.0).clamp(0.0, 1.0)
}

/// 现实时刻 → 秒针比例（纯函数）：秒/60，0..1
pub fn second_fraction(time_hms: &str) -> f32 {
    (parse_hms(time_hms).2 / 60.0).clamp(0.0, 1.0)
}

/// 加速时刻 → 12h 黄针比例（纯函数）：时对 12 取模，含分秒。
/// 这是「加速」语义的核心：rate=7.2 时黄针每现实日转 14.4 圈，与时针分开。
pub fn accelerated_hand_fraction(custom_time: &str) -> f32 {
    let (h, m, s) = parse_hms(custom_time);
    ((h % 12.0 + m / 60.0 + s / 3600.0) / 12.0).clamp(0.0, 1.0)
}

/// 加速时刻 → 当日进度比例（纯函数）：时对 24 取模，0:00=0、24:00=1。
/// 金圈厚度与色谱都按这个比例走，与倍率无关。
pub fn day_progress(custom_time: &str) -> f32 {
    let (h, m, s) = parse_hms(custom_time);
    ((h % 24.0 + m / 60.0 + s / 3600.0) / 24.0).clamp(0.0, 1.0)
}

/// 现实时刻 → 是否 PM（纯函数）：12:00 起算下午；异常形态按 AM
pub fn is_pm(time_hms: &str) -> bool {
    parse_hms(time_hms).0 >= 12.0
}

/// 加速日序号（1 起；custom_time 小时 ≥24 显示“第 2 天”）——纯函数
pub fn custom_day(custom_time: &str) -> i64 {
    (parse_hms(custom_time).0 as i64) / 24 + 1
}

/// 加速时刻 → 当天 HH:MM:SS（纯函数）：小时对 24 取模，分秒原样；35:30:48 → 11:30:48
pub fn day_time(custom_time: &str) -> String {
    let (h, m, s) = parse_hms(custom_time);
    format!("{:02}:{:02}:{:02}", (h % 24.0) as i64, m as i64, s as i64)
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

/// 盒子间距（B1+B2 → B3 → B11 → B18 → C2 统一 20px）
const BOX_GAP: f32 = 20.0;
/// 页头内容墨迹底到分隔线的固定间距
const HEADER_RULE_GAP: f32 = 10.0;
/// B3 盒每侧固定占位：半日胶囊中心距盘心 = 半盘 + 38，胶囊外缘再 +26
const DIAL_SIDE_W: f32 = 64.0;

/// 星盘直径（纯函数）：B3 盒高与盒宽（扣两侧胶囊）取小，钳 [120,560]——小窗也保留星盘
pub fn dial_diameter(avail_w: f32, avail_h: f32) -> f32 {
    avail_h.min(avail_w - DIAL_SIDE_W * 2.0).clamp(120.0, 560.0)
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
        // 页面区高度以 Ui 的 max_rect 为准（与 window 的 page_rect 一致）
        let page_rect = ui.max_rect();
        let page_h = {
            let viewport_h = ui.ctx().input(|i| {
                i.viewport()
                    .inner_rect
                    .map_or(f32::INFINITY, |r| r.height())
            });
            page_rect.height().min(viewport_h)
        };
        let page_w = page_rect.width();
        let left = page_rect.left();
        let m = clock_metrics(page_h);
        let header = header_layout(ui, &info, page_w);
        let header_h = header.height();
        // 页面 = 盒子垂直堆叠：页头(B1+B2) / B3 星盘 / B11 读数 / B18 倍率条（C2 在页面外）
        let fixed = header_h + m.readout_h + m.rate_h + BOX_GAP * 3.0;
        let b3_h = page_h - fixed;

        // 星点背景（低对比、静态；点击穿透）
        draw_stars(ui);

        let real_time = time_part(&info.standard_datetime);
        let hour = hour_fraction(real_time);
        let minute = minute_fraction(real_time);
        let second = second_fraction(real_time);
        let accel = accelerated_hand_fraction(&info.custom_time);
        let day_t = day_progress(&info.custom_time);
        let pm = is_pm(real_time);

        // 盒子堆叠器：显式 y 游标，盒子只在自己的 rect 内绘制
        let box_rect = |y: f32, h: f32| {
            egui::Rect::from_min_size(egui::pos2(left, y), egui::vec2(page_w, h))
        };

        let mut y = page_rect.top();
        draw_stage_header(ui, &info, &header, box_rect(y, header_h));
        y += header_h + BOX_GAP;
        // B3 吃剩余高度；盘径在盒内按宽高取小（AM/PM 尺寸与位置固定）
        let b3 = box_rect(y, b3_h.max(0.0));
        draw_dial(
            ui,
            b3.center(),
            dial_diameter(b3.width(), b3.height()),
            hour,
            minute,
            second,
            accel,
            day_t,
            pm,
        );
        y += b3_h + BOX_GAP;
        draw_readout(ui, &info, m, box_rect(y, m.readout_h));
        y += m.readout_h + BOX_GAP;
        let b18 = box_rect(y, m.rate_h);
        ui.scope_builder(egui::UiBuilder::new().max_rect(b18), |b18_ui| {
            draw_rate(b18_ui, state, bus, m.rate_h);
        });
    }
}

/// 页头盒子布局（盒子模型）：B1 日期 / B2 农历 各自墨迹 bbox，合并高 = max
struct HeaderLayout {
    label_size: f32,
    meta_font: egui::FontId,
    b1: text_ink::Ink,
    meta: text_ink::Ink,
    b2_h: f32,
    line_gap: f32,
    two_line: bool,
    b1_w: f32,
    line1: String,
}

impl HeaderLayout {
    /// 内容墨迹区高（不含到分隔线的固定间距）
    fn content_h(&self) -> f32 {
        self.b1.h.max(self.b2_h)
    }

    /// 页头盒高 = 内容墨迹区 + 到分隔线的 20px
    fn height(&self) -> f32 {
        self.content_h() + HEADER_RULE_GAP
    }
}

/// 量页头两个子盒（B1 日期 / B2 农历）的墨迹 bbox 与宽度跌落档位
fn header_layout(ui: &egui::Ui, info: &accel_core::types::TimeInfo, page_w: f32) -> HeaderLayout {
    let label_size = 14.0;
    let label_font = crate::theme::font_semibold(label_size);
    let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let b1 = text_ink::measure(ui, &info.chinese_date, &label_font, InkKind::Cjk);
    let b1_w = ui
        .painter()
        .layout_no_wrap(
            info.chinese_date.clone(),
            label_font.clone(),
            crate::theme::ink().strong,
        )
        .rect
        .width();
    let (lunar, phase, god) = split_lunar_info(&info.lunar_info);
    // 「丙午年（马年）八月十三午时」→ 去掉末尾时辰，保留年/生肖/月/日
    let lunar_chars: Vec<char> = lunar.chars().collect();
    let lunar_short: String = lunar_chars
        .iter()
        .take(lunar_chars.len().saturating_sub(2))
        .collect();
    let god_short = god
        .split('（')
        .next()
        .unwrap_or(&god)
        .trim_end_matches("方向");
    let viewport_h = ui
        .ctx()
        .input(|i| i.viewport().inner_rect.map_or(f32::INFINITY, |r| r.height()));
    // 行数按高度两级（与 C2 同界 560）；文本长度按页头可用宽两级
    let two_line = viewport_h >= 560.0;
    let full_text = page_w >= 560.0;
    let line1 = if full_text {
        format!("农历 {lunar_short} · 月相 {phase} · 财神 {god_short}")
    } else {
        format!("农历 {lunar_short}")
    };
    let meta = text_ink::measure(ui, &line1, &meta_font, InkKind::Cjk);
    let line_gap = 2.0;
    let b2_h = meta.h
        + if two_line {
            line_gap + meta.h
        } else {
            0.0
        };
    HeaderLayout {
        label_size,
        meta_font,
        b1,
        meta,
        b2_h,
        line_gap,
        two_line,
        b1_w,
        line1,
    }
}

/// 舞台页头盒：B1 日期（左）/ B2 农历（右）各按墨迹 bbox 定位，分隔线贴盒底
fn draw_stage_header(
    ui: &mut egui::Ui,
    info: &accel_core::types::TimeInfo,
    layout: &HeaderLayout,
    rect: egui::Rect,
) {
    let ink = crate::theme::ink();
    let top = rect.top();
    let content_h = layout.content_h();
    let b1_top = top + (content_h - layout.b1.h) / 2.0;
    ui.painter().circle_filled(
        egui::pos2(rect.left() + 6.0, b1_top + layout.b1.h / 2.0),
        3.5,
        crate::theme::accent_color(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 18.0, b1_top - layout.b1.top_pad),
        egui::Align2::LEFT_TOP,
        &info.chinese_date,
        crate::theme::font_semibold(layout.label_size),
        ink.strong,
    );
    // 日期 hover：周数 / 节日 / 节气
    let date_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.top()),
        egui::vec2(18.0 + layout.b1_w, rect.height()),
    );
    let date_resp = ui.interact(
        date_rect,
        egui::Id::unique("stage_date_tip"),
        egui::Sense::hover(),
    );
    if date_resp.hovered() {
        draw_date_tooltip(
            ui,
            rect.left() + 6.0,
            rect.bottom() + 6.0,
            info,
            &layout.meta_font,
        );
    }

    let right = rect.right() - 6.0;
    let b2_top = top + (content_h - layout.b2_h) / 2.0;
    if layout.two_line {
        let line2_top = b2_top + layout.meta.h + layout.line_gap;
        ui.painter().text(
            egui::pos2(right, b2_top - layout.meta.top_pad),
            egui::Align2::RIGHT_TOP,
            &layout.line1,
            layout.meta_font.clone(),
            ink.muted,
        );
        let line2_center = line2_top + layout.meta.h / 2.0;
        draw_yi_ji(
            ui,
            egui::pos2(right, line2_center - layout.meta.center_offset),
            &info.lunar_yi,
            &info.lunar_ji,
            &layout.meta_font,
        );
    } else {
        ui.painter().text(
            egui::pos2(right, b2_top - layout.meta.top_pad),
            egui::Align2::RIGHT_TOP,
            &layout.line1,
            layout.meta_font.clone(),
            ink.muted,
        );
    }
    // 农历 hover：无论两行/一行，都创建宜忌 tooltip
    let tip_rect = egui::Rect::from_min_max(
        egui::pos2(rect.center().x, rect.top()),
        egui::pos2(right, rect.bottom()),
    );
    let resp = ui.interact(
        tip_rect,
        egui::Id::unique("stage_lunar_tip"),
        egui::Sense::hover(),
    );
    if resp.hovered() && (!info.lunar_yi.is_empty() || !info.lunar_ji.is_empty()) {
        draw_lunar_tooltip(ui, rect.right(), rect.bottom() + 6.0, info, &layout.meta_font);
    }
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom()),
            egui::pos2(rect.right(), rect.bottom()),
        ],
        egui::Stroke::new(1.0, crate::theme::border_color()),
    );
}

/// 页头日期 tooltip：今年第 N 周 + 今日节日 / 节气（有则显示）
fn draw_date_tooltip(
    ui: &egui::Ui,
    left: f32,
    top: f32,
    info: &accel_core::types::TimeInfo,
    font: &egui::FontId,
) {
    let ink = crate::theme::ink();
    let date = info.standard_datetime.get(..10).unwrap_or("");
    let week = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|d| d.iso_week().week())
        .unwrap_or(0);
    let mut text = format!("第 {week} 周");
    if !info.lunar_festival.is_empty() {
        text.push_str(" · ");
        text.push_str(&info.lunar_festival);
    }
    if !info.lunar_jieqi.is_empty() {
        text.push_str(" · ");
        text.push_str(&info.lunar_jieqi);
    }
    let pad_x = 8.0;
    let pad_y = 6.0;
    let line_h = font.size * 1.5;
    let w = ui
        .painter()
        .layout_no_wrap(text.clone(), font.clone(), ink.muted)
        .rect
        .width()
        + pad_x * 2.0;
    let h = pad_y * 2.0 + line_h;
    let ctx = ui.ctx().clone();
    egui::Area::new(egui::Id::unique("stage_date_tip_card"))
        .order(egui::Order::Tooltip)
        .fixed_pos(egui::pos2(left, top))
        .show(&ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
            let p = ui.painter();
            p.rect_filled(
                rect,
                egui::epaint::CornerRadius::same(8),
                crate::theme::card_bg(),
            );
            p.rect_stroke(
                rect,
                egui::epaint::CornerRadius::same(8),
                egui::Stroke::new(1.0, crate::theme::border_strong_color()),
                egui::StrokeKind::Inside,
            );
            p.text(
                egui::pos2(rect.left() + pad_x, rect.top() + pad_y),
                egui::Align2::LEFT_TOP,
                &text,
                font.clone(),
                ink.muted,
            );
        });
}

/// 页头农历 tooltip（一行模式）：右缘贴页头 10px、顶在页头下方 6px，展示宜忌
fn draw_lunar_tooltip(
    ui: &egui::Ui,
    right: f32,
    top: f32,
    info: &accel_core::types::TimeInfo,
    font: &egui::FontId,
) {
    let ink = crate::theme::ink();
    let success = ink.success;
    let danger = ink.danger;
    let measure = |text: &str, color| {
        ui.painter()
            .layout_no_wrap(text.to_string(), font.clone(), color)
            .rect
            .width()
    };
    let w_yi_label = measure("宜 ", success);
    let w_ji_label = measure("忌 ", danger);
    // 每 5 条一行；高度按行数自适应
    let yi_lines: Vec<String> = info
        .lunar_yi
        .chunks(5)
        .map(|c| c.join(" · "))
        .collect();
    let ji_lines: Vec<String> = info
        .lunar_ji
        .chunks(5)
        .map(|c| c.join(" · "))
        .collect();
    let label_w = w_yi_label.max(w_ji_label);
    let max_line_w = yi_lines
        .iter()
        .chain(ji_lines.iter())
        .map(|l| measure(l, ink.muted))
        .fold(0.0f32, f32::max);
    let pad_x = 8.0;
    let pad_y = 6.0;
    let line_h = font.size * 1.5;
    let w = label_w + max_line_w + pad_x * 2.0;
    let rows = yi_lines.len() + ji_lines.len();
    let h = pad_y * 2.0 + rows as f32 * line_h;
    let x = (right - 10.0 - w).max(0.0);
    let ctx = ui.ctx().clone();
    egui::Area::new(egui::Id::unique("stage_lunar_tip_card"))
        .order(egui::Order::Tooltip)
        .fixed_pos(egui::pos2(x, top))
        .show(&ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
            let p = ui.painter();
            p.rect_filled(
                rect,
                egui::epaint::CornerRadius::same(8),
                crate::theme::card_bg(),
            );
            p.rect_stroke(
                rect,
                egui::epaint::CornerRadius::same(8),
                egui::Stroke::new(1.0, crate::theme::border_strong_color()),
                egui::StrokeKind::Inside,
            );
            let mut y = rect.top() + pad_y;
            let text_x = rect.left() + pad_x + label_w;
            for (i, line) in yi_lines.iter().enumerate() {
                if i == 0 {
                    p.text(
                        egui::pos2(rect.left() + pad_x, y),
                        egui::Align2::LEFT_TOP,
                        "宜 ",
                        font.clone(),
                        success,
                    );
                }
                p.text(
                    egui::pos2(text_x, y),
                    egui::Align2::LEFT_TOP,
                    line,
                    font.clone(),
                    ink.muted,
                );
                y += line_h;
            }
            for (i, line) in ji_lines.iter().enumerate() {
                if i == 0 {
                    p.text(
                        egui::pos2(rect.left() + pad_x, y),
                        egui::Align2::LEFT_TOP,
                        "忌 ",
                        font.clone(),
                        danger,
                    );
                }
                p.text(
                    egui::pos2(text_x, y),
                    egui::Align2::LEFT_TOP,
                    line,
                    font.clone(),
                    ink.muted,
                );
                y += line_h;
            }
        });
}

/// 页头宜忌行（右对齐）：宜/忌前缀语义色，各取前 3 条，超出加「等」
fn draw_yi_ji(
    ui: &egui::Ui,
    right_center: egui::Pos2,
    yi: &[String],
    ji: &[String],
    font: &egui::FontId,
) {
    if yi.is_empty() && ji.is_empty() {
        return;
    }
    let ink = crate::theme::ink();
    let short = |items: &[String]| -> String {
        let take = items
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        if items.len() > 3 {
            format!("{take} 等")
        } else {
            take
        }
    };
    let yi_text = short(yi);
    let ji_text = short(ji);
    let measure = |text: &str, color| {
        ui.painter()
            .layout_no_wrap(text.to_string(), font.clone(), color)
            .rect
            .width()
    };
    let success = ink.success;
    let danger = ink.danger;
    let w_yi = measure("宜 ", success);
    let w_yi_text = measure(&yi_text, ink.muted);
    let w_ji = measure("忌 ", danger);
    let w_ji_text = measure(&ji_text, ink.muted);
    let gap = 16.0;
    let total = w_yi + w_yi_text + gap + w_ji + w_ji_text;
    let mut x = right_center.x - total;
    let y = right_center.y;
    if !yi_text.is_empty() {
        ui.painter().text(
            egui::pos2(x, y),
            egui::Align2::LEFT_CENTER,
            "宜 ",
            font.clone(),
            success,
        );
        x += w_yi;
        ui.painter().text(
            egui::pos2(x, y),
            egui::Align2::LEFT_CENTER,
            &yi_text,
            font.clone(),
            ink.muted,
        );
        x += w_yi_text;
    }
    x += gap;
    if !ji_text.is_empty() {
        ui.painter().text(
            egui::pos2(x, y),
            egui::Align2::LEFT_CENTER,
            "忌 ",
            font.clone(),
            danger,
        );
        x += w_ji;
        ui.painter().text(
            egui::pos2(x, y),
            egui::Align2::LEFT_CENTER,
            &ji_text,
            font.clone(),
            ink.muted,
        );
    }
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

/// 星盘（PL020）：60 刻度 + 12/3/6/9 内侧 + 四针 + 金圈 + AM/PM
#[allow(clippy::too_many_arguments)]
fn draw_dial(
    ui: &egui::Ui,
    center: egui::Pos2,
    dial_d: f32,
    hour: f32,
    minute: f32,
    second: f32,
    accel: f32,
    progress: f32,
    pm: bool,
) {
    let ink = crate::theme::ink();
    let accent = crate::theme::accent_color();
    let lume = crate::theme::lume_color();
    let idle = crate::theme::tick_idle_color();
    let outer_r = dial_d / 2.0 - 18.0;
    // 12h 刻度：60 小格；只有 12 个整点长刻度夜光绿 + GPU 光晕，短刻度保持原色
    let mut ticks = Vec::with_capacity(12);
    for i in 0..60 {
        let a = i as f32 / 60.0;
        let ang = day_angle(a);
        let hour_tick = i % 5 == 0;
        let len = if hour_tick { 14.0 } else { 6.0 };
        let color = if hour_tick { lume } else { idle };
        let width = if hour_tick { 2.0 } else { 1.0 };
        let (x1, y1) = polar(center.x, center.y, outer_r, ang);
        let (x2, y2) = polar(center.x, center.y, outer_r - len, ang);
        if hour_tick {
            ticks.push((egui::pos2(x1, y1), egui::pos2(x2, y2), width * 0.5, color));
        } else {
            ui.painter().line_segment(
                [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                egui::Stroke::new(width, color),
            );
        }
    }
    // 先铺 GPU 圆头光晕（底层），再画亮绿实线覆盖中心，避免光晕把实线压黑
    crate::widgets::ring_glow::paint_ticks(ui, center, outer_r + 14.0, &ticks);
    for &(p1, p2, half, color) in &ticks {
        ui.painter()
            .line_segment([p1, p2], egui::Stroke::new(half * 2.0, color));
        // 端点补同色圆点，盖住 CPU 线段的抗锯齿暗像素
        ui.painter().circle_filled(p1, 0.6, color);
        ui.painter().circle_filled(p2, 0.6, color);
    }
    // 四正位 18px 夜光绿；数字底光走 GPU 径向光斑（小半径，避免圆斑）
    let num_size = crate::theme::font_size("dial_num");
    let cardinal = egui::FontId::proportional(num_size);
    for (ang, label) in [(0.0, "12"), (90.0, "3"), (180.0, "6"), (270.0, "9")] {
        let (lx, ly) = polar(center.x, center.y, outer_r - 34.0, ang);
        let pos = egui::pos2(lx, ly);
        crate::widgets::ring_glow::paint_dot(
            ui,
            pos,
            0.0,
            crate::theme::with_alpha(lume, 110),
            12.0,
        );
        ui.painter().text(
            pos,
            egui::Align2::CENTER_CENTER,
            label,
            cardinal.clone(),
            lume,
        );
    }

    // 金圈：加速日进度环（厚度 0–10px、色谱色），0 点不画；贴刻度环外侧
    draw_ring(ui, center, outer_r, progress);

    let a1 = day_angle(accel);
    let hand_r = outer_r - 26.0;
    // 时针：米白粗短
    let ha = day_angle(hour);
    let (hx, hy) = polar(center.x, center.y, hand_r * 0.48, ha);
    let (htx, hty) = polar(center.x, center.y, -12.0, ha);
    ui.painter().line_segment(
        [egui::pos2(htx, hty), egui::pos2(hx, hy)],
        egui::Stroke::new(5.0, ink.strong),
    );
    // 分针：星紫
    let ma = day_angle(minute);
    let (mx, my) = polar(center.x, center.y, hand_r * 0.72, ma);
    let (mtx, mty) = polar(center.x, center.y, -10.0, ma);
    ui.painter().line_segment(
        [egui::pos2(mtx, mty), egui::pos2(mx, my)],
        egui::Stroke::new(3.0, crate::theme::hand_minute_color()),
    );
    // 秒针：朱红细长，带尾与端圆点
    let sa = day_angle(second);
    let (sx, sy) = polar(center.x, center.y, hand_r * 0.84, sa);
    let (stx, sty) = polar(center.x, center.y, -16.0, sa);
    let sec_color = crate::theme::hand_second_color();
    ui.painter().line_segment(
        [egui::pos2(stx, sty), egui::pos2(sx, sy)],
        egui::Stroke::new(1.2, sec_color),
    );
    ui.painter()
        .circle_filled(egui::pos2(sx, sy), 1.6, sec_color);
    // 黄针：加速时刻，略短于秒针、比时针/分针粗
    let (gx, gy) = polar(center.x, center.y, hand_r * 0.80, a1);
    let (tx, ty) = polar(center.x, center.y, -14.0, a1);
    ui.painter().line_segment(
        [egui::pos2(tx, ty), egui::pos2(gx, gy)],
        egui::Stroke::new(3.2, accent),
    );
    ui.painter().circle_filled(center, 4.0, ink.strong);
    draw_half_pills(ui, center, outer_r, pm);
}

/// 金圈（PL020）：加速日进度 → 实环 0–7px；外发光由 GPU shader 画（ring_glow）
fn draw_ring(ui: &egui::Ui, center: egui::Pos2, rim_r: f32, progress: f32) {
    let t = progress.clamp(0.0, 1.0);
    if t < 0.01 {
        return;
    }
    let thickness = (t * 7.0).max(0.8);
    let r = rim_r + 1.0 + thickness / 2.0;
    let color = spectrum_color(&crate::theme::ring_spectrum(), t);
    ui.painter()
        .circle_stroke(center, r, egui::Stroke::new(thickness, color));
    crate::widgets::ring_glow::paint(ui, center, r, thickness, color, 7.0);
}

/// 色谱插值（纯函数）：5 锚点等距映射，t=0 首色、t=1 尾色
fn spectrum_color(anchors: &[egui::Color32; 5], t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let pos = t * 4.0;
    let seg = (pos.floor() as usize).min(4);
    if seg >= 4 {
        return anchors[4];
    }
    mix_color(anchors[seg], anchors[seg + 1], pos - seg as f32)
}

/// 半日胶囊（PL020）：盘左右两侧与 3/9 点中线对齐，边缘距刻度环 20px；AM 橙红 / PM 淡蓝
fn draw_half_pills(ui: &egui::Ui, center: egui::Pos2, rim_r: f32, pm: bool) {
    let ctx = ui.ctx();
    let am_t = crate::widgets::motion::anim(
        ctx,
        egui::Id::unique("half_am"),
        if pm { 0.0 } else { 1.0 },
        160.0,
    );
    let pm_t = 1.0 - am_t;
    let bg = crate::theme::window_bg();
    let meta = crate::theme::ink().meta;
    let size = egui::vec2(52.0, 22.0);
    let radius = egui::epaint::CornerRadius::same(11);
    let dx = rim_r + 30.0 + size.x / 2.0;
    let y = center.y;
    for (offset, label, t, color) in [
        (-dx, "AM", am_t, crate::theme::half_am_color()),
        (dx, "PM", pm_t, crate::theme::half_pm_color()),
    ] {
        let rect = egui::Rect::from_center_size(egui::pos2(center.x + offset, y), size);
        if t > 0.01 {
            crate::widgets::ring_glow::paint_pill(
                ui,
                rect.center(),
                size * 0.5,
                11.0,
                crate::theme::with_alpha(color, (255.0 * t) as u8),
                6.0,
            );
        }
        let fill = mix_color(bg, color, t);
        let text = mix_color(meta, bg, t);
        ui.painter().rect_filled(rect, radius, fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            crate::theme::font_semibold(11.0),
            text,
        );
    }
}

/// 两色线性插值（纯函数）：t=0 取 a，t=1 取 b
fn mix_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    egui::Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

/// 高度断点（内部尺寸）：读数 hero / 现实 / 读数总高 / 倍率条
#[derive(Clone, Copy)]
struct ClockMetrics {
    hero: f32,
    real: f32,
    readout_h: f32,
    rate_h: f32,
}

/// 按页面区高度取盒内尺寸断点；盒子高度由内容 bbox 与剩余空间决定
fn clock_metrics(page_h: f32) -> ClockMetrics {
    let (hero, real, rate_h) = if page_h >= 720.0 {
        (56.0, 36.0, 66.0)
    } else if page_h >= 600.0 {
        (48.0, 32.0, 58.0)
    } else if page_h >= 520.0 {
        (44.0, 30.0, 50.0)
    } else if page_h >= 440.0 {
        (40.0, 28.0, 44.0)
    } else {
        (36.0, 26.0, 40.0)
    };
    ClockMetrics {
        hero,
        real,
        // B11 盒 = 标签行 12 + 行距 15 + 时间墨迹高（实测 0.62×hero）；盒底即时间字形底
        readout_h: 27.0 + hero * 0.62,
        rate_h,
    }
}

/// 盘下读数（PL020）：B11 盒子铺满页面宽；竖线居中，左区右对齐、右区左对齐；
/// 左列 `加速` 标签 + 「第 N 天 + 当天时间」同行，右列 `现实` 标签 + 36px 时间。
fn draw_readout(
    ui: &mut egui::Ui,
    info: &accel_core::types::TimeInfo,
    m: ClockMetrics,
    rect: egui::Rect,
) {
    let ink = crate::theme::ink();
    let area = rect;
    let center_x = area.center().x;

    let label_font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let accent = crate::theme::accent_color();

    let base = m.hero;
    let day = custom_day(&info.custom_time);
    let day_text = day_time(&info.custom_time);
    let day_font = crate::theme::font_semibold(18.0);
    let day_num_font = digits::digit_font(18.0);
    let measure = |text: &str, font: &egui::FontId| {
        ui.painter()
            .layout_no_wrap(text.to_string(), font.clone(), ink.strong)
            .rect
            .width()
    };
    let w_di = measure("第", &day_font);
    // 固定三位：个位补零，宽度恒为 3 槽（数字变化不改「第 NNN 天」整体宽）
    let day_num_text = format!("{day:03}");
    let w_num = digits::text_width(ui.painter(), "000", &day_num_font);
    let w_tian = measure("天", &day_font);
    let day_gap = 2.0;
    let day_w = w_di + day_gap + w_num + day_gap + w_tian;

    // 墨迹 bbox：盒顶 = 标签墨迹顶，盒底 = 时间字形底
    let label_ink = text_ink::measure(ui, "加速", &label_font, InkKind::Cjk);
    let cn_ink = text_ink::measure(ui, "天", &day_font, InkKind::Cjk);
    let num_ink = text_ink::measure(ui, &day_num_text, &day_num_font, InkKind::Digits);
    let digit_bottom = area.bottom();

    // 左区右对齐到 center-20；右区左对齐到 center+20
    let time_measure = digits::digit_font(base);
    let measured = digits::text_width(ui.painter(), &day_text, &time_measure);
    let left_avail = (center_x - 20.0 - area.left() - day_w - 12.0).max(40.0);
    let scale = hero_fit_scale(measured, left_avail);
    let hero_font = digits::digit_font(base * scale);
    let hero_w = measured * scale;
    let hero_ink = text_ink::measure(ui, "0", &hero_font, InkKind::Digits);
    let hero_y = area.bottom() - hero_ink.h / 2.0 - hero_ink.center_offset;
    let left_time_x = center_x - 20.0 - hero_w;
    let day_left = left_time_x - 12.0 - day_w;

    let label_y = area.top() - label_ink.top_pad;
    ui.painter().text(
        egui::pos2(day_left, label_y),
        egui::Align2::LEFT_TOP,
        "加速",
        label_font.clone(),
        accent,
    );
    ui.painter().text(
        egui::pos2(center_x + 20.0, label_y),
        egui::Align2::LEFT_TOP,
        "现实",
        label_font,
        ink.muted,
    );

    digits::draw_centered(
        ui.painter(),
        egui::pos2(left_time_x + hero_w / 2.0, hero_y),
        &day_text,
        &hero_font,
        ink.strong,
    );

    let mut x = day_left;
    // 「第 N 天」墨迹底 = 盒底；块内「第 / N / 天」三者墨迹中心对齐
    let cn_center = area.bottom() - cn_ink.h / 2.0;
    let cn_cy = cn_center - cn_ink.center_offset;
    let num_cy = cn_center - num_ink.center_offset;
    ui.painter().text(
        egui::pos2(x, cn_cy),
        egui::Align2::LEFT_CENTER,
        "第",
        day_font.clone(),
        accent,
    );
    x += w_di + day_gap;
    digits::draw_centered(
        ui.painter(),
        egui::pos2(x + w_num / 2.0, num_cy),
        &day_num_text,
        &day_num_font,
        ink.strong,
    );
    x += w_num + day_gap;
    ui.painter().text(
        egui::pos2(x, cn_cy),
        egui::Align2::LEFT_CENTER,
        "天",
        day_font,
        accent,
    );

    let real_text = time_part(&info.standard_datetime);
    let real_measure = digits::digit_font(m.real);
    let real_w = digits::text_width(ui.painter(), real_text, &real_measure);
    let real_label_x = center_x + 20.0;
    let real_time_x = real_label_x + day_w + 12.0;
    let real_avail = (area.right() - real_time_x).max(40.0);
    let real_scale = hero_fit_scale(real_w, real_avail);
    let real_font = digits::digit_font(m.real * real_scale);
    let real_ink = text_ink::measure(ui, "0", &real_font, InkKind::Digits);
    let real_y = area.bottom() - real_ink.h / 2.0 - real_ink.center_offset;
    digits::draw_centered(
        ui.painter(),
        egui::pos2(real_time_x + real_w * real_scale / 2.0, real_y),
        real_text,
        &real_font,
        ink.body,
    );
    ui.painter().line_segment(
        [
            egui::pos2(center_x, area.top()),
            egui::pos2(center_x, digit_bottom),
        ],
        egui::Stroke::new(1.0, crate::theme::border_color()),
    );
}

/// 短倍率条：宽 220–360、6px 轨、1×/365× 端标；高度按断点缩放
fn draw_rate(
    ui: &mut egui::Ui,
    state: &mut crate::bus::UiState,
    bus: &crate::bus::Bus,
    rate_h: f32,
) {
    let (min, max) = state.rate_bounds;
    let mut rate = state.rate;
    let mut set_rate = None;
    let mut apply_rate = None;
    crate::widgets::rate_strip::show(
        ui,
        min,
        max,
        &mut rate,
        rate_h,
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

// ===== src/pages/clock.rs 说明区 =====
// 输入：UiState（Tick 快照/倍率域）；输出：星盘 + readout + 倍率条 + SetRate/ApplyRate
// 逻辑步骤：compact(<380 高) 数字版 / 完整版：12h 星盘（60 刻度 + 四针 + 金圈 +
//   AM/PM 胶囊）→ 盘下 readout（第 N 天 + 当天时间 / 现实）→ 整数倍率条
// 设计理由（PL020）：三针走现实时间（稳定可读），黄针走加速 12h（延续金=加速）；
//   AM/PM 跟随现实半日，补足 12h 盘不辨上午下午；金圈按加速日进度从 0 长到 10px、
//   颜色走 5 锚点色谱，0 点不可见、24 点最厚；倍率收敛为 1–365 整数，量化行翻译
//   「现实 1 天 = 加速 N 天」
// 异常处理：time 为 None 显示读取中；各 hand/parse 异常回 0；hero_fit 下限 0.5
// 关联依赖：widgets(digits/motion/rate_strip)/tools::clock/theme 针色与半日色；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_part_extraction() {
        assert_eq!(time_part("2026-10-23 12:34:56"), "12:34:56");
        assert_eq!(time_part("bad"), "");
    }

    #[test]
    fn twelve_hour_hand_fractions() {
        assert_eq!(hour_fraction("00:00:00"), 0.0);
        assert_eq!(hour_fraction("12:00:00"), 0.0);
        assert!((hour_fraction("06:00:00") - 0.5).abs() < 1e-6);
        assert!((hour_fraction("18:30:00") - 0.541_666_7).abs() < 1e-5);
        assert_eq!(hour_fraction(""), 0.0);
        assert!((minute_fraction("00:30:00") - 0.5).abs() < 1e-6);
        assert!((minute_fraction("59:59:59") - 0.999_72).abs() < 1e-4);
        assert!((second_fraction("00:00:30") - 0.5).abs() < 1e-6);
        assert_eq!(second_fraction("bad"), 0.0);
    }

    #[test]
    fn accelerated_hand_wraps_12h_and_differs_from_hour() {
        // PL020 验收：加速 35:30:48 对 12 取模 → 11:30:48；与 04:55:52 时针必须有角差
        let acc = accelerated_hand_fraction("35:30:48");
        let hour = hour_fraction("04:55:52");
        assert!((acc - (11.0 + 30.0 / 60.0 + 48.0 / 3600.0) / 12.0).abs() < 1e-6);
        assert!((acc - hour).abs() > 0.1, "rate>1 黄针必须与时针分开");
        assert_eq!(accelerated_hand_fraction("12:00:00"), 0.0);
        assert_eq!(accelerated_hand_fraction("24:00:00"), 0.0);
        assert_eq!(accelerated_hand_fraction("bad"), 0.0);
    }

    #[test]
    fn pm_boundary_at_noon() {
        assert!(!is_pm("00:00:00"));
        assert!(!is_pm("11:59:59"));
        assert!(is_pm("12:00:00"));
        assert!(is_pm("23:59:59"));
        assert!(!is_pm("bad"));
    }

    #[test]
    fn day_progress_wraps_24h() {
        assert_eq!(day_progress("00:00:00"), 0.0);
        assert!((day_progress("12:00:00") - 0.5).abs() < 1e-6);
        assert!(
            (day_progress("35:30:48") - (11.0 + 30.0 / 60.0 + 48.0 / 3600.0) / 24.0).abs() < 1e-6
        );
        assert_eq!(day_progress("24:00:00"), 0.0);
        assert_eq!(day_progress("bad"), 0.0);
    }

    #[test]
    fn spectrum_hits_anchors() {
        let c = [
            egui::Color32::from_rgb(10, 0, 0),
            egui::Color32::from_rgb(20, 0, 0),
            egui::Color32::from_rgb(30, 0, 0),
            egui::Color32::from_rgb(40, 0, 0),
            egui::Color32::from_rgb(50, 0, 0),
        ];
        assert_eq!(spectrum_color(&c, 0.0), c[0]);
        assert_eq!(spectrum_color(&c, 0.5), c[2]);
        assert_eq!(spectrum_color(&c, 1.0), c[4]);
        assert_eq!(spectrum_color(&c, 0.25).r(), 20);
    }

    #[test]
    fn mix_color_endpoints_and_midpoint() {
        let a = egui::Color32::from_rgb(0, 0, 0);
        let b = egui::Color32::from_rgb(200, 100, 40);
        assert_eq!(mix_color(a, b, 0.0), a);
        assert_eq!(mix_color(a, b, 1.0), b);
        let m = mix_color(a, b, 0.5);
        assert_eq!((m.r(), m.g(), m.b()), (100, 50, 20));
    }

    #[test]
    fn custom_day_counts_24h_wraps() {
        assert_eq!(custom_day("23:59:59"), 1);
        assert_eq!(custom_day("24:00:00"), 2);
        assert_eq!(custom_day("35:30:48"), 2);
        assert_eq!(custom_day("251:38:42"), 11);
    }

    #[test]
    fn day_time_wraps_24h() {
        assert_eq!(day_time("00:00:00"), "00:00:00");
        assert_eq!(day_time("35:30:48"), "11:30:48");
        assert_eq!(day_time("24:00:00"), "00:00:00");
        assert_eq!(day_time("bad"), "00:00:00");
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
    fn dial_diameter_fits_box_width_and_height() {
        // B3 盒：盘径 = min(盒高, 盒宽 - 两侧胶囊固定占位 128)
        assert_eq!(dial_diameter(400.0, 600.0), 272.0);
        assert_eq!(dial_diameter(1000.0, 1000.0), 560.0);
        assert_eq!(dial_diameter(150.0, 200.0), 120.0);
        assert_eq!(dial_diameter(960.0, 420.0), 420.0);
    }

    #[test]
    fn hero_fit_shrinks_with_floor() {
        assert_eq!(hero_fit_scale(300.0, 340.0), 1.0);
        assert!((hero_fit_scale(400.0, 340.0) - 0.85).abs() < 1e-6);
        assert_eq!(hero_fit_scale(500.0, 100.0), 0.5);
    }
}
