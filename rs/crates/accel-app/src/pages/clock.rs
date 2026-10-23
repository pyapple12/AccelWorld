//! 时钟页（ui/panels/clock_panel.py + date_panel 的 Rust 对等物，PL014.05）
//! 标准时计卡 + 加速时间卡（hero + 进度环）+ 倍率卡（滑杆）+ 农历 chips 行

use crate::glass::surface::GlassScene;
use crate::tools::clock::split_lunar_info;
use crate::widgets::{chip, ring, slider};
use accel_core::types::FrontendCommand;

impl crate::pages::Page for crate::pages::ClockPage {
    /// 三卡一 chips 行装配（PL014.05）：全部数据取 UiState Tick 快照，零后端直查
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut crate::bus::UiState,
        bus: &crate::bus::Bus,
        scene: &mut GlassScene,
    ) {
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
                                    .size(crate::theme::font_size("heading"))
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
                            // FIX006.27：day_hours 理论可为 0（rate 派生）——分母钳制防除零
                            let passed = 1.0
                                - (info.remaining_hours / info.day_hours.max(1) as f64)
                                    .clamp(0.0, 1.0);
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
                                .size(crate::theme::font_size("body"))
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
                        // FIX006.2：回调只登记意图，落回本帧统一经 bus.send 出口
                        // （走 CmdQueue 待发队列——通道满不丢命令）
                        let mut set_rate: Option<f64> = None;
                        let mut apply_rate: Option<f64> = None;
                        slider::show(
                            ui,
                            crate::theme::layout_i64("slider_track") as f32,
                            min,
                            max,
                            &mut rate,
                            |r| set_rate = Some(r),
                            |r| apply_rate = Some(r),
                        );
                        if let Some(r) = set_rate {
                            bus.send(FrontendCommand::SetRate(r));
                        }
                        if let Some(r) = apply_rate {
                            bus.send(FrontendCommand::ApplyRate(r));
                        }
                        // 滑杆本地值立即回写（拖动期间 Tick 快照不回刷旧值）
                        state.rate = rate;
                        ui.add_space(16.0);
                        // 农历 chips 行：split 拆 lunar_info 三段
                        //（FIX007.3：id 带 clock_ 页前缀——切页按前缀统一隐藏）
                        if let Some(info) = &state.time {
                            let (lunar, phase, god) = split_lunar_info(&info.lunar_info);
                            ui.horizontal(|ui| {
                                chip::show(ui, scene, "clock_lunar_main", "农历", &lunar, false);
                                chip::show(ui, scene, "clock_lunar_phase", "月相", &phase, false);
                                chip::show(ui, scene, "clock_lunar_god", "财神", &god, false);
                            });
                        }
                    });
                });
            });
    }
}

// ===== src/pages/clock.rs 说明区 =====
// 输入：UiState（Tick 快照：时间/倍率域）；输出：三卡一 chips 装配 + SetRate/ApplyRate
// 逻辑步骤：页卡面注册（容器统一，页内不再重复）→ 标准时计（日期+走秒）→ 加速 hero +
//   进度环（已过占比，分母 day_hours.max(1) 防除零）→ 倍率滑杆（拖动 SetRate 实时/
//   松手 ApplyRate 落盘，回调登记后统一经 bus.send 出口）→ 农历三 chips（split 三段）
// 设计理由：Python clock_panel+date_panel 合并（数据同源 TimeInfo）；滑杆值本地
//   立即回写（拖动期间 Tick 快照回刷旧值问题——本地态优先，RateChanged 事件后由
//   后端权威值校准）；进度环显示已过占比（对齐 hero 语义）
// FIX006.2：命令一律经 bus.send（CmdQueue 待发队列保底，满载不丢）；FIX006.27：
//   day_hours 理论可为 0（rate 派生），分母钳制
// 异常处理：time 为 None（首 Tick 前）时间两卡跳过不崩
// 关联依赖：widgets（ring/slider/chip）/tools::clock；消费方：pages::Pages
