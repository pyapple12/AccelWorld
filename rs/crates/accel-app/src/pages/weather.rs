//! 天气页（PL029）：整页 ScrollArea；天气城市跟随世界页 hero，无城市下拉/刷新。
//! 第一块三列（城市/温度/图标）、三列指标、24 小时折线（3 小时标注）、七日范围条。

use crate::widgets::text_ink::{self, InkKind};
use crate::widgets::{digits, icons};
use accel_core::types::HourlyPoint;

/// 天气码 → 现象矢量图标（Open-Meteo WMO 码段映射；未知码回退云——纯函数可测）
pub fn icon_for_code(code: i64) -> icons::Icon {
    match code {
        0..=2 => icons::Icon::Sun,
        3 => icons::Icon::Cloud,
        45 | 48 => icons::Icon::Fog,
        51..=67 | 80..=82 => icons::Icon::Rain,
        71..=77 | 85 | 86 => icons::Icon::Snow,
        95..=99 => icons::Icon::Storm,
        _ => icons::Icon::Cloud,
    }
}

impl crate::pages::Page for crate::pages::WeatherPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, _bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        let city = weather_city(state);
        // 加载期 0.72 透明度，结束后 160ms 淡入（减少动效直取）
        let loading = state.weather_loading.contains(&city);
        let fade = if crate::theme::reduce_motion(ui.ctx()) {
            1.0
        } else {
            ui.ctx()
                .animate_bool_with_time(egui::Id::unique("weather_fade"), !loading, 0.16)
        };
        ui.set_opacity(0.72 + 0.28 * fade);

        let error = state.weather_errors.get(&city).cloned();
        let data = state.weather.get(&city).cloned();
        let city_en = city_english_name(state, &city);

        let block_gap = crate::theme::layout_f32("page_box_gap");
        // 内容区顶已有 drawer_content_pad_top；补到 page_box_gap = 距页头 20
        let top_extra = (block_gap - crate::theme::layout_f32("drawer_content_pad_top")).max(0.0);
        let error_h = crate::widgets::text_metrics::height(
            ui.ctx(),
            &crate::theme::font_regular(crate::theme::font_size("meta")),
            InkKind::Cjk,
        );
        let empty_h = crate::theme::layout_f32("weather_empty_h");
        let hero_h = hero_height(ui);
        let metric_h = metrics_height(ui);
        // PL029：折线高度自适应——先算其余块占用的高度，剩余空间给图表
        // （世界页/倒计时页的列表视口同一逻辑：填满可用高度，窗口不足时保底滚动）
        let has_hourly = data.as_ref().is_some_and(|w| !w.hourly.is_empty());
        let week_days = data.as_ref().map(|w| w.daily.len().min(7)).unwrap_or(0);
        let week_h = week_height(ui, week_days);
        let meta_digit = digits::digit_font(crate::theme::font_size("meta"));
        let axis_h = crate::widgets::text_metrics::height(ui.ctx(), &meta_digit, InkKind::Digits);
        let chart_h = if has_hourly {
            let viewport_h = ui.available_height();
            let error_part = if error.is_some() {
                error_h + block_gap
            } else {
                0.0
            };
            let week_part = if week_days > 0 {
                block_gap + week_h
            } else {
                0.0
            };
            let used = top_extra
                + error_part
                + hero_h
                + block_gap
                + metric_h
                + block_gap
                + week_part
                + block_gap;
            (viewport_h - used).max(crate::theme::layout_f32("weather_plot_min_h") + axis_h)
        } else {
            0.0
        };

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // 内容区顶已有 drawer_content_pad_top；补足 page_box_gap 后，
                // 第一个块距页头正好 20px
                ui.add_space(top_extra);
                let mut stack = crate::widgets::box_stack::BoxStack::with_gap(ui, block_gap);

                if let Some(err) = &error {
                    let rect = stack.next(ui, error_h);
                    let font = crate::theme::font_regular(crate::theme::font_size("meta"));
                    let mut text = format!("更新失败：{err}");
                    if let Some(w) = &data {
                        text.push_str(&format!(" · 显示上次数据 {:.1}°", w.temperature));
                    }
                    let text = fit_text(ui, &text, &font, rect.width());
                    let text_ink = text_ink::measure(ui, &text, &font, InkKind::Cjk);
                    let painter = ui.painter().with_clip_rect(rect);
                    painter.text(
                        egui::pos2(rect.left(), text_ink.top_for_bottom(rect.bottom())),
                        egui::Align2::LEFT_TOP,
                        text,
                        font,
                        ink.danger,
                    );
                }

                let Some(w) = data else {
                    let rect = stack.next(ui, empty_h);
                    if loading {
                        crate::widgets::empty::card_at(
                            ui,
                            rect,
                            "天气获取中…",
                            "正在拉取 hero 城市天气；已有数据会保留到新数据到达",
                        );
                    } else {
                        crate::widgets::empty::card_at(
                            ui,
                            rect,
                            "还没有天气数据",
                            "世界页选中的 hero 城市会在此自动显示天气",
                        );
                    }
                    return;
                };

                // 第一块：城市 / 温度 / 图标（三列均分）
                let hero = stack.next(ui, hero_h);
                draw_weather_hero(ui, hero, &city, &city_en, &w, &ink);

                // 第二块：体感 / 湿度 / 风力（三列均分）
                let metrics = stack.next(ui, metric_h);
                draw_metrics(ui, metrics, &w, &ink);

                // 第三块：24 小时折线（高度 = 剩余可用空间，最低 weather_plot_min_h）
                if has_hourly {
                    let rect = stack.next(ui, chart_h);
                    draw_hourly(ui, rect, &w.hourly, &ink);
                }

                // 第四块：七日范围条（无数据来源行）
                if week_days > 0 {
                    let rect = stack.next(ui, week_h);
                    draw_week(ui, rect, &w.daily, &ink);
                }
            });
    }
}

