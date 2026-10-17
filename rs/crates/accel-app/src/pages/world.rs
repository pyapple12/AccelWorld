//! 世界时钟页（PL014.09/.10）：城市卡矩阵（chrono-tz 走时 + 昼夜指示）+ 管理
//! 纯函数 now_in_tz 可测（DST 边界走 chrono-tz 表）；点卡选基准、加减号管理

use crate::bus::PageId;
use crate::glass::surface::GlassScene;
use accel_core::types::FrontendCommand;
use chrono::{DateTime, Timelike};
use tokio::sync::mpsc::Sender;

/// 时区走时（纯函数）：UTC 时刻 → IANA 时区本地 "HH:MM:SS" + 是否昼（6-18 时）
pub fn now_in_tz(utc: DateTime<chrono::Utc>, tz: &str) -> Option<(String, bool)> {
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

impl crate::pages::Page for crate::pages::WorldPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut crate::bus::UiState,
        cmd: &Sender<FrontendCommand>,
        scene: &mut GlassScene,
    ) {
        let page_rect = crate::window::content_page_rect(ui.ctx());
        crate::window::register_page_surface(scene, PageId::World, page_rect);
        // A005 #9：语义色 token（不再写字面色值）
        let ink = crate::theme::ink();

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(crate::window::page_inner_margin()))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(crate::window::content_left_inset());
                    ui.vertical(|ui| {
                        let utc_now = chrono::Utc::now();
                        // 城市卡矩阵：每帧读基准时区 + 常驻城市（自有副本：循环内需写 state，借用不可行——
                        // A005 #23 保留判定：pins 同时作为下拉候选基线，每帧 ≤50 短串量级可忽略）
                        let pins: Vec<String> = state.world_pins_view().to_vec();
                        let base_tz = state.base_timezone.clone();
                        for city in &pins {
                            let tz = accel_core::data::world_cities::tz_of(city);
                            let (walk, is_day) = tz
                                .and_then(|tz| now_in_tz(utc_now, tz))
                                .unwrap_or_else(|| ("--:--:--".into(), true));
                            ui.horizontal(|ui| {
                                let base = base_tz.as_deref() == Some(city.as_str());
                                let indicator = if is_day { "☀" } else { "☾" };
                                let label =
                                    egui::RichText::new(format!("{indicator} {city}  {walk}"))
                                        .size(18.0)
                                        .color(if base {
                                            crate::theme::accent_color()
                                        } else {
                                            ink.card
                                        });
                                if ui.selectable_label(base, label).clicked() {
                                    let _ = cmd.try_send(
                                        accel_core::types::FrontendCommand::SetBaseTimezone(
                                            tz.unwrap_or("").to_string(),
                                        ),
                                    );
                                    state.base_timezone = Some(tz.unwrap_or("").to_string());
                                }
                                // 减号（城市数 ≤1 禁用）
                                if pins.len() > 1 && ui.small_button("－").clicked() {
                                    let _ = cmd.try_send(
                                        accel_core::types::FrontendCommand::RemoveWorldCity(
                                            city.clone(),
                                        ),
                                    );
                                }
                            });
                        }
                        ui.add_space(12.0);
                        // 加号行：候选下拉（world_cities 全表 − 已添加）
                        ui.horizontal(|ui| {
                            ui.label("＋ 添加城市：");
                            let added: Vec<&str> = pins.iter().map(|s| s.as_str()).collect();
                            let candidates: Vec<&'static str> =
                                accel_core::data::world_cities::all_names()
                                    .into_iter()
                                    .filter(|n| !added.contains(n))
                                    .collect();
                            egui::ComboBox::from_id_salt("world_add_combo")
                                .selected_text("选择城市…")
                                .show_ui(ui, |ui| {
                                    for c in candidates {
                                        if ui.selectable_label(false, c).clicked() {
                                            let _ = cmd.try_send(
                                                accel_core::types::FrontendCommand::AddWorldCity(
                                                    c.to_string(),
                                                ),
                                            );
                                        }
                                    }
                                });
                        });
                    });
                });
            });
    }
}

// ===== src/pages/world.rs 说明区 =====
// 输入：UiState.world_pins 视图（world_pins 持久集）+ base_timezone 选中态；
// 输出：城市卡矩阵（昼夜指示/走时）+ SetBaseTimezone/Add/RemoveWorldCity 命令
// 逻辑步骤：每卡 chrono-tz now_in_tz 走时（6-18 昼）→ 点卡发基准时区命令 →
//   减号（≤1 禁用）→ 加号候选下拉（全表 − 已添加）
// 设计理由：走时纯函数 now_in_tz 可测（DST 边界走 chrono-tz 表——伦敦 3 月末切换
//   自动正确）；城市集由后端持久集驱动（AddWorldCity/RemoveWorldCity 已接线 PL012.10）
// 异常处理：tz 解析失败显示占位走时
// 关联依赖：data::world_cities/chrono-tz；消费方：pages::Pages
