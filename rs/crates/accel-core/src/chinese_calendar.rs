//! 农历/干支/节气/月相/财神（modules/chinese_calendar.py 的 Rust 对等物）
//! PL012 落地：lunar-rust 集成 + 出入清单 + 本地薄实现兜底

// ===== src/chinese_calendar.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL012 进入
// 设计约束：LunarInfo DTO 字段对齐 Python dataclass；lunar_info_str 拼装顺序与
//   标记契约逐字符对齐（节气/节日不入串——FIX003.5 定案延续）
