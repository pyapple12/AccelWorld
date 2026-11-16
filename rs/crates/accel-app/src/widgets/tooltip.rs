//! 单行 tooltip 卡片：日期 tooltip 与坞 tooltip 共用同一套盒子、内边距与颜色。
//! 卡片只负责底色/边框/圆角，文字仍按 CJK 盒底 + dy 定位。

use egui::{Color32, FontId, Rect, Sense, Ui};

use super::text_ink::{self, Ink, InkKind};

/// 量单行卡片：文字宽 + 左右 pad；高 = 文字盒高 + 上下 pad
pub fn measure(ui: &Ui, text: &str, font: &FontId) -> (egui::Vec2, Ink) {
    let ink = text_ink::measure(ui, text, font, InkKind::Cjk);
    let pad_x = crate::theme::layout_f32("tooltip_pad_x");
    let pad_y = crate::theme::layout_f32("tooltip_pad_y");
    let w = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
        .rect
        .width();
    (egui::vec2(w + pad_x * 2.0, ink.h + pad_y * 2.0), ink)
}

/// 画卡片底板：card_bg + border_strong + radius.sm
pub fn frame(ui: &Ui, rect: Rect) {
    let radius = crate::theme::radius_px("sm") as u8;
    let p = ui.painter();
    p.rect_filled(
        rect,
        egui::epaint::CornerRadius::same(radius),
        crate::theme::card_bg(),
    );
    p.rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(radius),
        egui::Stroke::new(1.0, crate::theme::border_strong_color()),
        egui::StrokeKind::Inside,
    );
}

/// 画单行卡片与文字：文字盒底贴 pad 后的行盒底
pub fn paint(ui: &Ui, text: &str, font: &FontId, rect: Rect, ink: Ink) {
    frame(ui, rect);
    let pad_x = crate::theme::layout_f32("tooltip_pad_x");
    let pad_y = crate::theme::layout_f32("tooltip_pad_y");
    let row_bottom = rect.top() + pad_y + ink.h;
    ui.painter().text(
        egui::pos2(rect.left() + pad_x, ink.top_for_bottom(row_bottom)),
        egui::Align2::LEFT_TOP,
        text,
        font.clone(),
        crate::theme::ink().muted,
    );
}

/// 固定位置画一张单行卡片：调用方只给左上角
pub fn show_at(ui: &Ui, id: egui::Id, text: &str, font: &FontId, left: f32, top: f32) {
    let (size, ink) = measure(ui, text, font);
    let ctx = ui.ctx().clone();
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .fixed_pos(egui::pos2(left, top))
        .show(&ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            paint(ui, text, font, rect, ink);
        });
}

// ===== 说明区 =====
// 输入：Ui + 文案 + 字体；输出：卡片尺寸 + 绘制
// 逻辑：文字宽 + tooltip_pad_x×2；高 = h_cjk + tooltip_pad_y×2；文字按盒底 + dy
// 设计理由：日期与坞 tooltip 共用一套外观，避免原生 Tooltip 的全局样式
// 异常处理：无 panic 面；空文案按 0 宽处理
// 关联配置：ui.json layout.tooltip_pad_x/y、radius.sm、colors.card_bg/border_strong
