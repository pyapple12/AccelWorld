//! 世界时钟页（PL028）：固定 hero（城市/时间/日期/时区偏移）+ 可滚动城市列表 +
//! 固定底部搜索。滚动套件与倒计时共用 `widgets::list_scroll`。

use accel_core::types::FrontendCommand;
use accel_core::world_search::{self, WorldCityEntry};
use chrono::{DateTime, Datelike, Offset, Timelike, Utc};

use crate::bus::UiState;
use crate::widgets::text_ink::{self, InkKind};
use crate::widgets::{box_stack, digits, field, icons, list_scroll, motion, primary_button};

/// 时区走时（纯函数）：UTC 时刻 → IANA 时区本地 "HH:MM:SS" + 是否昼（6-18 时）
pub fn now_in_tz(utc: DateTime<Utc>, tz: &str) -> Option<(String, bool)> {
    let tz_parsed: chrono_tz::Tz = tz.parse().ok()?;
    let local = utc.with_timezone(&tz_parsed);
    let hour = local.hour();
    Some((
        format!(
            "{:02}:{:02}:{:02}",
            local.hour(),
            local.minute(),
            local.second()
        ),
        (6..18).contains(&hour),
    ))
}

/// 周几中文（日期 hero 用）
fn weekday_cn(weekday: chrono::Weekday) -> &'static str {
    match weekday {
        chrono::Weekday::Mon => "周一",
        chrono::Weekday::Tue => "周二",
        chrono::Weekday::Wed => "周三",
        chrono::Weekday::Thu => "周四",
        chrono::Weekday::Fri => "周五",
        chrono::Weekday::Sat => "周六",
        chrono::Weekday::Sun => "周日",
    }
}

/// 时区偏移值：+8 / -7 / +5:30（卡片第二行不显示“时区”二字）
fn tz_offset_value(utc: DateTime<Utc>, tz: &str) -> Option<String> {
    let tz_parsed: chrono_tz::Tz = tz.parse().ok()?;
    let local = utc.with_timezone(&tz_parsed);
    let seconds = local.offset().fix().local_minus_utc();
    let sign = if seconds < 0 { "-" } else { "+" };
    let abs = seconds.abs();
    let hours = abs / 3600;
    let minutes = (abs % 3600) / 60;
    if minutes == 0 {
        Some(format!("{sign}{hours}"))
    } else {
        Some(format!("{sign}{hours}:{minutes:02}"))
    }
}

/// 时区偏移文案（hero 用）：+8 时区 / -7 时区 / +5:30 时区
fn tz_offset_label(utc: DateTime<Utc>, tz: &str) -> Option<String> {
    tz_offset_value(utc, tz).map(|v| format!("{v} 时区"))
}

/// 人口文案：300000 → 30 万（四舍五入到万；标签前不再带“人口”二字）
fn population_label(population: i64) -> String {
    if population <= 0 {
        return String::new();
    }
    let wan = ((population as f64 / 10_000.0).round() as i64).max(1);
    format!("{wan} 万")
}

/// 城市当地日期相对本机本地日期：昨天 / 今天 / 明天
fn day_label(utc: DateTime<Utc>, tz: &str) -> &'static str {
    let Ok(tz_parsed) = tz.parse::<chrono_tz::Tz>() else {
        return "今天";
    };
    let city_date = utc.with_timezone(&tz_parsed).date_naive();
    let local_date = utc.with_timezone(&chrono::Local).date_naive();
    match (city_date - local_date).num_days() {
        -1 => "昨天",
        0 => "今天",
        1 => "明天",
        _ => "今天",
    }
}

/// 普通文字宽度（盒子宽度测量用；仅取 advance）
fn galley_width(ui: &egui::Ui, text: &str, font: &egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
        .rect
        .width()
}

