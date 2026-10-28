//! 反馈动效基元（PL019）：所有动画统一走 `anim`——减少动效时直取目标值，
//! 空闲不启动动画；颜色插值供 hover/按压的色调过渡使用。

use egui::{Color32, Context, Id};

/// 数值动画（时长毫秒）：reduce 直取；否则 egui 线性插值
pub fn anim(ctx: &Context, id: Id, target: f32, ms: f32) -> f32 {
    if crate::theme::reduce_motion(ctx) {
        target
    } else {
        ctx.animate_value_with_time(id, target, (ms / 1000.0).max(0.001))
    }
}

/// hover 进度 0..1（140ms 等由调用方 token 决定）
pub fn hover_t(ctx: &Context, id: Id, hovered: bool, ms: f32) -> f32 {
    anim(ctx, id, if hovered { 1.0 } else { 0.0 }, ms)
}

/// 两色线性插值（sRGB；t 已钳 0..1）
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

// ===== src/widgets/motion.rs 说明区 =====
// 输入：Context + 稳定 Id + 目标值/布尔 + 时长；输出：当前动画值
// 逻辑步骤：reduce 判断 → 直取或 animate_value_with_time → 颜色按 t 插值
// 设计理由（PL019）：把“减少动效/时长 token/空闲不新增重绘”收口到一处，
//   页面只描述目标状态，避免每个控件各写一套 0.1/0.2 魔法数
// 异常处理：时长为 0 时钳 1ms，不 panic
// 关联依赖：theme::reduce_motion；消费方：dock/world/countdown/switch/rate_strip
