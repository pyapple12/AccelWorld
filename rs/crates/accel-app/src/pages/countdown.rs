//! 倒计时页（PL017 重排）：PageHeader（Pages 统一）+ 可滚内容 + 固定 Footer。
//! hero = 下一个目标（不与列表重复：列表只列其余目标）+ 节日 meta 行 + 目标行卡；
//! 添加表单常驻页脚，目标再多也够得着。纯逻辑（校验/最近目标/派生）保持可测。

use crate::tools::countdown::format_remaining;
use crate::widgets::text_ink::{self, InkKind};
use crate::widgets::{digits, field, icons};
use accel_core::types::FrontendCommand;
use chrono::{NaiveDate, NaiveDateTime, TimeZone};

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

/// 加号校验（纯函数）：目标不晚于 now 拒（NotYet）、同名拒（Dup）、非法输入拒（Empty）
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
    if at <= now {
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
            AddErr::NotYet => write!(f, "目标时刻必须晚于当前时刻"),
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

        // 页脚高度：字段实际行高 + footer_gap + 按钮实际行高 + footer_bottom
        let footer_h = footer_height(ui);
        let avail = ui.max_rect();
        let body_h = (avail.height() - footer_h - crate::theme::layout_f32("footer_gap")).max(0.0);
        let body_rect = egui::Rect::from_min_size(avail.min, egui::vec2(avail.width(), body_h));
        let footer_rect = egui::Rect::from_min_size(
            egui::pos2(
                avail.left(),
                body_rect.bottom() + crate::theme::layout_f32("footer_gap"),
            ),
            egui::vec2(avail.width(), footer_h),
        );

        // PL022：固定头 = hero + 近期节日，不参与滚动；只有 E9 目标卡进列表视口
        let festivals = state.festivals.clone();
        let hero_h = hero.as_ref().map(|_| hero_height(ui));
        let meta_h = if festivals.is_empty() {
            None
        } else {
            let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
            Some(crate::widgets::text_metrics::height(
                ui.ctx(),
                &meta_font,
                InkKind::Cjk,
            ))
        };
        let mut header_h = 0.0;
        if let Some(h) = hero_h {
            header_h += h + crate::widgets::box_stack::BOX_GAP;
        }
        if let Some(h) = meta_h {
            header_h += h + crate::widgets::box_stack::BOX_GAP;
        }
        let header_h = header_h.min(body_h);
        let header_rect =
            egui::Rect::from_min_size(body_rect.min, egui::vec2(body_rect.width(), header_h));
        let list_rect = egui::Rect::from_min_max(
            egui::pos2(body_rect.left(), header_rect.bottom()),
            body_rect.max,
        );

        if header_h > crate::theme::layout_f64("list_visible_eps") {
            ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
                ui.set_clip_rect(header_rect);
                let mut stack = crate::widgets::box_stack::BoxStack::new(ui);
                // E2 hero 盒；中国结图标点击恢复默认春节
                if let Some((name, at)) = &hero {
                    let rect = stack.next(ui, hero_height(ui));
                    if draw_hero(ui, rect, name, *at, now, &ink) {
                        self.selected_festival = None;
                    }
                }

                // E7 近期节日盒：后端快照 5 个，可点击切换 hero
                if let Some(h) = meta_h {
                    let rect = stack.next(ui, h);
                    if let Some(clicked) =
                        draw_festivals(ui, rect, &festivals, self.selected_festival.as_ref(), &ink)
                    {
                        self.selected_festival = Some(clicked);
                    }
                }
            });
        }

        let row_h = target_row_height(ui);
        if crate::layout_probe::layout_on() {
            use std::sync::atomic::{AtomicU64, Ordering};
            static FRAME: AtomicU64 = AtomicU64::new(0);
            if FRAME.fetch_add(1, Ordering::Relaxed).is_multiple_of(30) {
                let gap = crate::widgets::box_stack::BOX_GAP;
                let hero_name = hero_name_line_h(ui);
                let hero_full = hero_height(ui);
                let row_full = target_row_height(ui);
                let field_h = crate::widgets::field::fixed_height(ui);
                let button_h = crate::widgets::primary_button::height(ui);
                let footer_gap = crate::theme::layout_f32("footer_gap");
                let footer_bottom = crate::theme::layout_f32("footer_bottom");
                eprintln!(
                    "[LAYOUT_COUNTDOWN] avail=({:.1},{:.1},{:.1},{:.1}) footer_h={:.1} body_h={:.1} body=({:.1},{:.1},{:.1},{:.1}) footer=({:.1},{:.1},{:.1},{:.1}) hero_h={:.1} meta_h={:.1} gap={:.1} header_h={:.1} header=({:.1},{:.1},{:.1},{:.1}) list=({:.1},{:.1},{:.1},{:.1}) row_h={:.1} rows={} rows_x_h={:.1} hero_name={:.1} hero_full={:.1} row_full={:.1} field={:.1} button={:.1} footer_parts=({:.1}+{:.1}+{:.1}+{:.1}) hero_present={} festivals={}",
                    avail.left(),
                    avail.top(),
                    avail.width(),
                    avail.height(),
                    footer_h,
                    body_h,
                    body_rect.left(),
                    body_rect.top(),
                    body_rect.width(),
                    body_rect.height(),
                    footer_rect.left(),
                    footer_rect.top(),
                    footer_rect.width(),
                    footer_rect.height(),
                    hero_h.unwrap_or(0.0),
                    meta_h.unwrap_or(0.0),
                    gap,
                    header_h,
                    header_rect.left(),
                    header_rect.top(),
                    header_rect.width(),
                    header_rect.height(),
                    list_rect.left(),
                    list_rect.top(),
                    list_rect.width(),
                    list_rect.height(),
                    row_h,
                    targets.len(),
                    targets.len() as f32 * row_h,
                    hero_name,
                    hero_full,
                    row_full,
                    field_h,
                    button_h,
                    field_h,
                    footer_gap,
                    button_h,
                    footer_bottom,
                    hero.is_some(),
                    festivals.len()
                );
            }
        }
        if targets.is_empty() {
            // 空态与滚动无关：不建 ScrollArea，CTA + 虚线框静态占列表矩形
            if list_rect.height() > crate::theme::layout_f64("list_visible_eps") {
                ui.scope_builder(egui::UiBuilder::new().max_rect(list_rect), |ui| {
                    ui.set_clip_rect(list_rect);
                    let mut stack = crate::widgets::box_stack::BoxStack::new(ui);
                    // CTA 盒高 = 固定行高；点击区只扩展命中，不占布局
                    let cta_font = crate::theme::font_medium(crate::theme::font_size("body"));
                    let cta_h =
                        crate::widgets::text_metrics::height(ui.ctx(), &cta_font, InkKind::Cjk);
                    let cta = stack.next(ui, cta_h);
                    draw_empty_cta(ui, cta, state, &ink);
                    // 虚线框吃掉 CTA 之后的剩余高度；不足最小内容高则整块消失
                    let used = cta.bottom() - list_rect.top() + crate::widgets::box_stack::BOX_GAP;
                    let remain_h = list_rect.height() - used;
                    if remain_h >= crate::widgets::empty::min_height(ui) {
                        let card = stack.next(ui, remain_h);
                        crate::widgets::empty::card_at(
                            ui,
                            card,
                            "还没有自定义目标",
                            "在下方填写名称与时刻",
                        );
                    }
                });
            }
        } else if list_rect.height() + crate::theme::layout_f64("list_visible_eps") >= row_h {
            // 兜底通过：列表视口只装目标卡；滑杆/渐隐/三角贴视口，不跨固定头与页脚
            crate::widgets::list_scroll::show(
                ui,
                list_rect,
                "countdown_list",
                "LAYOUT_COUNTDOWN_LIST",
                &mut self.list_scroll,
                |ui| {
                    let mut stack = crate::widgets::box_stack::BoxStack::new(ui);
                    let pulse = crate::widgets::motion::list_pulse(state, "countdown");
                    let pulse_base = crate::theme::layout_f64("list_pulse_base");
                    let pulse_span = crate::theme::layout_f64("list_pulse_span");
                    let last = targets.len().saturating_sub(1);
                    for (i, (name, at)) in targets.iter().enumerate() {
                        let rect = stack.next(ui, row_h);
                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            if i == last && pulse < 1.0 {
                                ui.set_opacity(pulse_base + pulse_span * pulse);
                            }
                            draw_target_row(ui, rect, state, bus, name, *at, now, &ink);
                        });
                    }
                },
            );
        }

        // E15 页脚盒
        ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
            draw_footer(ui, state, bus, &now, &targets, &mut self.time_error);
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
    let text_y = label_ink.top_for_bottom(rect.bottom());
    crate::layout_probe::rect(ui, "cd.fest", rect);
    crate::layout_probe::text(
        ui,
        "cd.fest_label",
        label,
        &font,
        InkKind::Cjk,
        egui::pos2(rect.left(), text_y),
        egui::Align2::LEFT_TOP,
    );
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
        let resp = ui
            .interact(
                item,
                egui::Id::unique(("festival_item", i)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let is_sel = selected.is_some_and(|(d, n)| *d == *date && n == name);
        let hover = crate::widgets::motion::hover_t(
            ui.ctx(),
            egui::Id::unique(("festival_hover", i)),
            resp.hovered() || is_sel,
            crate::theme::motion_ms("card_hover_ms"),
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
                    egui::pos2(line_x, rect.bottom() + 2.0),
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
        egui::pos2(rect.center().x, cta_ink.top_for_bottom(rect.bottom())),
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

/// hero 名称行高：查全局角色表（「下一个节日」10px 与节日名 12px 取最大）
fn hero_name_line_h(ui: &egui::Ui) -> f32 {
    let label = crate::theme::font_medium(crate::theme::font_size("meta"));
    let name = crate::theme::font_semibold(crate::theme::font_size("meta"));
    crate::widgets::text_metrics::height(ui.ctx(), &label, InkKind::Cjk).max(
        crate::widgets::text_metrics::height(ui.ctx(), &name, InkKind::Cjk),
    )
}

/// hero 盒高：三行固定行高 + 10 间距；全部来自启动指标表
fn hero_height(ui: &egui::Ui) -> f32 {
    let remain_font = crate::theme::font_regular(crate::theme::font_size("title"));
    let value_font = crate::theme::font_semibold(crate::theme::font_size("title"));
    let digit_font = digits::digit_font(crate::theme::font_size("title"));
    let date_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let date_digit_font = digits::digit_font(crate::theme::font_size("meta"));
    let name_h = hero_name_line_h(ui);
    let line3 = crate::widgets::text_metrics::height(ui.ctx(), &remain_font, InkKind::Cjk)
        .max(crate::widgets::text_metrics::height(
            ui.ctx(),
            &value_font,
            InkKind::Cjk,
        ))
        .max(crate::widgets::text_metrics::height(
            ui.ctx(),
            &digit_font,
            InkKind::Digits,
        ));
    let date_h = crate::widgets::text_metrics::height(ui.ctx(), &date_font, InkKind::Cjk).max(
        crate::widgets::text_metrics::height(ui.ctx(), &date_digit_font, InkKind::Digits),
    );
    let pad_y = crate::theme::layout_f32("hero_pad_y");
    let line_gap = crate::theme::layout_f32("hero_line_gap");
    2.0 * pad_y + name_h + line_gap + line3 + line_gap + date_h
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
    let label_font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let name_font = crate::theme::font_semibold(crate::theme::font_size("meta"));
    let date_prefix_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let date_digit_font = digits::digit_font(crate::theme::font_size("meta"));
    let pad_x = crate::theme::layout_f32("hero_pad_x");
    let pad_y = crate::theme::layout_f32("hero_pad_y");
    let line_gap = crate::theme::layout_f32("hero_line_gap");
    let text_gap = crate::theme::spacing_f32("sm");
    let label_ink = text_ink::measure(ui, "下一个节日", &label_font, InkKind::Cjk);
    let name_ink = text_ink::measure(ui, name, &name_font, InkKind::Cjk);
    let date_prefix = "到期日：";
    let date_value = at.format("%Y-%m-%d %H:%M").to_string();
    let date_prefix_w = ui
        .painter()
        .layout_no_wrap(date_prefix.to_string(), date_prefix_font.clone(), ink.meta)
        .rect
        .width();
    let date_value_w = digits::text_width(ui.painter(), &date_value, &date_digit_font);

    let (value_text, ended) = format_remaining(at, now);
    let (day_text, time_text) = if ended {
        ("", "")
    } else {
        value_text
            .split_once('天')
            .map(|(d, t)| (d, t.trim()))
            .unwrap_or((value_text.as_str(), ""))
    };
    let remain_font = crate::theme::font_regular(crate::theme::font_size("title"));
    let value_font = crate::theme::font_semibold(crate::theme::font_size("title"));
    let digit_font = digits::digit_font(crate::theme::font_size("title"));
    let remain_w = ui
        .painter()
        .layout_no_wrap("还剩".to_string(), remain_font.clone(), ink.strong)
        .rect
        .width();
    // 固定留 3 位：天数槽宽按 000 算死，2 位时左侧空 1 槽，秒数变化不抖
    let day_template_w = digits::text_width(ui.painter(), "000", &digit_font);
    let day_actual_w = digits::text_width(ui.painter(), day_text, &digit_font);
    let time_w = digits::text_width(ui.painter(), "00:00:00", &digit_font);
    let tian_w = ui
        .painter()
        .layout_no_wrap("天".to_string(), value_font.clone(), ink.strong)
        .rect
        .width();

    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        crate::theme::card_bg(),
    );
    painter.rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(crate::theme::radius_px("md") as u8),
        egui::Stroke::new(
            crate::theme::layout_f64("hero_border"),
            crate::theme::accent_color(),
        ),
        egui::StrokeKind::Inside,
    );

    // 第一行：固定行高，两个字号共用同一条基线
    let line_h = hero_name_line_h(ui);
    let top = rect.top() + pad_y;
    let line_bottom = top + line_h;
    let name_y = name_ink.top_for_bottom(line_bottom);
    let label_y = label_ink.top_for_bottom(line_bottom);
    let label_w = ui
        .painter()
        .layout_no_wrap("下一个节日".to_string(), label_font.clone(), ink.muted)
        .rect
        .width();
    crate::layout_probe::rect(ui, "cd.hero", rect);
    crate::layout_probe::text(
        ui,
        "cd.hero_label",
        "下一个节日",
        &label_font,
        InkKind::Cjk,
        egui::pos2(rect.left() + pad_x, label_y),
        egui::Align2::LEFT_TOP,
    );
    crate::layout_probe::text(
        ui,
        "cd.hero_name",
        name,
        &name_font,
        InkKind::Cjk,
        egui::pos2(rect.left() + pad_x + label_w + text_gap, name_y),
        egui::Align2::LEFT_TOP,
    );
    painter.text(
        egui::pos2(rect.left() + pad_x, label_y),
        egui::Align2::LEFT_TOP,
        "下一个节日",
        label_font,
        crate::theme::accent_color(),
    );
    painter.text(
        egui::pos2(rect.left() + pad_x + label_w + text_gap, name_y),
        egui::Align2::LEFT_TOP,
        name,
        name_font,
        ink.strong,
    );

    // 第二行：四个独立盒：还剩 / 天数 / 天 / 时间，盒中心对同一水平中线
    let third_top = top + line_h + line_gap;
    let remain_h = crate::widgets::text_metrics::height(ui.ctx(), &remain_font, InkKind::Cjk);
    let tian_h = crate::widgets::text_metrics::height(ui.ctx(), &value_font, InkKind::Cjk);
    let time_h = crate::widgets::text_metrics::height(ui.ctx(), &digit_font, InkKind::Digits);
    let line3 = remain_h.max(tian_h).max(time_h);
    let center_y = third_top + line3 / 2.0;
    let right = rect.right() - pad_x;
    let time_box = egui::Rect::from_min_size(
        egui::pos2(right - time_w, center_y - time_h / 2.0),
        egui::vec2(time_w, time_h),
    );
    let gap = 5.0;
    let tian_box = egui::Rect::from_min_size(
        egui::pos2(time_box.left() - gap - tian_w, center_y - tian_h / 2.0),
        egui::vec2(tian_w, tian_h),
    );
    let day_box = egui::Rect::from_min_size(
        egui::pos2(
            tian_box.left() - gap - day_template_w,
            center_y - time_h / 2.0,
        ),
        egui::vec2(day_template_w, time_h),
    );
    let remain_box = egui::Rect::from_min_size(
        egui::pos2(day_box.left() - 6.0 - remain_w, center_y - remain_h / 2.0),
        egui::vec2(remain_w, remain_h),
    );
    if ended {
        let text_w = ui
            .painter()
            .layout_no_wrap(value_text.clone(), value_font.clone(), ink.danger)
            .rect
            .width();
        let y = text_ink::top_for_center(ui, &value_text, &value_font, InkKind::Cjk, center_y);
        painter.text(
            egui::pos2(right - text_w, y),
            egui::Align2::LEFT_TOP,
            &value_text,
            value_font,
            ink.danger,
        );
    } else {
        let remain_y = text_ink::top_for_center(ui, "还剩", &remain_font, InkKind::Cjk, center_y);
        painter.text(
            egui::pos2(remain_box.left(), remain_y),
            egui::Align2::LEFT_TOP,
            "还剩",
            remain_font,
            crate::theme::accent_color(),
        );
        digits::draw_bottom(
            &painter,
            day_box.bottom(),
            day_box.right() - day_actual_w / 2.0,
            day_text,
            &digit_font,
            ink.strong,
        );
        let tian_y = text_ink::top_for_center(ui, "天", &value_font, InkKind::Cjk, center_y);
        painter.text(
            egui::pos2(tian_box.left(), tian_y),
            egui::Align2::LEFT_TOP,
            "天",
            value_font.clone(),
            ink.strong,
        );
        digits::draw_bottom(
            &painter,
            time_box.bottom(),
            time_box.center().x,
            time_text,
            &digit_font,
            ink.strong,
        );
    }

    // 第三行：到期日；前缀中文与数字各自字墨中线对同一行中心
    let date_top = third_top + line3 + line_gap;
    let date_h =
        crate::widgets::text_metrics::height(ui.ctx(), &date_prefix_font, InkKind::Cjk).max(
            crate::widgets::text_metrics::height(ui.ctx(), &date_digit_font, InkKind::Digits),
        );
    let date_center = date_top + date_h / 2.0;
    painter.text(
        egui::pos2(
            rect.left() + pad_x,
            text_ink::top_for_center(
                ui,
                date_prefix,
                &date_prefix_font,
                InkKind::Cjk,
                date_center,
            ),
        ),
        egui::Align2::LEFT_TOP,
        date_prefix,
        date_prefix_font,
        ink.meta,
    );
    digits::draw_center(
        &painter,
        date_center,
        rect.left() + pad_x + date_prefix_w + date_value_w / 2.0,
        &date_value,
        &date_digit_font,
        ink.meta,
    );

    // 中国结：第一行右对齐；点击恢复默认春节
    let icon_center = egui::pos2(rect.right() - pad_x - text_gap, top + line_h / 2.0);
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
#[allow(clippy::too_many_arguments)]
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
    let date_font = digits::digit_font(crate::theme::font_size("meta"));
    let name_ink = text_ink::measure(ui, name, &name_font, InkKind::Cjk);
    let date_text = at.format("%Y-%m-%d %H:%M").to_string();
    let date_w = digits::text_width(ui.painter(), &date_text, &date_font);
    let name_h = crate::widgets::text_metrics::height(ui.ctx(), &name_font, InkKind::Cjk);
    let time_font = digits::digit_font(crate::theme::font_size("section"));
    let time_h = crate::widgets::text_metrics::height(ui.ctx(), &time_font, InkKind::Digits);
    let tian_font = crate::theme::font_semibold(crate::theme::font_size("section"));
    let tian_h = crate::widgets::text_metrics::height(ui.ctx(), &tian_font, InkKind::Cjk);
    let pad_x = crate::theme::layout_f32("card_pad_x");
    let pad_y = crate::theme::layout_f32("card_pad_y");
    let icon_size = crate::theme::layout_f32("card_icon_size");
    let focus_size = crate::theme::layout_f32("card_focus_size");
    let focus_radius = crate::theme::layout_f32("card_focus_radius");
    let focus_stroke = crate::theme::layout_f64("card_focus_stroke");
    let border = crate::theme::layout_f64("card_border");
    let trash_gap = crate::theme::layout_f32("card_trash_gap");
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
            border,
            crate::widgets::motion::lerp_color(
                crate::theme::border_color(),
                crate::theme::border_strong_color(),
                hover_t,
            ),
        ),
        egui::StrokeKind::Inside,
    );
    let name_box = egui::Rect::from_min_size(
        egui::pos2(rect.left() + pad_x, rect.top() + pad_y),
        egui::vec2(rect.width() - 2.0 * pad_x, name_h),
    );
    crate::layout_probe::rect(ui, "cd.row", rect);
    crate::layout_probe::text(
        ui,
        "cd.row_name",
        name,
        &name_font,
        InkKind::Cjk,
        egui::pos2(name_box.left(), name_ink.top_for_bottom(name_box.bottom())),
        egui::Align2::LEFT_TOP,
    );
    ui.painter().text(
        egui::pos2(name_box.left(), name_ink.top_for_bottom(name_box.bottom())),
        egui::Align2::LEFT_TOP,
        name,
        name_font,
        ink.body,
    );
    let value_center = rect.center().y;
    let date_bottom = rect.bottom() - pad_y;
    digits::draw_bottom(
        ui.painter(),
        date_bottom,
        rect.left() + pad_x + date_w / 2.0,
        &date_text,
        &date_font,
        ink.meta,
    );
    if crate::layout_probe::layout_on() {
        let y = text_ink::measure(ui, &date_text, &date_font, InkKind::Digits)
            .top_for_bottom(date_bottom);
        crate::layout_probe::ink(
            ui,
            "cd.row_date_ink",
            &date_text,
            &date_font,
            egui::pos2(rect.left() + pad_x, y),
            egui::Align2::LEFT_TOP,
        );
    }
    let (text, ended) = format_remaining(at, now);
    let value_right = rect.right() - pad_x - icon_size - trash_gap;
    if ended {
        let ended_w = ui
            .painter()
            .layout_no_wrap(text.clone(), tian_font.clone(), ink.danger)
            .rect
            .width();
        let ended_box = egui::Rect::from_min_size(
            egui::pos2(value_right - ended_w, value_center - tian_h / 2.0),
            egui::vec2(ended_w, tian_h),
        );
        let y = text_ink::top_for_center(ui, &text, &tian_font, InkKind::Cjk, ended_box.center().y);
        ui.painter().text(
            egui::pos2(ended_box.left(), y),
            egui::Align2::LEFT_TOP,
            &text,
            tian_font.clone(),
            ink.danger,
        );
    } else {
        let (day_text, time_text) = text
            .split_once('天')
            .map(|(d, t)| (d, t.trim()))
            .unwrap_or((text.as_str(), ""));
        let gap = crate::theme::layout_f32("countdown_unit_gap");
        let day_template_w = digits::text_width(ui.painter(), "000", &time_font);
        let day_actual_w = digits::text_width(ui.painter(), day_text, &time_font);
        let time_w = digits::text_width(ui.painter(), "00:00:00", &time_font);
        let tian_w = ui
            .painter()
            .layout_no_wrap("天".to_string(), tian_font.clone(), ink.muted)
            .rect
            .width();
        let time_box = egui::Rect::from_min_size(
            egui::pos2(value_right - time_w, value_center - time_h / 2.0),
            egui::vec2(time_w, time_h),
        );
        let tian_box = egui::Rect::from_min_size(
            egui::pos2(time_box.left() - gap - tian_w, value_center - tian_h / 2.0),
            egui::vec2(tian_w, tian_h),
        );
        let day_box = egui::Rect::from_min_size(
            egui::pos2(
                tian_box.left() - gap - day_template_w,
                value_center - time_h / 2.0,
            ),
            egui::vec2(day_template_w, time_h),
        );
        digits::draw_bottom(
            ui.painter(),
            day_box.bottom(),
            day_box.right() - day_actual_w / 2.0,
            day_text,
            &time_font,
            ink.muted,
        );
        if crate::layout_probe::layout_on() {
            let y = text_ink::measure(ui, day_text, &time_font, InkKind::Digits)
                .top_for_bottom(day_box.bottom());
            crate::layout_probe::ink(
                ui,
                "cd.row_day_ink",
                day_text,
                &time_font,
                egui::pos2(day_box.right() - day_actual_w, y),
                egui::Align2::LEFT_TOP,
            );
        }
        let tian_y =
            text_ink::top_for_center(ui, "天", &tian_font, InkKind::Cjk, tian_box.center().y);
        ui.painter().text(
            egui::pos2(tian_box.left(), tian_y),
            egui::Align2::LEFT_TOP,
            "天",
            tian_font,
            ink.muted,
        );
        if crate::layout_probe::layout_on() {
            crate::layout_probe::ink(
                ui,
                "cd.row_tian_ink",
                "天",
                &crate::theme::font_semibold(crate::theme::font_size("body")),
                egui::pos2(tian_box.left(), tian_y),
                egui::Align2::LEFT_TOP,
            );
        }
        digits::draw_bottom(
            ui.painter(),
            time_box.bottom(),
            time_box.center().x,
            time_text,
            &time_font,
            ink.muted,
        );
        if crate::layout_probe::layout_on() {
            let y = text_ink::measure(ui, time_text, &time_font, InkKind::Digits)
                .top_for_bottom(time_box.bottom());
            crate::layout_probe::ink(
                ui,
                "cd.row_time_ink",
                time_text,
                &time_font,
                egui::pos2(time_box.left(), y),
                egui::Align2::LEFT_TOP,
            );
        }
    }
    // 删除：Trash + 两步武装（Check），常显但低对比，悬停转 danger
    let del_key = format!("cd:{name}");
    let armed = matches!(&state.armed_delete, Some((k, _)) if *k == del_key);
    let del_box = egui::Rect::from_min_size(
        egui::pos2(
            rect.right() - pad_x - icon_size,
            rect.center().y - icon_size / 2.0,
        ),
        egui::vec2(icon_size, icon_size),
    );
    let del = ui.interact(
        del_box,
        egui::Id::unique(("cd_del", name)),
        egui::Sense::click(),
    );
    icons::draw(
        ui.painter(),
        del_box.center(),
        icon_size,
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
        let focus_box =
            egui::Rect::from_center_size(del_box.center(), egui::vec2(focus_size, focus_size));
        ui.painter().rect_stroke(
            focus_box,
            egui::epaint::CornerRadius::same(focus_radius as u8),
            egui::Stroke::new(focus_stroke, ink.knob),
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

/// 目标卡高度：上下 padding + 第一列（名称 + 行距 + 日期）；时间/垃圾桶列不参与
fn target_row_height(ui: &egui::Ui) -> f32 {
    let name_font = crate::theme::font_medium(crate::theme::font_size("body"));
    let date_font = digits::digit_font(crate::theme::font_size("meta"));
    let name_h = crate::widgets::text_metrics::height(ui.ctx(), &name_font, InkKind::Cjk);
    let date_h = crate::widgets::text_metrics::height(ui.ctx(), &date_font, InkKind::Digits);
    2.0 * crate::theme::layout_f32("card_pad_y")
        + name_h
        + crate::theme::layout_f32("card_line_gap")
        + date_h
}

/// 页脚高度：字段实际高度 + footer_gap + 按钮实际高度 + footer_bottom
fn footer_height(ui: &egui::Ui) -> f32 {
    crate::widgets::field::fixed_height(ui)
        + crate::theme::layout_f32("footer_gap")
        + crate::widgets::primary_button::height(ui)
        + crate::theme::layout_f32("footer_bottom")
}

/// 页脚：名称 + 时刻 + 添加；固定盒布局；时刻错误显示在标签行右端
fn draw_footer(
    ui: &mut egui::Ui,
    state: &mut crate::bus::UiState,
    bus: &crate::bus::Bus,
    now: &NaiveDateTime,
    targets: &[(String, NaiveDateTime)],
    time_error: &mut bool,
) {
    let ink = crate::theme::ink();
    let avail = ui.available_width();
    let gap = crate::theme::layout_f32("footer_gap");
    let button_w = crate::theme::layout_f32("button_width");
    // 名称:时刻 = 4:6（间距 10 从总宽扣除）
    let name_w = ((avail - gap) * 0.4).max(80.0);
    let at_w = ((avail - gap) * 0.6).max(80.0);
    ui.spacing_mut().item_spacing.y = 0.0;
    let draft_at_empty = state.countdown_draft_at.is_empty();
    let draft_name_empty = state.countdown_draft_name.is_empty();
    let (err_msg, local_at) = {
        let name = state.countdown_draft_name.clone();
        match crate::tools::countdown::parse_target_text(&state.countdown_draft_at, *now) {
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
                "时刻无法解析——格式应为 YYYY / YYYY-M / YYYY-M-D / M-D / YYYY-M-D HH:MM[:SS]"
                    .to_string(),
                None,
            ),
        }
    };
    let mut do_add = false;
    let mut name_resp: Option<egui::Response> = None;
    let mut at_resp: Option<egui::Response> = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        name_resp = Some(field::show_fixed(
            ui,
            "目标名称",
            "例如 项目发布",
            &mut state.countdown_draft_name,
            name_w,
        ));
        ui.add_space(gap);
        at_resp = Some(field::show_fixed(
            ui,
            "目标时刻",
            "YYYY-MM-DD HH:MM",
            &mut state.countdown_draft_at,
            at_w,
        ));
    });
    if name_resp.as_ref().is_some_and(|r| r.changed())
        || at_resp.as_ref().is_some_and(|r| r.changed())
    {
        *time_error = false;
    }
    ui.add_space(gap);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space((avail - button_w).max(0.0));
        do_add = crate::widgets::primary_button::show(ui, "添加", button_w).clicked();
    });
    // 回车：单行 TextEdit 会先 lost_focus，所以按 lost_focus + Enter 判断
    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        if at_resp.as_ref().is_some_and(|r| r.lost_focus()) {
            do_add = true;
        } else if name_resp.as_ref().is_some_and(|r| r.lost_focus()) {
            if state.countdown_draft_at.trim().is_empty() {
                if let Some(resp) = &at_resp {
                    resp.request_focus();
                }
            } else {
                do_add = true;
            }
        }
    }
    if let Some(resp) = name_resp
        && state.focus_countdown_name
    {
        resp.request_focus();
        state.focus_countdown_name = false;
    }
    if do_add {
        *time_error = local_at.is_none();
    }
    if do_add && local_at.is_none() && !err_msg.is_empty() {
        tracing::warn!("倒计时添加失败（静默）: {err_msg}");
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
    if *time_error {
        let font = crate::theme::font_semibold(crate::theme::font_size("meta"));
        let label_h = crate::widgets::text_metrics::height(ui.ctx(), &font, InkKind::Cjk);
        let rect = ui.max_rect();
        let err_ink = text_ink::measure(ui, "输入错误！", &font, InkKind::Cjk);
        ui.painter().text(
            egui::pos2(rect.right(), err_ink.top_for_bottom(rect.top() + label_h)),
            egui::Align2::RIGHT_TOP,
            "输入错误！",
            font,
            ink.danger,
        );
    }
}

// ===== src/pages/countdown.rs 说明区 =====
// 输入：UiState 的目标快照 + 草稿；输出：固定头（hero/节日）+ 目标列表 + 固定 Footer
// 逻辑步骤：目标快照 → 固定头（hero 默认春节/选中节日 + 近期节日）→
//   列表视口（仅目标行，剩余右对齐、Trash 两步确认；不足一张卡整块隐藏）→
//   Footer（名称/时刻/添加；时刻解析失败或校验失败时在标签行右端显示「输入错误！」）
// 设计理由（PL017/PL022）：hero 与节日固定在上方，滚动只服务目标卡——抽屉窄，
//   整页滚会把 hero 滚走；列表视口独立后滑杆/渐隐/三角只贴卡片区；空态不建 ScrollArea
// 异常处理：空草稿不产红字；DST 不存在不提交；输入错误内联显示；hero 空有兜底
// 关联依赖：tools::countdown/chinese_calendar/widgets(field/digits/empty/scroll_kit)；
//   消费方：pages::Pages 抽屉
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Duration};

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
