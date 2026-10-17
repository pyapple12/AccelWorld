//! 时钟页（ui/panels/clock_panel.py + date_panel 的 Rust 对等物，PL014.05）
//! 标准时计卡 + 加速时间卡（hero + 进度环）+ 倍率卡（滑杆）+ 农历 chips 行

use crate::bus::PageId;
use crate::glass::surface::GlassScene;
use crate::tools::clock::split_lunar_info;
use crate::widgets::{chip, ring, slider};
use accel_core::types::FrontendCommand;
use tokio::sync::mpsc::Sender;

impl crate::pages::Page for crate::pages::ClockPage {
    /// 三卡一 chips 行装配（PL014.05）：全部数据取 UiState Tick 快照，零后端直查
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut crate::bus::UiState,
        cmd: &Sender<FrontendCommand>,
        scene: &mut GlassScene,
    ) {
        let page_rect = crate::window::content_page_rect(ui.ctx());
        crate::window::register_page_surface(scene, PageId::Clock, page_rect);
        // A005 #9：语义色 token（不再写字面色值）
        let ink = crate::theme::ink();

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(crate::window::page_inner_margin()))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(crate::window::content_left_inset());
                    ui.vertical(|ui| {
                        // 标准时计卡：日期星期 + 标准时间大字
                        if let Some(info) = &state.time {
                            ui.heading(
                                egui::RichText::new(&info.chinese_date)
                                    .size(18.0)
                                    .color(ink.muted),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(info.standard_time())
                                    .size(crate::theme::font_size("std_time"))
                                    .color(ink.strong),
                            );
                            ui.add_space(16.0);
                            // 加速时间卡：hero 大字 + 进度环（已过占比）
                            let passed = 1.0
                                - (info.remaining_hours / info.day_hours as f64).clamp(0.0, 1.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&info.custom_time)
                                        .size(crate::theme::font_size("hero_time"))
                                        .color(crate::theme::accent_color()),
                                );
                                ring::show(
                                    ui,
                                    crate::theme::layout_i64("progress_ring_size") as f32,
                                    passed,
                                    &format!("{:.0}%", passed * 100.0),
                                );
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "一天 {} 小时 · 剩余 {:.2} 小时",
                                    info.day_hours, info.remaining_hours
                                ))
                                .size(14.0)
                                .color(ink.soft),
                            );
                        }
                        ui.add_space(16.0);
                        // 倍率卡：滑杆 + 当前值 + 百分比
                        ui.label(
                            egui::RichText::new(format!(
                                "倍率 {:.1} ×（{}）",
                                state.rate,
                                crate::tools::clock::fmt_percent(state.rate)
                            ))
                            .size(crate::theme::font_size("slider_value"))
                            .color(ink.strong),
                        );
                        let (min, max) = state.rate_bounds;
                        let mut rate = state.rate;
                        let tx_change = cmd.clone();
                        let tx_commit = cmd.clone();
                        slider::show(
                            ui,
                            crate::theme::layout_i64("slider_track") as f32,
                            min,
                            max,
                            &mut rate,
                            move |r| {
                                let _ = tx_change.try_send(FrontendCommand::SetRate(r));
                            },
                            move |r| {
                                let _ = tx_commit.try_send(FrontendCommand::ApplyRate(r));
                            },
                        );
                        // 滑杆本地值立即回写（拖动期间 Tick 快照不回刷旧值）
                        state.rate = rate;
                        ui.add_space(16.0);
                        // 农历 chips 行：split 拆 lunar_info 三段
                        if let Some(info) = &state.time {
                            let (lunar, phase, god) = split_lunar_info(&info.lunar_info);
                            ui.horizontal(|ui| {
                                chip::show(ui, scene, "lunar_main", "农历", &lunar, false);
                                chip::show(ui, scene, "lunar_phase", "月相", &phase, false);
                                chip::show(ui, scene, "lunar_god", "财神", &god, false);
                            });
                        }
                    });
                });
            });
    }
}

// ===== src/pages/clock.rs 说明区 =====
// 输入：UiState（Tick 快照：时间/倍率域）；输出：三卡一 chips 装配 + SetRate/ApplyRate
// 逻辑步骤：页卡面注册 → 标准时计（日期+走秒）→ 加速 hero + 进度环（已过占比）→
//   倍率滑杆（拖动 SetRate 实时/松手 ApplyRate 落盘）→ 农历三 chips（split 三段）
// 设计理由：Python clock_panel+date_panel 合并（数据同源 TimeInfo）；滑杆值本地
//   立即回写（拖动期间 Tick 快照回刷旧值问题——本地态优先，RateChanged 事件后由
//   后端权威值校准）；进度环显示已过占比（对齐 hero 语义）
// 异常处理：time 为 None（首 Tick 前）时间两卡跳过不崩
// 关联依赖：widgets（ring/slider/chip）/tools::clock；消费方：pages::Pages
