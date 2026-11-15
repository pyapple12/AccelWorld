//! 世界时钟页（PL017 重排）：紧凑城市行（名称/基准标签/昼夜/24px 走时）+ 固定
//! Footer 添加城市；去掉每卡常驻减号，删除改为 Trash 两步确认。
//! 纯函数 now_in_tz 可测（DST 边界走 chrono-tz 表）。

use crate::widgets::text_ink::InkKind;
use crate::widgets::{digits, icons};
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
        state.armed_delete_tick();
        let utc_now = chrono::Utc::now();
        let pins: Vec<String> = state.world_pins_view().to_vec();
        let base_tz = state.base_timezone.clone();

        // 固定页脚盒高 44；body 与页脚间距 10
        let avail = ui.max_rect();
        let footer_h =
            crate::widgets::select::height(ui).max(crate::widgets::text_metrics::height(
                ui.ctx(),
                &crate::theme::font_medium(crate::theme::font_size("meta")),
                InkKind::Cjk,
            )) + crate::theme::spacing_f32("sm");
        let body_h = (avail.height() - footer_h - crate::theme::layout_f32("footer_gap")).max(0.0);
        let body_rect = egui::Rect::from_min_size(avail.min, egui::vec2(avail.width(), body_h));
        let footer_rect = egui::Rect::from_min_size(
            egui::pos2(
                avail.left(),
                body_rect.bottom() + crate::theme::layout_f32("footer_gap"),
            ),
            egui::vec2(avail.width(), footer_h),
        );

        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut stack = crate::widgets::box_stack::BoxStack::new(ui);
                    if pins.is_empty() {
                        let rect = stack.next(ui, 96.0);
                        crate::widgets::empty::card_at(
                            ui,
                            rect,
                            "还没有城市",
                            "从下方添加一个城市，开始对照世界时间",
                        );
                        return;
                    }
                    let pulse = crate::widgets::motion::list_pulse(state, "world");
                    let last = pins.len().saturating_sub(1);
                    let name_font = crate::theme::font_semibold(crate::theme::font_size("body_lg"));
                    let sub_font = crate::theme::font_regular(crate::theme::font_size("meta"));
                    let walk_font = digits::digit_font(24.0);
                    for (i, name) in pins.iter().enumerate() {
                        // 行盒高 = max(固定名称行高 + 6 + 副行/基准 chip 高, 走时行高) + 上下 12
                        let name_h = crate::widgets::text_metrics::height(
                            ui.ctx(),
                            &name_font,
                            InkKind::Cjk,
                        );
                        let sub_h =
                            crate::widgets::text_metrics::height(ui.ctx(), &sub_font, InkKind::Cjk);
                        let walk_h = crate::widgets::text_metrics::height(
                            ui.ctx(),
                            &walk_font,
                            InkKind::Digits,
                        );
                        // 基准行用 18px chip，普通行用副行行高；两者与绘制端同一分块
                        let sub_block_h = if accel_core::data::world_cities::tz_of(name)
                            .is_some_and(|tz| base_tz.as_deref() == Some(tz))
                        {
                            18.0
                        } else {
                            sub_h
                        };
                        let row_h = (name_h + 6.0 + sub_block_h).max(walk_h) + 24.0;
                        let rect = stack.next(ui, row_h);
                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            if i == last && pulse < 1.0 {
                                ui.set_opacity(0.55 + 0.45 * pulse);
                            }
                            draw_city_row(
                                ui, rect, state, bus, name, &pins, &base_tz, utc_now, &ink,
                            );
                        });
                    }
                });
        });

        // 添加城市页脚盒：左标签（盒底/盒中线），右下拉（原生控件进盒）
        let label_font = crate::theme::font_medium(crate::theme::font_size("meta"));
        let label_text = "添加城市";
        let label_w = ui
            .painter()
            .layout_no_wrap(label_text.to_string(), label_font.clone(), ink.muted)
            .rect
            .width();
        let combo_w = (avail.width() - label_w - 14.0).clamp(120.0, 220.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
            ui.painter().text(
                egui::pos2(
                    footer_rect.left(),
                    crate::widgets::text_ink::top_for_center(
                        ui,
                        label_text,
                        &label_font,
                        InkKind::Cjk,
                        footer_rect.center().y,
                    ),
                ),
                egui::Align2::LEFT_TOP,
                label_text,
                label_font,
                ink.muted,
            );
            let candidates: Vec<String> = accel_core::data::world_cities::all_names()
                .into_iter()
                .filter(|n| !pins.iter().any(|p| p == n))
                .map(str::to_string)
                .collect();
            let select_h = crate::widgets::select::height(ui);
            let select_rect = egui::Rect::from_min_size(
                egui::pos2(
                    footer_rect.left() + label_w + 10.0,
                    footer_rect.center().y - select_h / 2.0,
                ),
                egui::vec2(combo_w, select_h),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(select_rect));
            if let Some(picked) = crate::widgets::select::show(
                &mut child,
                egui::Id::unique("world_add_select"),
                "选择城市…",
                &candidates,
                combo_w,
            ) {
                bus.send(FrontendCommand::AddWorldCity(picked));
                state.list_pulse = Some(("world".to_string(), std::time::Instant::now()));
            }
        });
    }
}

