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

        let avail = ui.max_rect();
        let footer_h = footer_height(ui);
        let body_h = (avail.height() - footer_h - crate::theme::layout_f32("footer_gap")).max(0.0);
        let body_rect = egui::Rect::from_min_size(avail.min, egui::vec2(avail.width(), body_h));
        let footer_rect = egui::Rect::from_min_size(
            egui::pos2(
                avail.left(),
                body_rect.bottom() + crate::theme::layout_f32("footer_gap"),
            ),
            egui::vec2(avail.width(), footer_h),
        );
        let quick_h = {
            let meta_font = crate::theme::font_medium(crate::theme::font_size("meta"));
            let label_h = crate::widgets::text_metrics::height(
                ui.ctx(),
                &meta_font,
                crate::widgets::text_ink::InkKind::Cjk,
            );
            let chip_h = quick_chip_height(ui);
            let gap = crate::theme::spacing_f32("xs");
            let pad = crate::theme::spacing_f32("lg");
            if avail.width() > 380.0 {
                label_h + gap + chip_h + pad
            } else {
                label_h + gap + chip_h + gap + chip_h + pad
            }
        };
        let row_h = {
            let time_h = crate::widgets::text_metrics::height(
                ui.ctx(),
                &digits::digit_font(crate::theme::font_size("title")),
                crate::widgets::text_ink::InkKind::Digits,
            );
            time_h.max(switch::height()) + crate::theme::spacing_f32("xl")
        };

        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut stack = crate::widgets::box_stack::BoxStack::new(ui);

                    // 快捷添加盒
                    let quick = stack.next(ui, quick_h);
                    ui.scope_builder(egui::UiBuilder::new().max_rect(quick), |ui| {
                        let label_font = crate::theme::font_medium(crate::theme::font_size("meta"));
                        let label_h = crate::widgets::text_metrics::height(
                            ui.ctx(),
                            &label_font,
                            crate::widgets::text_ink::InkKind::Cjk,
                        );
                        let quick_label_ink = crate::widgets::text_ink::measure(
                            ui,
                            "快捷添加",
                            &label_font,
                            crate::widgets::text_ink::InkKind::Cjk,
                        );
                        ui.painter().text(
                            egui::pos2(
                                quick.left(),
                                quick_label_ink.top_for_bottom(quick.top() + label_h),
                            ),
                            egui::Align2::LEFT_TOP,
                            "快捷添加",
                            label_font,
                            ink.muted,
                        );
                        ui.add_space(label_h + 6.0);
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
                    });

                    // 空态盒
                    if state.alarms.is_empty() {
                        let rect = stack.next(ui, 96.0);
                        crate::widgets::empty::card_at(
                            ui,
                            rect,
                            "还没有闹钟",
                            "用下方表单新建，或点上方快捷添加",
                        );
                    }

                    // 行卡盒
                    let alarms = state.alarms.clone();
                    let pulse = crate::widgets::motion::list_pulse(state, "alarm");
                    let last = alarms.len().saturating_sub(1);
                    for (i, alarm) in alarms.iter().enumerate() {
                        let rect = stack.next(ui, row_h);
                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            if i == last && pulse < 1.0 {
                                ui.set_opacity(0.55 + 0.45 * pulse);
                            }
                            draw_alarm_row(ui, rect, state, bus, alarm, &ink);
                        });
                    }
                });
        });

        ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
            draw_footer(ui, state, bus);
        });
    }
}

/// 快捷 chip 高度：文字盒高 + 上下 padding
fn quick_chip_height(ui: &egui::Ui) -> f32 {
    let font = crate::theme::font_medium(crate::theme::font_size("body"));
    crate::widgets::text_metrics::height(ui.ctx(), &font, crate::widgets::text_ink::InkKind::Cjk)
        + 2.0 * crate::theme::layout_f32("chip_pad_y")
}