/// 当前天气城市：世界页 hero；hero 尚未填充时回退持久化镜像 last_city
fn weather_city(state: &crate::bus::UiState) -> String {
    if state.world_hero_city.is_empty() {
        state.prefs.last_city.clone()
    } else {
        state.world_hero_city.clone()
    }
}

/// hero 城市英文名：内置 291 城从 SQLite 快照读 name_en，自定义城市读会话快照
fn city_english_name(state: &crate::bus::UiState, city: &str) -> String {
    if let Some(entry) = accel_core::world_search::by_name(city) {
        return entry.name_en.clone();
    }
    state
        .world_custom_cities
        .get(city)
        .map(|entry| entry.name_en.clone())
        .unwrap_or_default()
}

/// 第一块高度：上下 hero padding + 第一列第一行文字盒高 + 5 +
/// 第二列第二行文字盒高——图标与数字盒不参与定高
fn hero_height(ui: &egui::Ui) -> f32 {
    let title_font = crate::theme::font_medium(crate::theme::font_size("title"));
    let body_font = crate::theme::font_regular(crate::theme::font_size("body"));
    let row1 = crate::widgets::text_metrics::height(ui.ctx(), &title_font, InkKind::Cjk);
    let row2 = crate::widgets::text_metrics::height(ui.ctx(), &body_font, InkKind::Cjk);
    2.0 * crate::theme::layout_f32("hero_pad_y")
        + row1
        + crate::theme::layout_f32("weather_row_gap")
        + row2
}

/// 第二块高度：标签行 + 行间距 + 数值行
fn metrics_height(ui: &egui::Ui) -> f32 {
    let label_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let value_font = digits::digit_font(crate::theme::font_size("body"));
    let label_h = crate::widgets::text_metrics::height(ui.ctx(), &label_font, InkKind::Cjk);
    let value_h = crate::widgets::text_metrics::height(ui.ctx(), &value_font, InkKind::Mixed);
    label_h + crate::theme::layout_f32("weather_row_gap") + value_h
}

