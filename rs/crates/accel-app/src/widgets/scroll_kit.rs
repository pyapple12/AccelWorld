//! 抽屉列表滚动套件（PL022）：上下渐隐 + 小三角 + 卡片外侧 4px 金色浮钮（页面自绘）。
//! 只画视觉件；滚动本体仍由 egui ScrollArea 提供，视口矩形由调用方给。

/// 关闭 egui 自带滚动渐隐（默认 20px / 0.5）：项目只保留本套件的 16px 渐隐，
/// 否则两层渐隐在 16px 处叠出硬台阶。原生滑杆由调用方另行隐藏。
pub fn apply_style(ui: &mut egui::Ui) {
    let mut style = ui.spacing().scroll;
    style.fade.strength = 0.0;
    ui.spacing_mut().scroll = style;
}

/// 上下渐隐：高度为 0 的一侧不画；颜色渐到抽屉底 `card_bg`，
/// 观感是文字与描边淡出，卡面本身与底色同色所以不出现色块。
/// 上下各多画 1px 并放宽裁剪，压住裁剪边可能残留的半像素缝。
pub fn paint_fade(ui: &egui::Ui, rect: egui::Rect, top_h: f32, bottom_h: f32) {
    let bg = crate::theme::card_bg();
    let clip = rect.expand2(egui::vec2(0.0, 1.0));
    let painter = ui.painter().with_clip_rect(clip);
    if top_h > 0.5 {
        paint_band(&painter, rect, top_h, true, bg);
    }
    if bottom_h > 0.5 {
        paint_band(&painter, rect, bottom_h, false, bg);
    }
}

/// 单条渐变带：顶带从底色的不透明收到透明，底带相反。
fn paint_band(painter: &egui::Painter, rect: egui::Rect, h: f32, top: bool, bg: egui::Color32) {
    let h = h.min(rect.height()).max(0.0);
    if h <= 0.0 {
        return;
    }
    let (y0, y1) = if top {
        (rect.top() - 1.0, rect.top() + h)
    } else {
        (rect.bottom() - h, rect.bottom() + 1.0)
    };
    let (a0, a1) = if top { (255, 0) } else { (0, 255) };
    let c0 = egui::Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), a0);
    let c1 = egui::Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), a1);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(egui::pos2(rect.left(), y0), c0);
    mesh.colored_vertex(egui::pos2(rect.right(), y0), c0);
    mesh.colored_vertex(egui::pos2(rect.right(), y1), c1);
    mesh.colored_vertex(egui::pos2(rect.left(), y1), c1);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// 上下小三角：贴视口上/下缘内缩 6px，水平居中；只画调用方判定要显示的一侧。
/// `alpha` 为最终不透明度（0–1），滚动期由调用方做呼吸，静止期传常亮值。
pub fn paint_hints(ui: &egui::Ui, rect: egui::Rect, up: bool, down: bool, alpha: f32) {
    if !up && !down {
        return;
    }
    let size = crate::theme::layout_f32("scroll_hint_icon");
    let inset = crate::theme::layout_f32("scroll_hint_inset");
    let color = crate::theme::accent_color().gamma_multiply(alpha.clamp(0.0, 1.0));
    let painter = ui.painter().with_clip_rect(rect);
    let cx = rect.center().x;
    if up {
        paint_triangle(
            &painter,
            egui::pos2(cx, rect.top() + inset),
            size,
            true,
            color,
        );
    }
    if down {
        paint_triangle(
            &painter,
            egui::pos2(cx, rect.bottom() - inset),
            size,
            false,
            color,
        );
    }
}

/// 单个等腰三角：up 时尖朝上，down 时尖朝下。
fn paint_triangle(
    painter: &egui::Painter,
    center: egui::Pos2,
    size: f32,
    up: bool,
    color: egui::Color32,
) {
    let half = size / 2.0;
    let h = size * 0.72;
    let points = if up {
        vec![
            egui::pos2(center.x, center.y - h / 2.0),
            egui::pos2(center.x - half, center.y + h / 2.0),
            egui::pos2(center.x + half, center.y + h / 2.0),
        ]
    } else {
        vec![
            egui::pos2(center.x, center.y + h / 2.0),
            egui::pos2(center.x - half, center.y - h / 2.0),
            egui::pos2(center.x + half, center.y - h / 2.0),
        ]
    };
    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

// ===== src/widgets/scroll_kit.rs 说明区 =====
// 输入：当前 Ui / 列表视口矩形 / 渐隐高度 / 上下三角显隐与透明度
// 逻辑步骤：apply_style 关掉 egui 自带渐隐 → ScrollArea 画完后 paint_fade 渐隐 →
//   paint_hints 画贴边三角；三角盖在渐隐之上，仍按调用方给的透明度呼吸
// 设计理由（PL022）：倒计时只有目标卡参与滚动，渐隐/三角必须只作用于这块视口；
//   egui 自带渐隐是 20px/0.5，和自绘 16px 叠在一起会在 16px 处出硬台阶，故置零；
//   渐隐用四顶点 Mesh 插值到 card_bg，一条 quad 完成，不产生横条色带；
//   上下各多画 1px 并放宽裁剪，压住裁剪边的半像素缝
// 异常处理：负高/零高钳制跳过，无 panic 面
// 关联配置：ui.json layout.scroll_* 与 motion.scroll_*；消费方：pages/countdown.rs
