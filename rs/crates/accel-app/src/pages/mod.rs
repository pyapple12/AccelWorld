//! 页面模块总装（ui/panels/ 的 Rust 对等物）
//! PL014.11 装配：Pages 容器 + 五页实体（各页 impl Page 在各页文件，本文件只做容器与装配）

use crate::bus::{Bus, PageId, UiState};

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
/// UIR：scene 参数删除（玻璃面系统整体废弃）
pub trait Page {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut UiState, bus: &Bus);
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

    /// 当前页绘制（UIR 实心装配：单一 CentralPanel + 统一内边距——页板玻璃面
    /// 注册已随 glass 模块删除）
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut UiState, bus: &Bus) {
        // UiState.current_page 单源：Pages.current 镜像自 state.current_page（侧栏点击直改）
        self.current = state.current_page;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::color32_at("window_bg", 23))
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ui, |inner| {
                let page: &mut dyn Page = match self.current {
                    PageId::Clock => &mut self.clock,
                    PageId::Countdown => &mut self.countdown,
                    PageId::World => &mut self.world,
                    PageId::Weather => &mut self.weather,
                    PageId::Alarm => &mut self.alarm,
                };
                page.ui(inner, state, bus);
            });
        // 设置浮窗绘制在 window.rs::paint_settings_pop
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

// 页面枚举循环工具（A005 #15：next_page 生产零调用已删除——UIR 后导航点击切换
// 内联在 window.rs 侧栏）

// ===== src/pages/mod.rs 说明区 =====
// 输入：UiState（current_page/settings_open 等）+ 各页自持临时输入态；输出：当前页装配
// 逻辑步骤：current 镜像 state.current_page → 统一 CentralPanel（实心 window_bg 底
//   + 20 内边距）→ 分发 page.ui
// 设计理由：容器只管装配与分发（各页 impl 分散在 pages/*.rs，禁大文件后拆）；
//   页面 trait 传 `&Bus`（FIX006.2：命令出口经 CmdQueue 待发队列，满载不丢）；
//   UIR：scene 参数删除（玻璃面系统整体废弃——实心主题无面注册）
// 异常处理：无 panic 面（页 id 固定五档，match 穷尽）
// 关联依赖：crate::bus（Bus/PageId/UiState）；消费方：window.rs
