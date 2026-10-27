//! 页面模块总装（UIR4：星盘主仪恒驻 home + 非时钟页右侧仪器抽屉）
//! Pages 容器 + 五页实体（各页 impl Page 在各页文件，本文件只做容器与装配）

use crate::bus::{Bus, PageId, UiState};

// 页面子模块（各页 impl Page 在本文件内通过 include 的子模块实体落地）
pub mod alarm;
pub mod clock;
pub mod countdown;
pub mod ime_sim;
pub mod settings;
pub mod weather;
pub mod world;

/// 页面 trait（z.plan PL014·A 定案：各页自持临时输入态，持久态在后端）
/// ui 挂根 Ui；ctx 经 ui.ctx() 另取
/// FIX006.2：命令出口传 `&Bus`（页面经 bus.send 走 CmdQueue 待发队列，满载不丢命令）
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

    /// 总装配（PL017）：非时钟页/设置占右侧抽屉（PageHeader + Scroll + Footer），
    /// 星盘主仪恒驻中央 CentralPanel；<760 抽屉全宽（compact），开合只缩让不遮挡。
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut UiState, bus: &Bus) {
        self.current = state.current_page;
        // ESC：优先关设置，否则回时钟（页头关闭的键盘等价）
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            if state.settings_open {
                state.settings_open = false;
            } else if state.current_page != PageId::Clock {
                state.current_page = PageId::Clock;
            }
        }
        let drawer_open = self.current != PageId::Clock || state.settings_open;

        // 抽屉开合弹簧（0=收起 → 1=全开；reduce 直取；ζ=1 无过冲）
        let target = if drawer_open { 1.0 } else { 0.0 };
        let reduce = crate::theme::reduce_motion(ui.ctx());
        if reduce {
            state.drawer_spring.value = target;
            state.drawer_spring.velocity = 0.0;
        } else {
            let dt = ui.ctx().input(|i| i.stable_dt);
            let (f, z) = crate::theme::drawer_spring_params();
            if state.drawer_spring.step(target, dt, f, z) {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f32(0.016));
            }
        }
        let spring_v = state.drawer_spring.value.clamp(0.0, 1.0);
        let compact = ui.available_width() < 760.0;

        if spring_v > 0.005 && drawer_open {
            let full_w = if compact {
                ui.available_width()
            } else {
                crate::theme::drawer_width(ui.available_width())
            };
            egui::Panel::right("page_drawer")
                .exact_size(full_w * spring_v)
                .frame(
                    egui::Frame::new()
                        .fill(crate::theme::card_bg())
                        .stroke(egui::Stroke::new(1.0, crate::theme::border_color())),
                )
                .show(ui, |inner| {
                    let (title, subtitle) = if state.settings_open {
                        ("设置", Some("外观 · 动效 · 关于"))
                    } else {
                        (page_title(self.current), None)
                    };
                    if crate::widgets::page_header::show(inner, title, subtitle) {
                        if state.settings_open {
                            state.settings_open = false;
                        } else {
                            state.current_page = PageId::Clock;
                        }
                    }
                    let content = egui::Frame::new().inner_margin(egui::Margin {
                        left: 20,
                        right: 20,
                        top: 8,
                        bottom: 16,
                    });
                    content.show(inner, |body| {
                        if state.settings_open {
                            settings::ui(body, state);
                        } else {
                            let page: &mut dyn Page = match self.current {
                                PageId::Clock => &mut self.clock,
                                PageId::Countdown => &mut self.countdown,
                                PageId::World => &mut self.world,
                                PageId::Weather => &mut self.weather,
                                PageId::Alarm => &mut self.alarm,
                            };
                            page.ui(body, state, bus);
                        }
                    });
                });
        }

        // 星盘主仪（时钟页恒驻 home；compact 抽屉全宽时中央宽度为 0，自然隐藏）
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::window_bg())
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ui, |inner| {
                self.clock.ui(inner, state, bus);
            });
    }
}

/// 抽屉页标题（结构即信息；PL017 页头）
pub fn page_title(page: PageId) -> &'static str {
    match page {
        PageId::Clock => "时钟",
        PageId::Countdown => "倒计时",
        PageId::World => "世界时钟",
        PageId::Weather => "天气",
        PageId::Alarm => "闹钟",
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

/// 时钟页（impl 在 pages/clock.rs——星盘主仪）
#[derive(Default)]
pub struct ClockPage;

/// 倒计时页（impl 在 pages/countdown.rs——抽屉页）
#[derive(Default)]
pub struct CountdownPage;

/// 世界时钟页（impl 在 pages/world.rs——抽屉页）
#[derive(Default)]
pub struct WorldPage;

/// 天气页（impl 在 pages/weather.rs——抽屉页）
#[derive(Default)]
pub struct WeatherPage;

/// 闹钟页（impl 在 pages/alarm.rs——抽屉页）
#[derive(Default)]
pub struct AlarmPage;

// ===== src/pages/mod.rs 说明区 =====
// 输入：UiState（current_page/settings_open 等）+ 各页自持临时输入态；输出：装配
// 逻辑步骤：ESC 关设置/回时钟 → current 镜像 → 非 Clock/设置 Panel::right 抽屉
//   （PageHeader 56 + ScrollArea + Footer；32% 宽钳 [320,400]；<760 全宽）→
//   CentralPanel 星盘主仪恒驻
// 设计理由（PL017）：时钟是「住在里面」的仪表——主仪永不下场，抽屉页是「查阅」
//   用的临时面板；三段式让页脚添加控件永远可达；设置与四页同构，不再是三行浮窗
// 异常处理：无 panic 面（页 id 固定五档，match 穷尽）
// 关联依赖：crate::bus（Bus/PageId/UiState）/widgets::page_header；消费方：window.rs
