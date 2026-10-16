//! Acrylic 窗口背板（ui/backdrop.py 的 Rust 对等物）
//! PL013.04 落地：DwmSetWindowAttribute(38, 3) + 失败降级实底色

// ===== src/platform/acrylic.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013.04 进入
// 语义对齐：不支持/失败静默降级实底并返 false；失焦收材质为系统行为不补偿
