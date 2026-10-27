//! AccelWorld 核心业务库（accel-core）
//! 镜像 Python 侧 modules/config/data/utils 四层，零 GUI 依赖；
//! 前端仅经 interface 的命令/事件总线与本库通信（z.plan 修订一）

pub mod alarm;
pub mod audio;
pub mod chinese_calendar;
pub mod data;
pub mod interface;
pub mod settings;
pub mod static_config;
pub mod time_dilation;
pub mod types;
pub mod util;
pub mod weather;

// ===== src/lib.rs 说明区 =====
// 模块清单（镜像映射表对位）：
//   types.rs           ← interface/types.py（跨层 DTO 转出）
//   interface.rs       ← interface/app_interface.py（七域契约 + 命令/事件总线）
//   time_dilation.rs   ← modules/time_dilation.py（PL011.10）
//   chinese_calendar.rs ← modules/chinese_calendar.py（PL012 落地）
//   weather.rs         ← modules/weather_service.py（PL011.11/.12）
//   alarm.rs           ← modules/alarm_service.py（PL012 落地）
//   settings.rs        ← config/settings.py（PL011.09）
//   static_config.rs   ← config/static/static_config.py（PL011.05）
//   data/              ← data/ 四静态表（cities/timezones/weather_codes/world_cities；
//                        PL011.06 三表 + PL012.09 world_cities；天气与世界时钟两表职责分离）
//   util/              ← utils/（retry/file_io/paths；logger 由 app 侧 tracing 承接）
// 依赖方向单向：interface → 业务模块 → data/util；禁止反向与横向 import
// utils/logger 的 Rust 对等物为 tracing（app 侧初始化），core 只发日志不配日志
