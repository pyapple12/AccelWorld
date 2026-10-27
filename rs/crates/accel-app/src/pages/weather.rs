//! 天气页（UIR 重塑）：当下 hero（大字温度 + 现象 + 要素行）+ 逐时温度曲线 +
//! 七日温度距条 + 城市选择与刷新（图标按钮）
//! 纯函数 icon_for_code/hero_anchors/ellipsize_to 可测（码映射/hero 布局/截断）

use crate::widgets::icons::Icon;
use crate::widgets::{digits, icon_button};
use accel_core::types::{FrontendCommand, HourlyPoint};

/// 天气码 → 现象矢量图标（Open-Meteo WMO 码段映射；未知码回退云——纯函数可测）
pub fn icon_for_code(code: i64) -> Icon {
    match code {
        0..=2 => Icon::Sun,
        3 => Icon::Cloud,
        45 | 48 => Icon::Fog,
        51..=67 | 80..=82 => Icon::Rain,
        71..=77 | 85 | 86 => Icon::Snow,
        95..=99 => Icon::Storm,
        _ => Icon::Cloud,
    }
}

// PL016.08：temp_curve_points 已随 egui_plot 化删除（归一采样被 Plot 数据空间
// 坐标取代；A008 #16 平线居中语义由 Plot y-bounds 固定承接）

/// hero 行水平锚点（PL016.16，纯函数可测）：返回（温度中心 x，现象文字左缘 x），
/// 均为距内容左缘偏移。宽抽屉逐像素保持既有节奏（104/214）；可用宽不足时现象
/// 文字回退「随温度右缘 + 间距」并尽量右锚；长温度右移避让图标（图标右缘 52）
fn hero_anchors(avail_w: f32, temp_w: f32, desc_w: f32, gap: f32) -> (f32, f32) {
    let icon_right = 52.0;
    let temp_cx = 104.0_f32.max(icon_right + gap + temp_w / 2.0);
    let temp_right = temp_cx + temp_w / 2.0;
    let right = avail_w - 8.0;
    let floor = temp_right + gap;
    let desc_x = (right - desc_w).max(floor).min(214.0_f32.max(floor));
    (temp_cx, desc_x)
}

/// 现象文字超预算截断（PL016.16，纯函数可测）：逐字回退加省略号，空串原样返回
fn ellipsize_to(desc: &str, budget: f32, width_of: impl Fn(&str) -> f32) -> String {
    if desc.is_empty() || width_of(desc) <= budget {
        return desc.to_string();
    }
    let mut kept = desc.to_string();
    while !kept.is_empty() && width_of(&format!("{kept}…")) > budget {
        kept.pop();
    }
    if kept.is_empty() {
        "…".to_string()
    } else {
        format!("{kept}…")
    }
}

