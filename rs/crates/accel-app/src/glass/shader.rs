//! WGSL 着色器（ui/gl/shaders.py 的 Rust 对等物）
//! PL013.05 落地：SDF 圆角/纵横比校正/光场双晕/rim/bezel/预乘输出逐条移植

// ===== src/glass/shader.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013.05 进入
// 移植清单（对照 t.hint HINT002 勾销）：uv 空间 SDF × vec2(aspect,1) 校正、
//   cov 以 d 判向（d<0 面内）、fragColor = vec4(col*alpha, alpha) 预乘、
//   uniform array<vec4,16> 布局对照表文档化入 z.plan
