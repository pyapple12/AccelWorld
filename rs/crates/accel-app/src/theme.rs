//! 主题设计系统（Python 侧 qss/token 体系的 Rust 对等物）
//! PL013 落地：Theme struct 装载 ui.json 全 token + apply(ctx) 深浅两套 Visuals

// ===== src/theme.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013 进入
// 设计约束：颜色/圆角/字号全走 token（零硬编码）；三态偏好
//   （auto/light/dark）经 SetTheme 命令→interface 持久化→事件回推重应用