/// 七日块高度：标题 + 10px + N 行（行高取文字盒/图标盒较大者）+ (N−1) 个行间距
fn week_height(ui: &egui::Ui, days: usize) -> f32 {
    if days == 0 {
        return 0.0;
    }
    let title_font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let value_font = digits::digit_font(crate::theme::font_size("body"));
    let title_h = crate::widgets::text_metrics::height(ui.ctx(), &title_font, InkKind::Cjk);
    let text_h = crate::widgets::text_metrics::height(ui.ctx(), &value_font, InkKind::Digits);
    let row_h = text_h.max(crate::theme::layout_f32("weather_week_icon_size"));
    title_h
        + crate::theme::layout_f32("weather_week_title_gap")
        + days as f32 * row_h
        + days.saturating_sub(1) as f32 * crate::theme::spacing_f32("sm")
}

/// 三列均分：返回左/中/右三个等宽矩形（列内无内边距）
fn equal_columns(rect: egui::Rect) -> [egui::Rect; 3] {
    let w = rect.width() / 3.0;
    [
        egui::Rect::from_min_size(rect.min, egui::vec2(w, rect.height())),
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + w, rect.top()),
            egui::vec2(w, rect.height()),
        ),
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0 * w, rect.top()),
            egui::vec2(w, rect.height()),
        ),
    ]
}

