//! 玻璃场景（ui/gl/glass_scene.py 的 Rust 对等物，PL013.06）
//! GlassSurface/GlassScene 契约与 Python 同名同语义；Vec 保序（16 面线性检索足够）

use egui::Rect;

/// 单块玻璃面（Python GlassSurface dataclass 镜像）
#[derive(Debug, Clone)]
pub struct GlassSurface {
    pub surface_id: String,
    /// 几何（像素坐标，画布系）
    pub rect: Rect,
    /// 端帽半径（像素）
    pub radius: f32,
    /// tint（rgb 0..1，ui.json glass.gl_tint 注入）
    pub tint: [f32; 3],
    /// 折射上限（uv 空间，默认 0.035）
    pub refraction: f32,
    /// 层级（低在前高在后）
    pub elevation: i32,
    /// 选中强度 0..1（金描边呼吸）
    pub selected: f32,
    pub visible: bool,
}

impl GlassSurface {
    /// 默认参数构造（tint 兜底 = Python 缺键兜底 (0.16, 0.14, 0.20)）
    pub fn new(surface_id: &str, rect: Rect) -> GlassSurface {
        GlassSurface {
            surface_id: surface_id.to_string(),
            rect,
            radius: 12.0,
            tint: [0.16, 0.14, 0.20],
            refraction: 0.035,
            elevation: 0,
            selected: 0.0,
            visible: true,
        }
    }
}

/// 玻璃面注册表：register/unregister/update_geometry/set_visible/set_selected/touch
/// repaint_hook 由 painter 注入（touch → ctx.request_repaint()，FIX004.7 直连等价）
pub struct GlassScene {
    surfaces: Vec<GlassSurface>,
    dirty: bool,
    repaint_hook: Option<Box<dyn Fn() + Send + Sync>>,
    /// touch 计数（探针/单测断言直连重绘生效）
    touch_count: u64,
}

impl Default for GlassScene {
    fn default() -> Self {
        Self::new()
    }
}

impl GlassScene {
    pub fn new() -> GlassScene {
        GlassScene {
            surfaces: Vec::new(),
            dirty: true,
            repaint_hook: None,
            touch_count: 0,
        }
    }

    /// 注入直连重绘回调（painter 持 ctx 弱引用实现）
    pub fn set_repaint_hook(&mut self, hook: Option<Box<dyn Fn() + Send + Sync>>) {
        self.repaint_hook = hook;
    }

    /// 置脏 + 直连重绘调度（hover 呼吸等动画驱动入口）
    pub fn touch(&mut self) {
        self.dirty = true;
        self.touch_count += 1;
        if let Some(hook) = &self.repaint_hook {
            hook();
        }
    }

    /// 注册或整体替换（同 id 幂等——Python 同语义）
    /// A005 #22：超 MAX_SURFACES 上限拒收并告警（原实现仅打包时按 elevation 截断，
    /// 理论上可丢高 elevation 的导航面——如 nav 会被误剔）
    pub fn register(&mut self, surface: GlassSurface) {
        match self
            .surfaces
            .iter()
            .position(|s| s.surface_id == surface.surface_id)
        {
            Some(pos) => self.surfaces[pos] = surface,
            None if self.surfaces.len() >= Self::MAX_SURFACES => {
                tracing::warn!(
                    "玻璃面数达上限 {}，拒收新面: {}",
                    Self::MAX_SURFACES,
                    surface.surface_id
                );
                return;
            }
            None => self.surfaces.push(surface),
        }
        self.dirty = true;
    }

    /// 注销（控件销毁时），无 id 静默
    pub fn unregister(&mut self, surface_id: &str) {
        if let Some(pos) = self
            .surfaces
            .iter()
            .position(|s| s.surface_id == surface_id)
        {
            self.surfaces.remove(pos);
            self.dirty = true;
        }
    }

