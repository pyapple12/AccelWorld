//! 展示运算纯函数总装（ui/tools/ 的 Rust 对等物）
//! PL014 落地：时钟/倒计时/闹钟文本的纯展示运算（零 IO 零状态）

pub mod alarm_text;
pub mod clock;
pub mod countdown;

// ===== src/tools/mod.rs 说明区 =====
// 对位：clock.rs ← ui/tools/clock_tools.py；countdown.rs ← countdown_tools.py；
//   alarm_text.rs ← alarm_text.py
// 铁律：纯函数（输入→输出，无 interface/无 IO），可独立单测——与 Python 同构
