//! 静态数据表总装（Python data/ 的 Rust 对等物）
//! PL011.06 落地：四表 include_str JSON + serde 强类型 + 访问函数

pub mod cities;
pub mod timezones;
pub mod weather_codes;
pub mod world_cities;

// ===== src/data/mod.rs 说明区 =====
// 数据来源：rs/data/*.json 三表 + rs/config/static/world_cities.json（共四表）与 Python
//   data/ 表原样一致（零改写平移；world_cities 为 PL012.09 新增表，落 config/static）
// 加载方式：include_str! 编译期嵌入（只读静态表，无须运行时文件 IO）
// 对位：cities.rs ← data/cities.py；timezones.rs ← data/timezones.py；
//   weather_codes.rs ← data/weather_codes.py；world_cities.rs ← PL012.09 新增
//   （世界时钟城市表，Python 侧原为两表，A005 #16 补记对位；FIX006.23 澄清与
//   天气域 data::cities 的职责分离）
