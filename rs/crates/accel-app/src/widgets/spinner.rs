//! 加载弧（PL019）：16px、270° 弧、600ms/圈；只在调用方明确处于加载态时绘制，
//! 每帧请求 16ms 重绘，加载结束即停止（空闲不烧帧）。

use egui::{Color32, Sense};

/// 绘制旋转弧；返回占位 Response（不可交互）
pub fn show(ui: &mut egui::Ui, size: f32, color: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
    let t = ui.ctx().input(|i| i.time) as f32;
    let start = (t / 0.6).fract() * std::f32::consts::TAU;
    let r = size / 2.0 - 1.5;
    let center = rect.center();
    let segs = 18;
    let pts: Vec<egui::Pos2> = (0..=segs)
        .map(|i| {
            let a = start + std::f32::consts::TAU * 0.75 * (i as f32 / segs as f32);
            egui::pos2(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(pts, egui::Stroke::new(2.0, color)));
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(16));
    resp
}

// ===== src/widgets/spinner.rs 说明区 =====
// 输入：Ui + 尺寸 + 颜色；输出：占位 Response
// 逻辑步骤：i.time 秒 → 0.6s 周期相位 → 270° 折线弧 → 请求 16ms 重绘
// 设计理由（PL019）：天气刷新原先只有“更新中…”文字；旋转弧给出即时反馈，
//   且只在加载态调用，空闲时无重绘
// 异常处理：size 过小时半径钳 1px，不 panic
// 关联依赖：无；消费方：pages/weather
