//! 闹钟页（PL017 重排）：可滚列表 + 固定 Footer；快捷添加为有边界 chip；
//! 行卡 64px（开关/24px 时刻/标签·重复/Trash），开关走语义绿。纯函数可测。

use crate::tools::alarm_text::format_repeat_display;
use crate::widgets::{digits, field, icons, switch};
use accel_core::alarm::{default_sound_type, default_sound_value, quick_target};
use accel_core::types::FrontendCommand;
use chrono::Local;

/// 快捷 chips 文案（纯函数可测）：分钟数 → "N 分钟后"
pub fn quick_label(minutes: i64) -> String {
    format!("{minutes} 分钟后")
}

/// HH:MM 合法性（纯函数可测，A008 #13）：两位小时(00-23)+冒号+两位分钟(00-59)
pub fn valid_hhmm(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 5
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2] == b':'
        && b[3].is_ascii_digit()
        && b[4].is_ascii_digit()
        && text[0..2].parse::<u8>().is_ok_and(|h| h <= 23)
        && text[3..5].parse::<u8>().is_ok_and(|m| m <= 59)
}

/// 按 char 数截断加省略号（纯函数可测）：按字符切保证中文边界安全
pub fn truncate_chars(text: &str, keep: usize) -> String {
    let total = text.chars().count();
    if total <= keep {
        return text.to_string();
    }
    if keep <= 1 {
        return "…".to_string();
    }
    let cut: String = text.chars().take(keep - 1).collect();
    format!("{cut}…")
}

impl crate::pages::Page for crate::pages::AlarmPage {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        state.armed_delete_tick();

        let footer_h = footer_height(ui.available_width());
        let body_h = (ui.available_height() - footer_h - 8.0).max(0.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("快捷添加")
                            .font(crate::theme::font_medium(crate::theme::font_size("meta")))
                            .color(ink.muted),
                    );
                    ui.add_space(6.0);
                    if ui.available_width() > 380.0 {
                        ui.horizontal_wrapped(|ui| {
                            for &minutes in crate::pages::quick_alarm_minutes() {
                                if quick_chip(ui, &quick_label(minutes)) {
                                    let draft = quick_target(minutes, Local::now());
                                    bus.send(FrontendCommand::AddAlarm(draft));
                                }
                                ui.add_space(6.0);
                            }
                        });
                    } else {
                        egui::Grid::new("quick_alarm_grid")
                            .num_columns(2)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                for (i, &minutes) in
                                    crate::pages::quick_alarm_minutes().iter().enumerate()
                                {
                                    if quick_chip(ui, &quick_label(minutes)) {
                                        let draft = quick_target(minutes, Local::now());
                                        bus.send(FrontendCommand::AddAlarm(draft));
                                    }
                                    if i % 2 == 1 {
                                        ui.end_row();
                                    }
                                }
                            });
                    }
                    ui.add_space(crate::theme::spacing("lg"));

                    if state.alarms.is_empty() {
                        crate::widgets::empty::show(
                            ui,
                            "还没有闹钟",
                            "用下方表单新建，或点上方快捷添加",
                        );
                    }
                    let alarms = state.alarms.clone();
                    for alarm in &alarms {
                        draw_alarm_row(ui, state, bus, alarm, &ink);
                        ui.add_space(6.0);
                    }
                });
        });

        ui.add_space(8.0);
        draw_footer(ui, state, bus);
    }
}

/// 快捷 chip：30px 有边界控件（hover 亮边，不填金）；返回点击
fn quick_chip(ui: &mut egui::Ui, text: &str) -> bool {
    let ink = crate::theme::ink();
    let font = crate::theme::font_medium(crate::theme::font_size("body"));
    let w = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), ink.body)
        .rect
        .width()
        + 24.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 30.0), egui::Sense::click());
    let hover = resp.hovered() || resp.has_focus();
    ui.painter().rect_filled(
        rect,
        egui::epaint::CornerRadius::same(8),
        if hover {
            crate::theme::hover_bg()
        } else {
            crate::theme::card_bg()
        },
    );
    ui.painter().rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(8),
        egui::Stroke::new(
            1.0,
            if hover {
                crate::theme::border_strong_color()
            } else {
                crate::theme::border_color()
            },
        ),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font,
        if hover { ink.strong } else { ink.body },
    );
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect,
            egui::epaint::CornerRadius::same(8),
            egui::Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    let keyboard = resp.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let clicked = resp.clicked() || keyboard;
    let _ = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
}

