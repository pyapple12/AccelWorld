//! 倒计时页（PL017 重排）：PageHeader（Pages 统一）+ 可滚内容 + 固定 Footer。
//! hero = 下一个目标（不与列表重复：列表只列其余目标）+ 节日 meta 行 + 目标行卡；
//! 添加表单常驻页脚，目标再多也够得着。纯逻辑（校验/最近目标/派生）保持可测。

use crate::tools::countdown::format_remaining;
use crate::widgets::text_ink::{self, InkKind};
use crate::widgets::{digits, field, icons};
use accel_core::types::FrontendCommand;
use chrono::{Duration, NaiveDate, NaiveDateTime, TimeZone};

/// hero 目标选取（纯函数，FIX006.3）：持久目标最近未到者 与 派生春节目标 取更近者
/// PL021：默认 hero 已固定为下一个春节，本函数仅供历史用例
#[cfg(test)]
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

/// 派生兜底目标（PL014.07 + FIX006.3）：次年春节 00:00（历史用例；UI 已改后端快照）
#[cfg(test)]
pub fn derived_spring_festival(today: NaiveDate) -> Option<(&'static str, NaiveDateTime)> {
    accel_core::chinese_calendar::next_spring_festival(today)
        .and_hms_opt(0, 0, 0)
        .map(|at| ("春节", at))
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
        let today = now.date();
        // 跨日清除选中：只允许未来节日
        if self
            .selected_festival
            .as_ref()
            .is_some_and(|(d, _)| *d <= today)
        {
            self.selected_festival = None;
        }
        let targets: Vec<(String, NaiveDateTime)> = state
            .countdown_targets
            .iter()
            .filter_map(|t| t.parse_at().map(|at| (t.name.clone(), at)))
            .collect();
        // hero 默认下一个春节（后端快照）；选中节日优先
        let hero: Option<(String, NaiveDateTime)> = self
            .selected_festival
            .as_ref()
            .and_then(|(d, name)| d.and_hms_opt(0, 0, 0).map(|at| (name.clone(), at)))
            .or_else(|| {
                state
                    .spring_festival
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|at| ("春节".to_string(), at))
            });

        // 固定页脚盒：高 122；body 与页脚盒间距 10
        let footer_h = footer_height(ui.available_width());
        let avail = ui.max_rect();
        let body_h = (avail.height() - footer_h - 10.0).max(0.0);
        let body_rect = egui::Rect::from_min_size(avail.min, egui::vec2(avail.width(), body_h));
        let footer_rect = egui::Rect::from_min_size(
            egui::pos2(avail.left(), body_rect.bottom() + 10.0),
            egui::vec2(avail.width(), footer_h),
        );

        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut stack = crate::widgets::box_stack::BoxStack::new(ui);
                    // E2 hero 盒；中国结图标点击恢复默认春节
                    if let Some((name, at)) = &hero {
                        let rect = stack.next(ui, hero_height(ui));
                        if draw_hero(ui, rect, name, *at, now, &ink) {
                            self.selected_festival = None;
                        }
                    }

                    // E7 近期节日盒：后端快照 5 个，可点击切换 hero
                    let festivals = state.festivals.clone();
                    if !festivals.is_empty() {
                        let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
                        let chips_ink = text_ink::measure(ui, "近期节日", &meta_font, InkKind::Cjk);
                        let rect = stack.next(ui, chips_ink.h);
                        if let Some(clicked) = draw_festivals(
                            ui,
                            rect,
                            &festivals,
                            self.selected_festival.as_ref(),
                            &ink,
                        ) {
                            self.selected_festival = Some(clicked);
                        }
                    }

                    // E9 目标行卡盒（hero 是春节/选中节日，不占用户目标）
                    if !targets.is_empty() {
                        let pulse = crate::widgets::motion::list_pulse(state, "countdown");
                        let last = targets.len().saturating_sub(1);
                        let name_font = crate::theme::font_medium(crate::theme::font_size("body"));
                        let date_font = crate::theme::font_regular(crate::theme::font_size("meta"));
                        for (i, (name, at)) in targets.iter().enumerate() {
                            // E9 盒高 = 名称墨迹 + 6 + 日期墨迹 + 上下 12
                            let name_ink = text_ink::measure(ui, name, &name_font, InkKind::Cjk);
                            let date_text = at.format("%Y-%m-%d %H:%M").to_string();
                            let date_ink =
                                text_ink::measure(ui, &date_text, &date_font, InkKind::Cjk);
                            let row_h = name_ink.h + 6.0 + date_ink.h + 24.0;
                            let rect = stack.slot(row_h);
                            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                if i == last && pulse < 1.0 {
                                    ui.set_opacity(0.55 + 0.45 * pulse);
                                }
                                draw_target_row(ui, rect, state, bus, name, *at, now, &ink);
                            });
                        }
                    }

                    // CTA 盒 + 空态盒
                    if targets.is_empty() {
                        // CTA 盒高 = 文字墨迹高；点击区只扩展命中，不占布局
                        let cta_font = crate::theme::font_medium(crate::theme::font_size("body"));
                        let cta_ink =
                            text_ink::measure(ui, "添加第一个目标", &cta_font, InkKind::Cjk);
                        let cta = stack.next(ui, cta_ink.h);
                        draw_empty_cta(ui, cta, state, &ink);
                        let used = cta.bottom() - body_rect.top() + 10.0;
                        let card_h = (body_h - used).clamp(96.0, 240.0);
                        let card = stack.next(ui, card_h);
                        crate::widgets::empty::card_at(
                            ui,
                            card,
                            "还没有自定义目标",
                            "在下方填写名称与时刻，或点上方节日快速开始",
                        );
                    }
                });
        });

        // E15 页脚盒
        ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
            draw_footer(ui, state, bus, &now, &targets);
        });
    }
}

