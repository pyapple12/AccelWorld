//! 天气页（PL017 重排）：整页 ScrollArea；hero 聚簇 + 三列 metric + 无框曲线 +
//! 三列七日范围条（数值在 card_bg 上，不压金条）+ 城市/刷新 + 错误/加载态。
//! 纯函数 icon_for_code 可测；旧 hero_anchors 固定像素布局随重构删除。

use crate::widgets::text_ink::InkKind;
use crate::widgets::{digits, icon_button, icons};
use accel_core::types::{FrontendCommand, HourlyPoint};

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
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        let city = state.prefs.last_city.clone();
        // 加载期 0.72 透明度，结束后 160ms 淡入（减少动效直取）
        let loading = state.weather_loading.contains(&city);
        let fade = if crate::theme::reduce_motion(ui.ctx()) {
            1.0
        } else {
            ui.ctx()
                .animate_bool_with_time(egui::Id::unique("weather_fade"), !loading, 0.16)
        };
        ui.set_opacity(0.72 + 0.28 * fade);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut stack = crate::widgets::box_stack::BoxStack::new(ui);

                // 城市/刷新盒
                let header_h = crate::widgets::select::height(ui) + crate::theme::spacing_f32("xs");
                let header = stack.next(ui, header_h);
                ui.scope_builder(egui::UiBuilder::new().max_rect(header), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let city_names = accel_core::interface::AppInterface::get_city_names();
                        if let Some(name) = crate::widgets::select::show(
                            ui,
                            egui::Id::unique("weather_city"),
                            &city,
                            &city_names,
                            150.0,
                        ) && name != city
                        {
                            bus.send(FrontendCommand::SetWeatherCity(name.clone()));
                            bus.send(FrontendCommand::RefreshWeather {
                                city: name.clone(),
                                force: false,
                            });
                            state.weather_loading.insert(name.clone());
                            state.prefs.last_city = name.clone();
                            state.toasts.push(crate::bus::Toast::info(
                                "天气城市",
                                &format!("已切换至 {name}"),
                            ));
                        }
                        if state.weather_loading.contains(&city) {
                            crate::widgets::spinner::show(ui, 16.0, ink.muted);
                        } else if icon_button::show(ui, icons::Icon::Refresh, 16.0, false)
                            .on_hover_text("刷新天气")
                            .clicked()
                        {
                            bus.send(FrontendCommand::RefreshWeather {
                                city: city.clone(),
                                force: true,
                            });
                            state.weather_loading.insert(city.clone());
                        }
                    });
                });

                let error = state.weather_errors.get(&city).cloned();
                let data = state.weather.get(&city).cloned();
                // 盒高全部查启动指标表（角色最大墨迹高，不是当前字符串）
                let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
                let body_font = crate::theme::font_regular(crate::theme::font_size("body"));
                let meta_h =
                    crate::widgets::text_metrics::height(ui.ctx(), &meta_font, InkKind::Cjk);
                let body_h =
                    crate::widgets::text_metrics::height(ui.ctx(), &body_font, InkKind::Cjk);
                let temp_h = crate::widgets::text_metrics::height(
                    ui.ctx(),
                    &digits::digit_font(44.0),
                    InkKind::Digits,
                );
                let error_h = meta_h + 14.0;
                let hero_h = temp_h + 6.0 + body_h + 32.0;
                let metric_h = meta_h + 4.0 + body_h + 16.0;
                let hourly_label_h = meta_h;
                let hourly_h = hourly_label_h
                    + crate::theme::layout_f32("weather_plot_h")
                    + crate::theme::spacing_f32("xl");
                let week_row_h = body_h + crate::theme::spacing_f32("sm");
                let week_h = meta_h
                    + crate::theme::spacing_f32("xs")
                    + 7.0 * week_row_h
                    + crate::theme::spacing_f32("sm");
                if let Some(err) = &error {
                    let rect = stack.next(ui, error_h);
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("更新失败：{err}"))
                                    .font(crate::theme::font_regular(crate::theme::font_size(
                                        "meta",
                                    )))
                                    .color(ink.danger),
                            );
                            if let Some(w) = &data {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "· 显示上次数据 {:.1}°",
                                        w.temperature
                                    ))
                                    .font(crate::theme::font_regular(crate::theme::font_size(
                                        "meta",
                                    )))
                                    .color(ink.meta),
                                );
                            }
                            if ui.small_button("重试").clicked() {
                                bus.send(FrontendCommand::RefreshWeather {
                                    city: city.clone(),
                                    force: true,
                                });
                                state.weather_loading.insert(city.clone());
                            }
                        });
                    });
                }

                let Some(w) = data else {
                    let rect = stack.next(ui, 96.0);
                    if state.weather_loading.contains(&city) {
                        crate::widgets::empty::card_at(
                            ui,
                            rect,
                            "天气获取中…",
                            "正在拉取当前天气；已有数据会保留到新数据到达",
                        );
                    } else {
                        crate::widgets::empty::card_at(
                            ui,
                            rect,
                            "还没有天气数据",
                            "选择城市后点刷新，或等待自动拉取",
                        );
                    }
                    return;
                };

                // hero 盒
                let hero = stack.next(ui, hero_h);
                draw_weather_hero(ui, hero, &w, &ink);

                // metric 盒（三列）
                let metrics = stack.next(ui, metric_h);
                ui.scope_builder(egui::UiBuilder::new().max_rect(metrics), |ui| {
                    ui.columns(3, |cols| {
                        metric(
                            &mut cols[0],
                            "体感",
                            &format!("{:.1}°C", w.apparent_temperature),
                        );
                        metric(&mut cols[1], "湿度", &format!("{:.0}%", w.humidity));
                        metric(&mut cols[2], "风力", &format!("{:.1}km/h", w.wind_speed));
                    });
                });

                // 逐小时盒
                if !w.hourly.is_empty() {
                    let rect = stack.next(ui, hourly_h);
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                        let label_box =
                            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), meta_h));
                        ui.painter().text(
                            label_box.left_center(),
                            egui::Align2::LEFT_CENTER,
                            "逐小时",
                            crate::theme::font_medium(crate::theme::font_size("meta")),
                            ink.muted,
                        );
                        ui.add_space(meta_h);
                        draw_hourly(ui, &w.hourly, &ink);
                    });
                }

                // 七日盒
                if !w.daily.is_empty() {
                    let rect = stack.next(ui, week_h);
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                        draw_week(ui, &w.daily, &ink);
                    });
                }

                // 数据来源盒
                let source_font = crate::theme::font_regular(11.0);
                let source_h = crate::widgets::text_metrics::line_height(ui.ctx(), &source_font)
                    + crate::theme::spacing_f32("sm");
                let source = stack.next(ui, source_h);
                let source_box = egui::Rect::from_min_size(
                    egui::pos2(
                        source.left(),
                        source.top() + crate::theme::spacing_f32("xs"),
                    ),
                    egui::vec2(source.width(), source_h - crate::theme::spacing_f32("xs")),
                );
                ui.painter().text(
                    source_box.left_center(),
                    egui::Align2::LEFT_CENTER,
                    "数据来源 Open-Meteo · 免费接口",
                    source_font,
                    ink.meta,
                );
            });
    }
}

