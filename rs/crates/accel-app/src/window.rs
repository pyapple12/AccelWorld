//! 主窗装配器（ui/main_window.py 的 Rust 对等物）
//! PL013 落地：AppHandle + eframe::App 实现 + 六页装配 + 窗口几何

// ===== src/window.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013 进入
// 设计约束：窗口几何走 ui.json token（零硬编码）；resize/关闭回调容错
//   （对齐 Qt 回调防护纪律的 Rust 对等：闭包不 panic 外抛）