/// E7 近期节日盒：5 个金色可点项；hover / 选中在文字底 +1px 画 2px 金线
fn draw_festivals(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    items: &[(NaiveDate, String)],
    selected: Option<&(NaiveDate, String)>,
    ink: &crate::theme::Ink,
) -> Option<(NaiveDate, String)> {
    let font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let label = "近期节日";
    let label_w = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), ink.muted)
        .rect
        .width();
    let label_ink = text_ink::measure(ui, label, &font, InkKind::Cjk);
    let text_y = rect.top() - label_ink.top_pad;
    ui.painter().text(
        egui::pos2(rect.left(), text_y),
        egui::Align2::LEFT_TOP,
        label,
        font.clone(),
        ink.muted,
    );

    let widths: Vec<f32> = items
        .iter()
        .map(|(_, name)| {
            ui.painter()
                .layout_no_wrap(name.clone(), font.clone(), ink.body)
                .rect
                .width()
        })
        .collect();
    let sum: f32 = widths.iter().sum();
    let gap = if items.len() > 1 {
        ((rect.width() - label_w - 10.0 - sum) / (items.len() - 1) as f32).clamp(4.0, 10.0)
    } else {
        0.0
    };

    let mut x = rect.left() + label_w + 10.0;
    let mut clicked = None;
    for (i, ((date, name), w)) in items.iter().zip(widths.iter()).enumerate() {
        let w = *w;
        let item = egui::Rect::from_min_max(
            egui::pos2(x, rect.top() - 2.0),
            egui::pos2(x + w, rect.bottom() + 4.0),
        );
        let resp = ui.interact(
            item,
            egui::Id::unique(("festival_item", i)),
            egui::Sense::click(),
        );
        let is_sel = selected.is_some_and(|(d, n)| *d == *date && n == name);
        let hover = crate::widgets::motion::hover_t(
            ui.ctx(),
            egui::Id::unique(("festival_hover", i)),
            resp.hovered() || is_sel,
            140.0,
        );
        let accent = crate::theme::accent_color();
        ui.painter().text(
            egui::pos2(x, text_y),
            egui::Align2::LEFT_TOP,
            name,
            font.clone(),
            accent,
        );
        if hover > 0.01 {
            let line_w = w * hover;
            let line_x = x + (w - line_w) / 2.0;
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(line_x, rect.bottom() + 1.0),
                    egui::vec2(line_w, 2.0),
                ),
                0,
                accent,
            );
        }
        if resp.clicked() {
            clicked = Some((*date, name.clone()));
        }
        x += w + gap;
    }
    clicked
}