/// 天气 hero 盒：图标 + 温度 + 描述，盒内按矩形定位
fn draw_weather_hero(
    ui: &egui::Ui,
    rect: egui::Rect,
    w: &accel_core::types::WeatherData,
    ink: &crate::theme::Ink,
) {
    let painter = ui.painter();
    icons::draw(
        painter,
        egui::pos2(rect.left() + 18.0 + 17.0, rect.center().y),
        34.0,
        icon_for_code(w.weather_code),
        ink.muted,
    );
    let temp_font = digits::digit_font(44.0);
    let temp = format!("{:.1}°", w.temperature);
    let temp_w = digits::text_width(painter, &temp, &temp_font);
    let temp_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &temp_font,
        crate::widgets::text_ink::InkKind::Digits,
    );
    let desc_font = crate::theme::font_regular(crate::theme::font_size("body"));
    let desc_h = crate::widgets::text_metrics::height(ui.ctx(), &desc_font, InkKind::Cjk);
    let temp_top = rect.top() + 16.0;
    let temp_center = temp_top + temp_h / 2.0;
    digits::draw_centered_ink(
        painter,
        egui::pos2(rect.left() + 62.0 + temp_w / 2.0, temp_center),
        &temp,
        &temp_font,
        ink.strong,
    );
    let desc_center = temp_top + temp_h + 6.0 + desc_h / 2.0;
    painter.text(
        egui::pos2(rect.left() + 62.0, desc_center),
        egui::Align2::LEFT_CENTER,
        &w.description,
        desc_font,
        ink.muted,
    );
}

