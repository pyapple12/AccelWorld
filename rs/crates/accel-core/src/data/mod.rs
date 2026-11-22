//! 静态数据表总装（城市主表在 SQLite，本模块保留数据表/拼音工具）
//! PL011.06 落地；PL029 起仅 timezones / weather_codes 两张 JSON 表

pub mod continents;
pub mod timezones;
pub mod weather_codes;
pub mod world_cities;

// ===== src/data/mod.rs 说明区 =====
// 数据来源：rs/data/*.json 两表（2026-11-22 PL029 删除 cities.json 17 城天气表）+
//   rs/assets/world_cache.sqlite（城市主表预置库）；world_cities.rs 仅保留拼音工具
// 加载方式：include_str! 编译期嵌入（只读静态表，无须运行时文件 IO）
// 对位：timezones.rs ← data/timezones.py；weather_codes.rs ← data/weather_codes.py；
//   world_cities.rs ← 仅拼音工具（城市主表已迁 SQLite，PL028 扩表；PL029 天气城市
//   改由世界页 hero 决定，旧 17 城坐标表退役）
