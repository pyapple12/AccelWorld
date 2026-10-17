//! 进度环控件（自绘圆弧类，PL014.03）
//! 外弧背景 + 进度弧（accent token）+ 中心百分比；自绘圆弧留边 ≥1px 纪律平移

use egui::{Pos2, Rect, Vec2};

/// 进度 → 弧度换算（纯函数）：0.0=12 点钟起顺时针 1.0=满圈；含 0/100 边界钳制
/// 返回 (起始角, 扫过角)，单位弧度（-90° 起即 12 点方向）
pub fn progress_to_arc(progress: f64) -> (f32, f32) {
    let p = progress.clamp(0.0, 1.0) as f32;
    let start = -std::f32::consts::FRAC_PI_2;
    let sweep = std::f32::consts::TAU * p;
    (start, sweep)
}

/// 进度环绘制（留边纪律：环径比控件短边小 ≥2px，抗锯齿切点不出界）
pub fn show(ui: &mut egui::Ui, size: f32, progress: f64, percent_text: &str) {
    // 留边：控件短边两侧各收 2px（≥1px 纪律取双倍余量）；A005 #9 起尺寸取
    // layout.progress_ring_size token（调用方传入），非 0 时以 token 为准
    let avail = ui.available_height().min(ui.available_width()) - 2.0;
    let side = if size > 0.0 { size.min(avail) } else { avail };
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side.max(40.0)), egui::Sense::hover());
    draw_at(ui.painter(), rect, progress, percent_text);
}

/// 指定矩形绘制（页面内嵌用）
pub fn draw_at(painter: &egui::Painter, rect: Rect, progress: f64, percent_text: &str) {
    // A005 #9：语义色 token
    let ink = crate::theme::ink();
    // 留边 ≥1px（自绘圆弧纪律：相切边界抗锯齿量化成平边）
    let inset = 2.0;
    let circle = Rect::from_min_size(
        rect.min + Vec2::splat(inset),
        rect.size() - Vec2::splat(inset * 2.0),
    );
    let center = circle.center();
    let radius = circle.width().min(circle.height()) / 2.0;
    let stroke_w = 6.0;
    let arc_r = radius - stroke_w;

    // 外弧（背景弱色全圈）
    painter.circle_stroke(
        center,
        arc_r,
        egui::Stroke::new(stroke_w, crate::theme::color32_at("ring_track", 70)),
    );
    // 进度弧（accent token；0 进度不画——零长度弧在部分驱动出锯齿伪影）
    // egui 0.37 无 Painter::arc——48 段折线逼近（半径 40+ 时弦差 <0.1px，观感真圆）
    let (start, sweep) = progress_to_arc(progress);
    if sweep > 0.001 {
        let segs = 48;
        let mut points: Vec<Pos2> = Vec::with_capacity(segs + 1);
        for i in 0..=segs {
            let angle = start + sweep * (i as f32 / segs as f32);
            points.push(egui::pos2(
                center.x + arc_r * angle.cos(),
                center.y + arc_r * angle.sin(),
            ));
        }
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(stroke_w, crate::theme::accent_color()),
        ));
    }
    // 中心百分比
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        percent_text,
        egui::FontId::proportional(crate::theme::font_size("label")),
        ink.strong,
    );
}

// ===== src/widgets/ring.rs 说明区 =====
// 输入：进度 0..1 + 百分比文案；输出：背景弧 + 进度弧 + 中心文字
// 逻辑步骤：progress_to_arc（12 点起顺时针 TAU×p）→ 背景全圈 stroke →
//   sweep>0.001 才画进度弧 → 中心文本
// 设计理由：自绘圆弧留边纪律（2026-09-12 定案）——circle 比控件 inset 2px，
//   弧线半径再收 stroke_w/2 保证描边不触裁剪界；accent 走 token（theme::accent_color）
// 异常处理：progress 越界钳 [0,1]；零进度跳弧（驱动伪影防御）
// 关联配置：progress_ring_size（调用方 layout token 取值）；
//   消费方：pages/clock.rs 加速时间卡（PL014.05）
#[cfg(test)]
mod tests {
    use super::progress_to_arc;

    #[test]
    fn arc_conversion_with_bounds() {
        // .03 验收：弧度换算含 0/100 边界与越界钳制
        let (start, sweep0) = progress_to_arc(0.0);
        assert_eq!(start, -std::f32::consts::FRAC_PI_2, "12 点起");
        assert_eq!(sweep0, 0.0, "零进度零扫角");
        let (_, sweep_half) = progress_to_arc(0.5);
        assert!((sweep_half - std::f32::consts::PI).abs() < 1e-6, "半进度 π");
        let (_, sweep_full) = progress_to_arc(1.0);
        assert!(
            (sweep_full - std::f32::consts::TAU).abs() < 1e-6,
            "满进度 2π"
        );
        // 越界钳制
        assert_eq!(progress_to_arc(-0.5).1, 0.0);
        assert_eq!(progress_to_arc(1.5).1, std::f32::consts::TAU);
    }
}
