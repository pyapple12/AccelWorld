//! 城市表（data/cities.py 的 Rust 对等物）
//! include_str 嵌入 rs/data/cities.json（与 Python CITIES 插入序一致），强类型检索

use std::sync::OnceLock;

use serde::Deserialize;

// 城市行（名称 + 经纬度，借用自编译期嵌入的 JSON 静态串）
#[derive(Debug, Deserialize)]
pub struct CityEntry {
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
}

const CITIES_JSON: &str = include_str!("../../../../data/cities.json");

static CITIES: OnceLock<Vec<CityEntry>> = OnceLock::new();

/// 全表（Python CITIES 插入序：北京/上海/…，下拉展示序 parity）
pub fn all() -> &'static [CityEntry] {
    CITIES.get_or_init(|| {
        serde_json::from_str(CITIES_JSON).expect("cities.json 解析失败（嵌入产物损坏）")
    })
}

/// 城市名 → 经纬度（Python CITIES.get(name) 对等，未知城市返 None）
pub fn get(name: &str) -> Option<(f64, f64)> {
    all()
        .iter()
        .find(|c| c.name == name)
        .map(|c| (c.lat, c.lon))
}

/// 全部城市名（插入序）
pub fn names() -> Vec<&'static str> {
    all().iter().map(|c| c.name).collect()
}

#[cfg(test)]
mod tests {
    use super::{all, get, names};

    #[test]
    fn entry_count_and_order_anchor() {
        // 条目数与首尾锚定（对 Python 导出夹具：17 城，北京头台北尾）
        let all = all();
        assert_eq!(all.len(), 17);
        assert_eq!(all[0].name, "北京");
        assert_eq!(all[16].name, "台北");
        assert_eq!(names().len(), 17);
    }

    #[test]
    fn coords_lookup() {
        // 样例值逐一相等（夹具锚点：北京/上海坐标）
        assert_eq!(get("北京"), Some((39.9042, 116.4074)));
        assert_eq!(get("上海"), Some((31.2304, 121.4737)));
        assert_eq!(get("不存在的城市"), None);
    }
}
