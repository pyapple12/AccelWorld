//! 玻璃场景（ui/gl/glass_scene.py 的 Rust 对等物）
//! PL013.06 落地：GlassScene 保序面表 + register/unregister/可见性/选中/touch

// ===== src/glass/surface.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013.06 进入
// 契约对齐：register 延迟到首次 show（防僵尸面）、销毁显式 unregister、
//   隐藏页 visible=False 剔除绘制、同 id 重复 register 幂等替换、面上限 16
