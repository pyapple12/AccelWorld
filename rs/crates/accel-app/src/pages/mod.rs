//! 页面模块总装（UIR4：星盘主仪恒驻 home + 非时钟页右侧仪器抽屉）
//! Pages 容器 + 五页实体（各页 impl Page 在各页文件，本文件只做容器与装配）

use crate::bus::{Bus, PageId, UiState};

// 页面子模块（各页 impl Page 在本文件内通过 include 的子模块实体落地）
pub mod alarm;
pub mod clock;
pub mod countdown;
pub mod ime_sim;
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

    /// 总装配（UIR4 模型）：非 Clock 页先占右侧仪器抽屉（卡面 + 发丝左缘），
    /// 星盘主仪恒驻中央 CentralPanel——抽屉开合只缩让、不遮挡、不消失
    /// PL016.06 medium：抽屉开合过冲滑入（bouncy 弹簧，动画期 16ms 提频）
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut UiState, bus: &Bus) {
        // UiState.current_page 单源：Pages.current 镜像自 state.current_page（仪器坞点击直改）
        self.current = state.current_page;

        // 抽屉开合弹簧（0=收起 → 1=全开；过冲到 ~1.1 再回摆；减少动效直取目标）
        let drawer_open = self.current != PageId::Clock;
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
        // 过冲钳 1.06（抽屉比全开宽 6% 的呼吸感足够，防大窗溢出）
        let spring_v = state.drawer_spring.value.min(1.06);

        if spring_v > 0.005 {
            let full_w = (ui.available_width() * 0.46).clamp(280.0, 420.0);
            egui::Panel::right("page_drawer")
                .exact_size(full_w * spring_v)
                .frame(
                    egui::Frame::new()
                        .fill(crate::theme::card_bg())
                        .stroke(egui::Stroke::new(1.0, crate::theme::border_color()))
                        .inner_margin(egui::Margin::same(16)),
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
        }

        // 星盘主仪（时钟页恒驻 home——仪器在住，抽屉在查）
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::color32_at("window_bg", 23))
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ui, |inner| {
                self.clock.ui(inner, state, bus);
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
// 逻辑步骤：current 镜像 → 非 Clock 页 Panel::right 抽屉（exact_size 宽 46% 钳
//   [280,420]，卡面 + 发丝描边）→ CentralPanel 星盘主仪恒驻
// 设计理由（UIR4）：时钟是「住在里面」的仪表——主仪永不下场，抽屉页是「查阅」
//   用的临时面板，开合只缩让主仪（dial_diameter 随宽自适应）；抽屉卡面与窗底
//   分层（文本层在卡面 5.93..14.16 全部实测达标）
// 异常处理：无 panic 面（页 id 固定五档，match 穷尽）
// 关联依赖：crate::bus（Bus/PageId/UiState）；消费方：window.rs