impl crate::pages::Page for crate::pages::WorldPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        state.armed_delete_tick();
        let utc_now = Utc::now();
        let pins = state.world_pins_view().to_vec();
        let hero_name = resolve_hero(&pins, &state.world_hero_city);
        let hero = entry_for(state, &hero_name);
        let hero_h = hero.as_ref().map(|_| hero_height(ui)).unwrap_or(0.0);

        // 页脚高度：搜索字段 + footer_gap + 搜索按钮 + footer_bottom（20）
        let footer_h = world_footer_height(ui);
        let avail = ui.max_rect();
        let body_h = (avail.height() - footer_h - crate::theme::layout_f32("footer_gap")).max(0.0);
        let body_rect = egui::Rect::from_min_size(avail.min, egui::vec2(avail.width(), body_h));
        let footer_rect = egui::Rect::from_min_size(
            egui::pos2(
                avail.left(),
                body_rect.bottom() + crate::theme::layout_f32("footer_gap"),
            ),
            egui::vec2(avail.width(), footer_h),
        );

        let mut list_rect = body_rect;
        if let Some(entry) = &hero {
            let hero_rect = egui::Rect::from_min_size(
                body_rect.min,
                egui::vec2(body_rect.width(), hero_h.min(body_rect.height())),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(hero_rect), |ui| {
                draw_hero(ui, hero_rect, entry, utc_now, &ink, state);
            });
            let top = hero_rect.bottom() + box_stack::BOX_GAP;
            list_rect = egui::Rect::from_min_max(
                egui::pos2(body_rect.left(), top),
                egui::pos2(body_rect.right(), body_rect.bottom()),
            );
        }

        let others: Vec<WorldCityEntry> = pins
            .iter()
            .filter(|name| *name != &hero_name)
            .filter_map(|name| entry_for(state, name))
            .collect();

        if list_rect.height() > crate::theme::layout_f64("list_visible_eps") {
            if others.is_empty() {
                ui.scope_builder(egui::UiBuilder::new().max_rect(list_rect), |ui| {
                    if list_rect.height() >= crate::widgets::empty::min_height(ui) {
                        crate::widgets::empty::card_at(
                            ui,
                            list_rect,
                            "还没有其他城市",
                            "从下方搜索并添加城市，hero 不参与列表",
                        );
                    }
                });
            } else {
                list_scroll::show(
                    ui,
                    list_rect,
                    "world_list",
                    "LAYOUT_WORLD_LIST",
                    &mut self.list_scroll,
                    |ui| {
                        let mut stack = box_stack::BoxStack::new(ui);
                        let pulse = motion::list_pulse(state, "world");
                        let pulse_base = crate::theme::layout_f64("list_pulse_base");
                        let pulse_span = crate::theme::layout_f64("list_pulse_span");
                        let last = others.len().saturating_sub(1);
                        for (i, entry) in others.iter().enumerate() {
                            let row_h = city_row_height(ui);
                            let rect = stack.next(ui, row_h);
                            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                if i == last && pulse < 1.0 {
                                    ui.set_opacity(pulse_base + pulse_span * pulse);
                                }
                                draw_city_row(ui, rect, state, bus, entry, &pins, utc_now, &ink);
                            });
                        }
                    },
                );
            }
        }

        let label_box = ui
            .scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
                draw_footer(ui, footer_rect, state, bus)
            })
            .inner;
        draw_search_results(ui, footer_rect, label_box.top(), state, bus, &ink);
    }
}

/// hero 城市：配置值仍在 pins 中则用；否则第一项；空列表返回空串
fn resolve_hero(pins: &[String], configured: &str) -> String {
    if pins.iter().any(|p| p == configured) {
        configured.to_string()
    } else {
        pins.first().cloned().unwrap_or_default()
    }
}

/// 城市名 → 条目（内置 50 城优先，其次 SQLite 自定义城市快照）
fn entry_for(state: &UiState, name: &str) -> Option<WorldCityEntry> {
    world_search::builtin_entry(name).or_else(|| state.world_custom_cities.get(name).cloned())
}

/// hero 盒高：上下 12 + 两行 + 行间 8
fn hero_height(ui: &egui::Ui) -> f32 {
    let name_font = crate::theme::font_semibold(crate::theme::font_size("body"));
    let time_font = digits::digit_font(crate::theme::font_size("display"));
    let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let line1 = crate::widgets::text_metrics::height(ui.ctx(), &name_font, InkKind::Cjk).max(
        crate::widgets::text_metrics::height(ui.ctx(), &time_font, InkKind::Digits),
    );
    let line2 = crate::widgets::text_metrics::height(ui.ctx(), &meta_font, InkKind::Cjk);
    crate::theme::layout_f32("hero_pad_y") * 2.0
        + line1
        + crate::theme::layout_f32("hero_line_gap")
        + line2
}

