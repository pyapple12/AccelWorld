//! 倒计时页（PL014.08）：hero 最近目标 + 目标卡网格 + 加号校验 + 减号
//! 纯逻辑（校验/最近目标选取）抽离为可测函数，UI 层仅装配

use crate::glass::surface::GlassScene;
use crate::tools::countdown::format_remaining;
use crate::widgets::chip;
use accel_core::types::FrontendCommand;
use chrono::{Duration, NaiveDate, NaiveDateTime, TimeZone};

/// hero 目标选取（纯函数，FIX006.3）：持久目标最近未到者 与 派生春节目标 取更近者
pub fn hero_target<'a>(
    targets: &[(&'a str, NaiveDateTime)],
    derived: Option<(&'a str, NaiveDateTime)>,
    now: NaiveDateTime,
) -> Option<(&'a str, NaiveDateTime)> {
    let best = next_target(targets, now);
    if let Some((dn, dat)) = derived
        && dat > now
        && best.is_none_or(|(_, at)| dat < at)
    {
        return Some((dn, dat));
    }
    best
}

/// 派生兜底目标（PL014.07 + FIX006.3）：次年春节 00:00（UI 层派生，不落盘）
pub fn derived_spring_festival(today: NaiveDate) -> Option<(&'static str, NaiveDateTime)> {
    accel_core::chinese_calendar::next_spring_festival(today)
        .and_hms_opt(0, 0, 0)
        .map(|at| ("春节", at))
}

/// 日粒度缓存槽类型（FIX007.7；模块级别名消解 clippy type_complexity）
type DateCache<T> = std::sync::OnceLock<std::sync::Mutex<Option<(NaiveDate, T)>>>;

/// 春节派生日缓存槽（hero 兜底目标）
static DERIVED_CACHE: DateCache<Option<(&'static str, NaiveDateTime)>> = DateCache::new();

/// 近期节日 chips 日缓存槽
static CHIPS_CACHE: DateCache<Vec<(NaiveDate, String)>> = DateCache::new();

/// 日粒度结果缓存（FIX007.7 可测核心）：同日期只算一次，跨日换槽——
/// 农历派生输入为日粒度（春节转换循环/节日 BTreeMap 重建/节气取表），
/// 原实现每帧重算属纯浪费；单槽即可（同一帧内日期恒定，日切时换值）
fn cached_by_date<T: Clone>(
    slot: &DateCache<T>,
    today: NaiveDate,
    load: impl FnOnce(NaiveDate) -> T,
) -> T {
    let mut guard = slot
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .expect("日粒度缓存锁中毒");
    if let Some((d, v)) = guard.as_ref()
        && *d == today
    {
        return v.clone();
    }
    let v = load(today);
    *guard = Some((today, v.clone()));
    v
}

/// 本地时刻解析（FIX006.8）：歧义取较早一侧；不存在（DST 前跳）返 None——不静默改写目标
pub fn local_from_naive(at: NaiveDateTime) -> Option<chrono::DateTime<chrono::Local>> {
    match chrono::Local.from_local_datetime(&at) {
        chrono::LocalResult::Single(d) => Some(d),
        chrono::LocalResult::Ambiguous(earliest, _) => Some(earliest),
        chrono::LocalResult::None => None,
    }
}

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
                        let now = chrono::Local::now().naive_local();
                        // 目标快照（state.countdown_targets：启动快照 + ConfigSaved 重拉）
                        let mut targets: Vec<(&str, NaiveDateTime)> = Vec::new();
                        for t in &state.countdown_targets {
                            if let Some(at) = t.parse_at() {
                                // A005 #23：借用名称不 clone（原实现每帧克隆全部目标名）
                                targets.push((t.name.as_str(), at));
                            }
                        }
                        // hero：最近目标大字（FIX006.3：持久目标与派生春节目标取更近者——空列表亦有兜底）
                        // FIX007.7：派生结果按日缓存（原每帧重算春节循环）
                        let derived =
                            cached_by_date(&DERIVED_CACHE, now.date(), derived_spring_festival);
                        if let Some((name, at)) = hero_target(&targets, derived, now) {
                            let (text, ended) = format_remaining(at, now);
                            ui.label(
                                egui::RichText::new(name)
                                    .size(crate::theme::font_size("heading"))
                                    .color(ink.muted),
                            );
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
                                    .size(crate::theme::font_size("body_lg"))
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
                                    .size(crate::theme::font_size("label"))
                                    .color(ink.card),
                                );
                                ui.label(
                                    egui::RichText::new(&text)
                                        .size(crate::theme::font_size("label"))
                                        .color(if ended { ink.danger } else { ink.muted }),
                                );
                                if ui.small_button("－").clicked() {
                                    bus.send(FrontendCommand::RemoveCountdownTarget(
                                        (*name).to_string(),
                                    ));
                                }
                            });
                        }
                        // 近期节日 chips（PL014.07 + FIX006.3：year_chips 派生取最近 6 枚，不落盘）
                        // FIX007.7：按日缓存（原每帧重建 BTreeMap/遍历两年规则/节气取表）
                        let chips = cached_by_date(&CHIPS_CACHE, now.date(), |d| {
                            accel_core::chinese_calendar::year_chips(d)
                        });
                        if !chips.is_empty() {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new("近期节日")
                                    .size(crate::theme::font_size("label"))
                                    .color(ink.muted),
                            );
                            ui.horizontal(|ui| {
                                for (i, (_, fest)) in chips.iter().take(6).enumerate() {
                                    // FIX007.3：固定序号 id（原日期键跨日累积逼近
                                    // MAX_SURFACES=16 上限）+ countdown_ 页前缀
                                    chip::show(
                                        ui,
                                        scene,
                                        &format!("countdown_festival_{i}"),
                                        "节日",
                                        fest,
                                        false,
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
                                && let Some(local) = local_from_naive(at)
                            {
                                bus.send(FrontendCommand::AddCountdownTarget {
                                    name: state.countdown_draft_name.clone(),
                                    at: local,
                                });
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
// 输入：UiState.countdown_targets（后端权威列表）+ 草稿输入态；输出：hero/网格/节日
//   chips/加号（校验）+ Add/RemoveCountdownTarget 命令
// 逻辑步骤：parse_at 装载 → hero 取「持久目标最近未到者 vs 派生春节目标」更近者
//   （FIX006.3）→ 网格行卡 + 减号 → 近期节日 chips（year_chips 取 6）→ 加号行
//   （validate_add：空名/同名/<24h 拒）→ 确认经 local_from_naive（FIX006.8 DST 歧义
//   取较早一侧、不存在则不提交）后发命令
// 设计理由：校验/最近目标/派生目标选取全抽纯函数（可测）；FIX006.3：PL014.07 的
//   「默认派生目标 = next_spring_festival（不落盘）+ 节日 chips」在 UI 层接线，空列表
//   亦有兜底（原实现仅显示「暂无目标」）；FIX006.2：命令统一经 bus.send（待发队列保底）
// 已知偏移：草稿日期输入用单行文本（日期件控件未引入——A005 #25 已移除未用的 egui_extras）
// 异常处理：非法输入/校验失败禁用确认并显示原因；DST 不存在时刻不提交（不改写目标）
// 关联依赖：core::chinese_calendar（节日派生）/tools::countdown/settings::CountdownTarget；
//   消费方：pages::Pages
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    /// 常规时刻（便于构造固定 now）
    fn at(y: i32, m: u32, d: u32, hh: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .expect("合法日期")
            .and_hms_opt(hh, 0, 0)
            .expect("合法时刻")
    }

    #[test]
    fn derived_target_present_for_empty_list() {
        // FIX006.3 回归：空 targets 时 hero 仍有派生春节目标（原实现显示「暂无目标」）
        let now = at(2026, 3, 1, 9);
        let derived = derived_spring_festival(now.date());
        let (name, dat) = derived.expect("派生目标应恒存在");
        assert_eq!(name, "春节");
        assert!(dat > now, "2026-03 的派生目标应为未来");
        assert_eq!(hero_target(&[], derived, now).map(|(n, _)| n), Some("春节"));
    }

    #[test]
    fn derived_target_rolls_to_next_year() {
        // 跨年用例：now = 12-31 时派生目标切次年春节（PL014.07 验收点）
        let now = at(2026, 12, 31, 9);
        let (_, dat) = derived_spring_festival(now.date()).expect("派生目标");
        assert!(dat > now);
        assert_eq!(dat.date().year(), 2027, "12-31 应切次年春节");
    }

    #[test]
    fn hero_prefers_nearer_target() {
        // 持久目标更近取持久目标；更远取派生目标
        let now = at(2026, 3, 1, 9);
        let near = now + Duration::days(10);
        assert_eq!(
            hero_target(&[("生日", near)], derived_spring_festival(now.date()), now)
                .map(|(n, _)| n),
            Some("生日")
        );
        let far = now + Duration::days(400);
        assert_eq!(
            hero_target(&[("远期", far)], derived_spring_festival(now.date()), now).map(|(n, _)| n),
            Some("春节")
        );
    }

    #[test]
    fn local_from_naive_never_silently_shifts() {
        // FIX006.8 回归：常规时刻解析后与输入一致（原实现 .single().unwrap_or(now) 会静默改写）
        let input = at(2027, 2, 6, 8);
        let local = local_from_naive(input).expect("常规时刻应可解析");
        assert_eq!(local.naive_local(), input, "不得改写用户所填时刻");
    }

    #[test]
    fn cached_by_date_loads_once_per_date() {
        // FIX007.7 验收：同日期两次调用只解析一次（计数桩）；跨日重新解析
        static SLOT: DateCache<u32> = DateCache::new();
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 23).expect("日期合法");
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 24).expect("日期合法");
        let mut loads = 0;
        let a = cached_by_date(&SLOT, d1, |_| {
            loads += 1;
            7u32
        });
        let b = cached_by_date(&SLOT, d1, |_| {
            loads += 1;
            99
        });
        assert_eq!(a, 7, "首调返回装载值");
        assert_eq!(b, 7, "同日期二次调用命中缓存");
        assert_eq!(loads, 1, "同日期只解析一次");
        let c = cached_by_date(&SLOT, d2, |_| {
            loads += 1;
            8
        });
        assert_eq!(c, 8, "跨日重新解析");
        assert_eq!(loads, 2, "跨日装载恰一次");
    }
}
