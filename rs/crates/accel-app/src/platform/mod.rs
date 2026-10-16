//! 平台集成总装（backdrop.py + system_tray.py 等系统层件的 Rust 对等物）
//! PL015 落地：Acrylic/托盘/通知/热键四件

pub mod acrylic;
pub mod hotkey;
pub mod notify;
pub mod tray;

// ===== src/platform/mod.rs 说明区 =====
// 对位：acrylic.rs ← ui/backdrop.py（DWM SYSTEMBACKDROP=3）；
//   tray.rs ← ui/system_tray.py（tray-icon/muda）；
//   notify.rs ← winrt-notification 通知；hotkey.rs ← global-hotkey
// 降级纪律：全部平台能力失败可降级且行为无损（对齐 Python 版静默降级语义）
