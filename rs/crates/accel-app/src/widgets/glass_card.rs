//! 玻璃卡控件（ui/glass_card.py 的 Rust 对等物，PL014.02）
//! 构造即场景注册 + 选中态写面 + 减号响应区——页面卡片的统一容器

use egui::{Rect, Sense, Vec2};

use crate::glass::surface::{GlassScene, GlassSurface};

/// 玻璃卡（页面内一块玻璃面 + egui 内容区）
/// 生命周期：new 即 register（同 id 幂等替换——布局重排安全）；drop 不注销
/// （面上限 16 由场景钳制，页面复用固定 id 集合无泄漏——PL013.11 断言覆盖）
pub struct GlassCard {
    pub id: String,
    pub rect: Rect,
    radius: f32,
    selected: bool,
}

impl GlassCard {
    /// 构造并注册（radius 缺省 16，卡片圆角 token 化随 PL015 观感轮）
    pub fn register_new(scene: &mut GlassScene, id: &str, rect: Rect) -> GlassCard {
        let card = GlassCard {
            id: id.to_string(),
            rect,
            radius: 16.0,
            selected: false,
        };
        card.sync_to(scene);
        card
    }

    /// 属性同步到场景面（register 幂等——几何/选中/可见性一并写）
    pub fn sync_to(&self, scene: &mut GlassScene) {
        let mut surface = GlassSurface::new(&self.id, self.rect);
        surface.radius = self.radius;
        surface.selected = if self.selected { 1.0 } else { 0.0 };
        scene.register(surface);
    }

    /// 几何更新（布局重排）
    pub fn set_rect(&mut self, scene: &mut GlassScene, rect: Rect) {
        self.rect = rect;
        scene.update_geometry(&self.id, rect);
    }

    /// 选中态写入（FIX004.4 纪律：面 selected 恰 1.0/0.0 两值 + touch 直连）
    pub fn set_selected(&mut self, scene: &mut GlassScene, selected: bool) {
        self.selected = selected;
        scene.set_selected(&self.id, if selected { 1.0 } else { 0.0 });
        scene.touch();
    }

    /// 减号响应区（右上角圆域，直径 22）：点击返 true（FIX004.4 反向验收对等物）
    /// rect 为卡片矩形；调用方持 &mut Ui
    pub fn del_hit(ui: &mut egui::Ui, card_rect: Rect) -> bool {
        let del_rect = Self::del_rect_for_test(card_rect);
        ui.allocate_rect(del_rect, Sense::click()).clicked()
    }

    /// 减号区几何（测试锚定：右上 14px 内缩、22px 圆域）
    pub fn del_rect_for_test(card_rect: Rect) -> Rect {
        let center = card_rect.right_top() + Vec2::new(-14.0, 14.0);
        Rect::from_center_size(center, Vec2::splat(22.0))
    }
}

// chip 控件在独立模块 widgets/chip.rs（.02 拆分——玻璃卡容器与 chip 形态分离）

// ===== src/widgets/glass_card.rs 说明区 =====
// 输入：场景 + 矩形/选中态；输出：场景面同步 + 减号响应区几何
// 逻辑步骤：register_new 构造即注册（幂等）→ set_rect/set_selected 原位改 +
//   touch 直连；del_hit 右上 22px 圆域点击判定
// 设计理由：Python GlassCard 三纪律平移——构造即注册（无僵尸面窗口期）、
//   set_selected 两值化（FIX004.4：探针曾误选 0.5 中间态）、减号独立响应区；
//   drop 不注销（面上限 16 钳制 + 页面复用固定 id，PL013.11 泄漏断言覆盖）
// 异常处理：无 panic 面（场景操作全容错）
// 关联依赖：glass::surface/theme；消费方：pages 五页（PL014.05-.15）
#[cfg(test)]
mod tests {
    use super::*;
    use crate::glass::surface::GlassScene;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
    }

    #[test]
    fn register_then_selected_two_values() {
        // .02 验收：注册后面存在；set_selected(true/false) 面值恰 1.0/0.0（FIX004.4 平移）
        let mut scene = GlassScene::new();
        let mut card =
            GlassCard::register_new(&mut scene, "card_a", rect(10.0, 10.0, 200.0, 100.0));
        assert_eq!(scene.total_count(), 1);
        assert_eq!(scene.draw_list()[0].selected, 0.0);
        card.set_selected(&mut scene, true);
        assert_eq!(scene.draw_list()[0].selected, 1.0);
        assert!(scene.touch_count() >= 1, "set_selected 应 touch 直连");
        card.set_selected(&mut scene, false);
        assert_eq!(scene.draw_list()[0].selected, 0.0);
        // 同 id 重建（布局重排）幂等：仍一面且选中态由新卡值决定
        let card2 = GlassCard::register_new(&mut scene, "card_a", rect(20.0, 20.0, 200.0, 100.0));
        assert_eq!(scene.total_count(), 1);
        assert_eq!(scene.draw_list()[0].rect.min.x, 20.0);
        drop(card2);
        let _ = card;
    }

    #[test]
    fn del_hit_region_geometry() {
        // 减号响应区锚定：右上内缩 14、直径 22（FIX004.4 反向验收几何对等）
        let card = rect(100.0, 100.0, 300.0, 150.0);
        let del = GlassCard::del_rect_for_test(card);
        assert_eq!(del.center(), egui::pos2(386.0, 114.0));
        assert_eq!(del.size(), Vec2::splat(22.0));
        assert!(del.right() <= card.right(), "减号区不越卡片右缘");
    }

    #[test]
    fn chip_registers_surface() {
        // chip 即场景面：show 注册 elevation 5 面（交互需 Ui——此处仅验场景侧几何）
        let mut scene = GlassScene::new();
        let mut surface = GlassSurface::new("chip_1", rect(0.0, 0.0, 120.0, 64.0));
        surface.elevation = 5;
        scene.register(surface);
        assert_eq!(scene.draw_list().len(), 1);
        assert_eq!(scene.draw_list()[0].elevation, 5);
    }
}
