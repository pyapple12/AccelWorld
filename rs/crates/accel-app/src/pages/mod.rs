//! 页面模块总装（ui/panels/ 的 Rust 对等物）
//! PL014.11 装配：Pages 容器 + 六页实体（各页 impl Page 在本文件）

use accel_core::types::FrontendCommand;
use egui::Context;
use tokio::sync::mpsc::Sender;

use crate::bus::{Bus, PAGE_ORDER, PageId, UiState};
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
pub trait Page {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut UiState,
        cmd: &Sender<FrontendCommand>,
        scene: &mut GlassScene,
    );
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

    /// 当前页绘制（每页 ui 首行注册自身玻璃面几何——随布局 rect 同步）
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut UiState,
        bus: &Bus,
        scene: &mut GlassScene,
        settings: &mut SettingsPop,
    ) {
        // UiState.current_page 单源：Pages.current 镜像自导航点击（bus.apply 侧）
        self.current = state.current_page;
        // 页面内容区避让左缘悬浮 rail（nav_pad：RAIL_X+PILL_W+呼吸空间 ≈ 180）
        let content = ui.ctx().input(|i| i.content_rect());
        let page_rect = egui::Rect::from_min_size(
            content.min + egui::vec2(180.0, 20.0),
            content.size() - egui::vec2(204.0, 40.0),
        );
        register_page_surface(scene, self.current, page_rect);

        let page: &mut dyn Page = match self.current {
            PageId::Clock => &mut self.clock,
            PageId::Countdown => &mut self.countdown,
            PageId::World => &mut self.world,
            PageId::Weather => &mut self.weather,
            PageId::Alarm => &mut self.alarm,
        };
        page.ui(ui, state, &bus.cmd_tx(), scene);

        // 设置浮窗（齿轮入口由 nav 触发 settings_open）
        if state.settings_open {
            settings.ui(ui.ctx(), state, bus);
        }
    }
}

/// 设置浮窗状态（PL013.10；窗口本体绘制在 window.rs——状态与主题选择在此）
#[derive(Debug, Default)]
pub struct SettingsPop {
    pub open: bool,
}

impl SettingsPop {
    pub fn ui(&mut self, _ctx: &Context, _state: &UiState, _bus: &Bus) {
        // 窗口绘制在 window.rs paint_settings_pop（需要 theme 重建权限）
        // 本方法保留为 PL014 设置项扩展挂点
    }
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

// 页面枚举循环工具（nav 点击循环用）
pub fn next_page(current: crate::bus::PageId) -> crate::bus::PageId {
    let idx = PAGE_ORDER.iter().position(|p| *p == current).unwrap_or(0);
    PAGE_ORDER[(idx + 1) % PAGE_ORDER.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn next_page_cycles_all_five() {
        // 循环序：五页首尾相接
        let seq = [
            (PageId::Clock, PageId::Countdown),
            (PageId::Countdown, PageId::World),
            (PageId::World, PageId::Weather),
            (PageId::Weather, PageId::Alarm),
            (PageId::Alarm, PageId::Clock),
        ];
        for (from, expect) in seq {
            assert_eq!(next_page(from), expect);
        }
    }
}
