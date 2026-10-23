//! 世界时钟页（UIR 重塑：磁贴网格）——三列城市磁贴（名称/等宽走时/昼夜图标/
//! 基准金点）+ 点卡选基准 + 悬停减号移除 + 底部添加下拉
//! 纯函数 now_in_tz 可测（DST 边界走 chrono-tz 表）；FIX007.2 城市名权威域延续

use crate::widgets::digits;
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
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        let utc_now = chrono::Utc::now();
        // 常驻城市（FIX007.2：权威域 = 城市名；显示直用名，走时经 tz_of 映射）
        let pins: Vec<String> = state.world_pins_view().to_vec();
        let base_tz = state.base_timezone.clone();

        // 三列磁贴网格（矢量圆角卡：名称 + 金点基准标 + 昼夜图标 + 等宽走时）；
        // 内容包 ScrollArea（城市多时不出窗）
        let gap = 10.0_f32;
        let cols = 3usize;
        // A008 #8：极窄窗负宽钳 0（clamp 下限 0，上限 260 恒大于下限不 panic）
        let tile_w =
            ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32).clamp(0.0, 260.0);
        let tile_h = 86.0_f32;
        let radius = crate::theme::radius_px("md") as u8;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("world_grid")
                .num_columns(cols)
                .spacing([gap, gap])
                .show(ui, |ui| {
                    for (idx, name) in pins.iter().enumerate() {
                        let (rect, hit) = ui
                            .allocate_exact_size(egui::vec2(tile_w, tile_h), egui::Sense::click());
                        let tz_str = accel_core::data::world_cities::tz_of(name);
                        let is_base = tz_str.is_some_and(|tz| base_tz.as_deref() == Some(tz));
                        let (walk, is_day) = tz_str
                            .and_then(|tz| now_in_tz(utc_now, tz))
                            .unwrap_or_else(|| ("--:--:--".to_string(), true));

                        // 磁贴底：卡面色；基准 = 金描边（选中态一眼可辨）
                        ui.painter().rect_filled(
                            rect,
                            egui::epaint::CornerRadius::same(radius),
                            crate::theme::card_bg(),
                        );
                        let stroke_c = if is_base {
                            crate::theme::accent_color()
                        } else {
                            crate::theme::border_color()
                        };
                        ui.painter().rect_stroke(
                            rect,
                            egui::epaint::CornerRadius::same(radius),
                            egui::Stroke::new(if is_base { 1.4 } else { 1.0 }, stroke_c),
                            egui::StrokeKind::Inside,
                        );
                        // 城市名 + 基准金点
                        ui.painter().text(
                            egui::pos2(rect.left() + 16.0, rect.top() + 12.0),
                            egui::Align2::LEFT_TOP,
                            name.as_str(),
                            egui::FontId::proportional(crate::theme::font_size("body_lg")),
                            if is_base { ink.strong } else { ink.card },
                        );
                        if is_base {
                            ui.painter().circle_filled(
                                egui::pos2(rect.left() + 8.0, rect.top() + 18.0),
                                3.0,
                                crate::theme::accent_color(),
                            );
                        }
                        // 昼夜图标（右上）
                        crate::widgets::icons::draw(
                            ui.painter(),
                            egui::pos2(rect.right() - 22.0, rect.top() + 20.0),
                            16.0,
                            if is_day {
                                crate::widgets::icons::Icon::Sun
                            } else {
                                crate::widgets::icons::Icon::Moon
                            },
                            ink.dim,
                        );
                        // 走时（等宽数字位；左下锚定）
                        let walk_font = digits::digit_font(crate::theme::font_size("city_time"));
                        let walk_w = digits::text_width(ui.painter(), &walk, &walk_font);
                        digits::draw_centered(
                            ui.painter(),
                            egui::pos2(rect.left() + 16.0 + walk_w / 2.0, rect.bottom() - 22.0),
                            &walk,
                            &walk_font,
                            if is_base { ink.strong } else { ink.muted },
                        );
                        // 减号（悬停显形；城市数 ≤1 隐藏——常驻集不可清空）
                        let mut del_hover = false;
                        if pins.len() > 1 {
                            let del_resp = ui.interact(
                                egui::Rect::from_center_size(
                                    egui::pos2(rect.right() - 22.0, rect.bottom() - 22.0),
                                    egui::vec2(24.0, 24.0),
                                ),
                                egui::Id::unique(("world_del", name.as_str())),
                                egui::Sense::click(),
                            );
                            del_hover = del_resp.hovered();
                            if del_hover {
                                crate::widgets::icons::draw(
                                    ui.painter(),
                                    egui::pos2(rect.right() - 22.0, rect.bottom() - 22.0),
                                    14.0,
                                    crate::widgets::icons::Icon::Minus,
                                    ink.danger,
                                );
                            }
                            if del_resp.clicked() {
                                bus.send(FrontendCommand::RemoveWorldCity(name.clone()));
                            }
                        }
                        // 点卡选基准（IANA 经 tz_of 映射——与 set_base_timezone 校验同域；
                        // 命中排除减号悬停——防误触）
                        if hit.clicked()
                            && !del_hover
                            && let Some(tz) = tz_str
                        {
                            bus.send(FrontendCommand::SetBaseTimezone(tz.to_string()));
                            state.base_timezone = Some(tz.to_string());
                        }
                        // 三列满行换行（原逐磁贴 end_row 退化为单列——截图实证修正）
                        if (idx + 1) % cols == 0 || idx + 1 == pins.len() {
                            ui.end_row();
                        }
                    }
                });
        });

        ui.add_space(10.0);
        // 加号行：候选下拉（world_cities 全表 − 已常驻名）
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("添加城市")
                    .size(crate::theme::font_size("label"))
                    .color(ink.muted),
            );
            let candidates: Vec<&'static str> = accel_core::data::world_cities::all_names()
                .into_iter()
                .filter(|n| !pins.iter().any(|p| p == n))
                .collect();
            egui::ComboBox::from_id_salt("world_add_combo")
                .selected_text("选择城市…")
                .width(160.0)
                .show_ui(ui, |ui| {
                    for c in candidates {
                        if ui.selectable_label(false, c).clicked() {
                            bus.send(FrontendCommand::AddWorldCity(c.to_string()));
                        }
                    }
                });
        });
    }
}

