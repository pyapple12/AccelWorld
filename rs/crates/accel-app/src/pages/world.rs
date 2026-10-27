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
        // 昼夜启发式（A009 #16 登记：纯展示用固定阈 6-18 时——非业务参数，
        // 无调参场景；走时数值本身恒按真实时区计算，与本标记无关）
        (6..18).contains(&hour),
    ))
}

/// 磁贴列数（纯函数，UIR3 断点）：>480 三列（磁贴 ≥150 走时与删除钮不叠）；
/// >300 两列；否则单列——断点取在元素碰撞处而非设备预设
pub fn tile_cols(avail_w: f32) -> usize {
    if avail_w > 480.0 {
        3
    } else if avail_w > 300.0 {
        2
    } else {
        1
    }
}

impl crate::pages::Page for crate::pages::WorldPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        // PL016.04：武装超时解除（FIX011.03 单源方法 + token 时长）
        state.armed_delete_tick();
        let utc_now = chrono::Utc::now();
        // 常驻城市（FIX007.2：权威域 = 城市名；显示直用名，走时经 tz_of 映射）
        let pins: Vec<String> = state.world_pins_view().to_vec();
        let base_tz = state.base_timezone.clone();

        // 磁贴网格（矢量圆角卡：名称 + 金点基准标 + 昼夜图标 + 等宽走时）；
        // 内容包 ScrollArea（城市多时不出窗）；间距/内边距全走 token
        // UIR3：列数随可用宽断点（340px 内容区三列磁贴 105px 走时会压到删除钮——
        // better-layout「结构撑到破为止」）
        let gap = crate::theme::spacing("md");
        let pad = crate::theme::layout_i64("card_pad") as f32;
        let cols = tile_cols(ui.available_width());
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
                        let hit = hit.on_hover_cursor(egui::CursorIcon::PointingHand);

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
                            egui::pos2(rect.left() + pad, rect.top() + 12.0),
                            egui::Align2::LEFT_TOP,
                            name.as_str(),
                            egui::FontId::proportional(crate::theme::font_size("body_lg")),
                            if is_base { ink.strong } else { ink.body },
                        );
                        if is_base {
                            ui.painter().circle_filled(
                                egui::pos2(rect.left() + pad - 8.0, rect.top() + 18.0),
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
                            ink.meta,
                        );
                        // 走时（等宽数字位；左下锚定）
                        let walk_font = digits::digit_font(crate::theme::font_size("time_sm"));
                        let walk_w = digits::text_width(ui.painter(), &walk, &walk_font);
                        digits::draw_centered(
                            ui.painter(),
                            egui::pos2(rect.left() + pad + walk_w / 2.0, rect.bottom() - 22.0),
                            &walk,
                            &walk_font,
                            if is_base { ink.strong } else { ink.body },
                        );
                        // 删除（PL016.04 两步确认：Minus→Check 武装态；城市数 ≤1 隐藏——
                        // 常驻集不可清空）
                        let mut del_hover = false;
                        if pins.len() > 1 {
                            let del_key = format!("world:{name}");
                            let armed = matches!(
                                &state.armed_delete,
                                Some((k, _)) if *k == del_key
                            );
                            let del_resp = ui.interact(
                                egui::Rect::from_center_size(
                                    egui::pos2(rect.right() - 22.0, rect.bottom() - 22.0),
                                    egui::vec2(24.0, 24.0),
                                ),
                                egui::Id::unique(("world_del", name.as_str())),
                                egui::Sense::click(),
                            );
                            del_hover = del_resp.hovered();
                            let del_resp = if armed {
                                del_resp.on_hover_text("再次点击确认移除")
                            } else {
                                del_resp
                            };
                            let del_resp = del_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                            crate::widgets::icons::draw(
                                ui.painter(),
                                egui::pos2(rect.right() - 22.0, rect.bottom() - 22.0),
                                14.0,
                                if armed {
                                    crate::widgets::icons::Icon::Check
                                } else {
                                    crate::widgets::icons::Icon::Minus
                                },
                                if armed || del_hover {
                                    ink.danger
                                } else {
                                    ink.meta
                                },
                            );
                            if del_resp.clicked() {
                                if armed {
                                    bus.send(FrontendCommand::RemoveWorldCity(name.clone()));
                                    state.armed_delete = None;
                                } else {
                                    state.armed_delete = Some((del_key, std::time::Instant::now()));
                                }
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

        // 组距纪律（better-layout）：磁贴网格组内距 md(12)，组间 ≥2× → xl(24)
        ui.add_space(crate::theme::spacing("xl"));
        // 加号行：候选下拉（world_cities 全表 − 已常驻名）
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("添加城市")
                    .size(crate::theme::font_size("body"))
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
    fn tile_cols_breaks_before_walk_overlaps_delete() {
        // UIR3 验收：宽窗三列；480 恰破降两列（340 最小窗内容区磁贴 155 不叠）；
        // 300 恰破降单列
        assert_eq!(tile_cols(700.0), 3);
        assert_eq!(tile_cols(480.0), 2);
        assert_eq!(tile_cols(340.0), 2);
        assert_eq!(tile_cols(300.0), 1);
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
