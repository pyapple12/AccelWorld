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
            egui::pos2(avail.left(), body_rect.bottom() + 10.0),
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

        if header_h > 0.5 {
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
        if targets.is_empty() {
            // 空态与滚动无关：不建 ScrollArea，CTA + 虚线框静态占列表矩形
            if list_rect.height() > 0.5 {
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
        } else if list_rect.height() + 0.5 >= row_h {
            // 兜底通过：列表视口只装目标卡；滑杆/渐隐/三角贴视口，不跨固定头与页脚
            ui.scope_builder(egui::UiBuilder::new().max_rect(list_rect), |ui| {
                crate::widgets::scroll_kit::apply_style(ui);
                let mut area = egui::ScrollArea::vertical()
                    .id_salt("countdown_list")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
                if let Some(y) = self.scroll_override.take() {
                    area = area.vertical_scroll_offset(y);
                }
                let out = area.show(ui, |ui| {
                    let mut stack = crate::widgets::box_stack::BoxStack::new(ui);
                    let pulse = crate::widgets::motion::list_pulse(state, "countdown");
                    let last = targets.len().saturating_sub(1);
                    for (i, (name, at)) in targets.iter().enumerate() {
                        let rect = stack.next(ui, row_h);
                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            if i == last && pulse < 1.0 {
                                ui.set_opacity(0.55 + 0.45 * pulse);
                            }
                            draw_target_row(ui, rect, state, bus, name, *at, now, &ink);
                        });
                    }
                });
                self.scroll_override = draw_list_scroll(
                    ui,
                    &out,
                    list_rect,
                    &mut self.scroll_last_offset,
                    &mut self.scroll_blink_until,
                );
            });
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
    let label = crate::theme::font_medium(10.0);
    let name = crate::theme::font_semibold(12.0);
    crate::widgets::text_metrics::height(ui.ctx(), &label, InkKind::Cjk).max(
        crate::widgets::text_metrics::height(ui.ctx(), &name, InkKind::Cjk),
    )
}

