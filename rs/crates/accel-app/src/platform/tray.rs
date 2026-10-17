//! 系统托盘（ui/system_tray.py 的 Rust 对等物，PL015.01）
//! tray-icon + muda：菜单树（显示/隐藏/分隔/当前倍率[禁用]/退出）+ 左键双击还原
//! 事件经 muda 全局通道（主线程 winit 消息泵驱动），window 每帧 drain → TrayCommand

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

// 菜单项 id（command_for 映射契约；倍率只读项无命令语义）
pub const ID_SHOW: &str = "show";
pub const ID_HIDE: &str = "hide";
pub const ID_RATE: &str = "rate";
pub const ID_QUIT: &str = "quit";

// 托盘图标（32×32 品牌金时钟环，rs/assets/accelworld-tray.png，gen_icon 产物）
const TRAY_PNG: &[u8] = include_bytes!("../../../../assets/accelworld-tray.png");

/// 托盘命令（菜单/双击事件归一后的窗口意图）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    Show,
    Hide,
    Quit,
}

/// 菜单项 id → 命令映射（纯函数：单测可注入，无须真实托盘；
/// 倍率只读项与未知 id 返 None——禁用项点击不产生命令）
pub fn command_for(id: &str) -> Option<TrayCommand> {
    match id {
        ID_SHOW => Some(TrayCommand::Show),
        ID_HIDE => Some(TrayCommand::Hide),
        ID_QUIT => Some(TrayCommand::Quit),
        _ => None,
    }
}

/// 托盘句柄集（TrayIcon 保活 + 倍率菜单文本更新入口）
/// 全部在主线程创建与调用（winit 事件循环泵消息，无须独立线程）
pub struct TrayHandles {
    _tray: TrayIcon,
    rate_item: MenuItem,
    // 上次悬停文本（tray-icon 无 tooltip getter——本地镜像实现未变化跳过语义）
    last_tooltip: std::cell::RefCell<String>,
}

impl TrayHandles {
    /// 构造托盘 + 菜单树（失败上抛由调用方降级——无托盘环境行为无损）
    pub fn new() -> Result<Self, String> {
        let icon = tray_icon().map_err(|e| format!("托盘图标解码失败: {e}"))?;
        let menu = Menu::new();
        let show = MenuItem::with_id(MenuId::new(ID_SHOW), "显示窗口", true, None);
        let hide = MenuItem::with_id(MenuId::new(ID_HIDE), "隐藏到托盘", true, None);
        let default_rate = accel_core::static_config::Static::get().base_f64("default_rate");
        let rate = MenuItem::with_id(
            MenuId::new(ID_RATE),
            format!("当前倍率: {default_rate:.1}x"),
            false,
            None,
        );
        let quit = MenuItem::with_id(MenuId::new(ID_QUIT), "退出", true, None);
        menu.append_items(&[&show, &hide, &PredefinedMenuItem::separator(), &rate, &quit])
            .map_err(|e| format!("托盘菜单构建失败: {e}"))?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(format!(
                "加速世界 - {}",
                accel_core::static_config::Static::get().version()
            ))
            .with_icon(icon)
            // 对齐 Python 交互：右键弹菜单，双击显示窗口（左键单击忽略）
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(true)
            .build()
            .map_err(|e| format!("托盘创建失败: {e}"))?;
        Ok(Self {
            _tray: tray,
            rate_item: rate,
            last_tooltip: std::cell::RefCell::new(String::new()),
        })
    }

    /// 倍率菜单文本 + 悬停文本更新（window 每秒节流推送；文本未变化时跳过防重绘）
    pub fn set_status(&self, rate: f64, tooltip: &str) {
        self.rate_item.set_text(format!("当前倍率: {rate:.1}x"));
        // Tooltip 相同则跳过（Python update_tooltip 未变化不 setToolTip 语义平移）
        if self.last_tooltip.borrow().as_str() != tooltip {
            self.last_tooltip.replace(tooltip.to_string());
            let _ = self._tray.set_tooltip(Some(tooltip));
        }
    }

    /// drain 菜单/双击事件 → 命令列表（每帧调用；muda 全局通道非阻塞消费）
    pub fn poll(&self) -> Vec<TrayCommand> {
        let mut out = Vec::new();
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some(cmd) = command_for(ev.id().as_ref()) {
                out.push(cmd);
            }
        }
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = ev
            {
                out.push(TrayCommand::Show);
            }
        }
        out
    }
}

/// 解码内嵌 PNG → 托盘 Icon（image crate 解 32×32 直通 RGBA）
fn tray_icon() -> Result<tray_icon::Icon, String> {
    let img = image::load_from_memory(TRAY_PNG)
        .map_err(|e| format!("{e}"))?
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).map_err(|e| format!("{e}"))
}

// ===== src/platform/tray.rs 说明区 =====
// 输入：muda MenuEvent/TrayIconEvent 全局通道 + window 每秒状态推送
// 输出：TrayCommand 流（Show/Hide/Quit）+ 菜单倍率文本/悬停文本
// 逻辑步骤：new（图标解码 → 菜单树 append → TrayIconBuilder 挂接）→ poll 每帧
//   drain 双通道 → set_status 节流更新
// 设计理由：Python QSystemTrayIcon 对等（菜单结构/双击还原/悬停文本逐项对齐）；
//   主线程创建（winit 泵消息）省独立线程与消息泵——托盘隐藏窗口消息由 eframe
//   事件循环统一分发；command_for 纯函数化使命令映射可单测（无须真实托盘）
// 异常处理：图标解码/菜单构建/托盘创建失败全部上抛 → window 降级无托盘运行
//   （对齐 Python 托盘可选语义）；set_tooltip 失败静默（外观层不影响行为）
// 已知偏移：①Python 左键单击弹出菜单，本版对齐 plan A 改右键弹菜单+双击还原
//   （plan「托盘菜单树」定案）；②通知时长 token（notification_duration_ms）不适用
//   ——WinRT toast 自管时长，token 留 base.json 不删（Python showMessage 遗产）
// 关联配置：default_rate（菜单初始文本）/版本（初始 tooltip）；消费方：window.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_mapping_covers_all_items() {
        // 菜单项回调注入式单测：三项命令各归位 + 只读项/未知项不产生命令
        assert_eq!(command_for(ID_SHOW), Some(TrayCommand::Show));
        assert_eq!(command_for(ID_HIDE), Some(TrayCommand::Hide));
        assert_eq!(command_for(ID_QUIT), Some(TrayCommand::Quit));
        assert_eq!(command_for(ID_RATE), None, "倍率只读项不得触发命令");
        assert_eq!(command_for("no_such"), None);
    }

    #[test]
    fn tray_construct_and_status_smoke() {
        // headless 托盘构造/销毁 smoke：交互会话应成功；受限环境允许失败但不 panic
        match TrayHandles::new() {
            Ok(tray) => {
                tray.set_status(3.5, "加速世界 - 2:00:00 @ 3.5x");
                tray.set_status(3.5, "加速世界 - 2:00:00 @ 3.5x"); // 未变化跳过路径
            }
            Err(e) => tracing::warn!("托盘构造失败（受限环境允许）: {e}"),
        }
    }
}