/// 城市行：名称 + 基准标签 + 昼/夜 + 右对齐走时 + Trash（两步确认）
#[allow(clippy::too_many_arguments)]
fn draw_city_row(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut crate::bus::UiState,
    bus: &crate::bus::Bus,
    name: &str,
    pins: &[String],
    base_tz: &Option<String>,
    utc_now: DateTime<chrono::Utc>,
    ink: &crate::theme::Ink,
) {
    let hit = ui.interact(
        rect,
        egui::Id::unique(("world_row", name)),
        egui::Sense::click(),
    );
    let tz_str = accel_core::data::world_cities::tz_of(name);
    let is_base = tz_str.is_some_and(|tz| base_tz.as_deref() == Some(tz));
    let (walk, is_day) = tz_str
        .and_then(|tz| now_in_tz(utc_now, tz))
        .unwrap_or_else(|| ("--:--:--".to_string(), true));

    let hover_t = crate::widgets::motion::hover_t(
        ui.ctx(),
        egui::Id::unique(("world_row", name)),
        hit.hovered() || hit.has_focus(),
        crate::theme::motion_ms("card_hover_ms"),
    );
    ui.painter().rect_filled(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        crate::widgets::motion::lerp_color(
            crate::theme::card_bg(),
            crate::theme::hover_bg(),
            hover_t,
        ),
    );
    ui.painter().rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        egui::Stroke::new(
            1.0,
            if is_base {
                crate::theme::border_strong_color()
            } else {
                crate::widgets::motion::lerp_color(
                    crate::theme::border_color(),
                    crate::theme::border_strong_color(),
                    hover_t,
                )
            },
        ),
        egui::StrokeKind::Inside,
    );
    let name_font = crate::theme::font_semibold(crate::theme::font_size("body_lg"));
    let sub_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let name_h = crate::widgets::text_metrics::height(ui.ctx(), &name_font, InkKind::Cjk);
    let sub_h = crate::widgets::text_metrics::height(ui.ctx(), &sub_font, InkKind::Cjk);
    let chip_h = 18.0;
    let sub_block_h = if is_base { chip_h } else { sub_h };
    let name_box = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 14.0, rect.top() + 12.0),
        egui::vec2(rect.width() - 28.0, name_h),
    );
    let name_ink = crate::widgets::text_ink::measure(ui, name, &name_font, InkKind::Cjk);
    ui.painter().text(
        egui::pos2(name_box.left(), name_ink.top_for_bottom(name_box.bottom())),
        egui::Align2::LEFT_TOP,
        name,
        name_font.clone(),
        ink.strong,
    );
    let sub_top = name_box.bottom() + 6.0;
    if is_base {
        // 基准：左侧 2px 金线 + 中性 chip（Pin + 文字），金色只做状态标记
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(rect.left() + 1.0, rect.center().y - 14.0),
                egui::vec2(2.0, 28.0),
            ),
            egui::epaint::CornerRadius::same(1),
            crate::theme::accent_color(),
        );
        let chip = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 14.0, sub_top),
            egui::vec2(52.0, chip_h),
        );
        ui.painter().rect_filled(
            chip,
            egui::epaint::CornerRadius::same(6),
            crate::theme::hover_bg(),
        );
        icons::draw(
            ui.painter(),
            egui::pos2(chip.left() + 9.0, chip.center().y),
            11.0,
            icons::Icon::Pin,
            crate::theme::accent_color(),
        );
        let chip_font = crate::theme::font_regular(11.0);
        ui.painter().text(
            egui::pos2(
                chip.left() + 18.0,
                crate::widgets::text_ink::top_for_center(
                    ui,
                    "基准",
                    &chip_font,
                    InkKind::Cjk,
                    chip.center().y,
                ),
            ),
            egui::Align2::LEFT_TOP,
            "基准",
            chip_font,
            ink.muted,
        );
    } else if hit.hovered() || hit.has_focus() {
        let hint_box = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 14.0, sub_top),
            egui::vec2(rect.width() - 28.0, sub_block_h),
        );
        let hint_font = crate::theme::font_regular(11.0);
        let hint_ink =
            crate::widgets::text_ink::measure(ui, "点击设为基准", &hint_font, InkKind::Cjk);
        ui.painter().text(
            egui::pos2(hint_box.left(), hint_ink.top_for_bottom(hint_box.bottom())),
            egui::Align2::LEFT_TOP,
            "点击设为基准",
            hint_font,
            ink.meta,
        );
    }

    let walk_font = digits::digit_font(24.0);
    let walk_w = digits::text_width(ui.painter(), &walk, &walk_font);
    digits::draw_bottom(
        ui.painter(),
        rect.bottom() - 12.0,
        rect.right() - 48.0 - walk_w / 2.0,
        &walk,
        &walk_font,
        if is_base { ink.strong } else { ink.body },
    );
    // 昼夜：小图标放在时间左侧；窄窗跳过（避免压字）
    if rect.width() > 280.0 {
        icons::draw(
            ui.painter(),
            egui::pos2(rect.right() - 56.0 - walk_w, rect.center().y),
            14.0,
            if is_day {
                icons::Icon::Sun
            } else {
                icons::Icon::Moon
            },
            ink.meta,
        );
    }

    // 删除：Trash（不是减号），常显低对比；两步确认
    let mut del_hover = false;
    if pins.len() > 1 {
        let del_key = format!("world:{name}");
        let armed = matches!(&state.armed_delete, Some((k, _)) if *k == del_key);
        let del_rect = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 18.0, rect.center().y),
            egui::vec2(24.0, 24.0),
        );
        let del = ui.interact(
            del_rect,
            egui::Id::unique(("world_del", name)),
            egui::Sense::click(),
        );
        del_hover = del.hovered();
        icons::draw(
            ui.painter(),
            del_rect.center(),
            14.0,
            if armed {
                icons::Icon::Check
            } else {
                icons::Icon::Trash
            },
            if armed || del_hover {
                ink.danger
            } else {
                ink.meta
            },
        );
        if del.has_focus() {
            ui.painter().rect_stroke(
                del_rect,
                egui::epaint::CornerRadius::same(6),
                egui::Stroke::new(1.5, ink.knob),
                egui::StrokeKind::Inside,
            );
        }
        let del = del
            .on_hover_text(if armed {
                "再次点击确认移除"
            } else {
                "移除城市"
            })
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if del.clicked() {
            if armed {
                bus.send(FrontendCommand::RemoveWorldCity(name.to_string()));
                state.armed_delete = None;
            } else {
                state.armed_delete = Some((del_key, std::time::Instant::now()));
            }
        }
    }

    let hit = hit.on_hover_cursor(egui::CursorIcon::PointingHand);
    if hit.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(2.0),
            egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
            egui::Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    let keyboard = hit.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    if (hit.clicked() || keyboard)
        && !del_hover
        && let Some(tz) = tz_str
    {
        bus.send(FrontendCommand::SetBaseTimezone(tz.to_string()));
        state.base_timezone = Some(tz.to_string());
    }
}

// ===== src/pages/world.rs 说明区 =====
// 输入：world_pins（城市名权威域）+ base_timezone；输出：城市行 + Set/RemoveWorldCity
// 逻辑步骤：快照 → 60px 行（名称/基准标签/昼夜/24px 走时/Trash）→ 点行设基准 →
//   Trash 两步确认 → Footer 下拉添加
// 设计理由（PL017）：原 86px 四角卡 + 每卡常驻减号，8 个减号读作“缺失值”；改
//   紧凑行 + Trash + “基准”文字，基准不只靠颜色；Footer 常驻，添加不被滚出
// 异常处理：tz 映射失败显示占位走时；城市数 ≤1 隐藏删除；窄窗跳过昼夜图标
// 关联依赖：data::world_cities/chrono-tz/widgets(digits/icons/empty)；
//   消费方：pages::Pages 抽屉
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pins_are_unique_city_names_with_walk() {
        let pins = accel_core::static_config::Static::get().base_str_list("default_world_pins");
        assert!(pins.len() >= 4);
        let mut sorted = pins.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), pins.len());
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