    /// 仅更新几何（resize/move），其余属性保持
    pub fn update_geometry(&mut self, surface_id: &str, rect: Rect) {
        if let Some(s) = self
            .surfaces
            .iter_mut()
            .find(|s| s.surface_id == surface_id)
        {
            s.rect = rect;
            self.dirty = true;
        }
    }

    /// 可见性切换（切页隐藏页卡面剔除绘制）；无 id 静默；无变化不置脏
    pub fn set_visible(&mut self, surface_id: &str, visible: bool) {
        if let Some(s) = self
            .surfaces
            .iter_mut()
            .find(|s| s.surface_id == surface_id)
            .filter(|s| s.visible != visible)
        {
            s.visible = visible;
            self.dirty = true;
        }
    }

    /// 选中强度写入（导航选中态）
    pub fn set_selected(&mut self, surface_id: &str, selected: f32) {
        if let Some(s) = self
            .surfaces
            .iter_mut()
            .find(|s| s.surface_id == surface_id)
            .filter(|s| (s.selected - selected).abs() > f32::EPSILON)
        {
            s.selected = selected.clamp(0.0, 1.0);
            self.dirty = true;
        }
    }

    /// 全场景 tint 覆写（A005 #5：ui.json glass.{mode}.gl_tint 注入——主题驱动；面注册
    /// 后由调用方每帧统一施加；有变化才置脏，未变零开销）
    pub fn set_tint_all(&mut self, tint: [f32; 3]) {
        let mut changed = false;
        for s in &mut self.surfaces {
            if (s.tint[0] - tint[0]).abs() > f32::EPSILON
                || (s.tint[1] - tint[1]).abs() > f32::EPSILON
                || (s.tint[2] - tint[2]).abs() > f32::EPSILON
            {
                s.tint = tint;
                changed = true;
            }
        }
        if changed {
            self.dirty = true;
        }
    }

    /// 全表面访问（A005 #23：painter 帧内栈数组收集用；原 draw_list 保留给探针/单测）
    pub fn surfaces(&self) -> &[GlassSurface] {
        &self.surfaces
    }

    /// 绘制列表：仅可见面，按 elevation 升序（Python surfaces() 同语义）
    pub fn draw_list(&self) -> Vec<&GlassSurface> {
        let mut list: Vec<&GlassSurface> = self.surfaces.iter().filter(|s| s.visible).collect();
        list.sort_by_key(|s| s.elevation);
        list
    }

    /// 全部面数（含不可见——面泄漏断言用）
    pub fn total_count(&self) -> usize {
        self.surfaces.len()
    }

    /// 面数上限（16 面，Python _MAX_SURFACES 对等）
    pub const MAX_SURFACES: usize = 16;

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn touch_count(&self) -> u64 {
        self.touch_count
    }
}

