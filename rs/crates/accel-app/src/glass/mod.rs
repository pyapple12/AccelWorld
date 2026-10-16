//! 玻璃渲染总装（ui/gl/ 的 Rust 对等物）
//! PL013 落地：WGSL 着色器 + 场景管理 + egui PaintCallback 绘制器

pub mod painter;
pub mod shader;
pub mod surface;

// ===== src/glass/mod.rs 说明区 =====
// 对位：shader.rs ← ui/gl/shaders.py（GLSL→WGSL 移植，HINT002 逐条勾销）；
//   surface.rs ← ui/gl/glass_scene.py（GlassScene 契约同名同语义）；
//   painter.rs ← ui/gl/glass_canvas.py（PaintCallback 跑 wgpu 渲染）
// 约定延续：预乘 alpha / uv 归一化 / cov 以 d 判向（d<0 面内）/ 16 面上限
