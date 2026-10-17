//! 倒计时页（PL014.08）：hero 最近目标 + 目标卡网格 + 加号校验 + 减号
//! 纯逻辑（校验/最近目标选取）抽离为可测函数，UI 层仅装配

use crate::bus::PageId;
use crate::glass::surface::GlassScene;
use crate::tools::countdown::format_remaining;
use accel_core::types::FrontendCommand;
use chrono::{Duration, NaiveDateTime, TimeZone};
use tokio::sync::mpsc::Sender;

/// 加号校验（纯函数）：距 now < 24h 拒（NotYet）、同名拒（Dup）、非法输入拒（Empty）
pub fn validate_add(
    name: &str,
    at: NaiveDateTime,
    now: NaiveDateTime,
    existing: &[String],
) -> Result<(), AddErr> {
    if name.is_empty() {
        return Err(AddErr::Empty);
    }
    if existing.iter().any(|e| e == name) {
        return Err(AddErr::Dup);
    }
    if at - now < Duration::hours(24) {
        return Err(AddErr::NotYet);
    }
    Ok(())
}

/// 加号校验错误
#[derive(Debug, PartialEq)]
pub enum AddErr {
    Empty,
    Dup,
    NotYet,
}

impl std::fmt::Display for AddErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddErr::Empty => write!(f, "名称不能为空"),
            AddErr::Dup => write!(f, "同名目标已存在"),
            AddErr::NotYet => write!(f, "目标须在 24 小时之后"),
        }
    }
}

/// 最近未到目标（hero 用）：targets 已按 at 升序则首个未到者
pub fn next_target(
    targets: &[(String, NaiveDateTime)],
    now: NaiveDateTime,
) -> Option<(String, NaiveDateTime)> {
    let mut sorted = targets.to_vec();
    sorted.sort_by_key(|(_, at)| *at);
    sorted.into_iter().find(|(_, at)| *at > now)
}