// ===== src/glass/surface.rs 说明区 =====
// 输入：GlassSurface 注册流（widget/导航/浮窗面）；输出：surfaces()（painter 帧内栈数组
//   消费）＋ draw_list（仅可见面按 elevation 升序——单测/探针消费）
// 逻辑步骤：register 幂等替换（超 MAX_SURFACES 拒收并告警）→ set_visible/set_selected/
//   set_tint_all 原位改（无变化不置脏）→ painter 收集 surfaces() 过滤可见面插入排序
// 设计理由：Python GlassScene 契约一比一（id 幂等注册/静默注销/elevation 绘制序）；
//   Vec 线性检索（16 面上限内零开销且保注册序稳定）；touch→repaint_hook 直连重绘
//   （FIX004.7 语义平移：hover 呼吸期高帧率，静态期无 touch 即零重绘）；touch_count
//   供探针断言 hook 生效
// FIX006.14 裁决（保留理由就地登记）：unregister/update_geometry/draw_list/total_count/
//   dirty()/clear_dirty 生产零调用，但属注册表契约 API 且被单测消费（生命周期/幂等替换/
//   隐藏过滤/无变化不置脏/elevation 序/面上限），按"测试即契约"保留；dirty 位保留用于
//   断言「无变化不置脏」（忙碌重绘防护的可证性），painter 已改 surfaces() 栈数组收集
// 异常处理：全部操作无 panic 面（未知 id 静默——控件销毁竞态安全）
// 关联依赖：egui::Rect（几何）；消费方：painter.rs（surfaces→uniforms）、pages/nav/widgets
#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
    }

    #[test]
    fn lifecycle_register_unregister() {
        // .06 验收：register 后含且 dirty、unregister 后无、未知 id 静默
        let mut scene = GlassScene::new();
        scene.register(GlassSurface::new("a", rect(0., 0., 10., 10.)));
        assert!(scene.dirty());
        assert_eq!(scene.total_count(), 1);
        scene.clear_dirty();
        scene.unregister("missing");
        assert!(!scene.dirty(), "未知 id 注销应静默");
        scene.unregister("a");
        assert_eq!(scene.total_count(), 0);
        assert!(scene.dirty());
    }

    #[test]
    fn register_same_id_idempotent_replace() {
        // 同 id 重复 register 幂等替换（不新增）
        let mut scene = GlassScene::new();
        scene.register(GlassSurface::new("a", rect(0., 0., 10., 10.)));
        let mut b = GlassSurface::new("a", rect(5., 5., 20., 20.));
        b.elevation = 3;
        scene.register(b);
        assert_eq!(scene.total_count(), 1);
        assert_eq!(scene.draw_list()[0].rect.min.x, 5.0);
        assert_eq!(scene.draw_list()[0].elevation, 3);
    }

    #[test]
    fn visible_filter_and_geometry_update() {
        // set_visible(false) 不进 draw_list；update_geometry 仅改几何
        let mut scene = GlassScene::new();
        scene.register(GlassSurface::new("a", rect(0., 0., 10., 10.)));
        scene.clear_dirty();
        scene.set_visible("a", false);
        assert!(scene.dirty());
        assert!(scene.draw_list().is_empty());
        assert_eq!(scene.total_count(), 1, "隐藏不注销");
        scene.clear_dirty();
        scene.set_visible("a", false);
        assert!(!scene.dirty(), "无变化不置脏");
        scene.update_geometry("a", rect(1., 1., 5., 5.));
        scene.set_visible("a", true);
        assert_eq!(scene.draw_list()[0].rect.min, egui::pos2(1.0, 1.0));
    }

    #[test]
    fn elevation_draw_order() {
        // elevation 升序绘制（低在前）
        let mut scene = GlassScene::new();
        let mut hi = GlassSurface::new("hi", rect(0., 0., 10., 10.));
        hi.elevation = 5;
        let mut lo = GlassSurface::new("lo", rect(0., 0., 10., 10.));
        lo.elevation = -1;
        scene.register(hi);
        scene.register(lo);
        let list = scene.draw_list();
        assert_eq!(list[0].surface_id, "lo");
        assert_eq!(list[1].surface_id, "hi");
    }

    #[test]
    fn touch_invokes_hook_and_counts() {
        // touch → hook 直连重绘（FIX004.7 等价）+ 计数
        let mut scene = GlassScene::new();
        let fired = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let fired2 = std::sync::Arc::clone(&fired);
        scene.set_repaint_hook(Some(Box::new(move || {
            fired2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })));
        scene.touch();
        scene.touch();
        assert_eq!(scene.touch_count(), 2);
        assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 2);
        scene.set_repaint_hook(None);
        scene.touch();
        assert_eq!(
            fired.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "卸钩后不再触发"
        );
    }

    #[test]
    fn selected_write_and_clamp() {
        // 选中强度写入 + [0,1] 钳制 + 无变化不置脏
        let mut scene = GlassScene::new();
        scene.register(GlassSurface::new("a", rect(0., 0., 10., 10.)));
        scene.clear_dirty();
        scene.set_selected("a", 1.5);
        assert_eq!(scene.draw_list()[0].selected, 1.0);
        assert!(scene.dirty());
        scene.clear_dirty();
        scene.set_selected("a", 1.0);
        assert!(!scene.dirty(), "同值不置脏");
    }
}
