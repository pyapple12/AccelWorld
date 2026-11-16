//! 底部仪器坞（PL017）：居中浮动 pill + 五页图标/标签 + 设置齿轮。
//! 选中态 = ink + 2px 金下划线（不填金块）；窄窗隐藏标签；键盘可聚焦。

use egui::{Align2, Id, Rect, Sense, Stroke};

use super::icons::{self, Icon};
use crate::bus::{PageId, UiState};

/// 坞内一项（页面 + 图标 + 标签；由 window.rs 用既有页→图标映射构造）
pub struct DockItem {
    pub page: PageId,
    pub icon: Icon,
    pub label: &'static str,
}

/// 绘制坞；直接改 `state.current_page` / `state.settings_open`。
pub fn show(ui: &mut egui::Ui, state: &mut UiState, items: &[DockItem]) {
    let ink = crate::theme::ink();
    let avail = ui.max_rect();
    // 文字是否保留只看坞高：lg 保留图标+文字，sm 只留图标
    let compact = avail.height() < crate::theme::layout_f32("dock_height_lg");
    // C2 固定在全窗居中，不随抽屉开合移动；抽屉只影响 B1–B18 页面内容
    let pill_w = (crate::theme::layout_f32("dock_pill_width"))
        .min(avail.width() - 24.0)
        .max(280.0);
    let pill_h = avail
        .height()
        .max(crate::theme::layout_f32("dock_pill_min_h"));
    let pill_w = (pill_w * (pill_h / crate::theme::layout_f32("dock_height_lg"))).max(280.0);
    let pill = Rect::from_center_size(avail.center(), egui::vec2(pill_w, pill_h));
    ui.painter().rect_filled(
        pill,
        egui::epaint::CornerRadius::same(16),
        crate::theme::card_bg(),
    );
    ui.painter().rect_stroke(
        pill,
        egui::epaint::CornerRadius::same(16),
        Stroke::new(1.0, crate::theme::border_strong_color()),
        egui::StrokeKind::Inside,
    );

    // 设置区在右端；1px 分隔线；五页均分剩余
    let gear_w = 52.0;
    let inner_left = pill.left() + 10.0;
    let inner_right = pill.right() - 10.0;
    let items_w = (inner_right - gear_w - inner_left).max(100.0);
    let item_w = items_w / items.len().max(1) as f32;
    let mut chosen: Option<PageId> = None;
    let mut toggle_settings = false;

    for (i, item) in items.iter().enumerate() {
        let rect = Rect::from_min_size(
            egui::pos2(inner_left + i as f32 * item_w, pill.top() + 2.0),
            egui::vec2(item_w, pill.height() - 4.0),
        );
        let resp = ui.interact(
            rect,
            Id::unique(("dock_item", page_key(item.page))),
            Sense::click(),
        );
        let active = state.current_page == item.page && !state.settings_open;
        let icon_box = draw_item(
            ui,
            rect,
            item.icon,
            item.label,
            active,
            resp.hovered() || resp.has_focus(),
            compact,
            ink.strong,
            ink.muted,
        );
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(3.0),
                egui::epaint::CornerRadius::same(10),
                Stroke::new(1.5, ink.knob),
                egui::StrokeKind::Inside,
            );
        }
        let keyboard = resp.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if compact && (resp.hovered() || resp.has_focus()) {
            show_icon_tooltip(
                ui,
                egui::Id::unique(("dock_tip", page_key(item.page))),
                item.label,
                icon_box.center().x,
                pill.top(),
            );
        }
        if resp.clicked() || keyboard {
            chosen = Some(item.page);
        }
    }

    // 分隔线 + 设置
    let sep_x = pill.right() - gear_w - 8.0;
    ui.painter().line_segment(
        [
            egui::pos2(sep_x, pill.top() + 14.0),
            egui::pos2(sep_x, pill.bottom() - 14.0),
        ],
        Stroke::new(1.0, crate::theme::border_color()),
    );
    let gear_rect = Rect::from_min_size(
        egui::pos2(sep_x + 8.0, pill.top() + 2.0),
        egui::vec2(gear_w - 8.0, pill.height() - 4.0),
    );
    let gear_resp = ui.interact(gear_rect, Id::unique("dock_settings"), Sense::click());
    let gear_icon_box = draw_item(
        ui,
        gear_rect,
        Icon::Gear,
        "设置",
        state.settings_open,
        gear_resp.hovered() || gear_resp.has_focus(),
        compact,
        ink.strong,
        ink.muted,
    );
    if gear_resp.has_focus() {
        ui.painter().rect_stroke(
            gear_rect.shrink(3.0),
            egui::epaint::CornerRadius::same(10),
            Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    let gear_keyboard = gear_resp.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let gear_resp = gear_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if compact && (gear_resp.hovered() || gear_resp.has_focus()) {
        show_icon_tooltip(
            ui,
            egui::Id::unique("dock_tip_settings"),
            "设置",
            gear_icon_box.center().x,
            pill.top(),
        );
    }
    if gear_resp.clicked() || gear_keyboard {
        toggle_settings = true;
    }
    if let Some(page) = chosen {
        state.current_page = page;
        state.settings_open = false;
    }
    if toggle_settings {
        state.settings_open = !state.settings_open;
    }
    // 选中下划线：在项间滑动（220ms）；页面内容仍瞬时切换
    let active_x = if state.settings_open {
        gear_rect.center().x
    } else {
        let idx = items
            .iter()
            .position(|it| it.page == state.current_page)
            .unwrap_or(0);
        inner_left + (idx as f32 + 0.5) * item_w
    };
    let x = crate::widgets::motion::anim(
        ui.ctx(),
        egui::Id::unique("dock_underline_x"),
        active_x,
        crate::theme::motion_ms("dock_underline_ms"),
    );
    ui.painter().rect_filled(
        Rect::from_min_size(
            egui::pos2(x - 12.0, pill.bottom() - 4.0),
            egui::vec2(24.0, 2.0),
        ),
        egui::epaint::CornerRadius::same(1),
        crate::theme::accent_color(),
    );
    ui.allocate_space(egui::vec2(avail.width(), avail.height()));
}

/// 单项绘制：图标 + 标签（窄窗隐藏）+ 选中下划线。
#[allow(clippy::too_many_arguments)]
fn draw_item(
    ui: &egui::Ui,
    rect: Rect,
    icon: Icon,
    label: &str,
    active: bool,
    highlighted: bool,
    compact: bool,
    strong: egui::Color32,
    muted: egui::Color32,
) -> Rect {
    if highlighted {
        ui.painter().rect_filled(
            rect.shrink(2.0),
            egui::epaint::CornerRadius::same(10),
            crate::theme::hover_bg(),
        );
    }
    let color = if active { strong } else { muted };
    let label_font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let label_h = super::text_metrics::height(ui.ctx(), &label_font, super::text_ink::InkKind::Cjk);
    let icon_size = if compact { 22.0 } else { 20.0 };
    let icon_gap = crate::theme::layout_f32("dock_icon_label_gap");
    let show_label = !compact;
    let group_h = if show_label {
        icon_size + icon_gap + label_h
    } else {
        icon_size
    };
    let group_top = rect.center().y - group_h / 2.0;
    let icon_box = egui::Rect::from_min_size(
        egui::pos2(rect.center().x - icon_size / 2.0, group_top),
        egui::vec2(icon_size, icon_size),
    );
    icons::draw(ui.painter(), icon_box.center(), icon_size, icon, color);
    if show_label {
        let label_top = group_top + icon_size + icon_gap;
        let label_box = egui::Rect::from_min_size(
            egui::pos2(rect.min.x, label_top),
            egui::vec2(rect.width(), label_h),
        );
        let label_ink =
            super::text_ink::measure(ui, label, &label_font, super::text_ink::InkKind::Cjk);
        ui.painter().text(
            egui::pos2(
                label_box.center().x,
                label_ink.top_for_bottom(label_box.bottom()),
            ),
            Align2::CENTER_TOP,
            label,
            label_font,
            color,
        );
    }
    icon_box
}

/// 矮窗工具提示：与日期 tooltip 同款卡片；底边贴坞顶 −2，水平中心贴图标盒中心
fn show_icon_tooltip(ui: &egui::Ui, id: egui::Id, label: &str, center_x: f32, dock_top: f32) {
    let font = crate::theme::font_medium(crate::theme::font_size("meta"));
    let (size, ink) = crate::widgets::tooltip::measure(ui, label, &font);
    let left = center_x - size.x / 2.0;
    let top = dock_top - 2.0 - size.y;
    let ctx = ui.ctx().clone();
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .fixed_pos(egui::pos2(left, top))
        .show(&ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            crate::widgets::tooltip::paint(ui, label, &font, rect, ink);
        });
}

