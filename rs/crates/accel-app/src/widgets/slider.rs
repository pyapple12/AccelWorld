//! 胶囊滑杆（ui/glass_card.py CapsuleSlider 的 Rust 对等物，PL014.04；UIR 实心蒙皮）
//! 三层几何：胶囊底轨 + accent 填充 + 旋钮；值映射与命令频率可测

use egui::{Rect, Sense, Vec2};

/// 滑杆值映射（纯函数）：轨道内指针比例 → 倍率值（0.1 步进四舍五入，域钳制）
/// Python 语义：int(round(rate×10)) 反向映射同式
pub fn ratio_to_rate(ratio: f64, min: f64, max: f64) -> f64 {
    let clamped = ratio.clamp(0.0, 1.0);
    let raw = min + clamped * (max - min);
    // 0.1 步进：×10 四舍五入 /10（f64 稳定表示由 0.1 步进表保证）
    ((raw * 10.0).round()) / 10.0
}

/// 倍率 → 轨道内比例（渲染用；反向同式）
pub fn rate_to_ratio(rate: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        return 0.0;
    }
    ((rate - min) / (max - min)).clamp(0.0, 1.0)
}

/// 填充几何（纯函数）：零进度 → 旋钮位整圆（用户定案：零进度保留完整圆帽）；
/// 正常进度 → 左端圆帽到指针圆帽的胶囊路径
/// 返回 (填充左圆心 x, 填充右圆心 x, 圆帽半径)——绘制层据此画双圆帽 + 连接矩形
pub fn fill_geometry(track: Rect, knob_x: f32, cap_r: f32) -> (f32, f32, f32) {
    let left = track.left() + cap_r;
    if knob_x <= left {
        // 零进度：左右圆心重合于左端 → 双圆合成完整圆（用户定案平移）
        return (left, left, cap_r);
    }
    (left, knob_x.max(left), cap_r)
}

/// 旋钮圆心 y = 轨道垂直中心（轨高奇偶均分——旋钮独立于轨高浮点居中）
pub fn knob_center_y(track: Rect) -> f32 {
    track.center().y
}

/// 滑杆交互 + 绘制（页面调用）：拖动/点击 → on_change(rate) 实时；
/// 松手/点击 → on_commit(rate)（FIX012.04 订正：点击同刻落盘，恰一发）
pub fn show(
    ui: &mut egui::Ui,
    track_h: f32,
    min: f64,
    max: f64,
    rate: &mut f64,
    mut on_change: impl FnMut(f64),
    mut on_commit: impl FnMut(f64),
) {
    let desired = Vec2::new(ui.available_width(), track_h + 22.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    let track = Rect::from_min_max(
        egui::pos2(rect.left() + 12.0, rect.center().y - track_h / 2.0),
        egui::pos2(rect.right() - 12.0, rect.center().y + track_h / 2.0),
    );

    // 拖动/点击 → 比例 → 倍率步进
    let mut dragged_rate: Option<f64> = None;
    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let ratio = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0) as f64;
        let new_rate = ratio_to_rate(ratio, min, max);
        if (new_rate - *rate).abs() > f64::EPSILON {
            *rate = new_rate;
            dragged_rate = Some(new_rate);
        }
    }
    // PL016.05（审核 #4）：键盘步进——聚焦后 ←→ 各 0.1，400ms 无按键自动落盘
    //（对齐拖动语义：连续调值只发 SetRate，停手恰一次 ApplyRate）
    let mut key_stepped = false;
    if resp.has_focus() {
        let step = ui.ctx().input(|i| {
            if i.key_pressed(egui::Key::ArrowLeft) {
                Some(-0.1)
            } else if i.key_pressed(egui::Key::ArrowRight) {
                Some(0.1)
            } else {
                None
            }
        });
        if let Some(d) = step {
            let new_rate =
                ratio_to_rate(rate_to_ratio(*rate, min, max) + d / (max - min), min, max);
            if (new_rate - *rate).abs() > f64::EPSILON {
                *rate = new_rate;
                key_stepped = true;
            }
        }
    }
    if let Some(r) = dragged_rate {
        on_change(r); // 实时生效（SetRate——拖动期每帧一发）
    }
    // PL016.05：键盘落盘核验（每帧）——最后步进后 400ms 无新步进 → 恰一次 ApplyRate
    let now = ui.ctx().input(|i| i.time);
    let commit_id = resp.id.with("kbd_commit");
    if key_stepped {
        on_change(*rate);
        ui.memory_mut(|m| m.data.insert_temp(commit_id, now));
    } else if let Some(last_step) = ui.memory(|m| m.data.get_temp::<f64>(commit_id))
        && now - last_step >= 0.4
    {
        ui.memory_mut(|m| m.data.remove::<f64>(commit_id));
        on_commit(*rate);
    }
    // FIX011.04：纯点击选值同刻落盘——egui 拖动阈值未过的 click 不产生
    // drag_stopped，原实现点击只发 SetRate 永不 ApplyRate（持久化悬置到退出
    // SaveNow 兜底）；clicked 与 drag_stopped 互斥，两分支各恰一次
    if resp.drag_stopped() || resp.clicked() {
        on_commit(*rate); // 松手/点击落盘（ApplyRate 恰一次）
    }

    draw(
        ui.painter(),
        track,
        track_h,
        rate_to_ratio(*rate, min, max) as f32,
    );

    // PL016.05：可见焦点环（键盘可达性的可见凭据）
    if resp.has_focus() {
        ui.painter().rect_stroke(
            track.expand(4.0),
            egui::epaint::CornerRadius::same(6),
            egui::Stroke::new(1.5, crate::theme::accent_color()),
            egui::StrokeKind::Inside,
        );
    }
}