/// CTA 盒：无目标时出现，整盒可点，文字居中
fn draw_empty_cta(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut crate::bus::UiState,
    _ink: &crate::theme::Ink,
) {
    let font = crate::theme::font_medium(crate::theme::font_size("body"));
    let cta_ink = text_ink::measure(ui, "添加第一个目标", &font, InkKind::Cjk);
    let resp = ui.interact(
        rect.expand2(egui::vec2(0.0, 6.0)),
        egui::Id::unique("countdown_empty_cta"),
        egui::Sense::click(),
    );
    ui.painter().text(
        egui::pos2(rect.center().x, rect.top() - cta_ink.top_pad),
        egui::Align2::CENTER_TOP,
        "添加第一个目标",
        font,
        crate::theme::accent_color(),
    );
    if resp
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        state.focus_countdown_name = true;
    }
}

/// hero 名称行高：固定取一组代表节日名的最大墨迹高，切换节日不改变盒高
fn hero_name_line_h(ui: &egui::Ui) -> f32 {
    static H: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *H.get_or_init(|| {
        let font = crate::theme::font_semibold(12.0);
        [
            "春节",
            "清明节",
            "元宵节",
            "端午节",
            "中秋节",
            "国庆节",
            "元旦",
            "除夕",
        ]
        .iter()
        .map(|s| text_ink::measure(ui, s, &font, InkKind::Cjk).h)
        .fold(0.0, f32::max)
    })
}

/// hero 盒高：三行内容墨迹 + 固定 10 间距（不是行框）
fn hero_height(ui: &egui::Ui) -> f32 {
    let remain_font = crate::theme::font_regular(28.0);
    let value_font = crate::theme::font_semibold(28.0);
    let date_font = crate::theme::font_regular(10.0);
    let name_h = hero_name_line_h(ui);
    let remain_ink = text_ink::measure(ui, "还剩", &remain_font, InkKind::Cjk);
    let tian_ink = text_ink::measure(ui, "天", &value_font, InkKind::Cjk);
    // 数字墨迹高固定取 "0" 模板，避免首位字形不同带来 0.xpx 高度抖动
    let digit_ink = text_ink::measure(ui, "0", &value_font, InkKind::Digits);
    let date_ink = text_ink::measure(ui, "到期日：2027-02-06 00:00", &date_font, InkKind::Cjk);
    let line3 = remain_ink.h.max(tian_ink.h).max(digit_ink.h);
    10.0 + name_h + 10.0 + line3 + 10.0 + date_ink.h + 10.0
}

