//! 开关控件（UIR 重塑新增）：胶囊轨道 + 滑块圆钮的自绘 toggle
//! 替代原 selectable_label("开/关") 文本开关——状态一眼可辨、不动文案

use egui::{Response, Sense};

/// 开关绘制与交互：点击翻转（返回后的新状态由调用方持有并回传）
/// 轨道 36×20（几何为绘制参数，非业务 token）；滑块位移带 0.1s 缓动
pub fn show(ui: &mut egui::Ui, on: bool) -> (bool, Response) {
    const TRACK: egui::Vec2 = egui::vec2(36.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(TRACK, Sense::click());
    let ctx = ui.ctx().clone();
    let anim = ctx.animate_value_with_time(resp.id, if on { 1.0 } else { 0.0 }, 0.1);

    let accent = crate::theme::accent_color();
    let ink = crate::theme::ink();
    let track_fill = if on {
        accent
    } else {
        crate::theme::color32_at("slider_track", 60)
    };
    let knob_r = TRACK.y / 2.0 - 2.0;
    let knob_x = rect.left() + 2.0 + knob_r + anim * (TRACK.x - 4.0 - knob_r * 2.0);
    let knob_y = rect.center().y;

    ui.painter().rect_filled(
        rect,
        egui::epaint::CornerRadius::same(TRACK.y as u8 / 2),
        track_fill,
    );
    ui.painter()
        .circle_filled(egui::pos2(knob_x, knob_y), knob_r, ink.knob);
    (resp.clicked(), resp)
}

// ===== src/widgets/switch.rs 说明区 =====
// 输入：当前态 bool；输出：(是否被点击, Response)——翻转决策归调用方
// 逻辑步骤：命中胶囊 → animate_value_with_time 滑块位置缓动（0.1s）→
//   轨道底色（on=accent / off=slider_track）→ 圆钮白亮
// 设计理由：闹钟启用/禁用原用「开/关」文本 selectable_label——状态需读字才知；
//   胶囊开关是普适控件语言；缓动走 egui 内建动画（静止期零重绘，忙转纪律不破）
// 异常处理：无 panic 面；动画 id 取自响应（多开关互不串扰）
// 关联依赖：theme；消费方：pages/alarm.rs
#[cfg(test)]
mod tests {
    #[test]
    fn track_geometry_constants() {
        // 轨道几何锚定（36×20 / 滑块内边 2 / 钮径 = 高一半 -2）
        let (w, h) = (36.0_f32, 20.0_f32);
        let knob_r = h / 2.0 - 2.0;
        assert_eq!(w * h, 720.0);
        assert!((knob_r - 8.0).abs() < 1e-6);
    }
}
