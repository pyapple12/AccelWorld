//! 倒计时页（ui/panels/countdown_panel.py 的 Rust 对等物）
//! PL014 落地：多目标倒计时 + 全年节日 chips + 默认明年春节 + 24h 加号规则

// ===== src/pages/countdown.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL014 进入
// 设计约束（用户产品定案）：取消手动输入；加号添加 24h 内目标；减号删除；
//   节日 chips 动态派生自 holidays 数据（春节日期动态算，根治 y.problems#7）