/// hero 卡：四行——下一个节日/春节、还剩 + 剩余（32px 右对齐）、到期日
fn draw_hero(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    name: &str,
    at: NaiveDateTime,
    now: NaiveDateTime,
    ink: &crate::theme::Ink,
) -> bool {
    let label_font = crate::theme::font_medium(10.0);
    let name_font = crate::theme::font_semibold(12.0);
    let date_font = crate::theme::font_regular(10.0);
    let label_ink = text_ink::measure(ui, "下一个节日", &label_font, InkKind::Cjk);
    let name_ink = text_ink::measure(ui, name, &name_font, InkKind::Cjk);
    let date_text = format!("到期日：{}", at.format("%Y-%m-%d %H:%M"));
    let date_ink = text_ink::measure(ui, &date_text, &date_font, InkKind::Cjk);

    let (value_text, ended) = format_remaining(at, now);
    let (day_text, time_text) = if ended {
        ("", "")
    } else {
        value_text
            .split_once('天')
            .map(|(d, t)| (d, t.trim()))
            .unwrap_or((value_text.as_str(), ""))
    };
    let mut remain_font = crate::theme::font_regular(28.0);
    let mut value_font = crate::theme::font_semibold(28.0);
    let mut remain_ink = text_ink::measure(ui, "还剩", &remain_font, InkKind::Cjk);
    let mut tian_ink = text_ink::measure(ui, "天", &value_font, InkKind::Cjk);
    let mut digit_ink = text_ink::measure(ui, "0", &value_font, InkKind::Digits);
    let mut remain_w = ui
        .painter()
        .layout_no_wrap("还剩".to_string(), remain_font.clone(), ink.strong)
        .rect
        .width();
    // 固定留 3 位：天数槽宽按 000 算死，2 位时左侧空 1 槽，秒数变化不抖
    let mut day_template_w = digits::text_width(ui.painter(), "000", &value_font);
    let mut day_actual_w = digits::text_width(ui.painter(), day_text, &value_font);
    let mut time_w = digits::text_width(ui.painter(), "00:00:00", &value_font);
    let mut tian_w = ui
        .painter()
        .layout_no_wrap("天".to_string(), value_font.clone(), ink.strong)
        .rect
        .width();
    let mut value_w = day_template_w + 5.0 + tian_w + 5.0 + time_w;
    let avail = (rect.width() - 20.0).max(1.0);
    if remain_w + 6.0 + value_w > avail {
        let scale = avail / (remain_w + 6.0 + value_w);
        remain_font = crate::theme::font_regular(28.0 * scale);
        value_font = crate::theme::font_semibold(28.0 * scale);
        remain_ink = text_ink::measure(ui, "还剩", &remain_font, InkKind::Cjk);
        tian_ink = text_ink::measure(ui, "天", &value_font, InkKind::Cjk);
        digit_ink = text_ink::measure(ui, "0", &value_font, InkKind::Digits);
        remain_w = ui
            .painter()
            .layout_no_wrap("还剩".to_string(), remain_font.clone(), ink.strong)
            .rect
            .width();
        day_template_w = digits::text_width(ui.painter(), "000", &value_font);
        day_actual_w = digits::text_width(ui.painter(), day_text, &value_font);
        time_w = digits::text_width(ui.painter(), "00:00:00", &value_font);
        tian_w = ui
            .painter()
            .layout_no_wrap("天".to_string(), value_font.clone(), ink.strong)
            .rect
            .width();
        value_w = day_template_w + 5.0 + tian_w + 5.0 + time_w;
    }

    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        crate::theme::card_bg(),
    );
    painter.rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        egui::Stroke::new(1.2, crate::theme::accent_color()),
        egui::StrokeKind::Inside,
    );

    // 第一行：固定行高内按墨迹底对齐；切换节日名不改变盒高
    let line_h = hero_name_line_h(ui);
    let top = rect.top() + 10.0;
    let name_top = top + line_h - name_ink.h;
    let label_top = top + line_h - label_ink.h;
    let label_w = ui
        .painter()
        .layout_no_wrap("下一个节日".to_string(), label_font.clone(), ink.muted)
        .rect
        .width();
    painter.text(
        egui::pos2(rect.left() + 10.0, label_top - label_ink.top_pad),
        egui::Align2::LEFT_TOP,
        "下一个节日",
        label_font,
        crate::theme::accent_color(),
    );
    painter.text(
        egui::pos2(
            rect.left() + 10.0 + label_w + 8.0,
            name_top - name_ink.top_pad,
        ),
        egui::Align2::LEFT_TOP,
        name,
        name_font,
        ink.strong,
    );

    // 第二行：还剩（28 常规 + accent） + 剩余（28 粗体 + strong），整行右对齐
    let third_top = top + line_h + 10.0;
    let third_h = remain_ink.h.max(tian_ink.h).max(digit_ink.h);
    let center_y = third_top + third_h / 2.0;
    let right = rect.right() - 10.0;
    let value_left = right - value_w;
    painter.text(
        egui::pos2(
            value_left - 6.0 - remain_w,
            center_y - remain_ink.h / 2.0 - remain_ink.top_pad,
        ),
        egui::Align2::LEFT_TOP,
        "还剩",
        remain_font,
        crate::theme::accent_color(),
    );
    if ended {
        let text_w = ui
            .painter()
            .layout_no_wrap(value_text.clone(), value_font.clone(), ink.danger)
            .rect
            .width();
        painter.text(
            egui::pos2(
                right - text_w,
                center_y - tian_ink.h / 2.0 - tian_ink.top_pad,
            ),
            egui::Align2::LEFT_TOP,
            &value_text,
            value_font,
            ink.danger,
        );
    } else {
        // 天数在 3 位模板区内右对齐；数字与天 5px；天与时间 5px
        let start = right - value_w;
        let day_x = start + day_template_w - day_actual_w;
        digits::draw_centered(
            &painter,
            egui::pos2(day_x + day_actual_w / 2.0, center_y),
            day_text,
            &value_font,
            ink.strong,
        );
        let tian_x = start + day_template_w + 5.0;
        painter.text(
            egui::pos2(tian_x, center_y - tian_ink.h / 2.0 - tian_ink.top_pad),
            egui::Align2::LEFT_TOP,
            "天",
            value_font.clone(),
            ink.strong,
        );
        let time_x = tian_x + tian_w + 5.0;
        digits::draw_centered(
            &painter,
            egui::pos2(time_x + time_w / 2.0, center_y),
            time_text,
            &value_font,
            ink.strong,
        );
    }

    // 第三行：到期日
    let date_top = third_top + third_h + 10.0;
    painter.text(
        egui::pos2(rect.left() + 10.0, date_top - date_ink.top_pad),
        egui::Align2::LEFT_TOP,
        &date_text,
        date_font,
        ink.meta,
    );

    // 中国结：第一行右对齐；点击恢复默认春节
    let icon_center = egui::pos2(rect.right() - 10.0 - 8.0, top + name_ink.h / 2.0);
    let icon_resp = ui.interact(
        egui::Rect::from_center_size(icon_center, egui::vec2(24.0, 24.0)),
        egui::Id::unique("countdown_hero_reset"),
        egui::Sense::click(),
    );
    icons::draw(
        &painter,
        icon_center,
        16.0,
        icons::Icon::Calendar,
        if icon_resp.hovered() {
            ink.strong
        } else {
            crate::theme::accent_color()
        },
    );
    icon_resp
        .on_hover_text("来年春节")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// 目标行卡：名称/日期左，剩余右对齐；删除为 Trash（悬停/聚焦可见提示）