/// 按列宽截断并补省略号（长城市名/英文名/描述不得压到相邻列）
fn fit_text(ui: &egui::Ui, text: &str, font: &egui::FontId, max_w: f32) -> String {
    if text.is_empty() {
        return String::new();
    }
    let width = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE)
            .rect
            .width()
    };
    if width(text) <= max_w {
        return text.to_owned();
    }
    let ell_w = width("…");
    let mut out = String::new();
    for ch in text.chars() {
        let mut probe = out.clone();
        probe.push(ch);
        if width(&probe) + ell_w > max_w {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

/// 第一块：左列城市/英文名左对齐，中列温度/描述中线对齐，右列图标盒右对齐并垂直居中整块
fn draw_weather_hero(
    ui: &egui::Ui,
    rect: egui::Rect,
    city: &str,
    city_en: &str,
    w: &accel_core::types::WeatherData,
    ink: &crate::theme::Ink,
) {
    let cols = equal_columns(rect);
    let gap = crate::theme::layout_f32("weather_row_gap");
    let title_font = crate::theme::font_medium(crate::theme::font_size("title"));
    let title_digit = digits::digit_font(crate::theme::font_size("title"));
    let body_font = crate::theme::font_regular(crate::theme::font_size("body"));
    let body_digit = digits::digit_font(crate::theme::font_size("body"));
    let icon_size = crate::theme::layout_f32("weather_icon_size");
    let pad_y = crate::theme::layout_f32("hero_pad_y");
    // 第一行高取第一列城市 CJK 盒；第二行高取第二列描述 CJK 盒（不取 max）
    let row1 = crate::widgets::text_metrics::height(ui.ctx(), &title_font, InkKind::Cjk);
    let row2 = crate::widgets::text_metrics::height(ui.ctx(), &body_font, InkKind::Cjk);
    let row1_bottom = rect.top() + pad_y + row1;
    let row2_bottom = row1_bottom + gap + row2;
    let painter = ui.painter();

    let city = fit_text(ui, city, &title_font, cols[0].width());
    let city_ink = text_ink::measure(ui, &city, &title_font, InkKind::Cjk);
    painter.text(
        egui::pos2(cols[0].center().x, city_ink.top_for_bottom(row1_bottom)),
        egui::Align2::CENTER_TOP,
        city,
        title_font,
        ink.strong,
    );

    let temp = format!("{:.1}°", w.temperature);
    digits::draw_bottom(
        painter,
        row1_bottom,
        cols[1].center().x,
        &temp,
        &title_digit,
        ink.strong,
    );

    if !city_en.is_empty() {
        let en = fit_text(ui, city_en, &body_digit, cols[0].width());
        let en_ink = text_ink::measure(ui, &en, &body_digit, InkKind::Latin);
        painter.text(
            egui::pos2(cols[0].center().x, en_ink.top_for_bottom(row2_bottom)),
            egui::Align2::CENTER_TOP,
            en,
            body_digit,
            ink.meta,
        );
    }

    let desc = fit_text(ui, &w.description, &body_font, cols[1].width());
    let desc_ink = text_ink::measure(ui, &desc, &body_font, InkKind::Cjk);
    painter.text(
        egui::pos2(cols[1].center().x, desc_ink.top_for_bottom(row2_bottom)),
        egui::Align2::CENTER_TOP,
        desc,
        body_font,
        ink.muted,
    );

    let icon_center = egui::pos2(cols[2].center().x, rect.center().y);
    icons::draw_ink_centered(
        ui,
        icon_center,
        icon_size,
        icon_for_code(w.weather_code),
        ink.muted,
    );
}

/// 第二块：体感/湿度/风力三列；标签中文 meta，数值 Barlow body；左/中/右对齐
fn draw_metrics(
    ui: &egui::Ui,
    rect: egui::Rect,
    w: &accel_core::types::WeatherData,
    ink: &crate::theme::Ink,
) {
    let cols = equal_columns(rect);
    let gap = crate::theme::layout_f32("weather_row_gap");
    let label_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let value_font = digits::digit_font(crate::theme::font_size("body"));
    let label_h = crate::widgets::text_metrics::height(ui.ctx(), &label_font, InkKind::Cjk);
    let value_h = crate::widgets::text_metrics::height(ui.ctx(), &value_font, InkKind::Mixed);
    let label_bottom = rect.top() + label_h;
    let value_bottom = label_bottom + gap + value_h;
    let painter = ui.painter();
    let metrics = [
        ("体感", format!("{:.1}°C", w.apparent_temperature)),
        ("湿度", format!("{:.0}%", w.humidity)),
        ("风力", format!("{:.1}km/h", w.wind_speed)),
    ];
    for (i, (label, value)) in metrics.iter().enumerate() {
        let col = cols[i];
        let label_ink = text_ink::measure(ui, label, &label_font, InkKind::Cjk);
        painter.text(
            egui::pos2(col.center().x, label_ink.top_for_bottom(label_bottom)),
            egui::Align2::CENTER_TOP,
            label,
            label_font.clone(),
            ink.muted,
        );
        digits::draw_bottom(
            painter,
            value_bottom,
            col.center().x,
            value,
            &value_font,
            ink.body,
        );
    }
}

/// 3 小时间隔标注点：3/6/9/12/15/18/21（首尾不标，24 点数据）
fn temp_label_indices(len: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 3;
    while i < len.saturating_sub(1) {
        out.push(i);
        i += 3;
    }
    out
}

/// 第三块：24 小时折线，每小时一个圆点；温度标签每 3 小时画在圆点上方 5px
fn draw_hourly(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    hourly: &[HourlyPoint],
    ink: &crate::theme::Ink,
) {
    let hours: Vec<&HourlyPoint> = hourly.iter().take(24).collect();
    if hours.len() < 2 {
        return;
    }
    let accent = crate::theme::accent_color();
    let point_radius = crate::theme::layout_f64("weather_point_radius");
    let label_gap = crate::theme::layout_f32("weather_point_label_gap");
    let line_width = crate::theme::layout_f64("weather_line_width");
    let meta_digit = digits::digit_font(crate::theme::font_size("meta"));
    let meta_h = crate::widgets::text_metrics::height(ui.ctx(), &meta_digit, InkKind::Digits);
    let axis_h = meta_h;
    let plot_h = (rect.height() - axis_h).max(1.0);

    let temps: Vec<f64> = hours.iter().map(|h| h.temperature).collect();
    let lo = temps.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = (hi - lo).max(1.0);
    let y_lo = lo - span * 0.30;
    let y_hi = hi + span * 0.40;
    let n = hours.len();

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        let plot_response = egui_plot::Plot::new("hourly_temp")
            .height(plot_h)
            .show_axes(false)
            .show_grid(false)
            .show_background(false)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false)
            .allow_double_click_reset(false)
            .set_margin_fraction(egui::vec2(0.04, 0.10))
            .default_x_bounds(-0.6, (n - 1) as f64 + 0.6)
            .default_y_bounds(y_lo, y_hi)
            .show(ui, |plot_ui| {
                for k in 1..=4 {
                    let y = y_lo + (y_hi - y_lo) * (k as f64 / 5.0);
                    plot_ui.hline(egui_plot::HLine::new(format!("grid{k}"), y).stroke(
                        egui::Stroke::new(0.8, crate::theme::with_alpha(ink.muted, 40)),
                    ));
                }
                let pts: Vec<[f64; 2]> = hours
                    .iter()
                    .enumerate()
                    .map(|(i, h)| [i as f64, h.temperature])
                    .collect();
                plot_ui.line(
                    egui_plot::Line::new("温度", pts.clone())
                        .color(accent)
                        .width(line_width),
                );
                plot_ui.points(
                    egui_plot::Points::new("每小时", pts)
                        .color(accent)
                        .radius(point_radius),
                );
            });
        let transform = plot_response.transform;
        let plot_rect = plot_response.response.rect;
        // 图外时间轴：按数据点坐标对齐，首尾夹在轴内（先分配，避免同时持有 ui 借用）
        let (axis, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), axis_h),
            egui::Sense::hover(),
        );
        let painter = ui.painter();
        for &i in &temp_label_indices(n) {
            let h = hours[i];
            let text = format!("{:.0}°", h.temperature);
            let w = digits::text_width(painter, &text, &meta_digit);
            let p =
                transform.position_from_point(&egui_plot::PlotPoint::new(i as f64, h.temperature));
            let x =
                p.x.clamp(plot_rect.left() + w / 2.0, plot_rect.right() - w / 2.0);
            let mut bottom = p.y - point_radius - label_gap;
            if bottom - meta_h < plot_rect.top() {
                bottom = p.y + point_radius + label_gap + meta_h;
            }
            digits::draw_bottom(painter, bottom, x, &text, &meta_digit, ink.strong);
        }

        for &i in &[0, n / 3, (2 * n) / 3, n - 1] {
            let time = hours[i].time.get(11..16).unwrap_or("--:--");
            let w = digits::text_width(painter, time, &meta_digit);
            let p = transform.position_from_point(&egui_plot::PlotPoint::new(i as f64, y_lo));
            let cx = p.x.clamp(axis.left() + w / 2.0, axis.right() - w / 2.0);
            digits::draw_bottom(painter, axis.bottom(), cx, time, &meta_digit, ink.meta);
        }
    });
}