/// 三层几何绘制：底轨胶囊 → 填充（零进度整圆）→ 旋钮
fn draw(painter: &egui::Painter, track: Rect, track_h: f32, ratio: f32) {
    // A005 #9：语义色 token
    let ink = crate::theme::ink();
    let cap_r = track_h / 2.0;
    // 底轨（弱轨色胶囊）
    painter.rect_filled(
        track,
        egui::epaint::CornerRadius::same(cap_r as u8),
        crate::theme::track_color(),
    );
    // 填充（accent token）
    let knob_x = track.left() + ratio * track.width();
    let (fx0, fx1, _) = fill_geometry(track, knob_x, cap_r);
    let fill_center_y = knob_center_y(track);
    painter.circle_filled(
        egui::pos2(fx0, fill_center_y),
        cap_r,
        crate::theme::accent_color(),
    );
    if fx1 > fx0 {
        painter.rect_filled(
            Rect::from_min_max(
                egui::pos2(fx0, fill_center_y - cap_r),
                egui::pos2(fx1, fill_center_y + cap_r),
            ),
            egui::epaint::CornerRadius::default(),
            crate::theme::accent_color(),
        );
        painter.circle_filled(
            egui::pos2(fx1, fill_center_y),
            cap_r,
            crate::theme::accent_color(),
        );
    }
    // 旋钮（白亮圆 + 细描边，圆心=轨道中心）
    painter.circle_filled(egui::pos2(knob_x, fill_center_y), cap_r + 3.0, ink.knob);
    painter.circle_stroke(
        egui::pos2(knob_x, fill_center_y),
        cap_r + 3.0,
        egui::Stroke::new(1.2, ink.meta),
    );
}

// ===== src/widgets/slider.rs 说明区 =====
// 输入：min/max/当前倍率 + change/commit 回调；输出：三层几何绘制 + 值映射
// 逻辑步骤：ratio_to_rate（0.1 步进）→ 拖动期 on_change 每变化帧一发 →
//   松手/点击 on_commit 恰一发；绘制底轨/填充（零进度整圆定案）/旋钮浮点居中
// 设计理由：Python CapsuleSlider 三纪律平移——零进度保留整圆（用户定案）、
//   TRACK_PAD 边距（fill/cap 内缩 12 起点防触裁剪界）、旋钮圆心=轨垂直中心
//   （浮点除 2 奇偶轨高均分）；命令频率语义（拖动 SetRate 高频/松手 ApplyRate
//   单发）由回调签名约束，频率单测在页面层以计数器验证
// 异常处理：max<=min 退化返 0 比例（防御除零）；指针越界 clamp
// 关联配置：倍率域 rate_min/rate_max（调用方传入）；消费方：pages/clock.rs
#[cfg(test)]
mod tests {
    use super::*;

    fn track() -> Rect {
        Rect::from_min_size(egui::pos2(100.0, 100.0), Vec2::new(400.0, 20.0))
    }

    #[test]
    fn ratio_rate_mapping_step_and_clamp() {
        // .04 验收：0.1 步进四舍五入 + 域钳制
        assert_eq!(ratio_to_rate(0.0, 1.0, 20.0), 1.0);
        assert_eq!(ratio_to_rate(1.0, 1.0, 20.0), 20.0);
        assert_eq!(ratio_to_rate(0.5, 1.0, 20.0), 10.5, "半程 → 10.5 步进");
        assert_eq!(ratio_to_rate(2.0, 1.0, 20.0), 20.0, "越界钳满");
        assert_eq!(ratio_to_rate(-1.0, 1.0, 20.0), 1.0, "越界钳最小");
        assert_eq!(
            rate_to_ratio(5.5, 1.0, 20.0),
            4.5 / 19.0,
            "比例=(rate-min)/(max-min)"
        );
        // 往返一致性（步进内）
        let r = ratio_to_rate(rate_to_ratio(7.3, 1.0, 20.0), 1.0, 20.0);
        assert_eq!(r, 7.3);
    }

    #[test]
    fn fill_geometry_zero_progress_keeps_full_circle() {
        // 用户定案：零进度填充为完整圆（左右圆心重合）
        let t = track();
        let (x0, x1, cap) = fill_geometry(t, t.left(), 10.0);
        assert_eq!(x0, x1, "零进度双圆心重合 → 整圆");
        assert_eq!(cap, 10.0);
        // 正常进度：左圆心锚定轨左端帽，右圆心随旋钮
        let knob = t.left() + 200.0;
        let (x0, x1, _) = fill_geometry(t, knob, 10.0);
        assert_eq!(x0, t.left() + 10.0, "左端帽圆心");
        assert_eq!(x1, knob, "右圆心=旋钮位");
    }

    #[test]
    fn knob_center_is_track_center() {
        // 轨高奇偶均分：旋钮圆心=轨垂直中心（浮点）
        let odd = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(400.0, 21.0));
        assert!((knob_center_y(odd) - 10.5).abs() < 1e-6);
        let even = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(400.0, 20.0));
        assert_eq!(knob_center_y(even), 10.0);
    }
}