/// hero：第一行城市名 + 大号 HH:MM:SS；第二行日期（周几）+ 时区偏移
fn draw_hero(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    entry: &WorldCityEntry,
    utc_now: DateTime<Utc>,
    ink: &crate::theme::Ink,
    state: &UiState,
) {
    let pulse = motion::list_pulse(state, "world_hero");
    let painter = ui.painter().with_clip_rect(rect);
    let _ = ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        crate::layout_probe::rect(ui, "world.hero", rect);
        let name_font = crate::theme::font_semibold(crate::theme::font_size("body"));
        let time_font = digits::digit_font(crate::theme::font_size("display"));
        let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
        let pad_x = crate::theme::layout_f32("hero_pad_x");
        let pad_y = crate::theme::layout_f32("hero_pad_y");
        let line_gap = crate::theme::layout_f32("hero_line_gap");
        let min_w = crate::theme::layout_f32("hero_min_w");
        let text_gap = crate::theme::spacing_f32("sm");
        let pulse_base = crate::theme::layout_f64("list_pulse_base");
        let pulse_span = crate::theme::layout_f64("list_pulse_span");
        ui.set_opacity(pulse_base + pulse_span * pulse);
        let walk = now_in_tz(utc_now, &entry.tz)
            .map(|(t, _)| t)
            .unwrap_or_else(|| "--:--:--".to_string());
        let walk_w = digits::text_width(ui.painter(), &walk, &time_font);
        let name_h = crate::widgets::text_metrics::height(ui.ctx(), &name_font, InkKind::Cjk);
        let time_h = crate::widgets::text_metrics::height(ui.ctx(), &time_font, InkKind::Digits);
        let line1_bottom = rect.top() + pad_y + name_h.max(time_h);

        painter.rect_filled(
            rect,
            egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
            crate::theme::card_bg(),
        );
        painter.rect_stroke(
            rect,
            egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
            egui::Stroke::new(
                crate::theme::layout_f64("hero_border"),
                crate::theme::accent_color(),
            ),
            egui::StrokeKind::Inside,
        );

        // 时间盒：第一行右端，盒底 = line1_bottom；名称盒右边界止于时间盒
        let time_box = egui::Rect::from_min_size(
            egui::pos2(rect.right() - pad_x - walk_w, line1_bottom - time_h),
            egui::vec2(walk_w, time_h),
        );
        let _ = digits::draw_bottom(
            ui.painter(),
            time_box.bottom(),
            time_box.center().x,
            &walk,
            &time_font,
            ink.strong,
        );
        // 名称盒：盒高 = 文字盒高，盒底与时间盒盒底对齐；右边界用时间盒左缘
        let name_box = egui::Rect::from_min_size(
            egui::pos2(rect.left() + pad_x, line1_bottom - name_h),
            egui::vec2(
                (time_box.left() - text_gap - (rect.left() + pad_x)).max(min_w),
                name_h,
            ),
        );
        let name_ink = text_ink::measure(ui, &entry.name, &name_font, InkKind::Cjk);
        ui.painter().text(
            egui::pos2(name_box.left(), name_ink.top_for_bottom(name_box.bottom())),
            egui::Align2::LEFT_TOP,
            &entry.name,
            name_font,
            ink.strong,
        );

        let Ok(tz) = entry.tz.parse::<chrono_tz::Tz>() else {
            return;
        };
        let date = utc_now.with_timezone(&tz);
        let date_text = format!("{} {}", date.format("%Y-%m-%d"), weekday_cn(date.weekday()));
        let tz_text = tz_offset_label(utc_now, &entry.tz).unwrap_or_else(|| "-- 时区".to_string());
        let meta_h = crate::widgets::text_metrics::height(ui.ctx(), &meta_font, InkKind::Cjk);
        let line2_bottom = line1_bottom + line_gap + meta_h;
        let tz_w = ui
            .painter()
            .layout_no_wrap(tz_text.clone(), meta_font.clone(), ink.meta)
            .rect
            .width();
        let tz_box = egui::Rect::from_min_size(
            egui::pos2(rect.right() - pad_x - tz_w, line2_bottom - meta_h),
            egui::vec2(tz_w, meta_h),
        );
        let date_box = egui::Rect::from_min_size(
            egui::pos2(rect.left() + pad_x, line2_bottom - meta_h),
            egui::vec2(
                (tz_box.left() - text_gap - (rect.left() + pad_x)).max(min_w),
                meta_h,
            ),
        );
        let date_ink = text_ink::measure(ui, &date_text, &meta_font, InkKind::Cjk);
        ui.painter().text(
            egui::pos2(date_box.left(), date_ink.top_for_bottom(date_box.bottom())),
            egui::Align2::LEFT_TOP,
            &date_text,
            meta_font.clone(),
            ink.muted,
        );
        let tz_ink = text_ink::measure(ui, &tz_text, &meta_font, InkKind::Cjk);
        ui.painter().text(
            egui::pos2(tz_box.left(), tz_ink.top_for_bottom(tz_box.bottom())),
            egui::Align2::LEFT_TOP,
            &tz_text,
            meta_font,
            ink.meta,
        );
    });
}

/// 卡片盒尺寸：全部由内容盒 + layout token 推导，不写固定行高
struct CardMetrics {
    name_font: egui::FontId,
    en_font: egui::FontId,
    day_font: egui::FontId,
    walk_font: egui::FontId,
    name_h: f32,
    en_h: f32,
    day_h: f32,
    walk_h: f32,
    row1_h: f32,
    row2_h: f32,
    pad_x: f32,
    pad_y: f32,
    line_gap: f32,
    text_gap: f32,
    icon_size: f32,
    icon_gap: f32,
    focus_size: f32,
    focus_radius: f32,
    focus_stroke: f32,
    focus_inset: f32,
}

/// 世界卡片度量：字号取六档，盒高取 text_metrics，间距全走 token
fn card_metrics(ui: &egui::Ui) -> CardMetrics {
    let name_font = crate::theme::font_semibold(crate::theme::font_size("body"));
    let en_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let day_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let walk_font = digits::digit_font(crate::theme::font_size("title"));
    let name_h = crate::widgets::text_metrics::height(ui.ctx(), &name_font, InkKind::Cjk);
    let en_h = crate::widgets::text_metrics::height(ui.ctx(), &en_font, InkKind::Cjk);
    let day_h = crate::widgets::text_metrics::height(ui.ctx(), &day_font, InkKind::Cjk);
    let walk_h = crate::widgets::text_metrics::height(ui.ctx(), &walk_font, InkKind::Digits);
    let row1_h = name_h.max(en_h);
    let icon_size = crate::theme::layout_f32("card_icon_size");
    let focus_size = crate::theme::layout_f32("card_focus_size");
    let row2_h = walk_h.max(day_h).max(icon_size).max(focus_size);
    CardMetrics {
        name_font,
        en_font,
        day_font,
        walk_font,
        name_h,
        en_h,
        day_h,
        walk_h,
        row1_h,
        row2_h,
        pad_x: crate::theme::layout_f32("card_pad_x"),
        pad_y: crate::theme::layout_f32("card_pad_y"),
        line_gap: crate::theme::layout_f32("card_line_gap"),
        text_gap: crate::theme::layout_f32("card_text_gap"),
        icon_size,
        icon_gap: crate::theme::layout_f32("card_icon_gap"),
        focus_size,
        focus_radius: crate::theme::layout_f32("card_focus_radius"),
        focus_stroke: crate::theme::layout_f64("card_focus_stroke"),
        focus_inset: crate::theme::layout_f32("card_focus_inset"),
    }
}

