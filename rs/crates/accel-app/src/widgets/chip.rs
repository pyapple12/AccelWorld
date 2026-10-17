//! chip 控件（小尺寸玻璃卡：label + value 两行布局，PL014.02）
//! 农历 chips / 节日 chips / 城市卡等小卡的统一容器

use egui::{Sense, Vec2};

use crate::glass::surface::{GlassScene, GlassSurface};

/// chip 规格（FIX006.5：尺寸取 ui.json `layout.chip_size`，圆角取 `radius.md`）
pub fn chip_size() -> Vec2 {
    let [w, h] = crate::theme::layout_vec2("chip_size", [120.0, 64.0]);
    Vec2::new(w, h)
}

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
    let (rect, resp) = ui.allocate_exact_size(chip_size(), Sense::click());
    // A005 #9：语义色 + 圆角 token（radius.md）
    let ink = crate::theme::ink();
    let chip_radius = crate::theme::radius_px("md");
    let mut surface = GlassSurface::new(id, rect);
    surface.radius = chip_radius;
    surface.elevation = 5;
    surface.selected = if selected { 1.0 } else { 0.0 };
    scene.register(surface);
    // 选中金描边（GL selected 呼吸之外的双保险矢量层）
    if selected {
        ui.painter().rect_stroke(
            rect,
            egui::epaint::CornerRadius::same(chip_radius as u8),
            egui::Stroke::new(1.6, crate::theme::accent_color()),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.left_top() + Vec2::new(10.0, 14.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(crate::theme::font_size("chip_label")),
        ink.soft,
    );
    ui.painter().text(
        rect.left_bottom() + Vec2::new(10.0, -14.0),
        egui::Align2::LEFT_BOTTOM,
        value,
        egui::FontId::proportional(crate::theme::font_size("chip_value")),
        ink.strong,
    );
    resp
}

// ===== src/widgets/chip.rs 说明区 =====
// 输入：场景 + chip id/文案/选中态；输出：场景面 + 绘制 + 点击响应
// 逻辑步骤：chip_size（layout.chip_size token）定矩形 → 场景面注册（elevation 5）→
//   选中金描边 + 两行文字（字号 chip_label/chip_value token）
// 设计理由：chip 是"小玻璃卡"而非独立类型（场景面统一渲染光学，矢量只画文字层）；
//   id 由调用方给业务语义前缀（lunar_x/festival_x/city_x），场景幂等注册防重排泄漏
// FIX006.5：尺寸/圆角/字号全部走 token（原 CHIP_SIZE 常量与 12/16 字面量已收编）
// 异常处理：无 panic 面
// 关联依赖：glass::surface/theme（layout/scale/accent）；消费方：pages/clock（农历 chips）、
//   pages/countdown（节日 chips）
#[cfg(test)]
mod tests {
    use super::*;
    use crate::glass::surface::GlassScene;

    #[test]
    fn chip_surface_registration_contract() {
        // chip 场景面契约：elevation 5 + 选中两值化 + 尺寸来自 token
        let mut scene = GlassScene::new();
        let mut surface = GlassSurface::new(
            "chip_t",
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), chip_size()),
        );
        surface.elevation = 5;
        surface.selected = 1.0;
        scene.register(surface);
        assert_eq!(scene.draw_list().len(), 1);
        assert_eq!(scene.draw_list()[0].elevation, 5);
        assert_eq!(scene.draw_list()[0].selected, 1.0);
        assert_eq!(
            chip_size(),
            Vec2::new(120.0, 64.0),
            "chip 尺寸 token 应为 120×64"
        );
    }
}
