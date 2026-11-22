//! 时钟页（PL020 12h 双半日）：星盘居中 + 盘下 readout + 短倍率条 + 农历 meta。
//! 三针走现实时间（时/分/秒），黄针走加速时刻 12h；AM/PM 胶囊跟随现实半日；
//! 金弧 = 黄针与时针的较短弧。纯函数 hour_fraction/minute_fraction/second_fraction/
//! accelerated_hand_fraction/is_pm/dial_diameter/readout_fit_size 可测。

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

/// 加速时刻 → 当日进度比例（纯函数）：时对 24 取模，0:00=0、23:59:59≈1。
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

/// 星盘两侧预留：盘边距 + 药丸宽 + 药丸到盘间距（全部读表）
fn dial_side_w() -> f32 {
    crate::theme::layout_f32("clock_dial_side_pad")
        + crate::theme::layout_f32("clock_pill_w")
        + crate::theme::layout_f32("clock_pill_dial_gap")
}

/// 星盘边长（纯函数）：B3 剩余高与盒宽（扣两侧预留）取小，下限 120、无上限
pub fn dial_diameter(avail_w: f32, avail_h: f32) -> f32 {
    avail_h.min(avail_w - dial_side_w() * 2.0).max(120.0)
}

/// 盘内等比缩放：320 母版 → 当前盘径
pub fn dial_scale(dial_d: f32) -> f32 {
    dial_d / crate::theme::layout_f32("clock_dial_design")
}

/// 横向跌落（纯函数）：从表内 ≤ base 且 ≥ min 的字号中，取第一个装得下的最大档
pub fn readout_fit_size(base: f32, avail: f32, min: f32, width_at: impl Fn(f32) -> f32) -> f32 {
    let mut sizes: Vec<f32> = crate::font_profile::get()
        .sizes()
        .iter()
        .map(|p| p.size)
        .filter(|s| *s >= min && *s <= base + 0.01)
        .collect();
    sizes.sort_by(|a, b| b.total_cmp(a));
    sizes.dedup();
    let mut chosen = min;
    for size in sizes {
        if width_at(size) <= avail {
            chosen = size;
            break;
        }
    }
    chosen
}

impl crate::pages::Page for crate::pages::ClockPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let Some(info) = state.time.clone() else {
            let text = "正在读取时间…";
            let font = crate::theme::font_regular(crate::theme::font_size("body"));
            let center = ui.max_rect().center();
            ui.painter().text(
                egui::pos2(
                    center.x,
                    text_ink::top_for_center(ui, text, &font, InkKind::Cjk, center.y),
                ),
                egui::Align2::CENTER_TOP,
                text,
                font,
                crate::theme::ink().muted,
            );
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
        let box_gap = crate::theme::layout_f32("page_box_gap");
        let m = clock_metrics(page_h, crate::theme::layout_f32("clock_readout_label_gap"));
        let header = header_layout(ui, &info, page_w);
        let header_h = header.height();
        // 页面 = 盒子垂直堆叠：页头(B1+B2) / B3 星盘 / B11 读数 / B18 倍率条（C2 在页面外）
        let rate_h = crate::widgets::rate_strip::height();
        let fixed = header_h + m.readout_h + rate_h + box_gap * 3.0;
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
        let box_rect =
            |y: f32, h: f32| egui::Rect::from_min_size(egui::pos2(left, y), egui::vec2(page_w, h));

        let mut y = page_rect.top();
        draw_stage_header(ui, &info, &header, box_rect(y, header_h));
        y += header_h + box_gap;
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
        if crate::layout_probe::layout_on() {
            use std::sync::atomic::{AtomicU64, Ordering};
            static FRAME: AtomicU64 = AtomicU64::new(0);
            if FRAME.fetch_add(1, Ordering::Relaxed).is_multiple_of(30) {
                let dial_d = dial_diameter(b3.width(), b3.height());
                let rule_gap = crate::theme::layout_f32("clock_header_rule_gap");
                let rule_w = crate::theme::layout_f32("clock_header_rule_w");
                let content_bottom = page_rect.top() + header.content_h();
                let rule_y = page_rect.top() + header_h - rule_w / 2.0;
                let readout_top = b3.bottom() + box_gap;
                let rate_top = readout_top + m.readout_h + box_gap;
                eprintln!(
                    "[CLOCK] page=({:.1},{:.1},{:.1},{:.1}) hdr=({:.1}|{:.1}+{:.1}+{:.1}={:.1}) block={:.1} rule=({:.1}+{:.1}) rule_y={:.1} content_bottom={:.1} dial_band=({:.1},{:.1},{:.1},{:.1}) dial={:.1} visible={:.1} ring=10 readout=({:.1},{:.1}) label=11.5 hero={:.1} real={:.1} day=19.5 rate=({:.1},{:.1}) two_line={}",
                    page_rect.left(),
                    page_rect.top(),
                    page_rect.width(),
                    page_rect.height(),
                    header.b1_h,
                    header.meta_h,
                    header.line_gap,
                    header.meta_h,
                    header.b2_h,
                    header_h,
                    rule_gap,
                    rule_w,
                    rule_y,
                    content_bottom,
                    b3.left(),
                    b3.top(),
                    b3.width(),
                    b3.height(),
                    dial_d,
                    dial_d - crate::theme::layout_f32("clock_dial_ring") * 2.0,
                    readout_top,
                    m.readout_h,
                    m.hero,
                    m.real,
                    rate_top,
                    rate_h,
                    header.two_line
                );
            }
        }
        y += b3_h + box_gap;
        draw_readout(ui, &info, m, box_rect(y, m.readout_h));
        y += m.readout_h + box_gap;
        let b18 = box_rect(y, rate_h);
        ui.scope_builder(egui::UiBuilder::new().max_rect(b18), |b18_ui| {
            draw_rate(b18_ui, state, bus);
        });
    }
}

