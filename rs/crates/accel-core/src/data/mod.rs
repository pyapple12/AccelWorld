//! 静态数据表总装（城市主表已迁 SQLite，本模块保留数据表/拼音工具）
//! PL011.06 落地：四表 include_str JSON + serde 强类型 + 访问函数

pub mod cities;
pub mod continents;
pub mod timezones;
pub mod weather_codes;
pub mod world_cities;

// ===== src/data/mod.rs 说明区 =====
// 数据来源：rs/data/*.json 三表 + rs/assets/world_cache.sqlite（城市主表预置库）；
//   world_cities.rs 仅保留拼音工具，城市数据运行时在 SQLite `city` 表
// 加载方式：include_str! 编译期嵌入（只读静态表，无须运行时文件 IO）
// 对位：cities.rs ← data/cities.py；timezones.rs ← data/timezones.py；
//   weather_codes.rs ← data/weather_codes.py；world_cities.rs ← 仅拼音工具
//   （城市主表已迁 SQLite，PL028 扩表）