/// 城市行高：上下 padding + 第一行 + 行距 + 第二行，全部由内容盒推导
fn city_row_height(ui: &egui::Ui) -> f32 {
    let m = card_metrics(ui);
    2.0 * m.pad_y + m.row1_h + m.line_gap + m.row2_h
}

/// 城市行：第一行中文+英文居中；第二行昨天/今天/明天 + 时间偏移组居中 + pin/Trash
#[allow(clippy::too_many_arguments)]
fn draw_city_row(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut UiState,
    bus: &crate::bus::Bus,
    entry: &WorldCityEntry,
    pins: &[String],
    utc_now: DateTime<Utc>,
    ink: &crate::theme::Ink,
) {
    let name = entry.name.as_str();
    let hit = ui.interact(
        rect,
        egui::Id::unique(("world_row", name)),
        egui::Sense::click(),
    );
    let walk = now_in_tz(utc_now, &entry.tz)
        .map(|(t, _)| t)
        .unwrap_or_else(|| "--:--:--".to_string());
    let hover_t = motion::hover_t(
        ui.ctx(),
        egui::Id::unique(("world_row", name)),
        hit.hovered() || hit.has_focus(),
        crate::theme::motion_ms("card_hover_ms"),
    );
    ui.painter().rect_filled(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        motion::lerp_color(crate::theme::card_bg(), crate::theme::hover_bg(), hover_t),
    );
    ui.painter().rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        egui::Stroke::new(
            crate::theme::layout_f64("card_border"),
            motion::lerp_color(
                crate::theme::border_color(),
                crate::theme::border_strong_color(),
                hover_t,
            ),
        ),
        egui::StrokeKind::Inside,
    );

    let m = card_metrics(ui);
    let row1_bottom = rect.top() + m.pad_y + m.row1_h;
    let row2_top = row1_bottom + m.line_gap;
    let row2_center = row2_top + m.row2_h / 2.0;

    // 第一行：中文 + 英文同一行，整体水平居中；两文字盒盒底对齐
    let name_w = galley_width(ui, name, &m.name_font);
    let en_text = entry.name_en.as_str();
    let en_w = if en_text.is_empty() {
        0.0
    } else {
        galley_width(ui, en_text, &m.en_font)
    };
    let name_gap = if en_text.is_empty() { 0.0 } else { m.text_gap };
    let group_w = name_w + name_gap + en_w;
    let group_left = rect.center().x - group_w / 2.0;
    let name_box = egui::Rect::from_min_size(
        egui::pos2(group_left, row1_bottom - m.name_h),
        egui::vec2(name_w, m.name_h),
    );
    let name_ink = text_ink::measure(ui, name, &m.name_font, InkKind::Cjk);
    ui.painter().text(
        egui::pos2(name_box.left(), name_ink.top_for_bottom(name_box.bottom())),
        egui::Align2::LEFT_TOP,
        name,
        m.name_font.clone(),
        ink.strong,
    );
    if !en_text.is_empty() {
        let en_box = egui::Rect::from_min_size(
            egui::pos2(name_box.right() + name_gap, row1_bottom - m.en_h),
            egui::vec2(en_w, m.en_h),
        );
        let en_ink = text_ink::measure(ui, en_text, &m.en_font, InkKind::Cjk);
        ui.painter().text(
            egui::pos2(en_box.left(), en_ink.top_for_bottom(en_box.bottom())),
            egui::Align2::LEFT_TOP,
            en_text,
            m.en_font.clone(),
            ink.meta,
        );
    }

    // 第二行右端：图标组右对齐到卡片内边距，组内 pin → gap → 删除；
    // 交互盒就是图标盒，不再另设命中盒
    let icon_group_w = 2.0 * m.icon_size + m.icon_gap;
    let icon_group_left = rect.right() - m.pad_x - icon_group_w;
    let icon_top = row2_center - m.icon_size / 2.0;
    let pin_box = egui::Rect::from_min_size(
        egui::pos2(icon_group_left, icon_top),
        egui::vec2(m.icon_size, m.icon_size),
    );
    let del_box = egui::Rect::from_min_size(
        egui::pos2(pin_box.right() + m.icon_gap, icon_top),
        egui::vec2(m.icon_size, m.icon_size),
    );

    // 第二行左端：昨天/今天/明天，垂直居中第二行
    let day_text = day_label(utc_now, &entry.tz);
    let day_w = galley_width(ui, day_text, &m.day_font);
    let day_box = egui::Rect::from_min_size(
        egui::pos2(rect.left() + m.pad_x, row2_center - m.day_h / 2.0),
        egui::vec2(day_w, m.day_h),
    );
    let day_ink = text_ink::measure(ui, day_text, &m.day_font, InkKind::Cjk);
    ui.painter().text(
        egui::pos2(day_box.left(), day_ink.top_for_bottom(day_box.bottom())),
        egui::Align2::LEFT_TOP,
        day_text,
        m.day_font.clone(),
        ink.meta,
    );

    // 第二行中间：时间 + 偏移作为一组，水平中线对卡片中线，组内双轴居中
    let walk_w = digits::text_width(ui.painter(), &walk, &m.walk_font);
    let offset_text = tz_offset_value(utc_now, &entry.tz).unwrap_or_else(|| "--".to_string());
    let offset_w = galley_width(ui, &offset_text, &m.day_font);
    let offset_gap = m.text_gap;
    let time_offset_w = walk_w + offset_gap + offset_w;
    let time_offset_h = m.walk_h.max(m.day_h);
    let group_left = rect.center().x - time_offset_w / 2.0;
    let group_bottom = row2_center + time_offset_h / 2.0;
    let time_box = egui::Rect::from_min_size(
        egui::pos2(group_left, group_bottom - m.walk_h),
        egui::vec2(walk_w, m.walk_h),
    );
    digits::draw_bottom(
        ui.painter(),
        time_box.bottom(),
        time_box.center().x,
        &walk,
        &m.walk_font,
        ink.body,
    );
    let offset_box = egui::Rect::from_min_size(
        egui::pos2(time_box.right() + offset_gap, group_bottom - m.day_h),
        egui::vec2(offset_w, m.day_h),
    );
    let offset_ink = text_ink::measure(ui, &offset_text, &m.day_font, InkKind::Cjk);
    ui.painter().text(
        egui::pos2(
            offset_box.left(),
            offset_ink.top_for_bottom(offset_box.bottom()),
        ),
        egui::Align2::LEFT_TOP,
        &offset_text,
        m.day_font.clone(),
        ink.meta,
    );

    // Pin：交互盒 = 图标盒
    let pin = ui.interact(
        pin_box,
        egui::Id::unique(("world_pin", name)),
        egui::Sense::click(),
    );
    let pin_hover = motion::hover_t(
        ui.ctx(),
        egui::Id::unique(("world_pin_hover", name)),
        pin.hovered() || pin.has_focus(),
        crate::theme::motion_ms("card_hover_ms"),
    );
    icons::draw(
        ui.painter(),
        pin_box.center(),
        m.icon_size,
        icons::Icon::Pin,
        motion::lerp_color(ink.meta, ink.strong, pin_hover),
    );
    if pin.has_focus() {
        let focus_box =
            egui::Rect::from_center_size(pin_box.center(), egui::vec2(m.focus_size, m.focus_size));
        ui.painter().rect_stroke(
            focus_box,
            egui::epaint::CornerRadius::same(m.focus_radius as u8),
            egui::Stroke::new(m.focus_stroke, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    let pin = pin
        .on_hover_text("固定到顶部 hero")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if pin.clicked() {
        bus.send(FrontendCommand::SetWorldHero(name.to_string()));
        state.world_hero_city = name.to_string();
        state.list_pulse = Some(("world_hero".to_string(), std::time::Instant::now()));
    }

    // 删除：Trash（不是减号），两步确认
    if pins.len() > 1 {
        let del_key = format!("world:{name}");
        let armed = matches!(&state.armed_delete, Some((k, _)) if *k == del_key);
        let del = ui.interact(
            del_box,
            egui::Id::unique(("world_del", name)),
            egui::Sense::click(),
        );
        let del_hover = del.hovered();
        icons::draw(
            ui.painter(),
            del_box.center(),
            m.icon_size,
            if armed {
                icons::Icon::Check
            } else {
                icons::Icon::Trash
            },
            if armed || del_hover {
                ink.danger
            } else {
                ink.meta
            },
        );
        if del.has_focus() {
            let focus_box = egui::Rect::from_center_size(
                del_box.center(),
                egui::vec2(m.focus_size, m.focus_size),
            );
            ui.painter().rect_stroke(
                focus_box,
                egui::epaint::CornerRadius::same(m.focus_radius as u8),
                egui::Stroke::new(m.focus_stroke, ink.knob),
                egui::StrokeKind::Inside,
            );
        }
        let del = del
            .on_hover_text(if armed {
                "再次点击确认移除"
            } else {
                "移除城市"
            })
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if del.clicked() {
            if armed {
                bus.send(FrontendCommand::RemoveWorldCity(name.to_string()));
                state.armed_delete = None;
            } else {
                state.armed_delete = Some((del_key, std::time::Instant::now()));
            }
        }
    }

    if hit.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(m.focus_inset),
            egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
            egui::Stroke::new(m.focus_stroke, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
}

/// 弹层单行文字：固定盒高，按盒底画字（弹层同样不用原生 label）
fn popup_text_row(
    ui: &mut egui::Ui,
    height: f32,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    let ink = text_ink::measure(ui, text, &font, InkKind::Cjk);
    let text_h = crate::widgets::text_metrics::height(ui.ctx(), &font, InkKind::Cjk);
    let pad_x = crate::theme::layout_f32("search_popup_pad_x");
    let text_box = egui::Rect::from_min_size(
        egui::pos2(rect.left() + pad_x, rect.center().y - text_h / 2.0),
        egui::vec2((rect.width() - 2.0 * pad_x).max(1.0), text_h),
    );
    ui.painter().text(
        egui::pos2(text_box.left(), ink.top_for_bottom(text_box.bottom())),
        egui::Align2::LEFT_TOP,
        text,
        font,
        color,
    );
    resp
}

/// 页脚高度：搜索字段 + footer_gap + 搜索按钮 + footer_bottom
fn world_footer_height(ui: &egui::Ui) -> f32 {
    field::fixed_height(ui)
        + crate::theme::layout_f32("footer_gap")
        + primary_button::height(ui)
        + crate::theme::layout_f32("footer_bottom")
}

/// 底部搜索：字段 + 搜索按钮（照抄倒计时页脚三段）；返回按钮盒供弹层锚定
fn draw_footer(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut UiState,
    bus: &crate::bus::Bus,
) -> egui::Rect {
    // 与倒计时页脚同规则：清零 egui 隐式纵向间距，否则默认 3px 会吃掉 footer_bottom
    ui.spacing_mut().item_spacing.y = 0.0;
    crate::layout_probe::rect(ui, "world.footer", rect);
    let mut query = state.world_search_query.clone();
    let (field_resp, label_box) = field::show_fixed_with_label(
        ui,
        "添加城市",
        "中文 / 英文 / 拼音",
        &mut query,
        rect.width(),
    );
    if query != state.world_search_query {
        state.world_search_query = query;
        state.world_search_results.clear();
        state.world_search_error = None;
        state.world_searching = false;
    }
    ui.add_space(crate::theme::layout_f32("footer_gap"));
    // 与倒计时“添加”同尺寸：button_width 宽，右对齐
    let button_w = crate::theme::layout_f32("button_width");
    let button = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space((rect.width() - button_w).max(0.0));
            primary_button::show(ui, "搜索", button_w)
        })
        .inner;
    crate::layout_probe::rect(ui, "world.footer_button", button.rect);
    let enter = field_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    if button.clicked() || enter {
        start_search(state, bus);
    }
    label_box
}

/// 发起搜索（本地/缓存/geocoding 由 core 决定）
fn start_search(state: &mut UiState, bus: &crate::bus::Bus) {
    let query = state.world_search_query.trim().to_string();
    if query.is_empty() {
        return;
    }
    state.world_search_query = query.clone();
    state.world_search_results.clear();
    state.world_search_error = None;
    state.world_searching = true;
    state.world_search_seq = state.world_search_seq.wrapping_add(1);
    bus.send(FrontendCommand::SearchWorldCity(query));
}

/// 搜索候选弹层：底部页脚上方向上展开；多结果点选后添加
fn draw_search_results(
    ui: &egui::Ui,
    footer_rect: egui::Rect,
    anchor_top: f32,
    state: &mut UiState,
    bus: &crate::bus::Bus,
    ink: &crate::theme::Ink,
) {
    let has_results = !state.world_search_results.is_empty();
    let has_error = state.world_search_error.is_some();
    if !has_results && !has_error && !state.world_searching {
        return;
    }
    let now = Utc::now();
    let pad_x = crate::theme::layout_f32("search_popup_pad_x");
    let pad_y = crate::theme::layout_f32("search_popup_pad_y");
    let text_gap = crate::theme::layout_f32("search_popup_text_gap");
    let item_gap = crate::theme::layout_f32("search_popup_item_gap");
    let min_w = crate::theme::layout_f32("search_popup_min_w");
    let status_h = crate::theme::layout_f32("search_popup_status_h");
    let max_results = crate::theme::layout_i64("search_popup_max_results").max(1) as usize;
    let gap = crate::theme::layout_f32("search_popup_footer_gap");
    let row_radius = crate::theme::layout_f32("search_row_radius");
    let title_font = crate::theme::font_medium(crate::theme::font_size("body"));
    let sub_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let time_font = crate::widgets::digits::digit_font(crate::theme::font_size("body"));
    let title_h = crate::widgets::text_metrics::height(ui.ctx(), &title_font, InkKind::Cjk);
    let sub_h = crate::widgets::text_metrics::height(ui.ctx(), &sub_font, InkKind::Cjk);
    let time_h = crate::widgets::text_metrics::height(ui.ctx(), &time_font, InkKind::Digits);
    let item_h = title_h + text_gap + sub_h;
    let row_h = item_h + 2.0 * pad_y;
    let status_row_h = status_h + 2.0 * pad_y;
    let result_count = state.world_search_results.len().min(max_results);
    // 最多两行结果：单条按单条高，两条按两条高，超过两条固定两条高并滚动
    let visible = result_count.min(2);
    let content_h = if has_results {
        if visible == 0 {
            0.0
        } else {
            visible as f32 * row_h + visible.saturating_sub(1) as f32 * item_gap
        }
    } else {
        status_row_h
    };
    let menu_h = content_h;
    let width = footer_rect.width().max(min_w);
    let inner_w = width;
    let y = anchor_top - gap - menu_h;
    let mut picked: Option<(String, bool)> = None;
    let mut close = false;
    egui::Area::new(egui::Id::unique("world_search_results"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(footer_rect.left(), y))
        .show(ui.ctx(), |ui| {
            ui.set_opacity(crate::theme::layout_f64("search_popup_opacity"));
            egui::Frame::new()
                .fill(crate::theme::card_bg())
                .stroke(egui::Stroke::new(
                    crate::theme::layout_f64("card_border"),
                    crate::theme::border_strong_color(),
                ))
                .corner_radius(egui::epaint::CornerRadius::same(
                    crate::theme::radius_px("sm") as u8,
                ))
                .inner_margin(egui::Margin::ZERO)
                .show(ui, |ui| {
                    ui.set_width(inner_w);
                    ui.set_height(content_h);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if state.world_searching {
                        let font = crate::theme::font_regular(crate::theme::font_size("body"));
                        popup_text_row(ui, status_row_h, "搜索中…", font, ink.muted);
                        return;
                    }
                    if let Some(err) = &state.world_search_error {
                        let font = crate::theme::font_regular(crate::theme::font_size("body"));
                        let resp = popup_text_row(ui, status_row_h, err, font, ink.danger)
                            .on_hover_text("点击关闭");
                        if resp.clicked() {
                            close = true;
                        }
                        return;
                    }
                    let draw_item = |ui: &mut egui::Ui, entry: &WorldCityEntry| -> bool {
                        let (row, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), row_h),
                            egui::Sense::click(),
                        );
                        if resp.hovered() {
                            ui.painter().rect_filled(
                                row,
                                egui::epaint::CornerRadius::same(row_radius as u8),
                                crate::theme::hover_bg(),
                            );
                        }
                        let title_box = egui::Rect::from_min_size(
                            egui::pos2(row.left() + pad_x, row.top() + pad_y),
                            egui::vec2(row.width() - 2.0 * pad_x, title_h),
                        );
                        let sub_box = egui::Rect::from_min_size(
                            egui::pos2(title_box.left(), title_box.bottom() + text_gap),
                            egui::vec2(title_box.width(), sub_h),
                        );
                        let title_ink =
                            text_ink::measure(ui, &entry.name, &title_font, InkKind::Cjk);
                        ui.painter().text(
                            egui::pos2(
                                title_box.left(),
                                title_ink.top_for_bottom(title_box.bottom()),
                            ),
                            egui::Align2::LEFT_TOP,
                            &entry.name,
                            title_font.clone(),
                            ink.strong,
                        );
                        // 第一行右端：当地时间，字号与中文城市名一致
                        let time_text = now_in_tz(now, &entry.tz)
                            .map(|(t, _)| t)
                            .unwrap_or_else(|| "--:--:--".to_string());
                        let time_w = digits::text_width(ui.painter(), &time_text, &time_font);
                        let time_box = egui::Rect::from_min_size(
                            egui::pos2(title_box.right() - time_w, title_box.bottom() - time_h),
                            egui::vec2(time_w, time_h),
                        );
                        // 时间左侧 5px：昨天/今天/明天（meta 12）
                        let day_text = day_label(now, &entry.tz);
                        let day_w = galley_width(ui, day_text, &sub_font);
                        let day_gap = crate::theme::layout_f32("search_popup_day_gap");
                        let day_box = egui::Rect::from_min_size(
                            egui::pos2(
                                time_box.left() - day_gap - day_w,
                                title_box.bottom() - sub_h,
                            ),
                            egui::vec2(day_w, sub_h),
                        );
                        let day_ink = text_ink::measure(ui, day_text, &sub_font, InkKind::Cjk);
                        ui.painter().text(
                            egui::pos2(day_box.left(), day_ink.top_for_bottom(day_box.bottom())),
                            egui::Align2::LEFT_TOP,
                            day_text,
                            sub_font.clone(),
                            ink.meta,
                        );
                        digits::draw_bottom(
                            ui.painter(),
                            time_box.bottom(),
                            time_box.center().x,
                            &time_text,
                            &time_font,
                            ink.body,
                        );
                        // 英文名跟在中文后，间距 10px；不得压到右侧日期/时间
                        let show_en = !entry.name_en.is_empty() && entry.name_en != entry.pinyin;
                        if show_en {
                            let name_w = galley_width(ui, &entry.name, &title_font);
                            let en_w = galley_width(ui, &entry.name_en, &sub_font);
                            let name_gap = crate::theme::layout_f32("search_popup_name_gap");
                            if title_box.left() + name_w + name_gap + en_w
                                <= day_box.left() - name_gap
                            {
                                let en_x = title_box.left() + name_w + name_gap;
                                let en_ink =
                                    text_ink::measure(ui, &entry.name_en, &sub_font, InkKind::Cjk);
                                ui.painter().text(
                                    egui::pos2(en_x, en_ink.top_for_bottom(title_box.bottom())),
                                    egui::Align2::LEFT_TOP,
                                    &entry.name_en,
                                    sub_font.clone(),
                                    ink.meta,
                                );
                            }
                        }
                        let parts: Vec<String> = {
                            let continent = accel_core::data::continents::continent_label(
                                &entry.country_code,
                                &entry.tz,
                            );
                            let offset = tz_offset_value(now, &entry.tz).unwrap_or_default();
                            let population = population_label(entry.population);
                            let mut parts: Vec<String> = Vec::new();
                            if !continent.is_empty() {
                                parts.push(continent.to_string());
                            }
                            if !entry.country.is_empty() {
                                parts.push(entry.country.clone());
                            }
                            if !entry.admin1.is_empty() {
                                parts.push(entry.admin1.clone());
                            }
                            if !offset.is_empty() {
                                parts.push(offset);
                            }
                            if !population.is_empty() {
                                parts.push(population);
                            }
                            parts
                        };
                        if !parts.is_empty() {
                            let dot = "·";
                            let dot_w = galley_width(ui, dot, &sub_font);
                            let widths: Vec<f32> = parts
                                .iter()
                                .map(|p| galley_width(ui, p, &sub_font))
                                .collect();
                            let sep_gap = crate::theme::layout_f32("search_popup_sep_gap");
                            let total_width = |gap: f32| {
                                widths.iter().sum::<f32>()
                                    + (parts.len().saturating_sub(1)) as f32 * (2.0 * gap + dot_w)
                            };
                            // `·` 左右各 gap；整行放不下时从 5px 递减到 0
                            let mut gap = sep_gap;
                            while gap > 0.0 && total_width(gap) > sub_box.width() {
                                gap -= 1.0;
                            }
                            let mut x = sub_box.left();
                            for (i, part) in parts.iter().enumerate() {
                                let part_ink = text_ink::measure(ui, part, &sub_font, InkKind::Cjk);
                                ui.painter().text(
                                    egui::pos2(x, part_ink.top_for_bottom(sub_box.bottom())),
                                    egui::Align2::LEFT_TOP,
                                    part,
                                    sub_font.clone(),
                                    ink.meta,
                                );
                                x += widths[i];
                                if i + 1 < parts.len() {
                                    x += gap;
                                    let dot_ink =
                                        text_ink::measure(ui, dot, &sub_font, InkKind::Cjk);
                                    ui.painter().text(
                                        egui::pos2(x, dot_ink.top_for_bottom(sub_box.bottom())),
                                        egui::Align2::LEFT_TOP,
                                        dot,
                                        sub_font.clone(),
                                        ink.meta,
                                    );
                                    x += dot_w + gap;
                                }
                            }
                        }
                        resp.clicked()
                    };
                    if result_count <= 2 {
                        for (i, entry) in state
                            .world_search_results
                            .iter()
                            .take(result_count)
                            .enumerate()
                        {
                            if i > 0 {
                                ui.add_space(item_gap);
                            }
                            if draw_item(ui, entry) {
                                picked = Some((
                                    entry.name.clone(),
                                    world_search::builtin_entry(&entry.name).is_some(),
                                ));
                            }
                        }
                    } else {
                        let scroll = &mut ui.style_mut().spacing.scroll;
                        scroll.floating = false;
                        scroll.bar_width = 4.0;
                        scroll.bar_inner_margin = 2.0;
                        scroll.bar_outer_margin = 2.0;
                        scroll.foreground_color = true;
                        egui::ScrollArea::vertical()
                            .max_height(content_h)
                            .auto_shrink([false, false])
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                            )
                            .id_salt((
                                "world_search_results_scroll",
                                state.world_search_seq,
                                state.world_search_query.clone(),
                            ))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                for (i, entry) in state
                                    .world_search_results
                                    .iter()
                                    .take(result_count)
                                    .enumerate()
                                {
                                    if i > 0 {
                                        ui.add_space(item_gap);
                                    }
                                    if draw_item(ui, entry) {
                                        picked = Some((
                                            entry.name.clone(),
                                            world_search::builtin_entry(&entry.name).is_some(),
                                        ));
                                    }
                                }
                            });
                    }
                });
        });
    if close {
        state.world_search_results.clear();
        state.world_search_error = None;
        state.world_searching = false;
    }
    if let Some((name, built_in)) = picked {
        if built_in {
            bus.send(FrontendCommand::AddWorldCity(name));
        } else if let Some(entry) = state
            .world_search_results
            .iter()
            .find(|e| e.name == name)
            .cloned()
        {
            bus.send(FrontendCommand::AddWorldCityEntry(entry));
        }
        state.world_search_results.clear();
        state.world_search_error = None;
        state.world_searching = false;
        state.world_search_query.clear();
        state.list_pulse = Some(("world".to_string(), std::time::Instant::now()));
    }
}