/// 页头盒子布局（盒子模型）：B1 日期 / B2 农历 各自盒高，合并高 = max
struct HeaderLayout {
    label_size: f32,
    meta_font: egui::FontId,
    b1: text_ink::Ink,
    b1_h: f32,
    meta: text_ink::Ink,
    meta_h: f32,
    b2_h: f32,
    line_gap: f32,
    two_line: bool,
    b1_w: f32,
    line1: String,
}

impl HeaderLayout {
    /// 内容盒高（固定角色行高，不含到分隔线的固定间距）
    fn content_h(&self) -> f32 {
        self.b1_h.max(self.b2_h)
    }

    /// 页头块高 = 内容盒 + 5px 空气 + 1px 横线
    fn height(&self) -> f32 {
        self.content_h()
            + crate::theme::layout_f32("clock_header_rule_gap")
            + crate::theme::layout_f32("clock_header_rule_w")
    }
}

/// 页头行数/文本档位（FIX013.14 纯函数）：日期盒 + 右栏实测宽共同门控，
/// 不再只看 page_w 是否 ≥560；放不下就退化为短农历单行（宁少一行不叠字）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeaderMode {
    two_line: bool,
    full_text: bool,
}

fn header_mode(
    page_w: f32,
    viewport_h: f32,
    b1_w: f32,
    full_w: f32,
    short_w: f32,
    line2_w: f32,
) -> HeaderMode {
    // 左日期盒起点 18，日期右缘到右栏之间留 16；右栏自身右缩进 6
    let avail = (page_w - 18.0 - b1_w - 16.0 - 6.0).max(0.0);
    let full_text = full_w <= avail;
    let line1_w = if full_text { full_w } else { short_w };
    let two_line = viewport_h >= 560.0 && line2_w <= avail && line1_w.max(line2_w) <= avail;
    HeaderMode {
        two_line,
        full_text,
    }
}

/// 量页头两个子盒（B1 日期 / B2 农历）的盒高与宽度跌落档位
fn header_layout(ui: &egui::Ui, info: &accel_core::types::TimeInfo, page_w: f32) -> HeaderLayout {
    let label_size = crate::theme::font_size("body");
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
    let viewport_h = ui.ctx().input(|i| {
        i.viewport()
            .inner_rect
            .map_or(f32::INFINITY, |r| r.height())
    });
    let measure_w = |text: &str| {
        ui.painter()
            .layout_no_wrap(
                text.to_string(),
                meta_font.clone(),
                crate::theme::ink().muted,
            )
            .rect
            .width()
    };
    let line1_full = format!("农历 {lunar_short} · 月相 {phase} · 财神 {god_short}");
    let line1_short = format!("农历 {lunar_short}");
    let full_w = measure_w(&line1_full);
    // 极端长闰月等仍可能让短农历超出日期盒右侧可用宽；再退一档为“农历…”防叠字
    let short_avail = (page_w - 18.0 - b1_w - 16.0 - 6.0).max(0.0);
    let line1_short = if measure_w(&line1_short) > short_avail {
        "农历…".to_string()
    } else {
        line1_short
    };
    let short_w = measure_w(&line1_short);
    let yi_text = short_list(&info.lunar_yi);
    let ji_text = short_list(&info.lunar_ji);
    let line2_w = if yi_text.is_empty() && ji_text.is_empty() {
        0.0
    } else {
        measure_w("宜 ") + measure_w(&yi_text) + 16.0 + measure_w("忌 ") + measure_w(&ji_text)
    };
    let mode = header_mode(page_w, viewport_h, b1_w, full_w, short_w, line2_w);
    let line1 = if mode.full_text {
        line1_full
    } else {
        line1_short
    };
    let meta = text_ink::measure(ui, &line1, &meta_font, InkKind::Cjk);
    let b1_h = crate::widgets::text_metrics::height(ui.ctx(), &label_font, InkKind::Cjk);
    let meta_h = crate::widgets::text_metrics::height(ui.ctx(), &meta_font, InkKind::Cjk);
    let line_gap = crate::theme::layout_f32("clock_header_line_gap");
    let b2_h = meta_h
        + if mode.two_line {
            line_gap + meta_h
        } else {
            0.0
        };
    HeaderLayout {
        label_size,
        meta_font,
        b1,
        b1_h,
        meta,
        meta_h,
        b2_h,
        line_gap,
        two_line: mode.two_line,
        b1_w,
        line1,
    }
}