fn draw_target_row(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut crate::bus::UiState,
    bus: &crate::bus::Bus,
    name: &str,
    at: NaiveDateTime,
    now: NaiveDateTime,
    ink: &crate::theme::Ink,
) {
    let name_font = crate::theme::font_medium(crate::theme::font_size("body"));
    let date_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let name_ink = text_ink::measure(ui, name, &name_font, InkKind::Cjk);
    let date_text = at.format("%Y-%m-%d %H:%M").to_string();
    let date_ink = text_ink::measure(ui, &date_text, &date_font, InkKind::Cjk);
    let resp = ui.interact(
        rect,
        egui::Id::unique(("countdown_row", name)),
        egui::Sense::hover(),
    );
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
    let top = rect.top() + 12.0;
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, top - name_ink.top_pad),
        egui::Align2::LEFT_TOP,
        name,
        name_font,
        ink.body,
    );
    ui.painter().text(
        egui::pos2(
            rect.left() + 14.0,
            top + name_ink.h + 6.0 - date_ink.top_pad,
        ),
        egui::Align2::LEFT_TOP,
        &date_text,
        date_font,
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

/// 页脚高度：字段 54 + 10 + 按钮 32 + 10 + 错误行 16（统一，不再分窄宽）
fn footer_height(_avail: f32) -> f32 {
    122.0
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
    let gap = 10.0;
    let button_w = 88.0;
    // 名称:时刻 = 4:6（间距 10 从总宽扣除）
    let name_w = ((avail - gap) * 0.4).max(80.0);
    let at_w = ((avail - gap) * 0.6).max(80.0);
    ui.spacing_mut().item_spacing.y = 0.0;
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
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        name_resp = Some(field::show(
            ui,
            "目标名称",
            "例如 项目发布",
            &mut state.countdown_draft_name,
            name_w,
        ));
        ui.add_space(gap);
        field::show(
            ui,
            "目标时刻",
            "YYYY-MM-DD HH:MM",
            &mut state.countdown_draft_at,
            at_w,
        );
    });
    ui.add_space(gap);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space((avail - button_w).max(0.0));
        do_add = crate::widgets::primary_button::show(ui, "添加", button_w).clicked();
    });
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
        state.list_pulse = Some(("countdown".to_string(), std::time::Instant::now()));
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
}
