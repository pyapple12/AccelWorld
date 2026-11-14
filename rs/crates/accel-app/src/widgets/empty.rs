//! 空态区块（PL017）：虚线框 + 两行文字；两行各自固定盒高，整块垂直居中。
//! 虚线仅用于空槽（app-screens §11）；正文 ≥4.5:1。

use egui::{Align2, Stroke};

/// 标题盒高（16px 字号 + 防抖余量）
fn title_box_h(ui: &egui::Ui) -> f32 {
    crate::widgets::text_metrics::height(
        ui.ctx(),
        &crate::theme::font_semibold(crate::theme::font_size("body_lg")),
        crate::widgets::text_ink::InkKind::Cjk,
    )
}

/// 正文盒高（12px 字号 + 防抖余量）
fn body_box_h(ui: &egui::Ui) -> f32 {
    crate::widgets::text_metrics::height(
        ui.ctx(),
        &crate::theme::font_regular(crate::theme::font_size("meta")),
        crate::widgets::text_ink::InkKind::Cjk,
    )
}

/// 空态最小高度：标题盒 + 行间距 + 正文盒 + 上下各 empty_card_pad
pub fn min_height(ui: &egui::Ui) -> f32 {
    title_box_h(ui)
        + body_box_h(ui)
        + crate::theme::layout_f32("empty_line_gap")
        + 2.0 * crate::theme::layout_f32("empty_card_pad")
}

/// 在指定矩形内画空态框；两行盒子作为整体对准虚线框中心
pub fn card_at(ui: &egui::Ui, rect: egui::Rect, title: &str, body: &str) {
    let ink = crate::theme::ink();
    // 虚线框：四条边用 dashed_line（egui 自带 dash 采样）
    let stroke = Stroke::new(1.0, crate::theme::border_color());
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    for pair in corners.windows(2) {
        ui.painter().extend(egui::Shape::dashed_line(
            &[pair[0], pair[1]],
            stroke,
            5.0,
            4.0,
        ));
    }

    let title_font = crate::theme::font_semibold(crate::theme::font_size("body_lg"));
    let body_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let title_ink = crate::widgets::text_ink::measure(
        ui,
        title,
        &title_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let body_ink = crate::widgets::text_ink::measure(
        ui,
        body,
        &body_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );

    let title_h = title_box_h(ui);
    let body_h = body_box_h(ui);
    let gap = crate::theme::layout_f32("empty_line_gap");
    let block_h = title_h + gap + body_h;
    let block_top = rect.center().y - block_h / 2.0;

    let title_y = block_top + (title_h - title_ink.h) / 2.0 - title_ink.top_pad;
    let body_top = block_top + title_h + gap;
    let body_y = body_top + (body_h - body_ink.h) / 2.0 - body_ink.top_pad;

    ui.painter().text(
        egui::pos2(rect.center().x, title_y),
        Align2::CENTER_TOP,
        title,
        title_font,
        ink.body,
    );
    ui.painter().text(
        egui::pos2(rect.center().x, body_y),
        Align2::CENTER_TOP,
        body,
        body_font,
        ink.muted,
    );
}

// ===== src/widgets/empty.rs 说明区 =====
// 输入：Ui + 标题/说明；输出：虚线框与两行文字
// 逻辑步骤：标题盒 + empty_line_gap + 正文盒 = 内容块；块中心对准虚线框中心；
//   每行文字在各自固定盒内按墨迹中心摆放
// 设计理由（PL021 盒模型）：虚线框高度由页面剩余空间决定，文字盒位置只由盒高与
//   间距计算，不再用两行墨迹中心反推
// 异常处理：无 panic 面
// 关联依赖：theme/border/ink、text_metrics/text_ink；消费方：倒计时/世界/闹钟