/// 舞台页头盒：左 15.5 日期盒 / 右 11.5×2 + 3 = 26.0 农历盒，中线对齐；字底贴各自盒底
fn draw_stage_header(
    ui: &mut egui::Ui,
    info: &accel_core::types::TimeInfo,
    layout: &HeaderLayout,
    rect: egui::Rect,
) {
    let ink = crate::theme::ink();
    let top = rect.top();
    let center_y = top + layout.content_h() / 2.0;
    let b1_top = center_y - layout.b1_h / 2.0;
    let b1_bottom = center_y + layout.b1_h / 2.0;
    let b2_top = center_y - layout.b2_h / 2.0;
    let b2_bottom = center_y + layout.b2_h / 2.0;
    ui.painter().circle_filled(
        egui::pos2(rect.left() + 6.0, center_y),
        3.5,
        crate::theme::accent_color(),
    );
    // 左日期盒 15.5：盒中线 = 农历块中线；字底贴盒底，不越过盒底
    let date_font = crate::theme::font_semibold(layout.label_size);
    let date_pos = egui::pos2(rect.left() + 18.0, layout.b1.top_for_bottom(b1_bottom));
    crate::layout_probe::text(
        ui,
        "clock.date",
        &info.chinese_date,
        &date_font,
        InkKind::Cjk,
        date_pos,
        egui::Align2::LEFT_TOP,
    );
    crate::text_probe::line(
        ui,
        "clock.date.ink",
        &info.chinese_date,
        &date_font,
        InkKind::Cjk,
        date_pos,
        egui::Align2::LEFT_TOP,
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 18.0, b1_top),
            egui::pos2(rect.left() + 18.0 + layout.b1_w, b1_bottom),
        ),
    );
    ui.painter().text(
        date_pos,
        egui::Align2::LEFT_TOP,
        &info.chinese_date,
        date_font,
        ink.strong,
    );
    // 日期 hover：周数 / 节日 / 节气（命中域 = 15.5 盒，不占整个页头）
    let date_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), b1_top),
        egui::vec2(18.0 + layout.b1_w, layout.b1_h),
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
    if layout.two_line {
        let line2_bottom = b2_bottom;
        let line1_bottom = line2_bottom - layout.meta_h - layout.line_gap;
        let line1_pos = egui::pos2(right, layout.meta.top_for_bottom(line1_bottom));
        crate::layout_probe::text(
            ui,
            "clock.lunar1",
            &layout.line1,
            &layout.meta_font,
            InkKind::Cjk,
            line1_pos,
            egui::Align2::RIGHT_TOP,
        );
        let line1_w = ui
            .painter()
            .layout_no_wrap(
                layout.line1.clone(),
                layout.meta_font.clone(),
                egui::Color32::WHITE,
            )
            .rect
            .width();
        crate::text_probe::line(
            ui,
            "clock.lunar1.ink",
            &layout.line1,
            &layout.meta_font,
            InkKind::Cjk,
            line1_pos,
            egui::Align2::RIGHT_TOP,
            egui::Rect::from_min_max(
                egui::pos2(right - line1_w, line1_bottom - layout.meta_h),
                egui::pos2(right, line1_bottom),
            ),
        );
        ui.painter().text(
            line1_pos,
            egui::Align2::RIGHT_TOP,
            &layout.line1,
            layout.meta_font.clone(),
            ink.muted,
        );
        draw_yi_ji(
            ui,
            egui::pos2(right, line2_bottom),
            &info.lunar_yi,
            &info.lunar_ji,
            &layout.meta_font,
        );
    } else {
        let line1_pos = egui::pos2(right, layout.meta.top_for_bottom(b2_bottom));
        crate::layout_probe::text(
            ui,
            "clock.lunar1",
            &layout.line1,
            &layout.meta_font,
            InkKind::Cjk,
            line1_pos,
            egui::Align2::RIGHT_TOP,
        );
        let line1_w = ui
            .painter()
            .layout_no_wrap(
                layout.line1.clone(),
                layout.meta_font.clone(),
                egui::Color32::WHITE,
            )
            .rect
            .width();
        crate::text_probe::line(
            ui,
            "clock.lunar1.ink",
            &layout.line1,
            &layout.meta_font,
            InkKind::Cjk,
            line1_pos,
            egui::Align2::RIGHT_TOP,
            egui::Rect::from_min_max(
                egui::pos2(right - line1_w, b2_top),
                egui::pos2(right, b2_bottom),
            ),
        );
        ui.painter().text(
            line1_pos,
            egui::Align2::RIGHT_TOP,
            &layout.line1,
            layout.meta_font.clone(),
            ink.muted,
        );
    }
    // 农历 hover：右盒 = 26.0（两行）/ 11.5（一行），不占 5px + 线
    let tip_rect = egui::Rect::from_min_max(
        egui::pos2(rect.center().x, b2_top),
        egui::pos2(right, b2_bottom),
    );
    let resp = ui.interact(
        tip_rect,
        egui::Id::unique("stage_lunar_tip"),
        egui::Sense::hover(),
    );
    if resp.hovered() && (!info.lunar_yi.is_empty() || !info.lunar_ji.is_empty()) {
        draw_lunar_tooltip(
            ui,
            rect.right(),
            rect.bottom() + 6.0,
            info,
            &layout.meta_font,
        );
    }
    let rule_w = crate::theme::layout_f32("clock_header_rule_w");
    let rule_y = rect.bottom() - rule_w / 2.0;
    crate::layout_probe::rect(
        ui,
        "clock.rule",
        egui::Rect::from_min_size(
            egui::pos2(rect.left(), rule_y - rule_w / 2.0),
            egui::vec2(rect.width(), rule_w),
        ),
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rule_y),
            egui::pos2(rect.right(), rule_y),
        ],
        egui::Stroke::new(rule_w, crate::theme::border_color()),
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
    crate::widgets::tooltip::show_at(
        ui,
        egui::Id::unique("stage_date_tip_card"),
        &text,
        font,
        left,
        top,
    );
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
    let yi_lines: Vec<String> = info.lunar_yi.chunks(5).map(|c| c.join(" · ")).collect();
    let ji_lines: Vec<String> = info.lunar_ji.chunks(5).map(|c| c.join(" · ")).collect();
    let label_w = w_yi_label.max(w_ji_label);
    let max_line_w = yi_lines
        .iter()
        .chain(ji_lines.iter())
        .map(|l| measure(l, ink.muted))
        .fold(0.0f32, f32::max);
    let pad_x = crate::theme::layout_f32("tooltip_pad_x");
    let pad_y = crate::theme::layout_f32("tooltip_pad_y");
    let line_gap = crate::theme::layout_f32("tooltip_line_gap");
    let w = label_w + max_line_w + pad_x * 2.0;
    let rows = yi_lines.len() + ji_lines.len();
    if rows == 0 {
        return;
    }
    let row_ink = text_ink::measure(ui, "", font, InkKind::Cjk);
    let h = pad_y * 2.0 + rows as f32 * row_ink.h + rows.saturating_sub(1) as f32 * line_gap;
    let x = (right - 10.0 - w).max(0.0);
    let ctx = ui.ctx().clone();
    egui::Area::new(egui::Id::unique("stage_lunar_tip_card"))
        .order(egui::Order::Tooltip)
        .fixed_pos(egui::pos2(x, top))
        .show(&ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
            crate::widgets::tooltip::frame(ui, rect);
            let p = ui.painter();
            let text_x = rect.left() + pad_x + label_w;
            let mut row_top = rect.top() + pad_y;
            for (i, text_line) in yi_lines.iter().enumerate() {
                let row_bottom = row_top + row_ink.h;
                if i == 0 {
                    p.text(
                        egui::pos2(rect.left() + pad_x, row_ink.top_for_bottom(row_bottom)),
                        egui::Align2::LEFT_TOP,
                        "宜 ",
                        font.clone(),
                        success,
                    );
                }
                p.text(
                    egui::pos2(text_x, row_ink.top_for_bottom(row_bottom)),
                    egui::Align2::LEFT_TOP,
                    text_line,
                    font.clone(),
                    ink.muted,
                );
                row_top = row_bottom + line_gap;
            }
            for (i, text_line) in ji_lines.iter().enumerate() {
                let row_bottom = row_top + row_ink.h;
                if i == 0 {
                    p.text(
                        egui::pos2(rect.left() + pad_x, row_ink.top_for_bottom(row_bottom)),
                        egui::Align2::LEFT_TOP,
                        "忌 ",
                        font.clone(),
                        danger,
                    );
                }
                p.text(
                    egui::pos2(text_x, row_ink.top_for_bottom(row_bottom)),
                    egui::Align2::LEFT_TOP,
                    text_line,
                    font.clone(),
                    ink.muted,
                );
                row_top = row_bottom + line_gap;
            }
        });
}

