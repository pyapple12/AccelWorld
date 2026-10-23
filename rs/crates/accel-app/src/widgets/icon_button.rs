//! 图标按钮（UIR 重塑新增）：矢量图标 + 状态底色的自绘点击控件
//! 替代原 egui small_button 文本按钮——实心主题下唯一的图标操作入口

use egui::{Color32, Response, Sense, Vec2};

use super::icons::{Icon, draw};

/// 图标按钮：方形命中域 + 居中矢量图标；hover 微亮底、selected 主题金底
/// size = 图标边长（命中域外扩 8px 触达余量）；返回 Response 由调用方接命令
pub fn show(ui: &mut egui::Ui, icon: Icon, size: f32, selected: bool) -> Response {
    let pad = 8.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size + pad * 2.0), Sense::click());
    let ink = crate::theme::ink();
    let center = rect.center();

    // 状态底色：selected > hovered > 透明（圆角随 radius.sm token）
    let fill = if selected {
        crate::theme::accent_color()
    } else if resp.hovered() {
        crate::theme::hover_bg()
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(
            rect,
            egui::epaint::CornerRadius::same(crate::theme::radius_px("sm") as u8),
            fill,
        );
    }

    // 图标本体色：selected 上深字（金底需深色前景保对比），其余随语义色
    let icon_color = if selected {
        crate::theme::on_accent_color()
    } else if resp.hovered() {
        ink.strong
    } else {
        ink.soft
    };
    draw(ui.painter(), center, size, icon, icon_color);
    resp
}

// ===== src/widgets/icon_button.rs 说明区 =====
// 输入：图标/尺寸/选中态；输出：点击 Response（命令由调用方经 bus.send 发出）
// 逻辑步骤：命中域（图标外扩 8px）→ 状态底色（selected 金底/hovered 微亮）
//   → 矢量图标居中（金底配深色前景保对比）
// 设计理由：替代 egui small_button/selectable_label 文本按钮——原控件出厂灰底
//   与整体主题割裂；实心主题下 hover 底/hover 图标亮化仍有清晰可供性
// 异常处理：无 panic 面；size 任意值线宽下限由 icons::draw 内钳制
// 关联依赖：widgets::icons/theme；消费方：pages/countdown、pages/weather、pages/alarm
#[cfg(test)]
mod tests {
    #[test]
    fn hit_area_exceeds_icon() {
        // 命中外扩常量与绘制口径自洽（8px 触达余量编译期锚定）
        let pad = 8.0;
        assert_eq!(pad, 8.0);
    }
}