/// 单个温度段的固定槽参数：负号槽是否生效 + 数字槽数（1/2）
#[derive(Clone, Copy)]
struct TempSlots {
    sign_w: f32,
    cell: f32,
    dot_w: f32,
    has_sign: bool,
    digit_slots: f32,
}

/// 按固定槽位画低温/高温段：负号槽按周数据决定是否占宽；数字右对齐；
/// ° 固定在数字槽之后
fn draw_temp_segment(
    painter: &egui::Painter,
    font: &egui::FontId,
    part: &str,
    seg_x: f32,
    bottom: f32,
    color: egui::Color32,
    slots: TempSlots,
) {
    let (neg, rest) = match part.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, part),
    };
    let digits_part = rest.strip_suffix('°').unwrap_or(rest);
    debug_assert!(
        digits_part.chars().count() <= slots.digit_slots as usize,
        "七日温度位数超出槽位: {part}"
    );
    let sign_w = if slots.has_sign { slots.sign_w } else { 0.0 };
    if slots.has_sign && neg {
        digits::draw_bottom(painter, bottom, seg_x + sign_w / 2.0, "-", font, color);
    }
    let digit_right = seg_x + sign_w + slots.digit_slots * slots.cell;
    let digit_w = digits::text_width(painter, digits_part, font);
    digits::draw_bottom(
        painter,
        bottom,
        digit_right - digit_w / 2.0,
        digits_part,
        font,
        color,
    );
    digits::draw_bottom(
        painter,
        bottom,
        seg_x + sign_w + slots.digit_slots * slots.cell + slots.dot_w / 2.0,
        "°",
        font,
        color,
    );
}

