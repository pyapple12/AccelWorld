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

/// 最近未到目标（hero 用）：取未到项中 at 最小者（A005 #23：借用切片，无每帧克隆；
/// 排序改为单次线性取最小，避免 to_vec + sort）
pub fn next_target<'a>(
    targets: &[(&'a str, NaiveDateTime)],
    now: NaiveDateTime,
) -> Option<(&'a str, NaiveDateTime)> {
    targets
        .iter()
        .filter(|(_, at)| *at > now)
        .min_by_key(|(_, at)| *at)
        .copied()
}

impl crate::pages::Page for crate::pages::CountdownPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut crate::bus::UiState,
        cmd: &Sender<FrontendCommand>,
        scene: &mut GlassScene,
    ) {
        let page_rect = crate::window::content_page_rect(ui.ctx());
        crate::window::register_page_surface(scene, PageId::Countdown, page_rect);
        // A005 #9：语义色 token（不再写字面色值）
        let ink = crate::theme::ink();

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(crate::window::page_inner_margin()))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(crate::window::content_left_inset());
                    ui.vertical(|ui| {
                        let now = chrono::Local::now().naive_local();
                        // 目标快照（state.countdown_targets：启动快照 + ConfigSaved 重拉）
                        let mut targets: Vec<(&str, NaiveDateTime)> = Vec::new();
                        for t in &state.countdown_targets {
                            if let Some(at) = t.parse_at() {
                                // A005 #23：借用名称不 clone（原实现每帧克隆全部目标名）
                                targets.push((t.name.as_str(), at));
                            }
                        }
                        // hero：最近目标大字
                        if let Some((name, at)) = next_target(&targets, now) {
                            let (text, ended) = format_remaining(at, now);
                            ui.label(egui::RichText::new(name).size(18.0).color(ink.muted));
                            ui.label(
                                egui::RichText::new(&text)
                                    .size(crate::theme::font_size("hero_time"))
                                    .color(if ended {
                                        ink.danger
                                    } else {
                                        crate::theme::accent_color()
                                    }),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("暂无目标 · 点右上 ＋ 添加")
                                    .size(16.0)
                                    .color(ink.soft),
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
                                    .color(ink.card),
                                );
                                ui.label(egui::RichText::new(&text).size(15.0).color(if ended {
                                    ink.danger
                                } else {
                                    ink.muted
                                }));
                                if ui.small_button("－").clicked() {
                                    let _ = cmd.try_send(
                                        accel_core::types::FrontendCommand::RemoveCountdownTarget(
                                            (*name).to_string(),
                                        ),
                                    );
                                }
                            });
                        }
                        ui.add_space(12.0);
                        // 加号行：名称输入 + 日期/时间文本（解析单一来源：tools::countdown::parse_target_text）
                        ui.horizontal(|ui| {
                            ui.label("＋ 新目标：");
                            ui.text_edit_singleline(&mut state.countdown_draft_name);
                            ui.text_edit_singleline(&mut state.countdown_draft_at);
                            // A005 #11：统一走 tools::countdown::parse_target_text（19/16/10 位），
                            // 不再页内自解析 NaiveDate（原实现只收 YYYY-MM-DD 且强填 23:59:59）
                            let (err_msg, parsed_at) = {
                                let name = state.countdown_draft_name.clone();
                                match crate::tools::countdown::parse_target_text(
                                    &state.countdown_draft_at,
                                ) {
                                    Some(at) => {
                                        let names: Vec<String> =
                                            targets.iter().map(|(n, _)| (*n).to_string()).collect();
                                        match validate_add(&name, at, now, &names) {
                                            Ok(()) => (String::new(), Some(at)),
                                            Err(e) => (e.to_string(), None),
                                        }
                                    }
                                    None => ("日期格式 YYYY-MM-DD[ HH:MM[:SS]]".into(), None),
                                }
                            };
                            let valid = parsed_at.is_some();
                            if ui.add_enabled(valid, egui::Button::new("确认")).clicked()
                                && let Some(at) = parsed_at
                            {
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
                                ui.label(egui::RichText::new(&err_msg).color(ink.danger));
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
//   草稿日期输入用单行文本（日期件控件未引入——A005 #25 已移除未用的 egui_extras）
// 关联依赖：tools::countdown/settings::CountdownTarget；消费方：pages::Pages
