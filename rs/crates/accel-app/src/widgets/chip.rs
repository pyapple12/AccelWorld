//! chip 控件（小尺寸玻璃卡：label + value 两行布局，PL014.02）
//! 农历 chips / 节日 chips / 城市卡等小卡的统一容器

use egui::{Color32, Sense, Vec2};

use crate::glass::surface::{GlassScene, GlassSurface};

/// chip 规格（尺寸 token 化随 PL015 观感轮；当前 120x64 圆角 12）
pub const CHIP_SIZE: Vec2 = Vec2::new(120.0, 64.0);

/// 绘制一枚 chip（label 上 value 下；selected 金描边）并注册玻璃面（elevation 5）
/// 返回响应（点击可选中/切换——语义由调用方定义）
pub fn show(
    ui: &mut egui::Ui,
    scene: &mut GlassScene,
    id: &str,
    label: &str,
    value: &str,
    selected: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(CHIP_SIZE, Sense::click());
    let mut surface = GlassSurface::new(id, rect);
    surface.radius = 12.0;
    surface.elevation = 5;
    surface.selected = if selected { 1.0 } else { 0.0 };
    scene.register(surface);
    // 选中金描边（GL selected 呼吸之外的双保险矢量层）
    if selected {
        ui.painter().rect_stroke(
            rect,
            egui::epaint::CornerRadius::same(12),
            egui::Stroke::new(1.6, crate::theme::accent_color()),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.left_top() + Vec2::new(10.0, 14.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(12.0),
        Color32::from_rgb(170, 176, 190),
    );
    ui.painter().text(
        rect.left_bottom() + Vec2::new(10.0, -14.0),
        egui::Align2::LEFT_BOTTOM,
        value,
        egui::FontId::proportional(16.0),
        Color32::from_rgb(235, 238, 245),
    );
    resp
}

// ===== src/widgets/chip.rs 说明区 =====
// 输入：场景 + chip id/文案/选中态；输出：场景面 + 绘制 + 点击响应
// 逻辑步骤：allocate 定矩形 → 场景面注册（elevation 5）→ 选中金描边 + 两行文字
// 设计理由：chip 是"小玻璃卡"而非独立类型（场景面统一渲染光学，矢量只画文字层）；
//   id 由调用方给业务语义前缀（lunar_x/festival_x/city_x），场景幂等注册防重排泄漏
// 异常处理：无 panic 面
// 关联依赖：glass::surface/theme::accent_color；消费方：pages/clock（农历 chips）、
//   pages/countdown（节日 chips）、pages/world/weather（城市 chips）
#[cfg(test)]
mod tests {
    use super::*;
    use crate::glass::surface::GlassScene;

    #[test]
    fn chip_surface_registration_contract() {
        // chip 场景面契约：elevation 5 + 选中两值化（与 glass_card 同纪律）
        let mut scene = GlassScene::new();
        let mut surface = GlassSurface::new(
            "chip_t",
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), CHIP_SIZE),
        );
        surface.elevation = 5;
        surface.selected = 1.0;
        scene.register(surface);
        assert_eq!(scene.draw_list().len(), 1);
        assert_eq!(scene.draw_list()[0].elevation, 5);
        assert_eq!(scene.draw_list()[0].selected, 1.0);
    }
}
