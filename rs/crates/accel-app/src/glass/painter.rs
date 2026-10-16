//! 玻璃绘制器（ui/gl/glass_canvas.py 的 Rust 对等物）
//! PL013.06 落地：每帧对 scene 可见面构造 PaintCallback（wgpu 通路）

// ===== src/glass/painter.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013.06 进入
// 重绘纪律：touch → ctx.request_repaint()（FIX004.7 直连等价）；
//   空闲期 ctx.request_repaint_after 节流（egui 惯用法，静态期零重绘对齐）
