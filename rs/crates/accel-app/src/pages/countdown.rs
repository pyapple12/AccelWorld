//! 倒计时页（PL017 重排）：PageHeader（Pages 统一）+ 可滚内容 + 固定 Footer。
//! hero = 下一个目标（不与列表重复：列表只列其余目标）+ 节日 meta 行 + 目标行卡；
//! 添加表单常驻页脚，目标再多也够得着。纯逻辑（校验/最近目标/派生）保持可测。

use crate::tools::countdown::format_remaining;
use crate::widgets::{digits, field, icons};
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

/// 日粒度结果缓存（FIX007.7 可测核心）：同日期只算一次，跨日换槽
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

/// 最近未到目标（hero 用）：取未到项中 at 最小者
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
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut crate::bus::UiState, bus: &crate::bus::Bus) {
        let ink = crate::theme::ink();
        state.armed_delete_tick();
        let now = chrono::Local::now().naive_local();
        let targets: Vec<(String, NaiveDateTime)> = state
            .countdown_targets
            .iter()
            .filter_map(|t| t.parse_at().map(|at| (t.name.clone(), at)))
            .collect();
        let pairs: Vec<(&str, NaiveDateTime)> =
            targets.iter().map(|(n, at)| (n.as_str(), *at)).collect();
        let derived = cached_by_date(&DERIVED_CACHE, now.date(), derived_spring_festival);
        let hero = hero_target(&pairs, derived, now).map(|(n, at)| (n.to_string(), at));
        let hero_name = hero.as_ref().map(|(n, _)| n.clone());

        // 页脚固定；滚动只发生在内容区
        let footer_h = footer_height(ui.available_width());
        let body_h = (ui.available_height() - footer_h - 8.0).max(0.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), body_h), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some((name, at)) = &hero {
                        draw_hero(ui, name, *at, now, &ink);
                    } else {
                        crate::widgets::empty::show(
                            ui,
                            "还没有目标",
                            "在下方填写名称与时刻，添加第一个倒计时",
                        );
                    }

                    // 近期节日：一条 meta 行，不用 chips 盒子
                    let chips = cached_by_date(&CHIPS_CACHE, now.date(), |d| {
                        accel_core::chinese_calendar::year_chips(d)
                    });
                    if !chips.is_empty() {
                        ui.add_space(crate::theme::spacing("md"));
                        ui.horizontal_wrapped(|ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("近期节日")
                                        .font(crate::theme::font_regular(crate::theme::font_size(
                                            "meta",
                                        )))
                                        .color(ink.muted),
                                )
                                .wrap_mode(egui::TextWrapMode::Extend),
                            );
                            for (i, (_, fest)) in chips.iter().take(6).enumerate() {
                                if i > 0 {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("·")
                                                .font(crate::theme::font_regular(
                                                    crate::theme::font_size("meta"),
                                                ))
                                                .color(ink.meta),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                }
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(fest)
                                            .font(crate::theme::font_regular(
                                                crate::theme::font_size("meta"),
                                            ))
                                            .color(ink.body),
                                    )
                                    .wrap_mode(egui::TextWrapMode::Extend),
                                );
                            }
                        });
                    }

                    // 其余目标（hero 已单列，避免同一数字出现两次）
                    let rest: Vec<&(String, NaiveDateTime)> = targets
                        .iter()
                        .filter(|(n, _)| Some(n) != hero_name.as_ref())
                        .collect();
                    if !rest.is_empty() {
                        ui.add_space(crate::theme::spacing("lg"));
                        ui.label(
                            egui::RichText::new("其他目标")
                                .font(crate::theme::font_regular(crate::theme::font_size("meta")))
                                .color(ink.muted),
                        );
                        ui.add_space(6.0);
                        for (name, at) in rest {
                            draw_target_row(ui, state, bus, name, *at, now, &ink);
                        }
                    }

                    if targets.is_empty() {
                        ui.add_space(crate::theme::spacing("sm"));
                        let avail = ui.available_height();
                        if avail >= 150.0 {
                            // CTA 先画（不滚动也可见），空态卡填剩余空间
                            let (cta, resp) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 28.0),
                                egui::Sense::click(),
                            );
                            ui.painter().text(
                                egui::pos2(cta.center().x, cta.center().y),
                                egui::Align2::CENTER_CENTER,
                                "添加第一个目标",
                                crate::theme::font_medium(crate::theme::font_size("body")),
                                crate::theme::accent_color(),
                            );
                            if resp
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                state.focus_countdown_name = true;
                            }
                            let card_h = ui.available_height().clamp(96.0, 240.0);
                            crate::widgets::empty::card(
                                ui,
                                "还没有自定义目标",
                                "在下方填写名称与时刻，或点上方节日快速开始",
                                card_h,
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("还没有目标——在下方填写名称与时刻")
                                    .font(crate::theme::font_regular(crate::theme::font_size(
                                        "meta",
                                    )))
                                    .color(ink.meta),
                            );
                        }
                    }
                });
        });

        ui.add_space(8.0);
        draw_footer(ui, state, bus, &now, &targets);
    }
}

