//! 抽屉页头（PL017）：56px 标题 + 可选副标题 + 右端关闭；返回 true 表示点击关闭。
//! 统一页面名与关闭出口，替代原先“内容即页面、只能点 dock 回时钟”的导航缺口。

use egui::{Align2, Sense, Stroke};

use super::icons::{self, Icon};

/// 绘制页头；`subtitle` 为空时标题垂直居中。返回 close 是否被激活。
pub fn show(ui: &mut egui::Ui, title: &str, subtitle: Option<&str>) -> bool {
    let ink = crate::theme::ink();
    let height = crate::theme::page_header_height();
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());

    let title_font = crate::theme::font_semibold(crate::theme::font_size("title"));
    let title_y = if subtitle.is_some() {
        rect.top() + 10.0
    } else {
        rect.center().y - 10.0
    };
    ui.painter().text(
        egui::pos2(rect.left() + 20.0, title_y),
        Align2::LEFT_TOP,
        title,
        title_font,
        ink.strong,
    );
    if let Some(sub) = subtitle {
        ui.painter().text(
            egui::pos2(rect.left() + 20.0, rect.top() + 37.0),
            Align2::LEFT_TOP,
            sub,
            crate::theme::font_regular(crate::theme::font_size("meta")),
            ink.muted,
        );
    }

    // 关闭：36px 命中域，悬停底 + 焦点环；X 图标 16px
    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 24.0, rect.center().y),
        egui::vec2(36.0, 36.0),
    );
    let resp = ui.interact(
        close_rect,
        egui::Id::unique(("page_close", title)),
        Sense::click(),
    );
    let hover = resp.hovered();
    if hover || resp.has_focus() {
        ui.painter().rect_filled(
            close_rect,
            egui::epaint::CornerRadius::same(8),
            crate::theme::hover_bg(),
        );
    }
    if resp.has_focus() {
        ui.painter().rect_stroke(
            close_rect,
            egui::epaint::CornerRadius::same(8),
            Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    icons::draw(
        ui.painter(),
        close_rect.center(),
        16.0,
        Icon::Close,
        if hover { ink.strong } else { ink.muted },
    );
    let resp = resp
        .on_hover_text("关闭")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let keyboard = resp.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let clicked = resp.clicked() || keyboard;

    // 页头底发丝（结构即信息：页面与内容的分层）
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom()),
            egui::pos2(rect.right(), rect.bottom()),
        ],
        Stroke::new(1.0, crate::theme::border_color()),
    );
    clicked
}

// ===== src/widgets/page_header.rs 说明区 =====
// 输入：Ui + 标题/副标题；输出：关闭是否激活（调用方把 current_page 置回 Clock）
// 逻辑步骤：56px 横向分配 → 标题（20/600）/副标题（12）→ 关闭 36px 命中域（X/悬停/
//   焦点环/Enter·Space）→ 底部 1px 发丝
// 设计理由（PL017）：抽屉页原先没有页面名与关闭，用户只能靠 dock 猜；页头把
//   “我在哪/怎么回去”变成结构，而不是 tooltip。关闭键用 Close 而非文本 ×，
//   与矢量图标同语言；焦点环用亮白，避免与金色数据抢含义
// 异常处理：无 panic 面；标题宽度超出时由 egui 裁剪，调用方保证短标题
// 关联依赖：theme/icons；消费方：pages::Pages 抽屉总装
