//! 页面模块总装（ui/panels/ 的 Rust 对等物）
//! PL014.11 装配：Pages 容器 + 五页实体（各页 impl Page 在各页文件，本文件只做容器与装配）

use crate::bus::{Bus, PageId, UiState};
use crate::glass::surface::GlassScene;
use crate::window::register_page_surface;

// 页面子模块（各页 impl Page 在本文件内通过 include 的子模块实体落地）
pub mod alarm;
pub mod clock;
pub mod countdown;
pub mod ime_sim;
pub mod weather;
pub mod world;

/// 页面 trait（z.plan PL014·A 定案：各页自持临时输入态，持久态在后端）
/// ui 挂根 Ui（CentralPanel 0.37 形态）；ctx 经 ui.ctx() 另取
/// FIX006.2：命令出口传 `&Bus`（页面经 bus.send 走 CmdQueue 待发队列，满载不丢命令）
pub trait Page {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut UiState, bus: &Bus, scene: &mut GlassScene);
}

/// 页面容器（current + 五页 struct）
pub struct Pages {
    pub current: PageId,
    pub clock: ClockPage,
    pub countdown: CountdownPage,
    pub world: WorldPage,
    pub weather: WeatherPage,
    pub alarm: AlarmPage,
}

impl Default for Pages {
    fn default() -> Self {
        Self::new()
    }
}

impl Pages {
    pub fn new() -> Pages {
        Pages {
            current: PageId::Clock,
            clock: ClockPage,
            countdown: CountdownPage,
            world: WorldPage,
            weather: WeatherPage,
            alarm: AlarmPage,
        }
    }

    /// 当前页绘制（容器统一注册当前页玻璃面几何——FIX006.15：页内不再重复注册）
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut UiState,
        bus: &Bus,
        scene: &mut GlassScene,
    ) {
        // UiState.current_page 单源：Pages.current 镜像自导航点击（bus.apply 侧）
        self.current = state.current_page;
        // 页面内容区避让左缘悬浮 rail（A005 #9：由 nav_rail_width + page_margin token 派生）
        let page_rect = crate::window::content_page_rect(ui.ctx());
        register_page_surface(scene, self.current, page_rect);

        let page: &mut dyn Page = match self.current {
            PageId::Clock => &mut self.clock,
            PageId::Countdown => &mut self.countdown,
            PageId::World => &mut self.world,
            PageId::Weather => &mut self.weather,
            PageId::Alarm => &mut self.alarm,
        };
        page.ui(ui, state, bus, scene);
        // 设置浮窗绘制在 window.rs::paint_settings_pop（FIX006.14：本容器不再持空挂点）
    }
}

/// 快捷闹钟分钟选项（base.json quick_alarm_minutes token；A005 #9 零硬编码——
/// 缺键按 token 表损坏快速失败，不在代码侧复制数值）
/// FIX006.10：结果 OnceLock 缓存（消除每帧数组解析 + Vec 分配）
pub fn quick_alarm_minutes() -> &'static [i64] {
    static CACHE: std::sync::OnceLock<Vec<i64>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        accel_core::static_config::Static::get()
            .base
            .get("quick_alarm_minutes")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
            .unwrap_or_else(|| panic!("base.json 缺 quick_alarm_minutes token"))
    })
}

// ===== 五页实体 =====

/// 时钟页（impl 在 pages/clock.rs）
#[derive(Default)]
pub struct ClockPage;

/// 倒计时页（impl 在 pages/countdown.rs）
#[derive(Default)]
pub struct CountdownPage;

/// 世界时钟页（impl 在 pages/world.rs）
#[derive(Default)]
pub struct WorldPage;

/// 天气页（impl 在 pages/weather.rs）
#[derive(Default)]
pub struct WeatherPage;

/// 闹钟页（impl 在 pages/alarm.rs）
#[derive(Default)]
pub struct AlarmPage;

// 页面枚举循环工具（A005 #15：next_page 生产零调用已删除——导航点击切换见 nav.rs）

// ===== src/pages/mod.rs 说明区 =====
// 输入：UiState（current_page/settings_open 等）+ 各页自持临时输入态；输出：当前页装配
//   （容器统一注册页卡玻璃面几何）+ 五页 entity
// 逻辑步骤：current 镜像 state.current_page → content_page_rect 取几何 →
//   register_page_surface（容器侧，FIX006.15：页内不再重复注册）→ 分发 page.ui
// 设计理由：容器只管装配与分发表面（各页 impl 分散在 pages/*.rs，禁大文件后拆）；
//   页面 trait 传 `&Bus`（FIX006.2：命令出口经 CmdQueue 待发队列，满载不丢）；
//   FIX006.14：原 SettingsPop 空挂点已删（设置窗口实绘在 window.rs::paint_settings_pop，
//   经 state.settings_open 单一开关驱动，容器不再持纯空转发层）
// 异常处理：无 panic 面（页 id 固定五档，match 穷尽）
// 关联依赖：crate::bus（Bus/PageId/UiState）/glass::surface/window（几何）；消费方：window.rs
#[cfg(test)]
mod tests {
    use crate::bus::PageId;

    #[test]
    fn rapid_page_switch_no_surface_leak() {
        // .11 验收：连切 10 次页面，scene 面数稳定（每个切过的页 id 幂等注册一面，
        // 第二轮起零新增——与 Python GlassCard 注册模型同构）
        let mut scene = crate::glass::surface::GlassScene::new();
        let order = [
            PageId::Countdown,
            PageId::World,
            PageId::Weather,
            PageId::Alarm,
        ];
        for i in 0..10 {
            let page = order[i % order.len()];
            for p in [
                PageId::Clock,
                PageId::Countdown,
                PageId::World,
                PageId::Weather,
                PageId::Alarm,
            ] {
                scene.set_visible(&format!("page_{p:?}"), p == page);
            }
            let rect = egui::Rect::from_min_size(egui::pos2(180.0, 20.0), egui::vec2(600.0, 400.0));
            let mut s = crate::glass::surface::GlassSurface::new(&format!("page_{page:?}"), rect);
            s.radius = 16.0;
            scene.register(s);
            scene.set_visible(&format!("page_{page:?}"), true);
        }
        assert_eq!(
            scene.total_count(),
            4,
            "面数 = 切过的页面 id 数（前 4 次切换各注册一面，此后幂等零新增）"
        );
        assert_eq!(scene.draw_list().len(), 1, "仅当前页可见");
    }
}
