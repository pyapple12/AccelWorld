//! 世界时钟专用城市表（50 城；PL012.09 建表，FIX006.23 澄清职责边界）
//! 天气域城市候选另取 `data::cities`（rs/data/cities.json，17 城坐标表）——两表职责分离
//! config/static/world_cities.json 编译期嵌入；tz 逐条经 chrono-tz 编译期类型可解析

use std::sync::OnceLock;

use serde::Deserialize;

// 城市行（名称/IANA 时区/经纬度；JSON 内 region 分组字段无消费方，serde 自动忽略）
#[derive(Debug, Deserialize)]
pub struct WorldCity {
    pub name: &'static str,
    pub tz: &'static str,
    pub lat: f64,
    pub lon: f64,
}

const WORLD_CITIES_JSON: &str = include_str!("../../../../config/static/world_cities.json");

static CITIES: OnceLock<Vec<WorldCity>> = OnceLock::new();

/// 全表（表序：中国 → 东亚 → 南亚/中东 → 欧洲 → 美洲 → 大洋洲 → 非洲）
pub fn all() -> &'static [WorldCity] {
    CITIES.get_or_init(|| {
        serde_json::from_str(WORLD_CITIES_JSON).expect("world_cities.json 解析失败（嵌入产物损坏）")
    })
}

/// 城市名 → 行（未命中 None）
pub fn by_name(name: &str) -> Option<&'static WorldCity> {
    all().iter().find(|c| c.name == name)
}

/// 全部城市名（表序）
pub fn all_names() -> Vec<&'static str> {
    all().iter().map(|c| c.name).collect()
}

/// 城市名 → IANA 时区串
pub fn tz_of(name: &str) -> Option<&'static str> {
    by_name(name).map(|c| c.tz)
}

/// IANA 时区串 → 城市名（FIX007.2 迁移辅助：world_pins 权威域已改城市名，
/// 本函数仅服务旧存档 IANA → 城市名的装载期迁移；同 tz 多城表序首匹配）
pub fn name_of_tz(tz: &str) -> Option<&'static str> {
    all().iter().find(|c| c.tz == tz).map(|c| c.name)
}

// ===== src/data/world_cities.rs 说明区 =====
// 输入：rs/config/static/world_cities.json（50 城静态表）；输出：强类型检索
//   （by_name/tz_of 名域 + name_of_tz 迁移辅助）
// 设计理由：世界时钟专用城市表（FIX006.23/FIX007.11：天气城市候选另用 rs/data/
//   cities.json 17 城坐标表——两表职责分离，无「统一城市源」）；FIX007.2：常驻集
//   权威域 = 城市名（原 FIX006.1 的 IANA 存储在同 tz 多城时丢失身份——加入大阪
//   显示东京、上海被北京吞掉）；表放 config/static（随版本发布不运行时改）而
//   加载器在 data/（数据访问职责）——include_str 路径跨约定，登记 z.plan；
//   tz 存字符串，消费侧 chrono_tz 解析（编译期无法对 JSON 内容静态校验，故单测
//   遍历全表运行期断言可解析）
// 数据来源：原 17 城坐标自 Python CITIES 平移；增补城市经纬度/IANA 按公开
//   地理常识核对（PL012.09 落位，后续可按需扩至 60 上限内）
// 异常处理：JSON 解析失败 expect 快速失败（嵌入产物损坏级）；region 分组字段
//   无消费方（FIX007.10 结构体已删该字段，serde 忽略 JSON 冗余键）
// 关联依赖：serde_json/chrono-tz；消费方：interface 城市域（PL012.10）、PL014 世界时钟页
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn table_size_and_uniqueness() {
        // 表规模 40-60 区间 + 城市名唯一
        let all = all();
        assert!(
            all.len() >= 40 && all.len() <= 60,
            "表规模应 40-60，实际 {}",
            all.len()
        );
        let names: HashSet<&str> = all_names().into_iter().collect();
        assert_eq!(names.len(), all.len(), "城市名必须唯一");
    }

    #[test]
    fn every_tz_parses_and_coords_valid() {
        // 遍历全表：tz 经 chrono-tz 解析 OK；lat/lon 定义域合法
        for c in all() {
            let tz: Result<chrono_tz::Tz, _> = c.tz.parse();
            assert!(tz.is_ok(), "城市 {} 的 tz 无法解析: {}", c.name, c.tz);
            assert!(
                (-90.0..=90.0).contains(&c.lat),
                "城市 {} lat 越界: {}",
                c.name,
                c.lat
            );
            assert!(
                (-180.0..=180.0).contains(&c.lon),
                "城市 {} lon 越界: {}",
                c.name,
                c.lon
            );
        }
    }

    #[test]
    fn lookup_hit_and_miss() {
        // 检索命中/None 两分支 + 锚点值（A005 #15：coords_of 已删，坐标断言走 by_name）
        assert_eq!(tz_of("北京"), Some("Asia/Shanghai"));
        assert_eq!(tz_of("纽约"), Some("America/New_York"));
        assert_eq!(tz_of("悉尼"), Some("Australia/Sydney"));
        assert!(by_name("不存在城").is_none());
        let tokyo = by_name("东京").expect("东京应在表");
        assert_eq!((tokyo.lat, tokyo.lon), (35.6762, 139.6503));
    }

    #[test]
    fn default_world_pins_are_unique_city_names() {
        // FIX007.2 回归：默认常驻集 = 城市名（唯一显示；走时经 tz_of 映射 IANA；
        // name_of_tz 保留为旧存档装载期迁移辅助）
        let pins = crate::static_config::Static::get().base_str_list("default_world_pins");
        assert!(pins.len() >= 4, "默认常驻城市应有多条，实际 {}", pins.len());
        let mut sorted = pins.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), pins.len(), "默认常驻集城市名应唯一");
        for name in &pins {
            let tz = tz_of(name).unwrap_or_else(|| panic!("默认常驻应为表内城市: {name}"));
            let parsed: Result<chrono_tz::Tz, _> = tz.parse();
            assert!(parsed.is_ok(), "映射 IANA 应可解析: {name} → {tz}");
        }
        // 迁移辅助仍可用：IANA → 首匹配城市名（北京为 Asia/Shanghai 表序首城）
        assert_eq!(name_of_tz("Asia/Shanghai"), Some("北京"));
    }
}
