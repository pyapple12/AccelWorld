//! 统一城市表（PL012.09：世界时钟/天气多城市共用的城市检索层）
//! config/static/world_cities.json 编译期嵌入；tz 逐条经 chrono-tz 编译期类型可解析

use std::sync::OnceLock;

use serde::Deserialize;

// 城市行（名称/IANA 时区/经纬度/大洲分组）
#[derive(Debug, Deserialize)]
pub struct WorldCity {
    pub name: &'static str,
    pub tz: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub region: &'static str,
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

/// 城市名 → (lat, lon)（天气查询坐标检索）
pub fn coords_of(name: &str) -> Option<(f64, f64)> {
    by_name(name).map(|c| (c.lat, c.lon))
}

// ===== src/data/world_cities.rs 说明区 =====
// 输入：rs/config/static/world_cities.json（48 城静态表）；输出：强类型检索
// 设计理由：世界时钟（PL014 城市管理）与天气多城市共用同一城市源，杜绝两表漂移；
//   表放 config/static（随版本发布不运行时改）而加载器在 data/（数据访问职责）——
//   include_str 路径跨约定，登记 z.plan；tz 存字符串，消费侧 chrono_tz 解析
//   （编译期无法对 JSON 内容静态校验，故单测遍历全表运行期断言可解析）
// 数据来源：原 17 城坐标自 Python CITIES 平移；增补城市经纬度/IANA 按公开
//   地理常识核对（PL012.09 落位，后续可按需扩至 60 上限内）
// 关联依赖：serde_json；消费方：interface 城市域（PL012.10）、PL014 世界时钟/天气页
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
        // 检索命中/None 两分支 + 锚点值
        assert_eq!(tz_of("北京"), Some("Asia/Shanghai"));
        assert_eq!(tz_of("纽约"), Some("America/New_York"));
        assert_eq!(tz_of("悉尼"), Some("Australia/Sydney"));
        assert!(by_name("不存在城").is_none());
        let (lat, lon) = coords_of("东京").expect("东京应在表");
        assert_eq!((lat, lon), (35.6762, 139.6503));
    }
}
