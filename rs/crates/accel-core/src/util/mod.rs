//! 通用工具总装（Python utils/ 的 Rust 对等物）
//! PL011.07/.08 落地：file_io 原子写/白名单/转存 + retry 重试契约

pub mod file_io;
pub mod retry;

// ===== src/util/mod.rs 说明区 =====
// 对位：file_io.rs ← utils/file_utils.py；retry.rs ← utils/retry.py
// utils/logger.py 不迁移：Rust 侧由 tracing 承接（app 侧初始化订阅）
//   core 只发 tracing 事件不配订阅，与 Python「业务模块只 getLogger」同理
// utils/dataclass_utils.py 不迁移：serde derive 即其 Rust 对等物
// utils/monitor.py 由 app 侧性能探针体系承接（PL015 性能任务）