/// 闹钟行：开关 + 时刻 + 标签/重复 + Trash
fn draw_alarm_row(
    ui: &mut egui::Ui,
    state: &mut crate::bus::UiState,
    bus: &crate::bus::Bus,
    alarm: &accel_core::alarm::Alarm,
    ink: &crate::theme::Ink,
) {
    let (rect, row) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 64.0), egui::Sense::hover());
    ui.painter().rect_filled(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        crate::theme::card_bg(),
    );
    ui.painter().rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        egui::Stroke::new(1.0, crate::theme::border_color()),
        egui::StrokeKind::Inside,
    );

    let sw_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 32.0, rect.center().y),
        egui::vec2(switch::SWITCH_W, switch::SWITCH_H),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sw_rect));
    let sw = switch::show(
        &mut child,
        egui::Id::unique(("alarm_sw", &alarm.id)),
        alarm.enabled,
    );
    if sw {
        bus.send(FrontendCommand::ToggleAlarm(alarm.id.clone()));
    }

    let time_font = digits::digit_font(24.0);
    digits::draw_centered(
        ui.painter(),
        egui::pos2(rect.left() + 82.0, rect.center().y),
        &alarm.time,
        &time_font,
        if alarm.enabled { ink.strong } else { ink.meta },
    );

    let repeat = format_repeat_display(&alarm.repeat_days);
    let full_label = if repeat.is_empty() {
        alarm.label.clone()
    } else {
        format!("{} · {}", alarm.label, repeat)
    };
    let label_font = crate::theme::font_regular(crate::theme::font_size("body"));
    let label_x = rect.left() + 140.0;
    let avail = (rect.right() - 44.0 - label_x).max(0.0);
    let shown = truncate_to_width(ui, &full_label, &label_font, avail);
    ui.painter().text(
        egui::pos2(label_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &shown,
        label_font,
        if alarm.enabled { ink.body } else { ink.meta },
    );
    if shown != full_label {
        row.on_hover_text(&full_label);
    }

    // 删除（Trash + 两步确认）
    let del_key = format!("alarm:{}", alarm.id);
    let armed = matches!(&state.armed_delete, Some((k, _)) if *k == del_key);
    let del_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 20.0, rect.center().y),
        egui::vec2(28.0, 28.0),
    );
    let del = ui.interact(
        del_rect,
        egui::Id::unique(("alarm_del", &alarm.id)),
        egui::Sense::click(),
    );
    icons::draw(
        ui.painter(),
        del_rect.center(),
        16.0,
        if armed {
            icons::Icon::Check
        } else {
            icons::Icon::Trash
        },
        if armed || del.hovered() {
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
            "再次点击确认删除"
        } else {
            "删除闹钟"
        })
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if del.clicked() {
        if armed {
            bus.send(FrontendCommand::RemoveAlarm(alarm.id.clone()));
            state.armed_delete = None;
        } else {
            state.armed_delete = Some((del_key, std::time::Instant::now()));
        }
    }
}

/// 按像素宽度截断（复用 truncate_chars 的逐字回退；中文安全）
fn truncate_to_width(ui: &egui::Ui, text: &str, font: &egui::FontId, width: f32) -> String {
    if ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), crate::theme::ink().body)
        .rect
        .width()
        <= width
    {
        return text.to_string();
    }
    let mut chars = text.chars().count();
    while chars > 1 {
        let candidate = truncate_chars(text, chars);
        if ui
            .painter()
            .layout_no_wrap(candidate.clone(), font.clone(), crate::theme::ink().body)
            .rect
            .width()
            <= width
        {
            return candidate;
        }
        chars -= 1;
    }
    "…".to_string()
}

/// 页脚高度：窄抽屉（内容 <340）用两行表单，否则单行
fn footer_height(avail: f32) -> f32 {
    if avail < 340.0 {
        118.0
    } else {
        crate::theme::page_footer_height()
    }
}

