//! 页面模块总装（ui/panels/ 的 Rust 对等物）
//! PL013.11 装配：Pages 容器 + 五页占位（PL014 落地功能）+ 设置浮窗状态

use accel_core::types::FrontendCommand;
use egui::Context;
use tokio::sync::mpsc::Sender;

use crate::bus::{Bus, PAGE_ORDER, PageId, UiState};
use crate::glass::surface::GlassScene;
use crate::window::register_page_surface;

/// 页面 trait（z.plan PL014·A 定案：各页自持临时输入态，持久态在后端）
/// ui 挂根 Ui（CentralPanel 0.37 形态）；ctx 经 ui.ctx() 另取
pub trait Page {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &UiState,
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

// ===== 五页占位（PL014 落地功能；标题+说明文字接入导航全链路）=====

/// 占位页通用绘制：标题 + 说明（.13 收口五页占位）
/// 横向占位 156px 避让悬浮 rail（与页卡玻璃面 nav_pad 同口径）
fn placeholder_page_ui(ui: &mut egui::Ui, title: &str, hint: &str) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().inner_margin(egui::Margin::same(24)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(156.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.heading(
                        egui::RichText::new(title)
                            .size(crate::theme::font_size("page_title"))
                            .color(egui::Color32::from_rgb(235, 238, 245)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(hint)
                            .size(14.0)
                            .color(egui::Color32::from_rgb(180, 186, 200)),
                    );
                });
            });
        });
}

pub struct ClockPage;

impl Page for ClockPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &UiState,
        _cmd: &Sender<FrontendCommand>,
        _scene: &mut GlassScene,
    ) {
        // 核心显示随 PL014.01（时间/倍率滑杆/农历）；本占位验证导航与字体链路
        let std_time = state
            .time
            .as_ref()
            .map(|t| t.standard_time().to_string())
            .unwrap_or_else(|| "--:--:--".into());
        placeholder_page_ui(
            ui,
            &format!("时钟页 · 标准 {std_time}"),
            "PL014.01 落地完整时钟页",
        );
    }
}

pub struct CountdownPage;

impl Page for CountdownPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _state: &UiState,
        _cmd: &Sender<FrontendCommand>,
        _scene: &mut GlassScene,
    ) {
        placeholder_page_ui(ui, "倒计时页", "PL014 落地多目标倒计时 + 节日 chips");
    }
}

pub struct WorldPage;

impl Page for WorldPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _state: &UiState,
        _cmd: &Sender<FrontendCommand>,
        _scene: &mut GlassScene,
    ) {
        placeholder_page_ui(ui, "世界时钟页", "PL014 落地城市管理");
    }
}

pub struct WeatherPage;

impl Page for WeatherPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &UiState,
        cmd: &Sender<FrontendCommand>,
        _scene: &mut GlassScene,
    ) {
        // 天气页占位即接真实链路：进入即触发刷新命令（PL014 增强 UI）
        if state.weather.is_empty() && state.weather_errors.is_empty() {
            cmd.try_send(FrontendCommand::RefreshWeather {
                city: state.prefs.last_city.clone(),
                force: false,
            })
            .ok();
        }
        placeholder_page_ui(ui, "天气页", "PL014 落地多城市天气卡");
    }
}

pub struct AlarmPage;

impl Page for AlarmPage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &UiState,
        _cmd: &Sender<FrontendCommand>,
        _scene: &mut GlassScene,
    ) {
        placeholder_page_ui(
            ui,
            &format!("闹钟页（{} 项）", state.alarms.len()),
            "PL014 落地闹钟列表 + 快捷按钮",
        );
    }
}

// ===== src/pages/mod.rs 说明区 =====
// 输入：UiState 快照 + 命令出口；输出：五页占位绘制 + 页面玻璃面注册
// 逻辑步骤：Pages::ui 镜像 current_page → 页卡面注册（几何随布局）→ 分发当前页
// 设计理由：占位页即接入全链路（导航切换/玻璃面生命周期/字体/主题），PL014 只
//   替换页内实现；WeatherPage 占位即触发 RefreshWeather 验证总线往返
// 已知偏移：register_page_surface 由 pages.ui 统一调用（每页只注册当前页——
//   PL009 十二僵尸面教训的 Rust 对策：面上限 16，惰性注册+切页显隐）
// 关联依赖：bus/nav/window（导航与装配）；消费方：window.rs AppHandle::update

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
        // .11 验收：连切 10 次页面，scene 面数稳定（导航 1 + 当前页 1 = 2，零泄漏）
        let mut scene = crate::glass::surface::GlassScene::new();
        let order = [
            PageId::Countdown,
            PageId::World,
            PageId::Weather,
            PageId::Alarm,
        ];
        for i in 0..10 {
            let page = order[i % order.len()];
            // 模拟 window.sync_page_visibility：全页显隐 + 当前页注册
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