// ===== src/pages/world.rs 说明区 =====
// 输入：world_pins + world_hero_city + world_custom_cities + 搜索候选
// 输出：固定 hero / 可滚动城市行 / 底部搜索 / SetWorldHero/AddWorldCity(Entry)/RemoveWorldCity
// 逻辑步骤：页脚固定高 → hero 固定头 → 其余城市进 list_scroll 视口 →
//   底部搜索字段/按钮 → 候选弹层点选添加
// 设计理由（PL028）：布局照抄倒计时；滚动套件共用 widgets::list_scroll；
//   hero 不参与列表；pin 只在 hover 行出现且位于删除左侧；hero 无替代不可删。
// 异常处理：无时区显示占位；无候选/错误显示弹层文本；hero 空回退首项。
// 关联依赖：data::world_cities / world_search / chrono-tz / widgets::{list_scroll,field,primary_button}
#[cfg(test)]
mod tests {
    use super::*;

    fn init_cities() {
        let dir = std::env::temp_dir().join(format!("accel_world_page_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = accel_core::world_search::WorldSearch::with_path(dir.join("world_cache.sqlite"));
    }

    #[test]
    fn default_pins_are_unique_city_names_with_walk() {
        init_cities();
        let pins = accel_core::static_config::Static::get().base_str_list("default_world_pins");
        assert!(pins.len() >= 4);
        let mut sorted = pins.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), pins.len());
        for name in &pins {
            let tz = accel_core::world_search::tz_of(name)
                .unwrap_or_else(|| panic!("默认常驻应为表内城市: {name}"));
            let (text, _) = now_in_tz(chrono::Utc::now(), tz)
                .unwrap_or_else(|| panic!("走时应可解析: {name} → {tz}"));
            assert_eq!(text.len(), 8, "走时应为 HH:MM:SS: {name} → {text}");
        }
    }

    #[test]
    fn tz_of_is_name_domain_only() {
        init_cities();
        assert_eq!(accel_core::world_search::tz_of("东京"), Some("Asia/Tokyo"));
        assert!(
            accel_core::world_search::tz_of("Asia/Tokyo").is_none(),
            "IANA 串不是城市名"
        );
    }

    #[test]
    fn hero_labels_use_local_offset_and_weekday() {
        let utc = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 11, 21, 12, 0, 0).unwrap();
        assert_eq!(
            tz_offset_label(utc, "Asia/Shanghai").as_deref(),
            Some("+8 时区")
        );
        assert_eq!(
            tz_offset_label(utc, "America/Los_Angeles").as_deref(),
            Some("-8 时区")
        );
        assert_eq!(weekday_cn(chrono::Weekday::Sat), "周六");
    }
}
