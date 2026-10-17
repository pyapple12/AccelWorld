//! 天气页（PL014.12/.13）：当前卡 + 逐时横条 + 七日概要 + 城市切换/管理 + 刷新

use crate::bus::PageId;
use crate::glass::surface::GlassScene;
use accel_core::types::FrontendCommand;
use tokio::sync::mpsc::Sender;

impl crate::pages::Page for crate::pages::WeatherPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut crate::bus::UiState,
        cmd: &Sender<FrontendCommand>,
        scene: &mut GlassScene,
    ) {
        let content = ui.ctx().input(|i| i.content_rect());
        let page_rect = egui::Rect::from_min_size(
            content.min + egui::vec2(180.0, 20.0),
            content.size() - egui::vec2(204.0, 40.0),
        );
        crate::window::register_page_surface(scene, PageId::Weather, page_rect);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(156.0);
                ui.vertical(|ui| {
                    let city = state.prefs.last_city.clone();
                    // 城市切换条：已集 chips + 刷新按钮
                    ui.horizontal(|ui| {
                        for c in state.weather_cities_view() {
                            let selected = *c == city;
                            if ui.selectable_label(selected, c.as_str()).clicked() && !selected {
                                let _ = cmd.try_send(FrontendCommand::RefreshWeather {
                                    city: c.clone(),
                                    force: false,
                                });
                                state.prefs.last_city = c.clone();
                            }
                        }
                        if ui.button("⟳ 刷新").clicked() {
                            let _ = cmd.try_send(FrontendCommand::RefreshWeather {
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
                                .color(egui::Color32::from_rgb(228, 117, 106)),
                        );
                    }
                    if let Some(w) = state.weather.get(&city) {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {}  {:.1}°C",
                                w.icon, w.weather, w.temperature
                            ))
                            .size(34.0)
                            .color(egui::Color32::from_rgb(235, 238, 245)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "体感 {:.1}°C · 湿度 {:.0}% · 风力 {:.1}km/h",
                                w.apparent_temperature, w.humidity, w.wind_speed
                            ))
                            .size(14.0)
                            .color(egui::Color32::from_rgb(170, 176, 190)),
                        );
                        ui.add_space(12.0);
                        // 逐小时横条（24 点）
                        if !w.hourly.is_empty() {
                            ui.label(
                                egui::RichText::new("逐小时")
                                    .size(15.0)
                                    .color(egui::Color32::from_rgb(200, 206, 220)),
                            );
                            ui.horizontal(|ui| {
                                for h in w.hourly.iter().take(24) {
                                    ui.vertical(|ui| {
                                        let hour = h.time.get(11..16).unwrap_or("--:--");
                                        ui.label(
                                            egui::RichText::new(hour)
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(170, 176, 190)),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}°", h.temperature))
                                                .size(14.0)
                                                .color(egui::Color32::from_rgb(235, 238, 245)),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", h.precip_prob))
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(140, 170, 210)),
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
                                    .size(15.0)
                                    .color(egui::Color32::from_rgb(200, 206, 220)),
                            );
                            for d in w.daily.iter().take(7) {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&d.date)
                                            .size(14.0)
                                            .color(egui::Color32::from_rgb(200, 206, 220)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.0}° / {:.0}°",
                                            d.t_max, d.t_min
                                        ))
                                        .size(14.0)
                                        .color(egui::Color32::from_rgb(235, 238, 245)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "☀ {}  ☾ {}",
                                            d.sunrise.get(11..16).unwrap_or("--"),
                                            d.sunset.get(11..16).unwrap_or("--")
                                        ))
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(170, 176, 190)),
                                    );
                                });
                            }
                        }
                    }
                });
            });
        });
        let _ = cmd;
    }
}

// ===== src/pages/weather.rs 说明区 =====
// 输入：UiState.weather/weather_errors（WeatherUpdated/Failed 事件快照）
// 输出：当前卡 + 逐时 24 横条 + 七日概要 + 城市切换 chips + 强制刷新
// 逻辑步骤：city=last_city → 切换 chips 发 RefreshWeather{city} → 刷新发 force:true →
//   weather 查数据域（错误域优先展示）
// 设计理由：UI 无状态重建——数据全靠事件快照；PL014.13 多城市切换走缓存路径
//   （force=false 命中不重发）；逐时/七日来自 PL014.11 扩展 DTO
// 异常处理：数据缺失（首查前）整段静默；失败红字展示
// 关联依赖：UiState.weather；消费方：pages::Pages