/// 三列 metric：标签/值都按墨迹定位（盒高由调用方按墨迹算）
fn metric(ui: &mut egui::Ui, label: &str, value: &str) {
    let ink = crate::theme::ink();
    let rect = ui.max_rect();
    let label_font = crate::theme::font_regular(11.0);
    let value_font = crate::theme::font_medium(crate::theme::font_size("body"));
    let label_h = crate::widgets::text_metrics::height(ui.ctx(), &label_font, InkKind::Cjk);
    let value_h = crate::widgets::text_metrics::height(ui.ctx(), &value_font, InkKind::Cjk);
    let label_center = rect.top() + 8.0 + label_h / 2.0;
    ui.painter().text(
        egui::pos2(rect.left(), label_center),
        egui::Align2::LEFT_CENTER,
        label,
        label_font,
        ink.muted,
    );
    let value_center = rect.top() + 8.0 + label_h + 4.0 + value_h / 2.0;
    ui.painter().text(
        egui::pos2(rect.left(), value_center),
        egui::Align2::LEFT_CENTER,
        value,
        value_font,
        ink.body,
    );
}

/// 逐小时曲线：无背景/无边框，高 150；x 标签 4–5 个、温度只标 min/max/当前
fn draw_hourly(ui: &mut egui::Ui, hourly: &[HourlyPoint], ink: &crate::theme::Ink) {
    let hours: Vec<&HourlyPoint> = hourly.iter().take(24).collect();
    if hours.len() < 2 {
        return;
    }
    let accent = crate::theme::accent_color();
    let temps: Vec<f64> = hours.iter().map(|h| h.temperature).collect();
    let lo = temps.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = (hi - lo).max(1.0);
    let y_lo = lo - span * 0.30;
    let y_hi = hi + span * 0.40;
    let n = hours.len();
    let meta = crate::theme::font_size("meta");
    let meta_font = egui::FontId::proportional(meta);
    let (min_i, max_i) = temps.iter().enumerate().fold((0, 0), |(mi, ma), (i, t)| {
        (
            if *t < temps[mi] { i } else { mi },
            if *t > temps[ma] { i } else { ma },
        )
    });
    let label_idx: Vec<usize> = {
        let mut v = vec![min_i, max_i, n / 2];
        v.sort_unstable();
        v.dedup();
        v
    };

    egui_plot::Plot::new("hourly_temp")
        .height(crate::theme::layout_f32("weather_plot_h"))
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
            plot_ui.line(egui_plot::Line::new("温度", pts).color(accent).width(2.0));
            for i in label_idx {
                let h = hours[i];
                plot_ui.points(
                    egui_plot::Points::new(format!("p{i}"), vec![[i as f64, h.temperature]])
                        .color(accent)
                        .radius(2.5),
                );
                plot_ui.text(
                    egui_plot::Text::new(
                        format!("t{i}"),
                        egui_plot::PlotPoint::new(i as f64, h.temperature),
                        egui::RichText::new(format!("{:.0}°", h.temperature))
                            .font(meta_font.clone())
                            .color(ink.strong),
                    )
                    .anchor(if i == 0 {
                        egui::Align2::LEFT_BOTTOM
                    } else if i + 1 == n {
                        egui::Align2::RIGHT_BOTTOM
                    } else {
                        egui::Align2::CENTER_BOTTOM
                    }),
                );
            }
            // 现在：最近小时点 + 竖线（超出数据范围不画）
            let now = chrono::Local::now().naive_local();
            let mut now_i = None;
            let mut best = i64::MAX;
            for (i, h) in hours.iter().enumerate() {
                let parsed = chrono::NaiveDateTime::parse_from_str(&h.time, "%Y-%m-%dT%H:%M")
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(&h.time, "%Y-%m-%d %H:%M:%S")
                    });
                if let Ok(t) = parsed {
                    let d = (t - now).num_minutes().abs();
                    if d < best {
                        best = d;
                        now_i = Some(i);
                    }
                }
            }
            if let Some(i) = now_i.filter(|_| best <= 90) {
                let h = hours[i];
                plot_ui.vline(
                    egui_plot::VLine::new("now", i as f64).stroke(egui::Stroke::new(
                        1.0,
                        crate::theme::with_alpha(ink.muted, 110),
                    )),
                );
                plot_ui.points(
                    egui_plot::Points::new("now_point", vec![[i as f64, h.temperature]])
                        .color(ink.body)
                        .radius(3.0),
                );
                plot_ui.text(
                    egui_plot::Text::new(
                        "now_label",
                        egui_plot::PlotPoint::new(i as f64, y_hi),
                        egui::RichText::new("现在")
                            .font(meta_font.clone())
                            .color(ink.meta),
                    )
                    .anchor(egui::Align2::CENTER_BOTTOM),
                );
            }
        });
    // 图外时间轴：四列等距，避免贴 plot 底缘被裁
    ui.columns(4, |cols| {
        for (k, i) in [0, n / 3, (2 * n) / 3, n - 1].into_iter().enumerate() {
            let time = hours[i].time.get(11..16).unwrap_or("--:--");
            cols[k].label(
                egui::RichText::new(time)
                    .font(egui::FontId::proportional(crate::theme::font_size("meta")))
                    .color(ink.meta),
            );
        }
    });
}

