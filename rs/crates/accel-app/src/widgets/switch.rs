//! 胶囊开关（PL017 复用件）：36×20，on 走语义绿、off 走弱轨；位置为主信号，
//! 颜色为辅助（不靠颜色单独传义）。供闹钟行与设置页共用。

use egui::{Sense, Stroke};

/// 开关本体尺寸（36×20，命中域同尺寸；桌面鼠标足够）
pub const SWITCH_W: f32 = 36.0;
pub const SWITCH_H: f32 = 20.0;

/// 绘制开关；返回是否被激活（指针点击或键盘 Enter/Space）。
pub fn show(ui: &mut egui::Ui, id: egui::Id, on: bool) -> bool {
    let ink = crate::theme::ink();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(SWITCH_W, SWITCH_H), Sense::click());
    let reduce = crate::theme::reduce_motion(ui.ctx());
    let anim = if reduce {
        if on { 1.0 } else { 0.0 }
    } else {
        ui.ctx()
            .animate_value_with_time(id, if on { 1.0 } else { 0.0 }, 0.1)
    };
    let track = if on {
        crate::theme::success_color()
    } else {
        crate::theme::track_color()
    };
    ui.painter().rect_filled(
        rect,
        egui::epaint::CornerRadius::same((SWITCH_H / 2.0) as u8),
        track,
    );
    let knob_r = SWITCH_H / 2.0 - 2.0;
    let knob_x = rect.left() + 2.0 + knob_r + anim * (SWITCH_W - 4.0 - knob_r * 2.0);
    ui.painter()
        .circle_filled(egui::pos2(knob_x, rect.center().y), knob_r, ink.knob);
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            egui::epaint::CornerRadius::same((SWITCH_H / 2.0) as u8 + 2),
            Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    let keyboard = resp.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let activated = resp.clicked() || keyboard;
    let _ = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    activated
}

// ===== src/widgets/switch.rs 说明区 =====
// 输入：Ui + 稳定 Id + on；输出：点击 Response
// 逻辑步骤：36×20 命中 → 0.1s 动效（reduce 直取终态）→ 轨色（on=success/off=track）
//   → 旋钮位置（左/右为主信号）→ 焦点环 → 指针对
// 设计理由（PL017）：原闹钟开关内联且 on=品牌金，与“金=数据”冲突；抽件后闹钟/
//   设置共用，on 改语义绿、位置仍独立可读；键盘可聚焦（可见环）
// 异常处理：无 panic 面；Id 由调用方按行稳定生成
// 关联依赖：theme(success/track/knob/reduce)；消费方：pages/alarm、pages/settings