impl crate::pages::Page for crate::pages::CountdownPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut crate::bus::UiState,
        cmd: &Sender<FrontendCommand>,
        scene: &mut GlassScene,
    ) {
        let content = ui.ctx().input(|i| i.content_rect());
        let page_rect = egui::Rect::from_min_size(
            content.min + egui::vec2(180.0, 20.0),
            content.size() - egui::vec2(204.0, 40.0),
        );
        crate::window::register_page_surface(scene, PageId::Countdown, page_rect);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::same(24)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(156.0);
                    ui.vertical(|ui| {
                        let now = chrono::Local::now().naive_local();
                        // 目标快照（同步查询——拉取式，同步语义见 interface）
                        let mut targets: Vec<(String, NaiveDateTime)> = Vec::new();
                        let tx = cmd.clone();
                        let _ = tx;
                        // 数据面：Bus iface 快照经 state 不可达——页面约定：倒计时列表
                        // 由 Tick 外的 ConfigSaved 驱动重拉；此处经 cmd 发查询是重设计，
                        // PL014.08 简化：Bus::iface 同步查询在 window 侧装配后存 state
                        for t in &state.countdown_targets {
                            if let Some(at) = t.parse_at() {
                                targets.push((t.name.clone(), at));
                            }
                        }
                        // hero：最近目标大字
                        if let Some((name, at)) = next_target(&targets, now) {
                            let (text, ended) = format_remaining(at, now);
                            ui.label(
                                egui::RichText::new(&name)
                                    .size(18.0)
                                    .color(egui::Color32::from_rgb(200, 206, 220)),
                            );
                            ui.label(
                                egui::RichText::new(&text)
                                    .size(crate::theme::font_size("hero_time"))
                                    .color(if ended {
                                        egui::Color32::from_rgb(228, 117, 106)
                                    } else {
                                        crate::theme::accent_color()
                                    }),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("暂无目标 · 点右上 ＋ 添加")
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(170, 176, 190)),
                            );
                        }
                        ui.add_space(16.0);
                        // 目标卡网格：名称/日期/剩余 + 减号（默认派生目标无减号——
                        // 派生目标不持久化，自然不在 targets 列表）
                        for (name, at) in &targets {
                            let (text, ended) = format_remaining(*at, now);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{name} · {}",
                                        at.format("%Y-%m-%d %H:%M")
                                    ))
                                    .size(15.0)
                                    .color(egui::Color32::from_rgb(225, 230, 240)),
                                );
                                ui.label(egui::RichText::new(&text).size(15.0).color(if ended {
                                    egui::Color32::from_rgb(228, 117, 106)
                                } else {
                                    egui::Color32::from_rgb(200, 206, 220)
                                }));
                                if ui.small_button("－").clicked() {
                                    let _ = cmd.try_send(
                                        accel_core::types::FrontendCommand::RemoveCountdownTarget(
                                            name.clone(),
                                        ),
                                    );
                                }
                            });
                        }
                        ui.add_space(12.0);
                        // 加号行：名称输入 + 日期/时间文本（自绘简化：单行文本框）
                        ui.horizontal(|ui| {
                            ui.label("＋ 新目标：");
                            ui.text_edit_singleline(&mut state.countdown_draft_name);
                            ui.text_edit_singleline(&mut state.countdown_draft_at);
                            let (err_msg, valid) = {
                                let name = state.countdown_draft_name.clone();
                                let parsed = state
                                    .countdown_draft_at
                                    .parse::<chrono::NaiveDate>()
                                    .ok()
                                    .and_then(|d| d.and_hms_opt(23, 59, 59));
                                match parsed {
                                    Some(at) => {
                                        let names: Vec<String> =
                                            targets.iter().map(|(n, _)| n.clone()).collect();
                                        match validate_add(&name, at, now, &names) {
                                            Ok(()) => (String::new(), true),
                                            Err(e) => (e.to_string(), false),
                                        }
                                    }
                                    None => ("日期格式 YYYY-MM-DD".into(), false),
                                }
                            };
                            if ui.add_enabled(valid, egui::Button::new("确认")).clicked() {
                                let at = state
                                    .countdown_draft_at
                                    .parse::<chrono::NaiveDate>()
                                    .ok()
                                    .and_then(|d| d.and_hms_opt(23, 59, 59))
                                    .expect("已校验");
                                let local = chrono::Local
                                    .from_local_datetime(&at)
                                    .single()
                                    .unwrap_or_else(chrono::Local::now);
                                let _ = cmd.try_send(
                                    accel_core::types::FrontendCommand::AddCountdownTarget {
                                        name: state.countdown_draft_name.clone(),
                                        at: local,
                                    },
                                );
                                state.countdown_draft_name.clear();
                                state.countdown_draft_at.clear();
                            }
                            if !err_msg.is_empty() && !valid {
                                ui.label(
                                    egui::RichText::new(&err_msg)
                                        .color(egui::Color32::from_rgb(228, 117, 106)),
                                );
                            }
                        });
                    });
                });
            });
    }
}

// ===== src/pages/countdown.rs 说明区 =====
// 输入：UiState.countdown_targets（后端权威列表）+ 草稿输入态；输出：hero/网格/加号
// 逻辑步骤：parse_at 装载 → hero 最近目标（next_target）→ 网格行卡 + 减号 →
//   加号行（validate_add 校验：空名/同名/<24h 拒）→ Add/RemoveCountdownTarget 命令
// 设计理由：校验与最近目标选取抽纯函数（可测）；默认派生目标（春节）不落盘，
//   天然不在 targets 列表 → 无减号（任务定案平移）
// 已知偏移：WorldPageAlisa 占位桥待删（世界页 impl 在 world.rs 时清理）；
//   草稿日期输入用单行文本（egui_extras 日期件留 PL015 观感轮）
// 关联依赖：tools::countdown/settings::CountdownTarget；消费方：pages::Pages
