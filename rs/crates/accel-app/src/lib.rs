//! AccelWorld RS 应用库（accel-app）
//! bin 形态（main/render_probe）共享的模块根；GUI 依赖仅本 crate

pub mod bus;
pub mod glass;
pub mod logging;
pub mod nav;
pub mod pages;
pub mod platform;
pub mod theme;
pub mod tools;
pub mod widgets;
pub mod window;

// ===== src/lib.rs 说明区 =====
// bin/main.rs 声明同名 mod 时与本库并存（bin 内 mod 为独立编译单元——为避免
// 双编译，main.rs 改为引用 lib：use accel_app::window; 模块树单源在本文件）
// 依赖方向：app → core 单向（CI 关卡 4 强制 core 零 egui/eframe）