/// hero 卡：下一个目标 + 大剩余 + 日期；不与列表重复
fn draw_hero(
    ui: &mut egui::Ui,
    name: &str,
    at: NaiveDateTime,
    now: NaiveDateTime,
    ink: &crate::theme::Ink,
) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 126.0),
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
        egui::Stroke::new(1.2, crate::theme::accent_color()),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 16.0, rect.top() + 12.0),
        egui::Align2::LEFT_TOP,
        "下一个",
        crate::theme::font_medium(crate::theme::font_size("meta")),
        crate::theme::accent_color(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 16.0, rect.top() + 31.0),
        egui::Align2::LEFT_TOP,
        name,
        crate::theme::font_semibold(crate::theme::font_size("body_lg")),
        ink.strong,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 16.0, rect.bottom() - 10.0),
        egui::Align2::LEFT_BOTTOM,
        at.format("%Y-%m-%d %H:%M").to_string(),
        crate::theme::font_regular(crate::theme::font_size("meta")),
        ink.meta,
    );
    let (text, ended) = format_remaining(at, now);
    let base = 40.0;
    let font = digits::digit_font(base);
    let measured = digits::text_width(ui.painter(), &text, &font);
    let scale = crate::pages::clock::hero_fit_scale(measured, rect.width() - 32.0);
    let font = digits::digit_font(base * scale);
    digits::draw_centered(
        ui.painter(),
        egui::pos2(
            rect.right() - 16.0 - measured * scale / 2.0,
            rect.center().y + 6.0,
        ),
        &text,
        &font,
        if ended { ink.danger } else { ink.strong },
    );
}