/// 七日：固定三列（日 44 | 范围条 flex | 低高 64 右对齐在 card_bg 上）
fn draw_week(ui: &mut egui::Ui, daily: &[accel_core::types::DailyPoint], ink: &crate::theme::Ink) {
    let days: Vec<&accel_core::types::DailyPoint> = daily.iter().take(7).collect();
    let week_min = days.iter().map(|d| d.t_min).fold(f64::INFINITY, f64::min);
    let week_max = days
        .iter()
        .map(|d| d.t_max)
        .fold(f64::NEG_INFINITY, f64::max);
    let span = (week_max - week_min).max(0.001);
    let meta_font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let body_font = egui::FontId::proportional(crate::theme::font_size("body"));
    let label_h = crate::widgets::text_metrics::height(ui.ctx(), &meta_font, InkKind::Cjk);
    let row_h = crate::widgets::text_metrics::height(ui.ctx(), &body_font, InkKind::Cjk)
        + crate::theme::spacing_f32("sm");
    let rect = ui.max_rect();
    let label_center = rect.top() + label_h / 2.0;
    ui.painter().text(
        egui::pos2(rect.left(), label_center),
        egui::Align2::LEFT_CENTER,
        "七日",
        meta_font,
        ink.muted,
    );
    ui.painter().text(
        egui::pos2(rect.right(), label_center),
        egui::Align2::RIGHT_CENTER,
        format!("{week_min:.0}° — {week_max:.0}°"),
        crate::theme::font_regular(11.0),
        ink.meta,
    );
    ui.add_space(label_h + crate::theme::spacing_f32("xs"));
    let text_center_off = row_h / 2.0;
    for d in &days {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_h),
            egui::Sense::hover(),
        );
        let date_w = 44.0;
        let value_w = 68.0;
        let bar_x = rect.left() + date_w;
        let bar_w = (rect.width() - date_w - value_w - 12.0).max(24.0);
        let day_text = d.date.get(5..10).unwrap_or(&d.date);
        ui.painter().text(
            egui::pos2(rect.left(), rect.top() + text_center_off),
            egui::Align2::LEFT_CENTER,
            day_text,
            body_font.clone(),
            ink.muted,
        );
        let track = egui::Rect::from_min_size(
            egui::pos2(bar_x, rect.center().y - 3.0),
            egui::vec2(bar_w, 6.0),
        );
        ui.painter().rect_filled(
            track,
            egui::epaint::CornerRadius::same(3),
            crate::theme::track_color(),
        );
        let lo = (d.t_min - week_min) / span;
        let hi = (d.t_max - week_min) / span;
        let fill = egui::Rect::from_min_max(
            egui::pos2(track.left() + (bar_w * lo as f32), track.top()),
            egui::pos2(
                track.left() + (bar_w * hi as f32).max(bar_w * lo as f32 + 4.0),
                track.bottom(),
            ),
        );
        ui.painter().rect_filled(
            fill,
            egui::epaint::CornerRadius::same(3),
            crate::theme::accent_color(),
        );
        // 数值固定在条右侧的 value 列，绝不压在 fill 上
        let value_text = format!("{:.0}°/{:.0}°", d.t_min, d.t_max);
        ui.painter().text(
            egui::pos2(rect.right(), rect.top() + text_center_off),
            egui::Align2::RIGHT_CENTER,
            value_text,
            body_font.clone(),
            ink.strong,
        );
    }
}

// ===== src/pages/weather.rs 说明区 =====
// 输入：UiState.weather/errors/loading + prefs.last_city；输出：城市/刷新 + hero +
//   三列 metric + 无框逐时曲线 + 三列七日范围条
// 逻辑步骤：ScrollArea（头行城市/刷新/更新中 → 错误行 + 旧值保留 → 数据/加载/空态
//   三分支 → hero 聚簇 → metric → 曲线（150px/4-5 x 标签/只标 min-max-当前）→
//   七日（日 44 | 条 flex | 值 68 在 card_bg）→ 数据源）
// 设计理由（PL017）：原七日数字压金条且右裁（1.54:1）、曲线 88px 带默认框、整页
//   无 ScrollArea；本版三列固定列 + show_background(false) + 整页滚动 + 错误重试
// 异常处理：无数据分加载/空态；失败保留旧值 + 重试；曲线 <2 点跳过
// 关联依赖：core types/interface + egui_plot/widgets(digits/icons/empty)；
//   消费方：pages::Pages 抽屉
#[cfg(test)]
mod tests {
    use super::icon_for_code;

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
}
