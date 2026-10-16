//! 胶囊导航（ui/panels/glass_nav.py 的 Rust 对等物）
//! PL013 落地：自绘胶囊按钮 + 五页固定序 + 底部齿轮 + hover 发光

// ===== src/nav.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013 进入
// 设计约束：圆角=高/2、选中金边框、hover 呼吸——全 token 驱动；
//   点击→UiState.current_page 变更 + 对应页玻璃面 set_visible 切换