/// 目标行卡：名称/日期左，剩余右对齐；删除为 Trash（悬停/聚焦可见提示）
fn draw_target_row(
    ui: &mut egui::Ui,
    state: &mut crate::bus::UiState,
    bus: &crate::bus::Bus,
    name: &str,
    at: NaiveDateTime,
    now: NaiveDateTime,
    ink: &crate::theme::Ink,
) {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 68.0), egui::Sense::hover());
    let hover_t = crate::widgets::motion::hover_t(
        ui.ctx(),
        egui::Id::unique(("countdown_row", name)),
        resp.hovered(),
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
            crate::widgets::motion::lerp_color(
                crate::theme::border_color(),
                crate::theme::border_strong_color(),
                hover_t,
            ),
        ),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.top() + 10.0),
        egui::Align2::LEFT_TOP,
        name,
        crate::theme::font_medium(crate::theme::font_size("body")),
        ink.body,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.bottom() - 9.0),
        egui::Align2::LEFT_BOTTOM,
        at.format("%Y-%m-%d %H:%M").to_string(),
        crate::theme::font_regular(crate::theme::font_size("meta")),
        ink.meta,
    );
    let (text, ended) = format_remaining(at, now);
    let font = digits::digit_font(crate::theme::font_size("time_sm"));
    let w = digits::text_width(ui.painter(), &text, &font);
    digits::draw_centered(
        ui.painter(),
        egui::pos2(rect.right() - 36.0 - w / 2.0, rect.center().y),
        &text,
        &font,
        if ended { ink.danger } else { ink.muted },
    );
    // 删除：Trash + 两步武装（Check），常显但低对比，悬停转 danger
    let del_key = format!("cd:{name}");
    let armed = matches!(&state.armed_delete, Some((k, _)) if *k == del_key);
    let del_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 18.0, rect.center().y),
        egui::vec2(24.0, 24.0),
    );
    let del = ui.interact(
        del_rect,
        egui::Id::unique(("cd_del", name)),
        egui::Sense::click(),
    );
    icons::draw(
        ui.painter(),
        del_rect.center(),
        14.0,
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
            "再次点击确认移除"
        } else {
            "删除目标"
        })
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if del.clicked() {
        if armed {
            bus.send(FrontendCommand::RemoveCountdownTarget(name.to_string()));
            state.armed_delete = None;
        } else {
            state.armed_delete = Some((del_key, std::time::Instant::now()));
        }
    }
}

/// 页脚高度：窄抽屉（内容 <340）用两行表单，否则单行
fn footer_height(avail: f32) -> f32 {
    if avail < 340.0 {
        118.0
    } else {
        crate::theme::page_footer_height()
    }
}