// ===== src/pages/world.rs 说明区 =====
// 输入：UiState.world_pins 视图（**城市名**持久集，FIX007.2 域统一）+ base_timezone；
// 输出：城市磁贴网格（名称/昼夜/等宽走时/基准金点）+ Set/RemoveWorldCity 命令
// 逻辑步骤：三列 Grid 磁贴 → tz_of 映射 IANA → now_in_tz 走时（6-18 昼）→
//   点卡发 SetBaseTimezone（IANA）→ 悬停减号发 RemoveWorldCity → 底部候选下拉
// 设计理由（UIR 重塑）：世界时钟的本质是"多只表"——磁贴即表盘（一城一面），
//   等宽数字位对齐各城走秒；基准城金框金点（选中态一眼可辨，替代旧整行高亮）；
//   减号悬停显形（界面安静、操作可达）
// FIX007.2 域统一延续：显示直用名；旧存档 IANA 由 settings::load 装载迁移；
//   表外值占位走时（--:--:--）且点卡不发命令
// 异常处理：tz 映射失败显示占位走时；城市数 ≤1 无减号（常驻集不可清空）
// 关联依赖：data::world_cities/chrono-tz/widgets(digits,icons)；消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pins_are_unique_city_names_with_walk() {
        // 延续 FIX007.2 回归：默认常驻集 = 城市名（唯一显示 + 每城可映射 IANA 出走时）
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
    }

    #[test]
    fn tz_of_is_name_domain_only() {
        // tz_of 语义 = 城市名 → IANA；对 IANA 入参必返 None（域不混用的可证锚点）
        assert_eq!(
            accel_core::data::world_cities::tz_of("东京"),
            Some("Asia/Tokyo")
        );
        assert!(
            accel_core::data::world_cities::tz_of("Asia/Tokyo").is_none(),
            "IANA 串不是城市名"
        );
    }
}