/// hero 盒高：三行固定行高 + 10 间距；全部来自启动指标表
fn hero_height(ui: &egui::Ui) -> f32 {
    let remain_font = crate::theme::font_regular(28.0);
    let value_font = crate::theme::font_semibold(28.0);
    let date_font = crate::theme::font_regular(10.0);
    let name_h = hero_name_line_h(ui);
    let line3 = crate::widgets::text_metrics::height(ui.ctx(), &remain_font, InkKind::Cjk)
        .max(crate::widgets::text_metrics::height(
            ui.ctx(),
            &value_font,
            InkKind::Cjk,
        ))
        .max(crate::widgets::text_metrics::height(
            ui.ctx(),
            &value_font,
            InkKind::Digits,
        ));
    let date_h = crate::widgets::text_metrics::height(ui.ctx(), &date_font, InkKind::Cjk);
    10.0 + name_h + 10.0 + line3 + 10.0 + date_h + 10.0
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

    // 第一行：固定行高，两个字号共用同一条基线
    let line_h = hero_name_line_h(ui);
    let top = rect.top() + 10.0;
    let line_bottom = top + line_h;
    let name_y = name_ink.top_for_bottom(line_bottom);
    let label_y = label_ink.top_for_bottom(line_bottom);
    let label_w = ui
        .painter()
        .layout_no_wrap("下一个节日".to_string(), label_font.clone(), ink.muted)
        .rect
        .width();
    painter.text(
        egui::pos2(rect.left() + 10.0, label_y),
        egui::Align2::LEFT_TOP,
        "下一个节日",
        label_font,
        crate::theme::accent_color(),
    );
    painter.text(
        egui::pos2(rect.left() + 10.0 + label_w + 8.0, name_y),
        egui::Align2::LEFT_TOP,
        name,
        name_font,
        ink.strong,
    );

    // 第二行：还剩（28 常规 + accent） + 剩余（28 粗体 + strong），固定中心线
    let third_top = top + line_h + 10.0;
    let line3 = crate::widgets::text_metrics::height(
        ui.ctx(),
        &crate::theme::font_regular(28.0),
        InkKind::Cjk,
    )
    .max(crate::widgets::text_metrics::height(
        ui.ctx(),
        &crate::theme::font_semibold(28.0),
        InkKind::Cjk,
    ))
    .max(crate::widgets::text_metrics::height(
        ui.ctx(),
        &crate::theme::font_semibold(28.0),
        InkKind::Digits,
    ));
    let center_y = third_top + line3 / 2.0;
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
        digits::draw_centered_ink(
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
        digits::draw_centered_ink(
            &painter,
            egui::pos2(time_x + time_w / 2.0, center_y),
            time_text,
            &value_font,
            ink.strong,
        );
    }

    // 第三行：到期日（盒内墨迹底贴盒底）
    let date_top = third_top + line3 + 10.0;
    let date_h = crate::widgets::text_metrics::height(ui.ctx(), &date_font, InkKind::Cjk);
    let date_bottom = date_top + date_h;
    painter.text(
        egui::pos2(
            rect.left() + 10.0,
            date_bottom - date_ink.h - date_ink.top_pad,
        ),
        egui::Align2::LEFT_TOP,
        &date_text,
        date_font,
        ink.meta,
    );

    // 中国结：第一行右对齐；点击恢复默认春节
    let icon_center = egui::pos2(rect.right() - 10.0 - 8.0, top + line_h / 2.0);
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
    let date_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let name_ink = text_ink::measure(ui, name, &name_font, InkKind::Cjk);
    let date_text = at.format("%Y-%m-%d %H:%M").to_string();
    let date_ink = text_ink::measure(ui, &date_text, &date_font, InkKind::Cjk);
    let name_h = crate::widgets::text_metrics::height(ui.ctx(), &name_font, InkKind::Cjk);
    let date_h = crate::widgets::text_metrics::height(ui.ctx(), &date_font, InkKind::Cjk);
    let time_font = digits::digit_font(crate::theme::font_size("time_sm"));
    let time_h = crate::widgets::text_metrics::height(ui.ctx(), &time_font, InkKind::Digits);
    let line2_h = date_h.max(time_h);
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
    let name_box = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 14.0, rect.top() + 10.0),
        egui::vec2(rect.width() - 28.0, name_h),
    );
    ui.painter().text(
        egui::pos2(name_box.left(), name_ink.top_for_bottom(name_box.bottom())),
        egui::Align2::LEFT_TOP,
        name,
        name_font,
        ink.body,
    );
    let line2_top = name_box.bottom() + 5.0;
    let line2_bottom = line2_top + line2_h;
    let line2_center = line2_top + line2_h / 2.0;
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, date_ink.top_for_bottom(line2_bottom)),
        egui::Align2::LEFT_TOP,
        &date_text,
        date_font,
        ink.meta,
    );
    let (text, ended) = format_remaining(at, now);
    let value_right = rect.right() - 36.0;
    if ended {
        let w = digits::text_width(ui.painter(), &text, &time_font);
        digits::draw_centered(
            ui.painter(),
            egui::pos2(value_right - w / 2.0, line2_center),
            &text,
            &time_font,
            ink.danger,
        );
    } else {
        let (day_text, time_text) = text
            .split_once('天')
            .map(|(d, t)| (d, t.trim()))
            .unwrap_or((text.as_str(), ""));
        let gap = 3.0;
        let day_template_w = digits::text_width(ui.painter(), "000", &time_font);
        let day_actual_w = digits::text_width(ui.painter(), day_text, &time_font);
        let time_w = digits::text_width(ui.painter(), "00:00:00", &time_font);
        let tian_font = crate::theme::font_semibold(crate::theme::font_size("time_sm"));
        let tian_w = ui
            .painter()
            .layout_no_wrap("天".to_string(), tian_font.clone(), ink.muted)
            .rect
            .width();
        let tian_ink = text_ink::measure(ui, "天", &tian_font, InkKind::Cjk);
        let value_w = day_template_w + gap + tian_w + gap + time_w;
        let start = value_right - value_w;
        let day_x = start + day_template_w - day_actual_w;
        digits::draw_centered(
            ui.painter(),
            egui::pos2(day_x + day_actual_w / 2.0, line2_center),
            day_text,
            &time_font,
            ink.muted,
        );
        let tian_x = start + day_template_w + gap;
        ui.painter().text(
            egui::pos2(tian_x, tian_ink.top_for_bottom(line2_bottom)),
            egui::Align2::LEFT_TOP,
            "天",
            tian_font,
            ink.muted,
        );
        let time_x = tian_x + tian_w + gap;
        digits::draw_centered(
            ui.painter(),
            egui::pos2(time_x + time_w / 2.0, line2_center),
            time_text,
            &time_font,
            ink.muted,
        );
    }
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

/// 目标卡固定盒高：10 + 名称盒 + 5 + 第二行（日期/剩余取高）+ 10。
/// 绘制与“不足一张卡就整块隐藏”的兜底共用，避免两处各算一遍。
fn target_row_height(ui: &egui::Ui) -> f32 {
    let name_font = crate::theme::font_medium(crate::theme::font_size("body"));
    let date_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let time_font = digits::digit_font(crate::theme::font_size("time_sm"));
    let name_h = crate::widgets::text_metrics::height(ui.ctx(), &name_font, InkKind::Cjk);
    let date_h = crate::widgets::text_metrics::height(ui.ctx(), &date_font, InkKind::Cjk);
    let time_h = crate::widgets::text_metrics::height(ui.ctx(), &time_font, InkKind::Digits);
    10.0 + name_h + 5.0 + date_h.max(time_h) + 10.0
}

