//! 页面模块总装（ui/panels/ 的 Rust 对等物）
//! PL014 落地：六页功能（时钟/倒计时/世界时钟/天气/闹钟 + 设置浮窗）

pub mod alarm;
pub mod clock;
pub mod countdown;
pub mod weather;
pub mod world;

// ===== src/pages/mod.rs 说明区 =====
// 对位：clock.rs ← panels/clock_panel.py；countdown.rs ← countdown_panel.py；
//   world.rs ← world_panel.py（新：城市管理 PL014 落地）；
//   weather.rs ← weather_panel.py（新：多城市增强 PL014 落地）；
//   alarm.rs ← alarm_panel.py（新：快捷按钮 PL014 落地）
// 铁律：pages 零后端 import，一切经 interface 命令/事件（ui/ 零后端 import 同理）