/// 页 id → 稳定字符串键（PageId 未实现 Hash；用于 widget Id）
fn page_key(page: PageId) -> &'static str {
    match page {
        PageId::Clock => "clock",
        PageId::Countdown => "countdown",
        PageId::World => "world",
        PageId::Weather => "weather",
        PageId::Alarm => "alarm",
    }
}

// ===== src/widgets/dock.rs 说明区 =====
// 输入：Ui + UiState + 五项（页面/图标/标签）；输出：状态变更（页面/设置开关）
// 逻辑步骤：居中 pill（560×56，card_bg + border_strong）→ 五页均分 → 每项
//   图标 20 + 3 + 标签 11.5（整组中线 = 项中线）+ 悬停底 + 焦点环 + 选中金下划线 → 右端 1px 分隔 + 齿轮
//   全高有标签不弹 tooltip；矮窗仅图标时 hover/focus 在图标盒上方居中弹标签
// 设计理由（PL017）：原 dock 左对齐 5 个无标签图标、选中填金块、齿轮隔 1500px；
//   改为居中带标签 pill，选中用 ink + 下划线（金只做数据），窄窗自动隐藏标签
// 异常处理：无 panic 面；items 为空时只画齿轮（调用方恒传五页）
// 关联依赖：bus(PageId/UiState)/icons/theme；消费方：window.rs 底栏
