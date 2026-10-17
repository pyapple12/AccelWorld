//! 天气页（PL014.12；A005 #4 收敛为单城）：当前卡 + 逐时横条 + 七日概要 + 刷新

use crate::glass::surface::GlassScene;
use accel_core::types::FrontendCommand;

impl crate::pages::Page for crate::pages::WeatherPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut crate::bus::UiState,
        bus: &crate::bus::Bus,
        _scene: &mut GlassScene,
    ) {
        // A005 #9：语义色 token（不再写字面色值）
        let ink = crate::theme::ink();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(crate::window::content_left_inset());
                ui.vertical(|ui| {
                    let city = state.prefs.last_city.clone();
                    // 城市选择 + 刷新（A005 #4：单城市模型——从天气城市表选城并持久化 last_city）
                    ui.horizontal(|ui| {
                        let city_names = accel_core::interface::AppInterface::get_city_names();
                        egui::ComboBox::from_id_salt("weather_city")
                            .selected_text(city.as_str())
                            .show_ui(ui, |ui| {
                                for name in &city_names {
                                    if ui.selectable_label(name == &city, name).clicked()
                                        && name != &city
                                    {
                                        bus.send(FrontendCommand::SetWeatherCity(name.clone()));
                                        bus.send(FrontendCommand::RefreshWeather {
                                            city: name.clone(),
                                            force: false,
                                        });
                                        state.prefs.last_city = name.clone();
                                    }
                                }
                            });
                        if ui.button("⟳ 刷新").clicked() {
                            bus.send(FrontendCommand::RefreshWeather {
                                city: city.clone(),
                                force: true,
                            });
                        }
                    });
                    ui.add_space(8.0);
                    // 当前卡 / 失败提示
                    if let Some(err) = state.weather_errors.get(&city) {
                        ui.label(
                            egui::RichText::new(format!("{city} 获取失败：{err}"))
                                .color(ink.danger),
                        );
                    }
                    if let Some(w) = state.weather.get(&city) {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {}  {:.1}°C",
                                w.icon, w.weather, w.temperature
                            ))
                            .size(crate::theme::font_size("std_time"))
                            .color(ink.strong),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "体感 {:.1}°C · 湿度 {:.0}% · 风力 {:.1}km/h",
                                w.apparent_temperature, w.humidity, w.wind_speed
                            ))
                            .size(crate::theme::font_size("body"))
                            .color(ink.soft),
                        );
                        ui.add_space(12.0);
                        // 逐小时横条（24 点）
                        if !w.hourly.is_empty() {
                            ui.label(
                                egui::RichText::new("逐小时")
                                    .size(crate::theme::font_size("label"))
                                    .color(ink.muted),
                            );
                            ui.horizontal(|ui| {
                                for h in w.hourly.iter().take(24) {
                                    ui.vertical(|ui| {
                                        let hour = h.time.get(11..16).unwrap_or("--:--");
                                        ui.label(
                                            egui::RichText::new(hour)
                                                .size(crate::theme::font_size("meta"))
                                                .color(ink.soft),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}°", h.temperature))
                                                .size(crate::theme::font_size("body"))
                                                .color(ink.strong),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", h.precip_prob))
                                                .size(crate::theme::font_size("tiny"))
                                                .color(ink.info),
                                        );
                                    });
                                }
                            });
                            ui.add_space(12.0);
                        }
                        // 七日概要
                        if !w.daily.is_empty() {
                            ui.label(
                                egui::RichText::new("七日概要")
                                    .size(crate::theme::font_size("label"))
                                    .color(ink.muted),
                            );
                            for d in w.daily.iter().take(7) {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&d.date)
                                            .size(crate::theme::font_size("body"))
                                            .color(ink.muted),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.0}° / {:.0}°",
                                            d.t_max, d.t_min
                                        ))
                                        .size(crate::theme::font_size("body"))
                                        .color(ink.strong),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "☀ {}  ☾ {}",
                                            d.sunrise.get(11..16).unwrap_or("--"),
                                            d.sunset.get(11..16).unwrap_or("--")
                                        ))
                                        .size(crate::theme::font_size("meta"))
                                        .color(ink.soft),
                                    );
                                });
                            }
                        }
                    }
                });
            });
        });
    }
}

// ===== src/pages/weather.rs 说明区 =====
// 输入：UiState.weather/weather_errors（WeatherUpdated/Failed 事件快照）+ prefs.last_city
// 输出：当前卡 + 逐时 24 横条 + 七日概要 + 城市选择 + 强制刷新
// 逻辑步骤：city=last_city → 城市下拉（换城发 SetWeatherCity + RefreshWeather）→
//   刷新发 RefreshWeather{force:true} → weather 查数据域（失败红字优先展示）
// 设计理由：UI 无状态重建——数据全靠事件快照；逐时/七日来自 PL014.11 扩展 DTO；
//   **A005 #4 定案**：多城市增删（AddWeatherCity/RemoveWeatherCity）未接线且 Python 版
//   本就是单城市（last_city），按"不长期保留废弃方案"删除死协议变体与 UiState.
//   weather_cities / weather_cities_view，收敛单城——更正的是 z.plan/README 的过度承诺
// FIX006.2：命令统一经 bus.send（待发队列保底）；FIX006.5：字号全部走 scale token；
//   FIX006.4：冷启动首拉在 window.rs 装配期发出（本页仅消费事件快照）
// 异常处理：数据缺失（首查前）整段静默；失败红字展示（FIX005.13 后失败会清旧数据）
// 关联依赖：UiState.weather；消费方：pages::Pages