/// 页脚：标签 + HH:MM + 添加；窄宽两行；错误恒占位 16px
fn draw_footer(ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
    let ink = crate::theme::ink();
    let avail = ui.available_width();
    let button_w = 88.0;
    let narrow = avail < 340.0;
    let (label_w, time_w) = if narrow {
        let w = ((avail - 8.0) / 2.0).max(80.0);
        (w, w)
    } else {
        let time = (avail * 0.26).clamp(80.0, 120.0);
        ((avail - time - button_w - 16.0).clamp(100.0, 200.0), time)
    };
    let mut do_add = false;
    if narrow {
        ui.horizontal(|ui| {
            field::show(
                ui,
                "标签",
                "例如 起床",
                &mut state.alarm_draft_label,
                label_w,
            );
            ui.add_space(8.0);
            field::show(ui, "时间", "HH:MM", &mut state.alarm_draft_time, time_w);
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space((avail - button_w).max(0.0));
            do_add = crate::widgets::primary_button::show(ui, "添加", button_w).clicked();
        });
    } else {
        ui.horizontal(|ui| {
            field::show(
                ui,
                "标签",
                "例如 起床",
                &mut state.alarm_draft_label,
                label_w,
            );
            ui.add_space(8.0);
            field::show(ui, "时间", "HH:MM", &mut state.alarm_draft_time, time_w);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.add_space(field::field_height() - 32.0);
                do_add = crate::widgets::primary_button::show(ui, "添加", button_w).clicked();
            });
        });
    }
    if do_add && valid_hhmm(&state.alarm_draft_time) {
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
    let (hint_rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::hover());
    if !state.alarm_draft_time.is_empty() && !valid_hhmm(&state.alarm_draft_time) {
        ui.painter().text(
            egui::pos2(hint_rect.left(), hint_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "时间格式应为 HH:MM（如 07:30）",
            crate::theme::font_regular(crate::theme::font_size("meta")),
            ink.danger,
        );
    }
}

// ===== src/pages/alarm.rs 说明区 =====
// 输入：UiState.alarms + 草稿；输出：快捷添加/列表行（开关/时刻/标签/Trash）/Footer
// 逻辑步骤：ScrollArea（快捷 chip + 空态/行卡）→ 固定 Footer（标签/时间/添加，
//   错误恒占位）→ quick_target/AddAlarm、ToggleAlarm、RemoveAlarm
// 设计理由（PL017）：原快捷 chips 是裸文本 + 金底 hover、开关 on=品牌金、添加行
//   只有 placeholder 且随列表滚走；本版 chip 有边界、开关语义绿、Footer 常驻、
//   字段 label 上置；行 64px 在 280 抽屉仍可读
// 异常处理：HH:MM 非法红字但不发送；空列表给虚线空态；标签超宽省略 + tooltip
// 关联依赖：core::alarm/tools::alarm_text/widgets(field/switch/digits/empty)；
//   消费方：pages::Pages 抽屉
#[cfg(test)]
mod tests {
    use super::{quick_label, truncate_chars, valid_hhmm};

    #[test]
    fn quick_label_format() {
        assert_eq!(quick_label(15), "15 分钟后");
        assert_eq!(quick_label(240), "240 分钟后");
    }

    #[test]
    fn truncate_chars_keeps_cjk_boundary() {
        assert_eq!(truncate_chars("早起", 5), "早起");
        assert_eq!(truncate_chars("早起上班", 3), "早起…");
        assert_eq!(truncate_chars("早起上班", 1), "…");
    }

    #[test]
    fn valid_hhmm_matrix() {
        assert!(valid_hhmm("07:30"));
        assert!(valid_hhmm("23:59"));
        assert!(valid_hhmm("00:00"));
        assert!(!valid_hhmm("9:30"));
        assert!(!valid_hhmm("24:00"));
        assert!(!valid_hhmm("07:60"));
        assert!(!valid_hhmm("0730"));
        assert!(!valid_hhmm("07:3"));
        assert!(!valid_hhmm(""));
    }
}
