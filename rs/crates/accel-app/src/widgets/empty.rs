//! 空态区块（PL017）：保留区块框 + 标题 + 一句行动邀请；不把空态做成一行孤字。
//! 虚线仅用于空槽（app-screens §11）；正文 ≥4.5:1。

use egui::{Align2, Sense, Stroke};

/// 绘制 96px 空态框；返回整块 Response（调用方可叠加按钮）。
pub fn show(ui: &mut egui::Ui, title: &str, body: &str) -> egui::Response {
    let ink = crate::theme::ink();
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 96.0), Sense::hover());
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
    ui.painter().text(
        egui::pos2(rect.center().x, rect.center().y - 12.0),
        Align2::CENTER_CENTER,
        title,
        crate::theme::font_semibold(crate::theme::font_size("body_lg")),
        ink.body,
    );
    ui.painter().text(
        egui::pos2(rect.center().x, rect.center().y + 14.0),
        Align2::CENTER_CENTER,
        body,
        crate::theme::font_regular(crate::theme::font_size("meta")),
        ink.muted,
    );
    resp
}

// ===== src/widgets/empty.rs 说明区 =====
// 输入：Ui + 标题/说明；输出：整块 Response
// 逻辑步骤：96px 区块 → 虚线四边 → 标题 16/600 + 说明 12 muted 居中
// 设计理由（PL017）：空态保留“这块地方还在”的结构（app-screens §11），而不是
//   把一句话丢在页面顶部；虚线允许用于空槽，不用于常规卡
// 异常处理：无 panic 面
// 关联依赖：theme/border/ink；消费方：倒计时/世界/闹钟
