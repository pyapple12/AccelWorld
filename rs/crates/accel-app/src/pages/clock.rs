//! 时钟页（ui/panels/clock_panel.py 的 Rust 对等物）
//! PL014 落地：时间显示/倍率滑杆/农历信息串

// ===== src/pages/clock.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL014 进入
// 设计约束：数字用 Bahnschrift 等宽口径（PL013.03 字体地基）；倍率滑杆
//   步进/范围走 base.json token；农历信息串经 interface 拉取（零后端耦合）
