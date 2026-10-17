//! 世界时钟页（PL014.09/.10）：城市卡矩阵（chrono-tz 走时 + 昼夜指示）+ 管理
//! 纯函数 now_in_tz 可测（DST 边界走 chrono-tz 表）；点卡选基准、加减号管理

use crate::glass::surface::GlassScene;
use accel_core::types::FrontendCommand;
use chrono::{DateTime, Timelike};

/// 常驻城市显示名（FIX006.1：常驻集以 IANA 串持久化，显示时反查城市名；未收录串回显原串）
pub fn display_name(tz: &str) -> &str {
    accel_core::data::world_cities::name_of_tz(tz).unwrap_or(tz)
}

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
        bus: &crate::bus::Bus,
        _scene: &mut GlassScene,
    ) {
        // A005 #9：语义色 token（不再写字面色值）
        let ink = crate::theme::ink();

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(crate::window::page_inner_margin()))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(crate::window::content_left_inset());
                    ui.vertical(|ui| {
                        let utc_now = chrono::Utc::now();
                        // 城市卡矩阵：常驻城市（FIX006.1：权威域 = IANA 时区串，与默认值/golden/基准时区同域）
                        let pins: Vec<String> = state.world_pins_view().to_vec();
                        let base_tz = state.base_timezone.clone();
                        for tz_str in &pins {
                            let (walk, is_day) = now_in_tz(utc_now, tz_str)
                                .unwrap_or_else(|| ("--:--:--".to_string(), true));
                            let name = display_name(tz_str);
                            ui.horizontal(|ui| {
                                let base = base_tz.as_deref() == Some(tz_str.as_str());
                                let indicator = if is_day { "☀" } else { "☾" };
                                let label =
                                    egui::RichText::new(format!("{indicator} {name}  {walk}"))
                                        .size(crate::theme::font_size("city_time"))
                                        .color(if base {
                                            crate::theme::accent_color()
                                        } else {
                                            ink.card
                                        });
                                if ui.selectable_label(base, label).clicked() {
                                    // 基准时区命令收 IANA 串（与 set_base_timezone 校验同域）
                                    bus.send(FrontendCommand::SetBaseTimezone(tz_str.clone()));
                                    state.base_timezone = Some(tz_str.clone());
                                }
                                // 减号（城市数 ≤1 禁用）
                                if pins.len() > 1 && ui.small_button("－").clicked() {
                                    bus.send(FrontendCommand::RemoveWorldCity(tz_str.clone()));
                                }
                            });
                        }
                        ui.add_space(12.0);
                        // 加号行：候选下拉（world_cities 全表 − 已添加）
                        ui.horizontal(|ui| {
                            ui.label("＋ 添加城市：");
                            let added: Vec<&str> = pins.iter().map(|s| s.as_str()).collect();
                            // 候选按城市名列出，排除其 IANA 串已在常驻集中者（FIX006.1 域统一）
                            let candidates: Vec<&'static str> =
                                accel_core::data::world_cities::all_names()
                                    .into_iter()
                                    .filter(|n| {
                                        !accel_core::data::world_cities::tz_of(n)
                                            .is_some_and(|tz| added.contains(&tz))
                                    })
                                    .collect();
                            egui::ComboBox::from_id_salt("world_add_combo")
                                .selected_text("选择城市…")
                                .show_ui(ui, |ui| {
                                    for c in candidates {
                                        if ui.selectable_label(false, c).clicked() {
                                            // 入参城市名，后端映射为 IANA 串落盘（FIX006.1）
                                            bus.send(FrontendCommand::AddWorldCity(c.to_string()));
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
// 输入：UiState.world_pins 视图（**IANA 时区串**持久集）+ base_timezone 选中态；
// 输出：城市卡矩阵（友好名 + 昼夜指示/走时）+ SetBaseTimezone/Add/RemoveWorldCity 命令
// 逻辑步骤：每卡 chrono-tz now_in_tz 走时（6-18 昼）→ 显示名经 name_of_tz 反查 →
//   点卡发基准时区命令（IANA）→ 减号（≤1 禁用）→ 加号候选下拉（全表名 − 时区已常驻者）
// 设计理由：走时纯函数 now_in_tz 可测（DST 边界走 chrono-tz 表——伦敦 3 月末切换
//   自动正确）；命令统一经 bus.send（FIX006.2：待发队列保底不丢命令）；城市集由后端
//   持久集驱动（AddWorldCity/RemoveWorldCity 已接线 PL012.10）
// FIX006.1 域统一：world_pins 权威域 = IANA 串（与 base.json 默认值、golden 夹具、
//   set_base_timezone 校验同域）；添加入口收城市名、由 interface 映射为 IANA 落盘；
//   显示经 display_name 反查（未收录串回显原串）
// 异常处理：tz 解析失败显示占位走时（--:--:--，昼夜按昼）
// 关联依赖：data::world_cities/chrono-tz；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_resolves_iana_to_city() {
        // FIX006.1 回归：常驻城市（IANA）反查友好名；未收录串回显原串
        assert_eq!(display_name("Asia/Shanghai"), "北京");
        assert_eq!(display_name("Asia/Tokyo"), "东京");
        assert_eq!(display_name("Unknown/Zone"), "Unknown/Zone");
    }

    #[test]
    fn default_pins_iana_render_walk() {
        // 原缺陷复现点：默认常驻城市为 IANA 串，按城市名检索 tz_of 会恒 None →
        // 全部显示 --:--:--；本测试锁定「IANA 直接解析即可出走时」
        let utc = chrono::Utc::now();
        let pins = accel_core::static_config::Static::get().base_str_list("default_world_pins");
        let mut any_walk = false;
        for tz in &pins {
            let walk = now_in_tz(utc, tz);
            assert!(walk.is_some(), "默认常驻城市应可解析走时: {tz}");
            let (text, _) = walk.expect("上文断言已保证");
            assert_eq!(text.len(), 8, "走时应为 HH:MM:SS: {tz} → {text}");
            any_walk = true;
        }
        assert!(any_walk, "默认常驻城市不应为空");
        // 对照：按城市名检索对 IANA 串必然失败（域混用的可证错值源头）
        assert!(
            accel_core::data::world_cities::tz_of("Asia/Shanghai").is_none(),
            "tz_of 语义为城市名 → IANA，对 IANA 入参必返 None"
        );
    }
}