/// 快捷 chip：有边界控件（hover 亮边，不填金）；返回点击
fn quick_chip(ui: &mut egui::Ui, text: &str) -> bool {
    let ink = crate::theme::ink();
    let font = crate::theme::font_medium(crate::theme::font_size("body"));
    let w = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), ink.body)
        .rect
        .width()
        + 2.0 * crate::theme::spacing_f32("md");
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(w, quick_chip_height(ui)), egui::Sense::click());
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
        egui::pos2(
            rect.center().x,
            crate::widgets::text_ink::top_for_center(
                ui,
                text,
                &font,
                crate::widgets::text_ink::InkKind::Cjk,
                rect.center().y,
            ),
        ),
        egui::Align2::CENTER_TOP,
        text,
        font.clone(),
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
    rect: egui::Rect,
    state: &mut crate::bus::UiState,
    bus: &crate::bus::Bus,
    alarm: &accel_core::alarm::Alarm,
    ink: &crate::theme::Ink,
) {
    let row = ui.interact(
        rect,
        egui::Id::unique(("alarm_row", &alarm.id)),
        egui::Sense::hover(),
    );
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
        egui::vec2(switch::width(), switch::height()),
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

    let time_font = digits::digit_font(crate::theme::font_size("title"));
    let time_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &time_font,
        crate::widgets::text_ink::InkKind::Digits,
    );
    let time_bottom = rect.top() + 12.0 + time_h;
    digits::draw_bottom(
        ui.painter(),
        time_bottom,
        rect.left() + 82.0,
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
    let label_bottom = rect.bottom() - 12.0;
    let label_ink = crate::widgets::text_ink::measure(
        ui,
        &shown,
        &label_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    ui.painter().text(
        egui::pos2(label_x, label_ink.top_for_bottom(label_bottom)),
        egui::Align2::LEFT_TOP,
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

/// 添加校验（FIX013.17）：格式 → 上限 → 同时间同标签重复；None=可发送
pub fn alarm_add_error(
    label: &str,
    time: &str,
    alarms: &[accel_core::alarm::Alarm],
    max: usize,
) -> Option<String> {
    if !valid_hhmm(time) {
        return Some("时间格式应为 HH:MM（00:00–23:59）".to_string());
    }
    if alarms.len() >= max {
        return Some(format!("最多 {max} 个闹钟，请先删除"));
    }
    if alarms.iter().any(|a| a.time == time && a.label == label) {
        return Some("已存在相同时间和标签的闹钟".to_string());
    }
    None
}

/// 页脚高度：字段固定盒 + footer_gap + 错误行（恒占位）+ footer_gap + 按钮 + footer_bottom
fn footer_height(ui: &egui::Ui) -> f32 {
    let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let meta_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &meta_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let gap = crate::theme::layout_f32("footer_gap");
    crate::widgets::field::fixed_height(ui)
        + gap
        + meta_h
        + gap
        + crate::widgets::primary_button::height(ui)
        + crate::theme::layout_f32("footer_bottom")
}

/// 页脚：标签 + HH:MM + 添加；固定盒布局
fn draw_footer(ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
    let avail = ui.available_width();
    let gap = crate::theme::layout_f32("footer_gap");
    let button_w = crate::theme::layout_f32("button_width");
    let label_w = ((avail - gap) * 0.4).max(80.0);
    let time_w = ((avail - gap) * 0.6).max(80.0);
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut do_add = false;
    let mut edited = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        edited |= field::show_fixed(
            ui,
            "标签",
            "例如 起床",
            &mut state.alarm_draft_label,
            label_w,
        )
        .changed();
        ui.add_space(gap);
        edited |=
            field::show_fixed(ui, "时间", "HH:MM", &mut state.alarm_draft_time, time_w).changed();
    });
    if edited {
        state.alarm_draft_error = None;
    }
    ui.add_space(gap);
    // 错误行恒占位（说明区承诺）：有错误画红字，无错误留空——列表体高度不跳动
    let err_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let err_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &err_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let (err_rect, _) = ui.allocate_exact_size(egui::vec2(avail, err_h), egui::Sense::hover());
    if let Some(err) = &state.alarm_draft_error {
        let ink = crate::widgets::text_ink::measure(
            ui,
            err,
            &err_font,
            crate::widgets::text_ink::InkKind::Cjk,
        );
        ui.painter().text(
            egui::pos2(err_rect.right(), ink.top_for_bottom(err_rect.bottom())),
            egui::Align2::RIGHT_TOP,
            err,
            err_font,
            crate::theme::ink().danger,
        );
    }
    ui.add_space(gap);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space((avail - button_w).max(0.0));
        do_add = crate::widgets::primary_button::show(ui, "添加", button_w).clicked();
    });
    if do_add {
        let max = accel_core::static_config::Static::get()
            .base_i64("max_alarms")
            .max(1) as usize;
        match alarm_add_error(
            &state.alarm_draft_label,
            &state.alarm_draft_time,
            &state.alarms,
            max,
        ) {
            Some(err) => state.alarm_draft_error = Some(err),
            None => {
                state.alarm_draft_error = None;
                bus.send(FrontendCommand::AddAlarm(accel_core::alarm::AlarmDraft {
                    label: state.alarm_draft_label.clone(),
                    time: state.alarm_draft_time.clone(),
                    sound_type: default_sound_type(),
                    sound_value: default_sound_value(),
                    repeat_days: Vec::new(),
                    enabled: true,
                }));
                state.list_pulse = Some(("alarm".to_string(), std::time::Instant::now()));
                state.alarm_draft_label.clear();
                state.alarm_draft_time.clear();
            }
        }
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
    use super::{alarm_add_error, quick_label, truncate_chars, valid_hhmm};

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
    fn alarm_add_error_covers_format_limit_and_duplicate() {
        let alarm = accel_core::alarm::Alarm {
            label: "起床".into(),
            time: "07:30".into(),
            sound_type: "preset".into(),
            sound_value: "classic".into(),
            repeat_days: Vec::new(),
            enabled: true,
            id: "a1".into(),
            created_at: "2026-11-22T07:00:00".into(),
        };
        assert!(alarm_add_error("", "9:5", &[], 10).is_some(), "非法格式");
        assert!(
            alarm_add_error("", "07:30", std::slice::from_ref(&alarm), 1).is_some(),
            "达到上限"
        );
        assert!(
            alarm_add_error("起床", "07:30", &[alarm], 10).is_some(),
            "同时间同标签重复"
        );
        assert!(alarm_add_error("", "07:30", &[], 10).is_none());
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
