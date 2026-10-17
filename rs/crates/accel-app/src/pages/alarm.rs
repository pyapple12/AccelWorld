//! 闹钟页（PL014.14/.15）：四快捷按钮 + 自定义添加 + 列表行卡（开关/删除）

use crate::glass::surface::GlassScene;
use crate::tools::alarm_text::format_repeat_display;
use accel_core::alarm::{default_sound_type, default_sound_value, quick_target};
use accel_core::types::FrontendCommand;
use chrono::Local;

impl crate::pages::Page for crate::pages::AlarmPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut crate::bus::UiState,
        bus: &crate::bus::Bus,
        _scene: &mut GlassScene,
    ) {
        // A005 #9：语义色 token（不再写字面色值）
        let ink = crate::theme::ink();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(crate::window::content_left_inset());
                ui.vertical(|ui| {
                    // 四快捷按钮（分钟数读 base.json quick_alarm_minutes token）
                    ui.label(
                        egui::RichText::new("快捷添加")
                            .size(crate::theme::font_size("label"))
                            .color(ink.muted),
                    );
                    ui.horizontal(|ui| {
                        for &minutes in crate::pages::quick_alarm_minutes() {
                            if ui.button(format!("＋{minutes} 分钟")).clicked() {
                                let draft = quick_target(minutes, Local::now());
                                bus.send(FrontendCommand::AddAlarm(draft));
                            }
                        }
                    });
                    ui.add_space(8.0);
                    // 自定义添加行：标签 + 时间（HH:MM 文本）
                    ui.horizontal(|ui| {
                        ui.label("自定义：");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.alarm_draft_label)
                                .hint_text("标签")
                                .desired_width(120.0),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut state.alarm_draft_time)
                                .hint_text("HH:MM")
                                .desired_width(80.0),
                        );
                        if ui.button("＋ 添加").clicked() {
                            // FIX006.5：默认铃声取 base.json token（不再代码侧硬编码）
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
                    ui.add_space(12.0);
                    // 列表行卡（AlarmsChanged 全量回推重建——UI 无状态）
                    if state.alarms.is_empty() {
                        ui.label(egui::RichText::new("暂无闹钟").color(ink.soft));
                    }
                    for alarm in &state.alarms {
                        ui.horizontal(|ui| {
                            // 时间大字
                            ui.label(
                                egui::RichText::new(&alarm.time)
                                    .size(crate::theme::font_size("alarm_time"))
                                    .color(if alarm.enabled { ink.strong } else { ink.dim }),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}  {}",
                                    alarm.label,
                                    format_repeat_display(&alarm.repeat_days)
                                ))
                                .size(crate::theme::font_size("body"))
                                .color(ink.muted),
                            );
                            // 开关
                            if ui
                                .selectable_label(
                                    alarm.enabled,
                                    if alarm.enabled { "开" } else { "关" },
                                )
                                .clicked()
                            {
                                // FIX006.2：命令统一经 bus.send（CmdQueue 待发队列保底）
                                bus.send(FrontendCommand::ToggleAlarm(alarm.id.clone()));
                            }
                            // 删除
                            if ui.small_button("删除").clicked() {
                                bus.send(FrontendCommand::RemoveAlarm(alarm.id.clone()));
                            }
                        });
                    }
                });
            });
        });
    }
}

// ===== src/pages/alarm.rs 说明区 =====
// 输入：UiState.alarms（AlarmsChanged 全量回推）+ 草稿输入态；输出：快捷按钮 +
//   自定义添加 + 列表行卡（时间大字/标签/重复文案/开关/删除）
// 逻辑步骤：quick_target(15/30/60/240) → AddAlarm；自定义文本行 → AddAlarm；
//   列表 ToggleAlarm/RemoveAlarm（后端 roundtrip 后 AlarmsChanged 回推刷新）
// 设计理由：快捷换算在 core（跨午夜顺延语义由 Alarm.should_trigger_on 覆盖）；
//   UI 无状态——列表只消费 AlarmsChanged 全量回推（后端权威）
// 异常处理：草稿时间非法由 AddAlarm 装载路径拒绝（warn + 忽略，页面无红字——
//   PL015 观感轮补页面级校验提示）
// 关联依赖：core::alarm（quick_target/AlarmDraft）/tools::alarm_text；消费方：pages::Pages