/// 第四块：七日标题 + 周范围 + 7 行（日期 / 图标 / 范围条 / 低高值），
/// 日期与温度固定槽宽，图标盒边长 = 行高，剩余宽度按 1:8:1 分给左右空隙与横线
fn draw_week(
    ui: &egui::Ui,
    rect: egui::Rect,
    daily: &[accel_core::types::DailyPoint],
    ink: &crate::theme::Ink,
) {
    let days: Vec<&accel_core::types::DailyPoint> = daily.iter().take(7).collect();
    if days.is_empty() {
        return;
    }
    let week_min = days.iter().map(|d| d.t_min).fold(f64::INFINITY, f64::min);
    let week_max = days
        .iter()
        .map(|d| d.t_max)
        .fold(f64::NEG_INFINITY, f64::max);
    let span = (week_max - week_min).max(0.001);
    let title_font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let range_font = digits::digit_font(crate::theme::font_size("meta"));
    let body_digit = digits::digit_font(crate::theme::font_size("body"));
    let title_h = crate::widgets::text_metrics::height(ui.ctx(), &title_font, InkKind::Cjk);
    let text_h = crate::widgets::text_metrics::height(ui.ctx(), &body_digit, InkKind::Digits);
    let icon_size = crate::theme::layout_f32("weather_week_icon_size");
    let icon_gap = crate::theme::layout_f32("weather_week_date_icon_gap");
    let title_gap = crate::theme::layout_f32("weather_week_title_gap");
    let row_h = text_h.max(icon_size);
    let row_gap = crate::theme::spacing_f32("sm");
    let bar_h = crate::theme::layout_f32("weather_week_bar_h");
    let bar_radius = crate::theme::layout_f32("weather_week_bar_radius");
    let min_fill = crate::theme::layout_f32("weather_week_bar_min_fill");
    let painter = ui.painter();
    let title_bottom = rect.top() + title_h;

    let title_ink = text_ink::measure(ui, "七日天气预测", &title_font, InkKind::Cjk);
    painter.text(
        egui::pos2(rect.left(), title_ink.top_for_bottom(title_bottom)),
        egui::Align2::LEFT_TOP,
        "七日天气预测",
        title_font,
        ink.muted,
    );
    let range_text = format!("{week_min:.0}° — {week_max:.0}°");
    let range_ink = text_ink::measure(ui, &range_text, &range_font, InkKind::Digits);
    painter.text(
        egui::pos2(rect.right(), range_ink.top_for_bottom(title_bottom)),
        egui::Align2::RIGHT_TOP,
        range_text,
        range_font,
        ink.meta,
    );

    let date_texts: Vec<String> = days
        .iter()
        .map(|d| d.date.get(5..10).unwrap_or(&d.date).to_string())
        .collect();
    let low_texts: Vec<String> = days.iter().map(|d| format!("{:.0}°", d.t_min)).collect();
    let high_texts: Vec<String> = days.iter().map(|d| format!("{:.0}°", d.t_max)).collect();
    let value_texts: Vec<String> = low_texts
        .iter()
        .zip(high_texts.iter())
        .map(|(low, high)| format!("{low}/{high}"))
        .collect();
    // 日期固定槽：4 数字槽 + 连字符槽（与 7 天具体日期无关）
    let date_w = digits::text_width(painter, "00-00", &body_digit);
    let cell = digits::cell_width(painter, &body_digit);
    let sign_w = digits::text_width(painter, "-", &body_digit);
    let dot_w = digits::text_width(painter, "°", &body_digit);
    let slash_w = digits::text_width(painter, "/", &body_digit);
    let digit_count = |t: &str| {
        t.trim_end_matches('°')
            .trim_start_matches('-')
            .chars()
            .count()
    };
    // 负号槽/十位槽由本周是否出现决定；出现一次则整周生效
    let low_sign = low_texts.iter().any(|t| t.starts_with('-'));
    let high_sign = high_texts.iter().any(|t| t.starts_with('-'));
    let low_digits = if low_texts.iter().any(|t| digit_count(t) >= 2) {
        2.0
    } else {
        1.0
    };
    let high_digits = if high_texts.iter().any(|t| digit_count(t) >= 2) {
        2.0
    } else {
        1.0
    };
    let low_slots = TempSlots {
        sign_w,
        cell,
        dot_w,
        has_sign: low_sign,
        digit_slots: low_digits,
    };
    let high_slots = TempSlots {
        sign_w,
        cell,
        dot_w,
        has_sign: high_sign,
        digit_slots: high_digits,
    };
    let low_w = if low_sign { sign_w } else { 0.0 } + low_digits * cell + dot_w;
    let high_w = if high_sign { sign_w } else { 0.0 } + high_digits * cell + dot_w;
    let temp_w = low_w + slash_w + high_w;
    let temp_start = rect.right() - temp_w;
    let gap_ratio = crate::theme::layout_f32("weather_week_bar_gap_ratio");
    let bar_ratio = crate::theme::layout_f32("weather_week_bar_ratio");
    let icon_box = row_h;
    let remaining = (rect.width() - date_w - icon_gap - icon_box - temp_w).max(0.0);
    let unit = remaining / (2.0 * gap_ratio + bar_ratio).max(0.001);
    let gap = unit * gap_ratio;
    let bar_w = (unit * bar_ratio).max(1.0);
    let row_radius = egui::epaint::CornerRadius::same(bar_radius.round() as u8);

    for (i, d) in days.iter().enumerate() {
        let top = rect.top() + title_h + title_gap + i as f32 * (row_h + row_gap);
        let row = egui::Rect::from_min_size(
            egui::pos2(rect.left(), top),
            egui::vec2(rect.width(), row_h),
        );
        let text_bottom = row.center().y + text_h / 2.0;

        // 日期：固定槽宽、盒内左对齐、盒中线 = 行中线
        digits::draw_bottom(
            painter,
            text_bottom,
            rect.left() + date_w / 2.0,
            &date_texts[i],
            &body_digit,
            ink.muted,
        );
        // 图标盒：边长 = 行高，盒中线 = 行中线
        icons::draw(
            painter,
            egui::pos2(
                rect.left() + date_w + icon_gap + icon_box / 2.0,
                row.center().y,
            ),
            icon_box,
            icon_for_code(d.weather_code),
            ink.muted,
        );
        // 温度：固定槽宽、整体右对齐；低温段 / 斜杠 / 高温段位置固定
        let (low, high) = value_texts[i]
            .split_once('/')
            .unwrap_or((&value_texts[i], ""));
        draw_temp_segment(
            painter,
            &body_digit,
            low,
            temp_start,
            text_bottom,
            ink.strong,
            low_slots,
        );
        draw_temp_segment(
            painter,
            &body_digit,
            high,
            temp_start + low_w + slash_w,
            text_bottom,
            ink.strong,
            high_slots,
        );
        digits::draw_bottom(
            painter,
            text_bottom,
            temp_start + low_w + slash_w / 2.0,
            "/",
            &body_digit,
            ink.strong,
        );

        let bar_x = rect.left() + date_w + icon_gap + icon_box + gap;
        let track = egui::Rect::from_min_size(
            egui::pos2(bar_x, row.center().y - bar_h / 2.0),
            egui::vec2(bar_w, bar_h),
        );
        painter.rect_filled(track, row_radius, crate::theme::track_color());
        let lo = ((d.t_min - week_min) / span) as f32;
        let hi = ((d.t_max - week_min) / span) as f32;
        let fill_left = track.left() + bar_w * lo;
        let remaining_fill = (track.right() - fill_left).max(1.0);
        let fill_w = (bar_w * (hi - lo)).max(min_fill).min(remaining_fill);
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(fill_left, track.top()),
                egui::vec2(fill_w, bar_h),
            ),
            row_radius,
            crate::theme::accent_color(),
        );
    }
}

