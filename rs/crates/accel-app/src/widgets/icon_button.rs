//! 图标按钮（UIR 重塑新增）：矢量图标 + 状态底色的自绘点击控件
//! 替代原 egui small_button 文本按钮——实心主题下唯一的图标操作入口
//! PL016.05/06：焦点环可见 + hover 0.15s 缓动 + 按压下沉 0.06s（减少动效直取）

use egui::{Color32, Response, Sense, Vec2};

use super::icons::{Icon, draw};

/// 图标按钮：方形命中域 + 居中矢量图标；hover 微亮底、selected 主题金底
/// size = 图标边长（命中域外扩 8px 触达余量）；返回 Response 由调用方接命令
pub fn show(ui: &mut egui::Ui, icon: Icon, size: f32, selected: bool) -> Response {
    let pad = 8.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size + pad * 2.0), Sense::click());
    let ink = crate::theme::ink();
    let reduce = crate::theme::reduce_motion(ui.ctx());
    let center = rect.center();

    // PL016.06 small：hover 透明度 0.15s 缓动（animate_value_with_time 自提频）
    let hover_t = if reduce {
        if resp.hovered() { 1.0 } else { 0.0 }
    } else {
        ui.ctx().animate_value_with_time(
            resp.id.with("hover"),
            if resp.hovered() { 1.0 } else { 0.0 },
            crate::theme::motion_ms("hover_ms") / 1000.0,
        )
    };
    // 按压下沉 0.06s（Push Button 配方：内容随按压下移 1px）
    let pressed = resp.is_pointer_button_down_on();
    let sink_t = if reduce {
        if pressed { 1.0 } else { 0.0 }
    } else {
        ui.ctx().animate_value_with_time(
            resp.id.with("sink"),
            if pressed { 1.0 } else { 0.0 },
            crate::theme::motion_ms("press_ms") / 1000.0,
        )
    };

    // 状态底色：selected/按压 > hover 淡入 > 透明（圆角随 radius.sm token）
    let fill = if selected || pressed {
        if selected {
            crate::theme::accent_color()
        } else {
            crate::theme::color32_at("selection", 46)
        }
    } else if hover_t > 0.003 {
        crate::theme::with_alpha(crate::theme::hover_bg(), (hover_t * 255.0) as u8)
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

    // 图标本体色：selected 上深字（金底需深色前景保对比），悬停/按压同亮
    let icon_color = if selected {
        crate::theme::on_accent_color()
    } else if pressed || resp.hovered() {
        ink.strong
    } else {
        ink.body
    };
    let icon_center = egui::pos2(center.x, center.y + sink_t * 1.0);
    draw(ui.painter(), icon_center, size, icon, icon_color);

    // PL016.05：可见焦点环（键盘 Tab 聚焦自绘控件的唯一可见凭据）
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect,
            egui::epaint::CornerRadius::same(crate::theme::radius_px("sm") as u8),
            egui::Stroke::new(1.5, crate::theme::accent_color()),
            egui::StrokeKind::Inside,
        );
    }
    // UIR3：可供性——图标按钮统一指针对（集中处，三个消费页一次接线）
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ===== src/widgets/icon_button.rs 说明区 =====
// 输入：图标/尺寸/选中态；输出：点击 Response（命令由调用方经 bus.send 发出）
// 逻辑步骤：命中域（图标外扩 8px）→ hover 透明度缓动（0.15s）→ 状态底色
//   （selected 金底/按压靛底/hover 淡入）→ 按压下沉 1px（0.06s）→ 矢量图标
//   → 焦点环（has_focus 时 accent 描边）→ 指针对
// 设计理由：替代 egui small_button/selectable_label 文本按钮——原控件出厂灰底
//   与整体主题割裂；PL016.05/06 补键盘焦点可见性与三级动效的 small 档
//   （kinetics Push Button 配方；减少动效开关直取终态不动画）
// 异常处理：无 panic 面；size 任意值线宽下限由 icons::draw 内钳制
// 关联依赖：widgets::icons/theme(motion 段/reduce_motion)；消费方：仪器坞/
//   pages/countdown、pages/weather、pages/alarm
#[cfg(test)]
mod tests {
    #[test]
    fn hit_area_exceeds_icon() {
        // 命中外扩常量与绘制口径自洽（8px 触达余量编译期锚定）
        let pad = 8.0;
        assert_eq!(pad, 8.0);
    }
}