/// 列表滚动视觉件：上下 16px 渐隐、上下金色三角（滚动期呼吸、静止常亮）、
/// 卡片右缘 +8px 的 4px 金色浮钮（右边距 8/4/8 居中）；返回拖拽浮钮时下一帧要写回的滚动偏移。
fn draw_list_scroll(
    ui: &egui::Ui,
    out: &egui::scroll_area::ScrollAreaOutput<()>,
    list_rect: egui::Rect,
    scroll_last_offset: &mut Option<f32>,
    scroll_blink_until: &mut f64,
) -> Option<f32> {
    let viewport = out.inner_rect;
    if viewport.height() <= 0.5 {
        return None;
    }
    let offset = out.state.clamped_offset().y;
    let max_offset = (out.content_size.y - viewport.height()).max(0.0);
    let scrollable = max_offset > 1.0;
    let at_top = offset <= 0.5;
    let at_bottom = offset >= max_offset - 0.5;

    // 滚动判定：偏移变化即刷新呼吸截止；停手后回到常亮
    let now = ui.input(|i| i.time);
    let blink_s = crate::theme::motion_ms("scroll_hint_blink_ms") as f64 / 1000.0;
    if let Some(prev) = *scroll_last_offset
        && (offset - prev).abs() > 0.1
    {
        *scroll_blink_until = now + blink_s;
    }
    *scroll_last_offset = Some(offset);
    let reduce = crate::theme::reduce_motion(ui.ctx());
    let blinking = scrollable && !reduce && now < *scroll_blink_until;
    if blinking {
        ui.ctx().request_repaint();
    }
    let base_alpha = crate::theme::layout_f32("scroll_hint_alpha_x100") / 100.0;
    // 1s 呼吸：1 → 0.35 → 1；静止时保持常亮，不闪
    let hint_alpha = if blinking {
        let phase = (now * std::f64::consts::TAU).cos() as f32;
        base_alpha * (0.675 + 0.325 * phase)
    } else {
        base_alpha
    };

    let fade = crate::theme::layout_f32("scroll_fade");
    let top_t = fade_t(ui, "countdown_fade_top", scrollable && !at_top);
    let bottom_t = fade_t(ui, "countdown_fade_bottom", scrollable && !at_bottom);
    crate::widgets::scroll_kit::paint_fade(ui, viewport, fade * top_t, fade * bottom_t);
    crate::widgets::scroll_kit::paint_hints(
        ui,
        viewport,
        scrollable && !at_top,
        scrollable && !at_bottom,
        hint_alpha,
    );

    // 金色浮钮：卡片右缘 +8px（右边距 8/4/8 居中），4px 宽；拖拽命中域左右各放宽 6px
    if !scrollable {
        return None;
    }
    let gap = crate::theme::layout_f32("scroll_bar_gap");
    let width = crate::theme::layout_f32("scroll_bar_w");
    let track_h = viewport.height();
    let thumb_h = (track_h * (viewport.height() / out.content_size.y.max(1.0)))
        .max(crate::theme::layout_f32("scroll_handle_min"))
        .min(track_h);
    let travel = (track_h - thumb_h).max(0.0);
    let range = max_offset.max(1.0);
    let ratio = (offset / range).clamp(0.0, 1.0);
    let x = list_rect.right() + gap;
    let hit = egui::Rect::from_min_max(
        egui::pos2(x - 6.0, viewport.top()),
        egui::pos2(x + width + 6.0, viewport.bottom()),
    );
    let resp = ui
        .interact(
            hit,
            egui::Id::unique("countdown_scroll_thumb"),
            egui::Sense::drag(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let mut thumb_ratio = ratio;
    let mut next_override = None;
    if resp.dragged() && travel > 0.0 {
        let dy = ui.input(|i| i.pointer.delta().y);
        let new_offset = (offset + dy / travel * max_offset).clamp(0.0, max_offset);
        thumb_ratio = (new_offset / range).clamp(0.0, 1.0);
        next_override = Some(new_offset);
    }
    let y = viewport.top() + thumb_ratio * travel;
    let radius = ((width / 2.0).max(1.0)) as u8;
    ui.painter().rect_filled(
        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, thumb_h)),
        egui::epaint::CornerRadius::same(radius),
        crate::theme::accent_color(),
    );
    next_override
}

/// 渐隐显隐动画：出现走 scroll_fade_ms，收走边走 scroll_edge_ms；减少动效直接落位。
fn fade_t(ui: &egui::Ui, id: &str, visible: bool) -> f32 {
    let target = if visible { 1.0 } else { 0.0 };
    if crate::theme::reduce_motion(ui.ctx()) {
        return target;
    }
    let ms = if visible {
        crate::theme::motion_ms("scroll_fade_ms")
    } else {
        crate::theme::motion_ms("scroll_edge_ms")
    };
    ui.ctx()
        .animate_value_with_time(egui::Id::unique(id), target, ms / 1000.0)
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
        ui.painter().text(
            egui::pos2(rect.right(), rect.top() + label_h / 2.0),
            egui::Align2::RIGHT_CENTER,
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