// ===== src/pages/weather.rs 说明区 =====
// 输入：UiState.world_hero_city + weather/errors/loading + world_custom_cities
// 输出：第一块三列（城市/温度/图标）+ 三列指标 + 24h 折线 + 七日范围条
// 逻辑步骤：hero 城市（last_city 只作回退）→ 错误行（无重试按钮）→ 数据/空态 →
//   第一块 hero_h=row1+row_gap+row2 → 第二块 label_h+row_gap+value_h →
//   第三块 plot_h+axis_h → 第四块 title+7 行；块间距 page_box_gap=20，顶部补足 20
// 设计理由（PL029）：天气城市跟随世界页 hero，删除城市下拉/刷新/17 城表/
//   SetWeatherCity；第一行 title 28（温度 Barlow）、第二行 body 16（英文 Barlow/
//   描述 MiSans）；图标 20 盒垂直居中整块；三列等宽、内容水平居中；折线每小时圆点，
//   温度每 3 小时标在圆点上方 5px；七日标题/范围同 meta 档，标题下 10px；
//   日期固定 4 数字 + 连字符槽、左对齐；日期后 10px 图标盒（边长=行高）；剩余
//   宽度按 1:8:1 分给左右空隙与横线；温度固定“负号+2 数字+°+/+负号+2 数字+°”槽、右对齐
// 异常处理：无数据分加载/空态；失败保留旧值并显示错误行；<2 个逐小时点跳过
// 关联配置：ui.json page_box_gap/drawer_content_pad_top/weather_* tokens；
//   消费方：pages::Pages 抽屉
#[cfg(test)]
mod tests {
    use super::{equal_columns, icon_for_code, temp_label_indices};