impl crate::pages::Page for crate::pages::WeatherPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        let city = state.prefs.last_city.clone();

        // 头行：城市下拉 + 刷新（图标按钮）
        ui.horizontal(|ui| {
            let city_names = accel_core::interface::AppInterface::get_city_names();
            egui::ComboBox::from_id_salt("weather_city")
                .selected_text(city.as_str())
                .width(120.0)
                .show_ui(ui, |ui| {
                    for name in &city_names {
                        if ui.selectable_label(name == &city, name).clicked() && name != &city {
                            bus.send(FrontendCommand::SetWeatherCity(name.clone()));
                            bus.send(FrontendCommand::RefreshWeather {
                                city: name.clone(),
                                force: false,
                            });
                            // PL016.03：在飞指示——发刷新即入集，收事件由 bus 出集
                            state.weather_loading.insert(name.clone());
                            state.prefs.last_city = name.clone();
                        }
                    }
                });
            if icon_button::show(ui, crate::widgets::icons::Icon::Refresh, 16.0, false).clicked() {
                bus.send(FrontendCommand::RefreshWeather {
                    city: city.clone(),
                    force: true,
                });
                state.weather_loading.insert(city.clone());
            }
        });
        ui.add_space(crate::theme::spacing("xs"));

        if let Some(err) = state.weather_errors.get(&city) {
            ui.label(egui::RichText::new(format!("{city} 天气拉取失败：{err}")).color(ink.danger));
        }
        let Some(w) = state.weather.get(&city) else {
            // PL016.03（审核 #2/#15）：无数据两态——获取中指示 / 行动邀请空态
            if state.weather_loading.contains(&city) {
                ui.label(
                    egui::RichText::new("天气获取中…")
                        .size(crate::theme::font_size("body_lg"))
                        .color(ink.muted),
                );
            } else {
                ui.label(
                    egui::RichText::new("选择上方城市开始查看天气——数据来自 Open-Meteo 免费接口")
                        .size(crate::theme::font_size("body_lg"))
                        .color(ink.muted),
                );
            }
            return;
        };

        // 当下 hero：现象矢量图标 + 大字温度（等宽数字位）+ 现象文字
        // （码表 emoji 在 egui 内置字体缺字形渲染 tofu——A008 #5 起图标随
        //   weather_code 映射矢量集，雨/雪/雾/雷各具其形）
        // PL016.16：锚点改 hero_anchors 相对布局——最小窗宽 520 下抽屉内容仅
        //   248px，原固定 214 偏移把现象文字推出右缘；宽抽屉逐像素保持原节奏
        let temp_text = format!("{:.1}°", w.temperature);
        let temp_font = digits::digit_font(crate::theme::font_size("display"));
        let desc_font = egui::FontId::proportional(crate::theme::font_size("body_lg"));
        let gap = crate::theme::spacing("md");
        let avail_w = ui.max_rect().width();
        let temp_w = digits::text_width(ui.painter(), &temp_text, &temp_font);
        let desc_w = digits::text_width(ui.painter(), &w.description, &desc_font);
        let (temp_cx, desc_x) = hero_anchors(avail_w, temp_w, desc_w, gap);
        let icon_x = ui.max_rect().left() + 8.0;
        let hero_y = ui.cursor().top() + 30.0;
        crate::widgets::icons::draw(
            ui.painter(),
            egui::pos2(icon_x + 24.0, hero_y),
            40.0,
            icon_for_code(w.weather_code),
            ink.muted,
        );
        digits::draw_centered(
            ui.painter(),
            egui::pos2(ui.max_rect().left() + temp_cx, hero_y),
            &temp_text,
            &temp_font,
            ink.strong,
        );
        let budget = (avail_w - 8.0 - desc_x).max(0.0);
        let desc = ellipsize_to(&w.description, budget, |t| {
            digits::text_width(ui.painter(), t, &desc_font)
        });
        ui.painter().text(
            egui::pos2(ui.max_rect().left() + desc_x, hero_y),
            egui::Align2::LEFT_CENTER,
            &desc,
            desc_font,
            ink.muted,
        );
        ui.allocate_space(egui::vec2(0.0, crate::theme::font_size("display") * 1.7));
        // 要素行（PL016.09：风力段前置 Wind 图标——语义不只靠文字，图标补充速识）
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "体感 {:.1}°C · 湿度 {:.0}%",
                    w.apparent_temperature, w.humidity
                ))
                .size(crate::theme::font_size("body"))
                .color(ink.muted),
            );
            // PL016.18：14→18px——线宽随尺寸缩放（size×0.09），14px 下三线间隙
            // 仅 ~0.4px 视觉读作「≈」；18px + 图标几何拉开线距后可辨
            crate::widgets::icons::draw(
                ui.painter(),
                egui::pos2(ui.cursor().left() + 9.0, ui.cursor().center().y),
                18.0,
                crate::widgets::icons::Icon::Wind,
                ink.muted,
            );
            ui.allocate_space(egui::vec2(26.0, 1.0));
            ui.label(
                egui::RichText::new(format!("风力 {:.1}km/h", w.wind_speed))
                    .size(crate::theme::font_size("body"))
                    .color(ink.muted),
            );
        });
        ui.add_space(crate::theme::spacing("sm"));

        // 逐时温度曲线（PL016.08 egui_plot 化——图表无 chrome：无轴/无图例/禁交互，
        // 单序列金线 + 3h 金点 + meta 标注；bounds 固定防每帧缩放跳动）
        if !w.hourly.is_empty() {
            ui.label(
                egui::RichText::new("逐小时")
                    .size(crate::theme::font_size("body"))
                    .color(ink.muted),
            );
            let hours: Vec<&HourlyPoint> = w.hourly.iter().take(24).collect();
            let accent = crate::theme::accent_color();
            let temps: Vec<f64> = hours.iter().map(|h| h.temperature).collect();
            let lo = temps.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let span = (hi - lo).max(1.0);
            let y_lo = lo - span * 0.35;
            let y_hi = hi + span * 0.45;
            let n = hours.len();
            let meta = crate::theme::font_size("meta");
            let meta_font = egui::FontId::proportional(meta);
            egui_plot::Plot::new("hourly_temp")
                .height(88.0)
                .show_axes(false)
                .show_grid(false)
                .allow_drag(false)
                .allow_zoom(false)
                .allow_scroll(false)
                .allow_boxed_zoom(false)
                .allow_double_click_reset(false)
                .set_margin_fraction(egui::Vec2::ZERO)
                .default_x_bounds(-0.6, (n - 1) as f64 + 0.6)
                .default_y_bounds(y_lo, y_hi)
                .show(ui, |plot_ui| {
                    // PL016.13（审核 #9 复核补遗）：图表无 chrome 含「网格线 3–7 条
                    // 淡线」——全关属矫枉过正，补 4 条横向淡线（温度参考系）
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
                    plot_ui.line(egui_plot::Line::new("温度", pts).color(accent).width(1.8));
                    // 3 小时标注（首尾必标）：金点 + 温度上标 + 时刻下标
                    for (i, h) in hours.iter().enumerate() {
                        if i % 3 != 0 && i != n - 1 {
                            continue;
                        }
                        let p = egui_plot::PlotPoint::new(i as f64, h.temperature);
                        plot_ui.points(
                            egui_plot::Points::new(
                                format!("p{i}"),
                                vec![[i as f64, h.temperature]],
                            )
                            .color(accent)
                            .radius(2.5),
                        );
                        plot_ui.text(
                            egui_plot::Text::new(
                                format!("t{i}"),
                                p,
                                egui::RichText::new(format!("{:.0}°", h.temperature))
                                    .font(meta_font.clone())
                                    .color(ink.strong),
                            )
                            .anchor(egui::Align2::CENTER_BOTTOM),
                        );
                        plot_ui.text(
                            egui_plot::Text::new(
                                format!("x{i}"),
                                egui_plot::PlotPoint::new(i as f64, y_lo),
                                egui::RichText::new(h.time.get(11..16).unwrap_or("--:--"))
                                    .font(meta_font.clone())
                                    .color(ink.meta),
                            )
                            .anchor(egui::Align2::CENTER_TOP),
                        );
                    }
                });
            ui.add_space(crate::theme::spacing("sm"));
        }

        // 七日温度距条（min–max 相对全周域的水平距条）
        if !w.daily.is_empty() {
            ui.label(
                egui::RichText::new("七日")
                    .size(crate::theme::font_size("body"))
                    .color(ink.muted),
            );
            let days: Vec<&accel_core::types::DailyPoint> = w.daily.iter().take(7).collect();
            let week_min = days.iter().map(|d| d.t_min).fold(f64::INFINITY, f64::min);
            let week_max = days
                .iter()
                .map(|d| d.t_max)
                .fold(f64::NEG_INFINITY, f64::max);
            let span = (week_max - week_min).max(0.001);
            let body_font = egui::FontId::proportional(crate::theme::font_size("body"));
            for d in &days {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(d.date.get(5..10).unwrap_or(&d.date))
                            .font(egui::FontId::monospace(crate::theme::font_size("body")))
                            .color(ink.muted),
                    );
                    // 距条（相对周域；accent 填充 + 全域弱轨）；A008 #8：极窄窗负宽钳 0；
                    // A009 #19：零宽跳过绘制（轨道不可见时不再画 4px 孤立金点）
                    let bar_w = (ui.available_width() - 110.0).max(0.0);
                    if bar_w > 0.0 {
                        let track = egui::Rect::from_min_size(
                            egui::pos2(ui.cursor().left(), ui.cursor().center().y - 3.0),
                            egui::vec2(bar_w, 6.0),
                        );
                        ui.painter().rect_filled(
                            track,
                            egui::epaint::CornerRadius::same(3),
                            crate::theme::track_color(),
                        );
                        let lo = (d.t_min - week_min) / span;
                        let hi = (d.t_max - week_min) / span;
                        let mut fill = track;
                        fill.min.x = track.left() + (bar_w * lo as f32);
                        fill.max.x = track.left() + (bar_w * hi as f32).max(fill.min.x + 4.0);
                        ui.painter().rect_filled(
                            fill,
                            egui::epaint::CornerRadius::same(3),
                            crate::theme::accent_color(),
                        );
                    }
                    ui.allocate_space(egui::vec2(bar_w + 10.0, 1.0));
                    ui.label(
                        egui::RichText::new(format!("{:.0}°/{:.0}°", d.t_min, d.t_max))
                            .font(body_font.clone())
                            .color(ink.strong),
                    );
                });
            }
        }
    }
}