/// 宜/忌短串（最多三项 + “等”）：页头测宽与绘制共用同一文本
fn short_list(items: &[String]) -> String {
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
}

/// 页头宜忌行（右对齐、盒底对齐；字底贴盒底，不越过盒底）：宜/忌前缀语义色，各取前 3 条，超出加「等」
fn draw_yi_ji(
    ui: &egui::Ui,
    right_bottom: egui::Pos2,
    yi: &[String],
    ji: &[String],
    font: &egui::FontId,
) {
    if yi.is_empty() && ji.is_empty() {
        return;
    }
    let ink = crate::theme::ink();
    let yi_text = short_list(yi);
    let ji_text = short_list(ji);
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
    let mut x = right_bottom.x - total;
    let yi_ink = text_ink::measure(ui, "宜", font, InkKind::Cjk);
    let y = yi_ink.top_for_bottom(right_bottom.y);
    let line2_text = format!("宜 {yi_text}  忌 {ji_text}");
    crate::text_probe::line(
        ui,
        "clock.lunar2.ink",
        &line2_text,
        font,
        InkKind::Cjk,
        egui::pos2(right_bottom.x - total, y),
        egui::Align2::LEFT_TOP,
        egui::Rect::from_min_max(
            egui::pos2(right_bottom.x - total, right_bottom.y - yi_ink.h),
            egui::pos2(right_bottom.x, right_bottom.y),
        ),
    );
    crate::layout_probe::rect(
        ui,
        "clock.lunar2",
        egui::Rect::from_min_size(
            egui::pos2(right_bottom.x - total, right_bottom.y - yi_ink.h),
            egui::vec2(total, yi_ink.h),
        ),
    );
    if !yi_text.is_empty() {
        ui.painter().text(
            egui::pos2(x, y),
            egui::Align2::LEFT_TOP,
            "宜 ",
            font.clone(),
            success,
        );
        x += w_yi;
        ui.painter().text(
            egui::pos2(x, y),
            egui::Align2::LEFT_TOP,
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
            egui::Align2::LEFT_TOP,
            "忌 ",
            font.clone(),
            danger,
        );
        x += w_ji;
        ui.painter().text(
            egui::pos2(x, y),
            egui::Align2::LEFT_TOP,
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
    let scale = dial_scale(dial_d);
    let ring = crate::theme::layout_f32("clock_dial_ring") * scale;
    let outer_r = dial_d / 2.0 - ring;
    let tick_long = crate::theme::layout_f32("clock_dial_tick_long") * scale;
    let tick_short = crate::theme::layout_f32("clock_dial_tick_short") * scale;
    let tick_long_w = crate::theme::layout_f32("clock_dial_tick_long_w") * scale;
    let tick_short_w = crate::theme::layout_f32("clock_dial_tick_short_w") * scale;
    let tick_cap = crate::theme::layout_f64("clock_dial_tick_cap") * scale;
    let tick_glow_offset = crate::theme::layout_f32("clock_dial_tick_glow_offset") * scale;
    let tick_glow_decay = crate::theme::layout_f64("clock_dial_tick_glow_decay") * scale;
    crate::layout_probe::rect(
        ui,
        "clock.dial_ticks",
        egui::Rect::from_center_size(center, egui::vec2(outer_r * 2.0, outer_r * 2.0)),
    );
    // 12h 刻度：60 小格；只有 12 个整点长刻度夜光绿 + GPU 光晕，短刻度保持原色
    let mut ticks = Vec::with_capacity(12);
    for i in 0..60 {
        let a = i as f32 / 60.0;
        let ang = day_angle(a);
        let hour_tick = i % 5 == 0;
        let len = if hour_tick { tick_long } else { tick_short };
        let color = if hour_tick { lume } else { idle };
        let width = if hour_tick { tick_long_w } else { tick_short_w };
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
    crate::widgets::ring_glow::paint_ticks(
        ui,
        center,
        outer_r + tick_glow_offset,
        tick_glow_decay,
        &ticks,
    );
    for &(p1, p2, half, color) in &ticks {
        ui.painter()
            .line_segment([p1, p2], egui::Stroke::new(half * 2.0, color));
        // 端点补同色圆点，盖住 CPU 线段的抗锯齿暗像素
        ui.painter().circle_filled(p1, tick_cap, color);
        ui.painter().circle_filled(p2, tick_cap, color);
    }
    // 四正位数字：图形刻度，按实际墨迹中线定位，不查字体盒表
    let num_size = crate::theme::layout_f32("clock_dial_num_size") * scale;
    let num_radius = crate::theme::layout_f32("clock_dial_num_radius") * scale;
    let num_bloom = crate::theme::layout_f32("clock_dial_num_bloom") * scale;
    let cardinal = egui::FontId::proportional(num_size);
    for (ang, label) in [(0.0, "12"), (90.0, "3"), (180.0, "6"), (270.0, "9")] {
        let (lx, ly) = polar(center.x, center.y, num_radius, ang);
        let pos = egui::pos2(lx, ly);
        let alpha = 110;
        let tag = match label {
            "12" => "clock.num12",
            "3" => "clock.num3",
            "6" => "clock.num6",
            _ => "clock.num9",
        };
        let text_top = text_ink::top_for_ink_center(ui, label, &cardinal, pos.y);
        crate::layout_probe::glow(ui, tag, pos, num_bloom, alpha);
        crate::layout_probe::ink(
            ui,
            tag,
            label,
            &cardinal,
            egui::pos2(pos.x, text_top),
            egui::Align2::CENTER_TOP,
        );
        crate::widgets::ring_glow::paint_dot(ui, pos, 0.0, lume, alpha as f32 / 255.0, num_bloom);
        ui.painter().text(
            egui::pos2(pos.x, text_top),
            egui::Align2::CENTER_TOP,
            label,
            cardinal.clone(),
            lume,
        );
    }

    // 金圈：加速日进度环（厚度 2–10×scale），跨日 1s 平滑，不消失
    let ring_min = crate::theme::layout_f64("clock_dial_ring_min") * scale;
    let ring_target = (progress * ring).max(ring_min);
    let ring_fade_s = crate::theme::layout_f64("clock_dial_ring_fade_ms") / 1000.0;
    let ring_thickness = ui.ctx().animate_value_with_time(
        egui::Id::unique("clock_ring_thickness"),
        ring_target,
        ring_fade_s,
    );
    draw_ring(
        ui,
        center,
        outer_r,
        progress,
        ring_thickness,
        crate::theme::layout_f32("clock_dial_ring_bloom") * scale,
    );

    let a1 = day_angle(accel);
    // 时针：米白粗短
    let ha = day_angle(hour);
    let (hx, hy) = polar(
        center.x,
        center.y,
        crate::theme::layout_f32("clock_dial_hand_hour") * scale,
        ha,
    );
    let (htx, hty) = polar(
        center.x,
        center.y,
        -crate::theme::layout_f32("clock_dial_tail_hour") * scale,
        ha,
    );
    ui.painter().line_segment(
        [egui::pos2(htx, hty), egui::pos2(hx, hy)],
        egui::Stroke::new(
            crate::theme::layout_f32("clock_dial_hand_hour_w") * scale,
            ink.strong,
        ),
    );
    // 分针：星紫
    let ma = day_angle(minute);
    let (mx, my) = polar(
        center.x,
        center.y,
        crate::theme::layout_f32("clock_dial_hand_minute") * scale,
        ma,
    );
    let (mtx, mty) = polar(
        center.x,
        center.y,
        -crate::theme::layout_f32("clock_dial_tail_minute") * scale,
        ma,
    );
    ui.painter().line_segment(
        [egui::pos2(mtx, mty), egui::pos2(mx, my)],
        egui::Stroke::new(
            crate::theme::layout_f32("clock_dial_hand_minute_w") * scale,
            crate::theme::hand_minute_color(),
        ),
    );
    // 秒针：朱红细长，带尾与端圆点
    let sa = day_angle(second);
    let (sx, sy) = polar(
        center.x,
        center.y,
        crate::theme::layout_f32("clock_dial_hand_second") * scale,
        sa,
    );
    let (stx, sty) = polar(
        center.x,
        center.y,
        -crate::theme::layout_f32("clock_dial_tail_second") * scale,
        sa,
    );
    let sec_color = crate::theme::hand_second_color();
    ui.painter().line_segment(
        [egui::pos2(stx, sty), egui::pos2(sx, sy)],
        egui::Stroke::new(
            crate::theme::layout_f64("clock_dial_hand_second_w") * scale,
            sec_color,
        ),
    );
    ui.painter().circle_filled(
        egui::pos2(sx, sy),
        crate::theme::layout_f64("clock_dial_second_tip") * scale,
        sec_color,
    );
    // 黄针：加速时刻，略短于秒针、比时针/分针粗
    let (gx, gy) = polar(
        center.x,
        center.y,
        crate::theme::layout_f32("clock_dial_hand_accel") * scale,
        a1,
    );
    let (tx, ty) = polar(
        center.x,
        center.y,
        -crate::theme::layout_f32("clock_dial_tail_accel") * scale,
        a1,
    );
    ui.painter().line_segment(
        [egui::pos2(tx, ty), egui::pos2(gx, gy)],
        egui::Stroke::new(
            crate::theme::layout_f64("clock_dial_hand_accel_w") * scale,
            accent,
        ),
    );
    ui.painter().circle_filled(
        center,
        crate::theme::layout_f32("clock_dial_hub") * scale,
        ink.strong,
    );
    draw_half_pills(ui, center, outer_r, ring, pm);
}

/// 金圈（PL020/PL024）：加速日进度 → 实环；厚度由调用方平滑后传入
fn draw_ring(
    ui: &egui::Ui,
    center: egui::Pos2,
    rim_r: f32,
    progress: f32,
    thickness: f32,
    bloom: f32,
) {
    let t = progress.clamp(0.0, 1.0);
    let r = rim_r + thickness / 2.0;
    let ring_outer = r + thickness / 2.0;
    crate::layout_probe::rect(
        ui,
        "clock.dial_gold",
        egui::Rect::from_center_size(center, egui::vec2(ring_outer * 2.0, ring_outer * 2.0)),
    );
    crate::layout_probe::rect(
        ui,
        "clock.dial_glow",
        egui::Rect::from_center_size(
            center,
            egui::vec2((ring_outer + bloom) * 2.0, (ring_outer + bloom) * 2.0),
        ),
    );
    let color = spectrum_color(&crate::theme::ring_spectrum(), t);
    ui.painter()
        .circle_stroke(center, r, egui::Stroke::new(thickness, color));
    crate::widgets::ring_glow::paint(ui, center, r, thickness, color, bloom);
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

/// 半日胶囊（PL023）：盘左右两侧与 3/9 点中线对齐，内缘距盘盒 20（距可见盘 30）；AM 橙红 / PM 淡蓝
fn draw_half_pills(ui: &egui::Ui, center: egui::Pos2, rim_r: f32, ring: f32, pm: bool) {
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
    let accent = crate::theme::accent_color();
    let pill_w = crate::theme::layout_f32("clock_pill_w");
    let pill_h = crate::theme::layout_f32("clock_pill_h");
    let pill_radius = crate::theme::layout_f32("clock_pill_radius");
    let pill_font_size = crate::theme::font_size("meta");
    let pill_gap = crate::theme::layout_f32("clock_pill_dial_gap");
    let size = egui::vec2(pill_w, pill_h);
    let radius = egui::epaint::CornerRadius::same(pill_radius as u8);
    let dx = rim_r + ring + pill_gap + pill_w / 2.0;
    let y = center.y;
    for (offset, label, t, color) in [
        (-dx, "AM", am_t, crate::theme::half_am_color()),
        (dx, "PM", pm_t, crate::theme::half_pm_color()),
    ] {
        let rect = egui::Rect::from_center_size(egui::pos2(center.x + offset, y), size);
        let tag = if label == "AM" {
            "clock.pill_am"
        } else {
            "clock.pill_pm"
        };
        crate::layout_probe::rect(ui, tag, rect);
        if t > 0.01 {
            crate::widgets::ring_glow::paint_pill(
                ui,
                rect.center(),
                size * 0.5,
                pill_radius,
                color,
                t,
                6.0,
            );
        }
        let fill = mix_color(bg, color, t);
        let text = mix_color(meta, accent, t);
        ui.painter().rect_filled(rect, radius, fill);
        let pill_font = crate::theme::font_semibold(pill_font_size);
        ui.painter().text(
            egui::pos2(
                rect.center().x,
                text_ink::top_for_center(ui, label, &pill_font, InkKind::Latin, rect.center().y),
            ),
            egui::Align2::CENTER_TOP,
            label,
            pill_font,
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

/// 高度断点（内部尺寸）：读数 hero / 现实 / 读数总高
#[derive(Clone, Copy)]
struct ClockMetrics {
    hero: f32,
    real: f32,
    readout_h: f32,
}

/// 按页面区高度取盒内字号断点；读数盒高由标签盒 + 间距 + 数值行盒算出
fn clock_metrics(page_h: f32, label_gap: f32) -> ClockMetrics {
    let pair = |hero_key: &str, real_key: &str| {
        (
            crate::theme::layout_f32(hero_key),
            crate::theme::layout_f32(real_key),
        )
    };
    let (hero, real) = if page_h >= 720.0 {
        pair("readout_hero_lg", "readout_real_lg")
    } else if page_h >= 600.0 {
        pair("readout_hero_md", "readout_real_md")
    } else if page_h >= 520.0 {
        pair("readout_hero_sm", "readout_real_sm")
    } else if page_h >= 440.0 {
        pair("readout_hero_xs", "readout_real_xs")
    } else {
        pair("readout_hero_xxs", "readout_real_xxs")
    };
    let cjk = |size: f32| crate::font_profile::get().for_size(size).h;
    let latin = |size: f32| crate::font_profile::get().for_size(size).h_latin;
    let label_h = cjk(crate::theme::font_size("meta"));
    let value_row_h = latin(hero)
        .max(latin(real))
        .max(cjk(crate::theme::font_size("section")));
    ClockMetrics {
        hero,
        real,
        readout_h: label_h + label_gap + value_row_h,
    }
}

/// 盘下读数（PL020）：B11 盒子铺满页面宽；竖线居中，左区右对齐、右区左对齐；
/// 左列 `加速` 标签 + 「第 N 天 + 当天时间」同行，右列 `现实` 标签 + real 时间。
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
    let day_font = crate::theme::font_semibold(crate::theme::font_size("section"));
    let day_num_font = digits::digit_font(crate::theme::font_size("section"));
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

    // 数值行盒底 = 时间盒底
    let label_ink = text_ink::measure(ui, "加速", &label_font, InkKind::Cjk);
    let cn_ink = text_ink::measure(ui, "天", &day_font, InkKind::Cjk);

    // 左区右对齐到 center-20；右区左对齐到 center+20
    let readout_min = crate::theme::layout_f32("readout_size_min");
    let left_avail = (center_x - 20.0 - area.left() - day_w - 12.0).max(40.0);
    let hero_size = readout_fit_size(base, left_avail, readout_min, |size| {
        digits::text_width(ui.painter(), &day_text, &digits::digit_font(size))
    });
    let hero_font = digits::digit_font(hero_size);
    let hero_w = digits::text_width(ui.painter(), &day_text, &hero_font);
    let left_time_x = center_x - 20.0 - hero_w;
    let day_left = left_time_x - 12.0 - day_w;
    let value_bottom = area.bottom();

    let label_h = crate::widgets::text_metrics::height(ui.ctx(), &label_font, InkKind::Cjk);
    let label_y = label_ink.top_for_bottom(area.top() + label_h);
    let label_bottom = area.top() + label_h;
    crate::layout_probe::text(
        ui,
        "clock.ro_label_accel",
        "加速",
        &label_font,
        InkKind::Cjk,
        egui::pos2(day_left, label_y),
        egui::Align2::LEFT_TOP,
    );
    crate::text_probe::boxed_line(
        ui,
        "clock.ro_label_accel",
        "加速",
        &label_font,
        InkKind::Cjk,
        egui::pos2(day_left, label_y),
        egui::Align2::LEFT_TOP,
        label_bottom,
    );
    crate::layout_probe::text(
        ui,
        "clock.ro_label_real",
        "现实",
        &label_font,
        InkKind::Cjk,
        egui::pos2(center_x + 20.0, label_y),
        egui::Align2::LEFT_TOP,
    );
    crate::text_probe::boxed_line(
        ui,
        "clock.ro_label_real",
        "现实",
        &label_font,
        InkKind::Cjk,
        egui::pos2(center_x + 20.0, label_y),
        egui::Align2::LEFT_TOP,
        label_bottom,
    );
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

    digits::draw_bottom(
        ui.painter(),
        value_bottom,
        left_time_x + hero_w / 2.0,
        &day_text,
        &hero_font,
        ink.strong,
    );
    let accel_h = crate::widgets::text_metrics::height(ui.ctx(), &hero_font, InkKind::Digits);
    crate::layout_probe::rect(
        ui,
        "clock.ro_accel",
        egui::Rect::from_min_size(
            egui::pos2(left_time_x, value_bottom - accel_h),
            egui::vec2(hero_w, accel_h),
        ),
    );

    let mut x = day_left;
    // 「第 / N / 天」三个内盒，各自盒中线对齐到外盒中线
    let day_h = crate::widgets::text_metrics::height(ui.ctx(), &day_font, InkKind::Cjk);
    let num_h = crate::widgets::text_metrics::height(ui.ctx(), &day_num_font, InkKind::Digits);
    let outer_h = day_h.max(num_h);
    let center_y = value_bottom - outer_h / 2.0;
    let di_top = center_y - day_h / 2.0;
    let di_bottom = center_y + day_h / 2.0;
    let num_bottom = center_y + num_h / 2.0;
    let cn_y = cn_ink.top_for_bottom(di_bottom);
    ui.painter().text(
        egui::pos2(x, cn_y),
        egui::Align2::LEFT_TOP,
        "第",
        day_font.clone(),
        accent,
    );
    crate::layout_probe::rect(
        ui,
        "clock.ro_day_di",
        egui::Rect::from_min_size(egui::pos2(x, di_top), egui::vec2(w_di, day_h)),
    );
    x += w_di + day_gap;
    let num_rect = digits::draw_bottom(
        ui.painter(),
        num_bottom,
        x + w_num / 2.0,
        &day_num_text,
        &day_num_font,
        ink.strong,
    );
    crate::layout_probe::rect(ui, "clock.ro_day_num", num_rect);
    x += w_num + day_gap;
    ui.painter().text(
        egui::pos2(x, cn_y),
        egui::Align2::LEFT_TOP,
        "天",
        day_font,
        accent,
    );
    crate::layout_probe::rect(
        ui,
        "clock.ro_day_tian",
        egui::Rect::from_min_size(egui::pos2(x, di_top), egui::vec2(w_tian, day_h)),
    );
    crate::layout_probe::rect(
        ui,
        "clock.ro_day",
        egui::Rect::from_min_size(
            egui::pos2(day_left, value_bottom - outer_h),
            egui::vec2(day_w, outer_h),
        ),
    );

    let real_text = time_part(&info.standard_datetime);
    let real_label_x = center_x + 20.0;
    let real_time_x = real_label_x + day_w + 12.0;
    let real_avail = (area.right() - real_time_x).max(40.0);
    let real_size = readout_fit_size(m.real, real_avail, readout_min, |size| {
        digits::text_width(ui.painter(), real_text, &digits::digit_font(size))
    });
    let real_font = digits::digit_font(real_size);
    let real_w = digits::text_width(ui.painter(), real_text, &real_font);
    digits::draw_bottom(
        ui.painter(),
        value_bottom,
        real_time_x + real_w / 2.0,
        real_text,
        &real_font,
        ink.body,
    );
    let real_h = crate::widgets::text_metrics::height(ui.ctx(), &real_font, InkKind::Digits);
    crate::layout_probe::rect(
        ui,
        "clock.ro_real",
        egui::Rect::from_min_size(
            egui::pos2(real_time_x, value_bottom - real_h),
            egui::vec2(real_w, real_h),
        ),
    );
    ui.painter().line_segment(
        [
            egui::pos2(center_x, area.top()),
            egui::pos2(center_x, value_bottom),
        ],
        egui::Stroke::new(1.0, crate::theme::border_color()),
    );
    crate::layout_probe::rect(
        ui,
        "clock.ro_line",
        egui::Rect::from_min_max(
            egui::pos2(center_x, area.top()),
            egui::pos2(center_x + 1.0, value_bottom),
        ),
    );
}

/// 短倍率条：宽 220–360、6px 轨、1×/365× 端标；固定 73.0 盒
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

// ===== src/pages/clock.rs 说明区 =====
// 输入：UiState（Tick 快照/倍率域）；输出：星盘 + readout + 倍率条 + SetRate/ApplyRate
// 逻辑步骤：12h 星盘（60 刻度 + 四针 + 金圈 + AM/PM 胶囊）→ 盘下 readout
//   （标签 11.5 + 5 + 数值行盒）→ 倍率条 73.0
// 设计理由（PL023/PL024）：页头 26.0/块 32.0；星盘盒 = min(剩余高, 页宽−164).max(120)；
//   盘内 320 母版，scale = dial_d/320，几何/字号/线宽/尾长/光晕全乘 scale；
//   四针固定 60/90/110/100，胶囊固定；读数三盒盒底对齐；倍率四固定盒
// 异常处理：time 为 None 显示读取中；各 hand/parse 异常回 0；读数横向按表跌落
// 关联依赖：widgets(digits/motion/rate_strip)/tools::clock/theme 针色与半日色；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_mode_degrades_before_overlap() {
        // FIX013.14：410 宽（抽屉打开）放不下宜忌 → 单行短农历，绝不叠日期
        let mode = header_mode(410.0, 720.0, 190.0, 320.0, 110.0, 226.0);
        assert!(!mode.full_text);
        assert!(!mode.two_line);
        // 560/590 宽：完整行或宜忌至少能放下；590 放不下宜忌时退化单行
        let mode = header_mode(560.0, 720.0, 190.0, 320.0, 110.0, 226.0);
        assert!(mode.full_text && mode.two_line);
        let mode = header_mode(590.0, 720.0, 190.0, 400.0, 110.0, 300.0);
        assert!(!mode.full_text);
        assert!(mode.two_line);
        let mode = header_mode(590.0, 720.0, 190.0, 400.0, 110.0, 380.0);
        assert!(!mode.two_line, "宜忌放不下时不得压日期");
        // 高度不足 560：即使宽度够也只保留单行
        let mode = header_mode(900.0, 500.0, 190.0, 400.0, 110.0, 380.0);
        assert!(!mode.two_line);
        // 900 宽足够：完整两行
        let mode = header_mode(900.0, 720.0, 190.0, 400.0, 110.0, 380.0);
        assert!(mode.full_text && mode.two_line);
    }

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
        // B3 盒：边长 = min(剩余高, 盒宽 − 两侧预留×2)，下限 120、无上限
        assert_eq!(dial_diameter(400.0, 600.0), 236.0);
        assert_eq!(dial_diameter(1000.0, 1000.0), 836.0);
        assert_eq!(dial_diameter(150.0, 200.0), 120.0);
        assert_eq!(dial_diameter(960.0, 420.0), 420.0);
        assert_eq!(dial_diameter(500.0, 800.0), 336.0);
    }

    #[test]
    fn dial_side_reserve_matches_pill_tokens() {
        let pad = crate::theme::layout_f32("clock_dial_side_pad");
        let pill = crate::theme::layout_f32("clock_pill_w");
        let gap = crate::theme::layout_f32("clock_pill_dial_gap");
        assert!((dial_side_w() - (pad + pill + gap)).abs() < 1e-6);
    }

    #[test]
    fn dial_scale_uses_320_design() {
        assert!((dial_scale(320.0) - 1.0).abs() < 1e-6);
        assert!((dial_scale(366.0) - 366.0 / 320.0).abs() < 1e-6);
        assert!((dial_scale(120.0) - 0.375).abs() < 1e-6);
    }

    #[test]
    fn dial_master_tokens_match_320() {
        for (key, want) in [
            ("clock_dial_design", 320.0),
            ("clock_dial_hand_hour", 60.0),
            ("clock_dial_hand_minute", 90.0),
            ("clock_dial_hand_second", 110.0),
            ("clock_dial_hand_accel", 100.0),
            ("clock_dial_tick_long", 14.0),
            ("clock_dial_tick_short", 6.0),
            ("clock_dial_num_size", 20.0),
            ("clock_dial_num_radius", 116.0),
        ] {
            assert_eq!(crate::theme::layout_f64(key), want, "{key}");
        }
        for (key, want) in [
            ("clock_dial_tick_cap", 0.6),
            ("clock_dial_tick_glow_decay", 1.6),
            ("clock_dial_hand_second_w", 1.2),
            ("clock_dial_hand_accel_w", 3.2),
            ("clock_dial_second_tip", 1.6),
            ("clock_dial_ring_min", 2.0),
            ("clock_dial_ring_fade_ms", 1000.0),
        ] {
            assert!((crate::theme::layout_f64(key) - want).abs() < 1e-6, "{key}");
        }
    }

    #[test]
    fn readout_fit_drops_to_largest_fitting_tier() {
        // 宽度 = 字号 × 10，便于验证档位选择
        let width = |size: f32| size * 10.0;
        assert_eq!(readout_fit_size(44.0, 440.0, 24.0, width), 44.0);
        assert_eq!(readout_fit_size(44.0, 430.0, 24.0, width), 40.0);
        assert_eq!(readout_fit_size(44.0, 390.0, 24.0, width), 36.0);
        assert_eq!(readout_fit_size(44.0, 240.0, 24.0, width), 24.0);
        assert_eq!(readout_fit_size(36.0, 300.0, 24.0, width), 28.0);
    }
}
