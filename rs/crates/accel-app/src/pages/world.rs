//! 世界时钟页（PL014.09/.10）：城市卡矩阵（chrono-tz 走时 + 昼夜指示）+ 管理
//! 纯函数 now_in_tz 可测（DST 边界走 chrono-tz 表）；点卡选基准、加减号管理

use crate::glass::surface::GlassScene;
use accel_core::types::FrontendCommand;
use chrono::{DateTime, Timelike};

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
                        // 城市卡矩阵：常驻城市（FIX007.2：权威域 = 城市名——同 tz 多城
                        // 身份可分，显示直用名；走时经 tz_of 映射 IANA，表外值占位）
                        let pins: Vec<String> = state.world_pins_view().to_vec();
                        let base_tz = state.base_timezone.clone();
                        for name in &pins {
                            let tz_str = accel_core::data::world_cities::tz_of(name);
                            let (walk, is_day) = tz_str
                                .and_then(|tz| now_in_tz(utc_now, tz))
                                .unwrap_or_else(|| ("--:--:--".to_string(), true));
                            let is_base = tz_str.is_some_and(|tz| base_tz.as_deref() == Some(tz));
                            ui.horizontal(|ui| {
                                let indicator = if is_day { "☀" } else { "☾" };
                                let label =
                                    egui::RichText::new(format!("{indicator} {name}  {walk}"))
                                        .size(crate::theme::font_size("city_time"))
                                        .color(if is_base {
                                            crate::theme::accent_color()
                                        } else {
                                            ink.card
                                        });
                                if ui.selectable_label(is_base, label).clicked()
                                    && let Some(tz) = tz_str
                                {
                                    // 基准时区命令收 IANA 串（与 set_base_timezone 校验同域）
                                    bus.send(FrontendCommand::SetBaseTimezone(tz.to_string()));
                                    state.base_timezone = Some(tz.to_string());
                                }
                                // 减号（城市数 ≤1 禁用；入参城市名——与落盘集同域）
                                if pins.len() > 1 && ui.small_button("－").clicked() {
                                    bus.send(FrontendCommand::RemoveWorldCity(name.clone()));
                                }
                            });
                        }
                        ui.add_space(12.0);
                        // 加号行：候选下拉（world_cities 全表 − 已常驻城市名）
                        ui.horizontal(|ui| {
                            ui.label("＋ 添加城市：");
                            let candidates: Vec<&'static str> =
                                accel_core::data::world_cities::all_names()
                                    .into_iter()
                                    .filter(|n| !pins.iter().any(|p| p == n))
                                    .collect();
                            egui::ComboBox::from_id_salt("world_add_combo")
                                .selected_text("选择城市…")
                                .show_ui(ui, |ui| {
                                    for c in candidates {
                                        if ui.selectable_label(false, c).clicked() {
                                            // 入参城市名，后端按城市名落盘（FIX007.2 域统一）
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
// 输入：UiState.world_pins 视图（**城市名**持久集，FIX007.2 域统一）+ base_timezone 选中态；
// 输出：城市卡矩阵（城市名 + 昼夜指示/走时）+ SetBaseTimezone/Add/RemoveWorldCity 命令
// 逻辑步骤：每卡 tz_of(name) → chrono-tz now_in_tz 走时（6-18 昼）→ 点卡发基准时区
//   命令（IANA，经 tz_of 映射）→ 减号（≤1 禁用）→ 加号候选下拉（全表名 − 已常驻名）
// 设计理由：走时纯函数 now_in_tz 可测（DST 边界走 chrono-tz 表——伦敦 3 月末切换
//   自动正确）；命令统一经 bus.send（FIX006.2：待发队列保底不丢命令）；城市集由后端
//   持久集驱动（AddWorldCity/RemoveWorldCity 已接线 PL012.10）
// FIX007.2 域统一：world_pins 权威域 = 城市名（原 IANA 存储同 tz 多城身份丢失——
//   加入大阪显示东京/上海被北京吞）；显示直用名（身份即所见），走时经 tz_of 映射；
//   旧存档 IANA 串由 settings::load 装载期迁移；基准时区仍 IANA（SetBaseTimezone）
// 异常处理：tz 映射失败（表外值）显示占位走时（--:--:--，昼夜按昼）；点表外卡不发命令
// 关联依赖：data::world_cities/chrono-tz；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::*;
    use accel_core::data::world_cities::tz_of;

    #[test]
    fn default_pins_are_unique_city_names_with_walk() {
        // FIX007.2 回归：默认常驻集 = 城市名（唯一显示 + 每城可映射 IANA 出走时）；
        // 原缺陷（FIX006.1）为 IANA 存储——同 tz 多城（东京/大阪）身份互串
        let pins = accel_core::static_config::Static::get().base_str_list("default_world_pins");
        assert!(pins.len() >= 4, "默认常驻城市应有多条，实际 {}", pins.len());
        let mut sorted = pins.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), pins.len(), "默认常驻集城市名应唯一");
        for name in &pins {
            let tz = accel_core::data::world_cities::tz_of(name)
                .unwrap_or_else(|| panic!("默认常驻应为表内城市: {name}"));
            let (text, _) = now_in_tz(chrono::Utc::now(), tz)
                .unwrap_or_else(|| panic!("走时应可解析: {name} → {tz}"));
            assert_eq!(text.len(), 8, "走时应为 HH:MM:SS: {name} → {text}");
        }
        // 同 tz 多城身份可分：东京/大阪同属 Asia/Tokyo 但为不同城市名
        // （原 IANA 存储下两者合并为一串——先加者吞掉后加者）
        assert_eq!(
            accel_core::data::world_cities::tz_of("东京"),
            accel_core::data::world_cities::tz_of("大阪"),
            "东京/大阪应同 tz（Asia/Tokyo）"
        );
        assert_ne!("东京", "大阪");
    }

    #[test]
    fn tz_of_is_name_domain_only() {
        // tz_of 语义 = 城市名 → IANA；对 IANA 入参必返 None（域不混用的可证锚点）
        assert_eq!(tz_of("东京"), Some("Asia/Tokyo"));
        assert!(tz_of("Asia/Tokyo").is_none(), "IANA 串不是城市名");
    }
}
