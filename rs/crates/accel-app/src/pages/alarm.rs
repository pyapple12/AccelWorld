//! 闹钟页（UIR 重塑）：快捷 chips + 自定义添加（图标按钮）+ 列表行卡
//! （自绘开关 + 悬停垃圾桶）；时间走等宽数字位
//! 纯函数 quick_label 可测（快捷 chips 文案）

use crate::tools::alarm_text::format_repeat_display;
use crate::widgets::{digits, icon_button};
use accel_core::alarm::{default_sound_type, default_sound_value, quick_target};
use accel_core::types::FrontendCommand;
use chrono::Local;

/// 快捷 chips 文案（纯函数可测）：分钟数 → "N 分钟后"
pub fn quick_label(minutes: i64) -> String {
    format!("{minutes} 分钟后")
}

impl crate::pages::Page for crate::pages::AlarmPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();

        // 内容包 ScrollArea（闹钟多时不出窗）
        egui::ScrollArea::vertical().show(ui, |ui| {
            // 快捷添加（分钟数读 base.json quick_alarm_minutes token；胶囊 chip 按钮）
            ui.label(
                egui::RichText::new("快捷添加")
                    .size(crate::theme::font_size("label"))
                    .color(ink.muted),
            );
            ui.horizontal(|ui| {
                let radius = crate::theme::radius_px("sm") as u8;
                for &minutes in crate::pages::quick_alarm_minutes() {
                    let text = quick_label(minutes);
                    let galley_w = ui
                        .painter()
                        .layout_no_wrap(
                            text.clone(),
                            egui::FontId::proportional(crate::theme::font_size("body")),
                            ink.card,
                        )
                        .rect
                        .width();
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(galley_w + 24.0, 30.0),
                        egui::Sense::click(),
                    );
                    let fill = if resp.hovered() {
                        crate::theme::accent_color()
                    } else {
                        crate::theme::color32_at("faint_bg", 10)
                    };
                    ui.painter()
                        .rect_filled(rect, egui::epaint::CornerRadius::same(radius), fill);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(crate::theme::font_size("body")),
                        if resp.hovered() {
                            crate::theme::color32_at("bg_dark", 20)
                        } else {
                            ink.card
                        },
                    );
                    if resp.clicked() {
                        let draft = quick_target(minutes, Local::now());
                        bus.send(FrontendCommand::AddAlarm(draft));
                    }
                }
            });
            ui.add_space(8.0);

            // 自定义添加行：标签 + HH:MM + 加号图标按钮
            ui.horizontal(|ui| {
                egui::TextEdit::singleline(&mut state.alarm_draft_label)
                    .hint_text("标签")
                    .desired_width(140.0)
                    .show(ui);
                egui::TextEdit::singleline(&mut state.alarm_draft_time)
                    .hint_text("HH:MM")
                    .desired_width(80.0)
                    .show(ui);
                let add = icon_button::show(ui, crate::widgets::icons::Icon::Plus, 16.0, false);
                if add.on_hover_text("添加闹钟").clicked() {
                    // FIX006.5：默认铃声取 base.json token；FIX006.2：统一经 bus.send
                    bus.send(FrontendCommand::AddAlarm(accel_core::alarm::AlarmDraft {
                        label: state.alarm_draft_label.clone(),
                        time: state.alarm_draft_time.clone(),
                        sound_type: default_sound_type(),
                        sound_value: default_sound_value(),
                        repeat_days: Vec::new(),
                        enabled: true,
                    }));
                    state.alarm_draft_label.clear();
                    state.alarm_draft_time.clear();
                }
            });
            ui.add_space(10.0);

            // 列表行卡（AlarmsChanged 全量回推重建——UI 无状态）
            if state.alarms.is_empty() {
                ui.label(
                    egui::RichText::new("暂无闹钟")
                        .size(crate::theme::font_size("body_lg"))
                        .color(ink.soft),
                );
            }
            let radius = crate::theme::radius_px("md") as u8;
            for alarm in &state.alarms {
                let row_h = 56.0_f32;
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), row_h),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(
                    rect,
                    egui::epaint::CornerRadius::same(radius),
                    crate::theme::color32_at("faint_bg", 10),
                );
                // 开关（自绘胶囊；点击翻转发 ToggleAlarm）
                let sw_rect = egui::Rect::from_center_size(
                    egui::pos2(rect.left() + 34.0, rect.center().y),
                    egui::vec2(36.0, 20.0),
                );
                let sw_resp = ui.interact(
                    sw_rect,
                    egui::Id::unique(("alarm_sw", &alarm.id)),
                    egui::Sense::click(),
                );
                // 复用 switch 绘制几何（此处直接画——响应态与行卡同层）
                let on = alarm.enabled;
                let anim =
                    ui.ctx()
                        .animate_value_with_time(sw_resp.id, if on { 1.0 } else { 0.0 }, 0.1);
                let knob_r = sw_rect.height() / 2.0 - 2.0;
                let knob_x =
                    sw_rect.left() + 2.0 + knob_r + anim * (sw_rect.width() - 4.0 - knob_r * 2.0);
                let track_fill = if on {
                    crate::theme::accent_color()
                } else {
                    crate::theme::color32_at("slider_track", 60)
                };
                ui.painter().rect_filled(
                    sw_rect,
                    egui::epaint::CornerRadius::same(sw_rect.height() as u8 / 2),
                    track_fill,
                );
                ui.painter().circle_filled(
                    egui::pos2(knob_x, sw_rect.center().y),
                    knob_r,
                    ink.knob,
                );
                if sw_resp.clicked() {
                    bus.send(FrontendCommand::ToggleAlarm(alarm.id.clone()));
                }
                // 时刻（等宽数字位；禁用行降档）
                let time_font = digits::digit_font(crate::theme::font_size("alarm_time"));
                digits::draw_centered(
                    ui.painter(),
                    egui::pos2(rect.left() + 106.0, rect.center().y),
                    &alarm.time,
                    &time_font,
                    if alarm.enabled { ink.strong } else { ink.dim },
                );
                // 标签 + 重复文案
                ui.painter().text(
                    egui::pos2(rect.left() + 178.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!(
                        "{}  {}",
                        alarm.label,
                        format_repeat_display(&alarm.repeat_days)
                    ),
                    egui::FontId::proportional(crate::theme::font_size("body")),
                    if alarm.enabled { ink.muted } else { ink.dim },
                );
                // 垃圾桶（悬停显形；RemoveAlarm）
                let del_resp = ui.interact(
                    egui::Rect::from_center_size(
                        egui::pos2(rect.right() - 28.0, rect.center().y),
                        egui::vec2(28.0, 28.0),
                    ),
                    egui::Id::unique(("alarm_del", &alarm.id)),
                    egui::Sense::click(),
                );
                if del_resp.hovered() {
                    crate::widgets::icons::draw(
                        ui.painter(),
                        egui::pos2(rect.right() - 28.0, rect.center().y),
                        16.0,
                        crate::widgets::icons::Icon::Trash,
                        ink.danger,
                    );
                }
                if del_resp.clicked() {
                    bus.send(FrontendCommand::RemoveAlarm(alarm.id.clone()));
                }
                ui.add_space(6.0);
            }
        });
    }
}

