//! 天气页（UIR 重塑）：当下 hero（大字温度 + 现象 + 要素行）+ 逐时温度曲线 +
//! 七日温度距条 + 城市选择与刷新（图标按钮）
//! 纯函数 temp_curve_points 可测（温度序列 → 归一化折线采样点）

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

/// 温度曲线采样点（纯函数）：等宽横轴布点 + 归一化纵轴（min/max 双锚，平线居中）
/// 返回 (x, y)（y 越小越冷——屏幕坐标）；空序列返空
pub fn temp_curve_points(temps: &[f64], width: f32, height: f32) -> Vec<(f32, f32)> {
    if temps.is_empty() || width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }
    let min = temps.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).max(0.001);
    // A008 #16：等温序列取中位（原 ratio=0 → 平线贴底缘，与「平线居中」注释相反）
    let flat = max - min <= f64::EPSILON;
    let n = temps.len();
    temps
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let x = if n == 1 {
                width / 2.0
            } else {
                width * (i as f32 / (n - 1) as f32)
            };
            let ratio = if flat {
                0.5_f32
            } else {
                ((t - min) / span) as f32
            };
            let y = height * (1.0 - ratio);
            (x, y)
        })
        .collect()
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
                            state.prefs.last_city = name.clone();
                        }
                    }
                });
            if icon_button::show(ui, crate::widgets::icons::Icon::Refresh, 16.0, false).clicked() {
                bus.send(FrontendCommand::RefreshWeather {
                    city: city.clone(),
                    force: true,
                });
            }
        });
        ui.add_space(crate::theme::spacing("xs"));

        if let Some(err) = state.weather_errors.get(&city) {
            ui.label(egui::RichText::new(format!("{city} 获取失败：{err}")).color(ink.danger));
        }
        let Some(w) = state.weather.get(&city) else {
            return;
        };

        // 当下 hero：现象矢量图标 + 大字温度（等宽数字位）+ 现象文字
        // （码表 emoji 在 egui 内置字体缺字形渲染 tofu——A008 #5 起图标随
        //   weather_code 映射矢量集，雨/雪/雾/雷各具其形）
        let temp_text = format!("{:.1}°", w.temperature);
        let temp_font = digits::digit_font(crate::theme::font_size("display"));
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
            egui::pos2(icon_x + 96.0, hero_y),
            &temp_text,
            &temp_font,
            ink.strong,
        );
        ui.painter().text(
            egui::pos2(icon_x + 96.0 + 110.0, hero_y),
            egui::Align2::LEFT_CENTER,
            &w.description,
            egui::FontId::proportional(crate::theme::font_size("body_lg")),
            ink.muted,
        );
        ui.allocate_space(egui::vec2(0.0, crate::theme::font_size("display") * 1.7));
        ui.label(
            egui::RichText::new(format!(
                "体感 {:.1}°C · 湿度 {:.0}% · 风力 {:.1}km/h",
                w.apparent_temperature, w.humidity, w.wind_speed
            ))
            .size(crate::theme::font_size("body"))
            .color(ink.muted),
        );
        ui.add_space(crate::theme::spacing("sm"));

        // 逐时温度曲线（24 点折线 + 3 小时刻度标注；基线发丝 + 刻度点）
        if !w.hourly.is_empty() {
            ui.label(
                egui::RichText::new("逐小时")
                    .size(crate::theme::font_size("body"))
                    .color(ink.muted),
            );
            let hours: Vec<&HourlyPoint> = w.hourly.iter().take(24).collect();
            let curve_w = ui.available_width();
            let curve_h = 56.0;
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(curve_w, curve_h), egui::Sense::hover());
            let temps: Vec<f64> = hours.iter().map(|h| h.temperature).collect();
            let pts = temp_curve_points(&temps, curve_w, curve_h - 16.0);
            if pts.len() > 1 {
                let poly: Vec<egui::Pos2> = pts
                    .iter()
                    .map(|(x, y)| egui::pos2(rect.left() + x, rect.top() + 8.0 + y))
                    .collect();
                ui.painter().add(egui::Shape::line(
                    poly,
                    egui::Stroke::new(1.6, crate::theme::accent_color()),
                ));
                // 3 小时刻度：时间下标 + 温度上标（首尾必标）
                let meta_font = egui::FontId::proportional(crate::theme::font_size("meta"));
                for (i, h) in hours.iter().enumerate() {
                    if i % 3 != 0 && i != hours.len() - 1 {
                        continue;
                    }
                    let Some(&(x, y)) = pts.get(i) else { continue };
                    let px = rect.left() + x;
                    let py = rect.top() + 8.0 + y;
                    // UIR3：3h 标注点改金刻度（2×6 竖签——星轨刻度语言在曲线上的回响，
                    // 替代通用圆点）
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(px - 1.0, py - 3.0),
                            egui::vec2(2.0, 6.0),
                        ),
                        egui::epaint::CornerRadius::same(1),
                        crate::theme::accent_color(),
                    );
                    ui.painter().text(
                        egui::pos2(px, rect.top() + 2.0),
                        egui::Align2::CENTER_TOP,
                        format!("{:.0}°", h.temperature),
                        meta_font.clone(),
                        ink.strong,
                    );
                    ui.painter().text(
                        egui::pos2(px, rect.bottom() - 2.0),
                        egui::Align2::CENTER_BOTTOM,
                        h.time.get(11..16).unwrap_or("--:--"),
                        meta_font.clone(),
                        ink.meta,
                    );
                }
            }
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
// 逻辑步骤：城市下拉（换城发 SetWeatherCity + RefreshWeather）→ 刷新图标按钮
//   （force）→ 数据域（失败红字优先）→ hero → 曲线（temp_curve_points 归一采样 +
//   3 小时刻度）→ 七日距条（相对全周 min/max 域）
// 设计理由（UIR 重塑）：天气页的信息主角是"现在几度、接下来冷热走势"——大字
//   温度 + 一条曲线替代 24 根竖排横条（原实现 24 列 label 堆叠既挤又难读）；七日
//   距条让一周温差一眼成泡；现象图标随 weather_code 映射矢量集（A008 #5——码表
//   emoji 字段仅数据保留，egui 内置字体缺字形不直出）
// FIX006.2 延续：命令统一经 bus.send；A005 #4 单城模型；UI 无状态重建
// 异常处理：数据缺失（首查前）整段静默；失败红字展示；空曲线/单点/平线退化居中
// 关联依赖：UiState.weather + widgets(digits/icon_button/icons)；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::{icon_for_code, temp_curve_points};

    #[test]
    fn curve_points_normalization() {
        // 归一化锚点：min 贴底、max 顶、单点居中、空序列空、平线居中（A008 #16）
        let pts = temp_curve_points(&[10.0, 20.0, 15.0], 100.0, 50.0);
        assert_eq!(pts.len(), 3);
        assert!((pts[0].1 - 50.0).abs() < 1e-5, "min 贴底（y=height）");
        assert!(pts[1].1.abs() < 1e-5, "max 顶（y=0）");
        assert!((pts[2].0 - 100.0).abs() < 1e-5, "末点贴右缘");
        assert!(temp_curve_points(&[], 100.0, 50.0).is_empty());
        assert_eq!(temp_curve_points(&[7.0], 100.0, 50.0).len(), 1);
        // 等温序列：全部居中（y=height/2），不再贴底缘
        for (_, y) in temp_curve_points(&[21.0, 21.0, 21.0], 100.0, 50.0) {
            assert!((y - 25.0).abs() < 1e-5, "平线应居中，实际 {y}");
        }
    }

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
}
