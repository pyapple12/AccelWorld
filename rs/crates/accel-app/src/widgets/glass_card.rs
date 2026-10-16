//! 玻璃卡控件（ui/glass_card.py 的 Rust 对等物）
//! PL013 落地：玻璃场景注册 + 选中态 + 几何同步；栅格降级双模式

// ===== src/widgets/glass_card.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013 进入
// 生命周期纪律：注册延迟到首次 show；销毁显式 unregister；
//   hide/show/move/resize 全路径容错（对齐 Qt 回调防护的 Rust 对等）