// ===== src/pages/alarm.rs 说明区 =====
// 输入：UiState.alarms（AlarmsChanged 全量回推）+ 草稿输入态；输出：快捷 chips +
//   自定义添加 + 列表行卡（开关/等宽时刻/标签重复/垃圾桶）
// 逻辑步骤：quick_target(15/30/60/240) → AddAlarm；自定义文本行 → AddAlarm；
//   行内开关 → ToggleAlarm；垃圾桶 → RemoveAlarm（后端 roundtrip 后回推刷新）
// 设计理由（UIR 重塑）：闹钟列表的视觉主角是时刻——等宽数字位 + 大档字号；
//   开关从「开/关」文本改胶囊开关（状态一眼可辨）；删除悬停显形（界面安静）；
//   快捷 chips 金底 hover（与 accent 单源）
// 异常处理：草稿时间非法由 AddAlarm 装载路径拒绝（warn + 忽略）；空列表安静文案
// 关联依赖：core::alarm（quick_target/AlarmDraft）/tools::alarm_text/widgets；
//   消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::quick_label;

    #[test]
    fn quick_label_format() {
        // 快捷 chips 文案锚点
        assert_eq!(quick_label(15), "15 分钟后");
        assert_eq!(quick_label(240), "240 分钟后");
    }
}