    #[test]
    fn icon_mapping_covers_wmo_segments() {
        use crate::widgets::icons::Icon;
        assert_eq!(icon_for_code(0), Icon::Sun);
        assert_eq!(icon_for_code(2), Icon::Sun);
        assert_eq!(icon_for_code(3), Icon::Cloud);
        assert_eq!(icon_for_code(45), Icon::Fog);
        assert_eq!(icon_for_code(63), Icon::Rain);
        assert_eq!(icon_for_code(81), Icon::Rain);
        assert_eq!(icon_for_code(73), Icon::Snow);
        assert_eq!(icon_for_code(86), Icon::Snow);
        assert_eq!(icon_for_code(95), Icon::Storm);
        assert_eq!(icon_for_code(42), Icon::Cloud);
    }

    #[test]
    fn three_columns_split_evenly() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 40.0));
        let cols = equal_columns(rect);
        assert!((cols[0].width() - 100.0).abs() < 1e-5);
        assert!((cols[1].left() - 110.0).abs() < 1e-5);
        assert!((cols[2].left() - 210.0).abs() < 1e-5);
        assert!((cols[2].right() - 310.0).abs() < 1e-5);
    }

    #[test]
    fn temp_labels_skip_first_and_last() {
        assert_eq!(temp_label_indices(24), vec![3, 6, 9, 12, 15, 18, 21]);
        assert_eq!(temp_label_indices(4), Vec::<usize>::new());
        assert_eq!(temp_label_indices(2), Vec::<usize>::new());
    }
}
