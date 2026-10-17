//! 平台集成总装（backdrop.py + system_tray.py 等系统层件的 Rust 对等物）
//! Acrylic（PL013.04）/ 托盘（PL015.01）/ 通知+热键（PL015.02）四件
//! Windows 真实现 + 非 Windows 降级桩（同接口空转——跨平台编译行为无损）

pub mod acrylic;

#[cfg(windows)]
pub mod hotkey;
#[cfg(windows)]
pub mod notify;
#[cfg(windows)]
pub mod tray;

#[cfg(not(windows))]
pub mod tray {
    // 非 Windows 降级桩：构造恒失败（调用方降级无托盘），事件流恒空
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TrayCommand {
        Show,
        Hide,
        Quit,
    }

    pub fn command_for(_id: &str) -> Option<TrayCommand> {
        None
    }

    pub struct TrayHandles;

    impl TrayHandles {
        pub fn new() -> Result<Self, String> {
            Err("非 Windows 平台无托盘".into())
        }

        pub fn set_status(&self, _rate: f64, _tooltip: &str) {}

        pub fn poll(&self) -> Vec<TrayCommand> {
            Vec::new()
        }
    }
}

#[cfg(not(windows))]
pub mod hotkey {
    // 非 Windows 降级桩：构造恒失败，动作流恒空
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HotkeyKind {
        Save,
        Quit,
        Theme,
    }

    pub struct HotkeyHandles;

    impl HotkeyHandles {
        pub fn new() -> Result<Self, String> {
            Err("非 Windows 平台无全局热键".into())
        }

        pub fn poll(&self) -> Vec<HotkeyKind> {
            Vec::new()
        }
    }
}

#[cfg(not(windows))]
pub mod notify {
    // 非 Windows 降级桩：通知恒不发送（行为无损）
    pub fn notify_alarm(_label: &str, _time: &str) -> bool {
        false
    }

    pub fn notify_save_failed() -> bool {
        false
    }
}

// ===== src/platform/mod.rs 说明区 =====
// 对位：acrylic.rs ← ui/backdrop.py（DWM SYSTEMBACKDROP=3）；
//   tray.rs ← ui/system_tray.py（tray-icon/muda）；
//   notify.rs ← WinRT toast（Python tray showMessage 对等）；hotkey.rs ← global-hotkey
// 降级纪律：全部平台能力失败可降级且行为无损（对齐 Python 版静默降级语义）——
//   Windows 内部构造失败由调用方降级，非 Windows 编译期即桩化（同接口零分叉）
