//! 闹钟页（ui/panels/alarm_panel.py 的 Rust 对等物，PL014 增强）
//! 用户产品定案：快捷 15/30 分钟/1h/4h 按钮 + 既有闹钟列表

// ===== src/pages/alarm.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL014 进入
// 快捷键语义：以当前时刻 +N 生成一次性闹钟（repeat_days 空），经 AddAlarm 命令；
//   列表渲染只消费 AlarmsChanged 全量回推（UI 无状态重建）