// ===== src/pages/weather.rs 说明区 =====
// 输入：UiState.weather/weather_errors（WeatherUpdated/Failed 事件快照）+ prefs.last_city
// 输出：当下 hero（大字温度/现象/要素行）+ 逐时温度曲线 + 七日距条 + 城市选择与刷新
// 逻辑步骤：城市下拉（换城发 SetWeatherCity + RefreshWeather + 入在飞集）→ 刷新
//   图标按钮（force + 入集）→ 错误行（旧值并存）→ 数据/获取中/空态三分支 → hero
//   （PL016.16 hero_anchors 相对布局 + ellipsize_to 截断）→
//   曲线（egui_plot 无 chrome：单序列金线 + 3h 金点 + meta 标注）→ 七日距条
// 设计理由（UIR 重塑 + PL016.08/16）：天气页的信息主角是"现在几度、接下来冷热走势"
//   ——大字温度 + 一条曲线替代 24 根竖排横条；曲线图化后无轴无图例零交互
//   （审核 #9 图表无 chrome 口径）；现象图标随 weather_code 映射矢量集；
//   hero 锚点随抽屉可用宽自适应——最小窗宽下现象文字不再被推出右缘
// 异常处理：无数据两态（获取中/行动邀请）；失败保留旧值 + 红字（PL016.03）；
//   空曲线段跳过
// 关联依赖：UiState.weather + widgets(digits/icon_button/icons/toast 经 bus) +
//   egui_plot；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::{ellipsize_to, hero_anchors, icon_for_code};

    #[test]
    fn icon_mapping_covers_wmo_segments() {
        // A008 #5 验收：码段映射各具其形；未知码回退云
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
        assert_eq!(icon_for_code(42), Icon::Cloud, "未知码回退云");
    }

    #[test]
    fn hero_anchors_wide_drawer_preserves_legacy_rhythm() {
        // PL016.16：宽抽屉（420 内容 388）逐像素保持原固定节奏 104/214
        let (temp_cx, desc_x) = hero_anchors(388.0, 80.0, 64.0, 12.0);
        assert!((temp_cx - 104.0).abs() < 1e-6);
        assert!((desc_x - 214.0).abs() < 1e-6);
    }

    #[test]
    fn hero_anchors_min_window_keeps_desc_inside() {
        // 最小窗宽 520：抽屉钳 280、内容 248——现象文字右缘不得越界
        let (temp_cx, desc_x) = hero_anchors(248.0, 80.0, 64.0, 12.0);
        assert!((temp_cx - 104.0).abs() < 1e-6);
        assert!(desc_x + 64.0 <= 240.0, "desc 右缘越界: {}", desc_x + 64.0);
    }

    #[test]
    fn hero_anchors_long_temp_shifts_to_clear_icon() {
        // 长温度（负两位数）右移避让图标，且现象文字不与温度重叠
        let (temp_cx, desc_x) = hero_anchors(388.0, 150.0, 64.0, 12.0);
        assert!(temp_cx - 75.0 >= 52.0, "温度左缘压图标: {}", temp_cx - 75.0);
        assert!(desc_x >= temp_cx + 75.0 + 12.0, "现象文字压温度: {desc_x}");
    }

    #[test]
    fn ellipsize_trims_to_budget_with_ellipsis() {
        let width_of = |s: &str| s.chars().count() as f32 * 10.0;
        assert_eq!(ellipsize_to("晴朗无云", 100.0, width_of), "晴朗无云");
        assert_eq!(ellipsize_to("晴朗无云", 35.0, width_of), "晴朗…");
        assert_eq!(ellipsize_to("晴", 5.0, width_of), "…");
        assert_eq!(ellipsize_to("", 5.0, width_of), "");
    }
}
