//! 国家代码 → 中文大陆（世界时钟搜索弹层第二行）
//! ISO 3166-1 alpha-2 映射；未覆盖的国家按时区首段回退。

use std::collections::HashMap;
use std::sync::OnceLock;

const CONTINENTS_JSON: &str = include_str!("../../../../config/static/continents.json");

static CONTINENTS: OnceLock<HashMap<String, String>> = OnceLock::new();

/// 国家代码 → 中文大陆（未覆盖 None）
pub fn continent_of(country_code: &str) -> Option<&'static str> {
    CONTINENTS
        .get_or_init(|| {
            serde_json::from_str(CONTINENTS_JSON).expect("continents.json 解析失败（嵌入产物损坏）")
        })
        .get(country_code)
        .map(String::as_str)
}

/// 大陆文案：国家代码优先，缺失按时区首段回退；都缺返回空串
pub fn continent_label(country_code: &str, tz: &str) -> &'static str {
    if let Some(c) = continent_of(country_code) {
        return c;
    }
    match tz.split('/').next().unwrap_or("") {
        "Asia" => "亚洲",
        "Europe" => "欧洲",
        "Africa" => "非洲",
        "America" => "美洲",
        "Australia" | "Pacific" => "大洋洲",
        "Antarctica" => "南极洲",
        _ => "",
    }
}

// ===== 说明区 =====
// 输入：ISO 3166-1 alpha-2 国家代码、IANA 时区
// 输出：中文大陆名（亚洲/欧洲/非洲/北美洲/南美洲/大洋洲/南极洲）
// 逻辑步骤：国家代码查表；未覆盖时按 IANA 首段粗分（America 不细分南北）
// 设计理由：geocoding 不返回大陆字段；国家代码表比经纬度规则稳定，内置城市也能复用
// 异常处理：表缺键返回 None，最终回退空串；JSON 损坏在首次访问快速失败
// 关联配置：rs/config/static/continents.json（include_str 编译期嵌入）