/// 页脚：名称 + 时刻 + 添加；窄宽两行（字段一行、主按钮一行）；错误行恒占位
fn draw_footer(
    ui: &mut egui::Ui,
    state: &mut crate::bus::UiState,
    bus: &crate::bus::Bus,
    now: &NaiveDateTime,
    targets: &[(String, NaiveDateTime)],
) {
    let ink = crate::theme::ink();
    let avail = ui.available_width();
    let button_w = 88.0;
    let narrow = avail < 340.0;
    let (name_w, at_w) = if narrow {
        let w = ((avail - 8.0) / 2.0).max(80.0);
        (w, w)
    } else {
        let at = (avail * 0.38).clamp(120.0, 180.0);
        ((avail - at - button_w - 16.0).clamp(96.0, 220.0), at)
    };
    let draft_at_empty = state.countdown_draft_at.is_empty();
    let draft_name_empty = state.countdown_draft_name.is_empty();
    let (err_msg, local_at) = {
        let name = state.countdown_draft_name.clone();
        match crate::tools::countdown::parse_target_text(&state.countdown_draft_at) {
            Some(at) => {
                let names: Vec<String> = targets.iter().map(|(n, _)| n.clone()).collect();
                match validate_add(&name, at, *now, &names) {
                    Ok(()) => match local_from_naive(at) {
                        Some(local) => (String::new(), Some(local)),
                        None => ("该时刻在本地时区不存在（夏令时切换跳过）".to_string(), None),
                    },
                    Err(_) if draft_name_empty => (String::new(), None),
                    Err(e) => (e.to_string(), None),
                }
            }
            None if draft_at_empty => (String::new(), None),
            None => (
                "时刻无法解析——格式应为 YYYY-MM-DD HH:MM[:SS] 或 YYYY-MM-DD".to_string(),
                None,
            ),
        }
    };
    let mut do_add = false;
    let mut name_resp: Option<egui::Response> = None;
    if narrow {
        ui.horizontal(|ui| {
            name_resp = Some(field::show(
                ui,
                "目标名称",
                "例如 项目发布",
                &mut state.countdown_draft_name,
                name_w,
            ));
            ui.add_space(8.0);
            field::show(
                ui,
                "目标时刻",
                "YYYY-MM-DD HH:MM",
                &mut state.countdown_draft_at,
                at_w,
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space((avail - button_w).max(0.0));
            do_add = crate::widgets::primary_button::show(ui, "添加", button_w).clicked();
        });
    } else {
        ui.horizontal(|ui| {
            name_resp = Some(field::show(
                ui,
                "目标名称",
                "例如 项目发布",
                &mut state.countdown_draft_name,
                name_w,
            ));
            ui.add_space(8.0);
            field::show(
                ui,
                "目标时刻",
                "YYYY-MM-DD HH:MM",
                &mut state.countdown_draft_at,
                at_w,
            );
            ui.add_space(8.0);
            ui.vertical(|ui| {
                // 与输入框底边对齐（字段 54：标签 16 + 4 + 输入 34）
                ui.add_space(field::field_height() - 32.0);
                do_add = crate::widgets::primary_button::show(ui, "添加", button_w).clicked();
            });
        });
    }
    if let Some(resp) = name_resp
        && state.focus_countdown_name
    {
        resp.request_focus();
        state.focus_countdown_name = false;
    }
    if do_add && let Some(local) = local_at {
        bus.send(FrontendCommand::AddCountdownTarget {
            name: state.countdown_draft_name.clone(),
            at: local,
        });
        state.countdown_draft_name.clear();
        state.countdown_draft_at.clear();
    }
    let (hint_rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::hover());
    if !err_msg.is_empty() {
        ui.painter().text(
            egui::pos2(hint_rect.left(), hint_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &err_msg,
            crate::theme::font_regular(crate::theme::font_size("meta")),
            ink.danger,
        );
    }
}

// ===== src/pages/countdown.rs 说明区 =====
// 输入：UiState 的目标快照 + 草稿；输出：hero/节日 meta/其他目标行/固定 Footer 添加
// 逻辑步骤：目标快照 → hero（最近目标 vs 派生春节；列表排除 hero，避免重复）→
//   节日 meta 行 → 其余目标行（剩余右对齐、Trash 两步确认）→ Footer（名称/时刻/
//   添加，错误恒占位 16px）
// 设计理由（PL017）：原 hero + 列表重复同一数字，3 个目标后 Footer 被 ScrollArea
//   挤出；本版 hero 单列 + 列表只列其余 + 页脚固定；56px hero 拆成 40px 并收号，
//   280px 抽屉不再两端裁切
// 异常处理：空草稿不产红字；DST 不存在不提交；非法格式同位红字；hero 空有兜底
// 关联依赖：tools::countdown/chinese_calendar/widgets(field/digits/empty)；
//   消费方：pages::Pages 抽屉
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    fn at(y: i32, m: u32, d: u32, hh: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .expect("合法日期")
            .and_hms_opt(hh, 0, 0)
            .expect("合法时刻")
    }

    #[test]
    fn derived_target_present_for_empty_list() {
        let now = at(2026, 3, 1, 9);
        let derived = derived_spring_festival(now.date());
        let (name, dat) = derived.expect("派生目标应恒存在");
        assert_eq!(name, "春节");
        assert!(dat > now);
        assert_eq!(hero_target(&[], derived, now).map(|(n, _)| n), Some("春节"));
    }

    #[test]
    fn derived_target_rolls_to_next_year() {
        let now = at(2026, 12, 31, 9);
        let (_, dat) = derived_spring_festival(now.date()).expect("派生目标");
        assert!(dat > now);
        assert_eq!(dat.date().year(), 2027);
    }

    #[test]
    fn hero_prefers_nearer_target() {
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
        let input = at(2027, 2, 6, 8);
        let local = local_from_naive(input).expect("常规时刻应可解析");
        assert_eq!(local.naive_local(), input);
    }

    #[test]
    fn cached_by_date_loads_once_per_date() {
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
        assert_eq!(a, 7);
        assert_eq!(b, 7);
        assert_eq!(loads, 1);
        let c = cached_by_date(&SLOT, d2, |_| {
            loads += 1;
            8
        });
        assert_eq!(c, 8);
        assert_eq!(loads, 2);
    }
}
