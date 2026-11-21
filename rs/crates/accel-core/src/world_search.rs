//! 世界时钟城市搜索（PL028）：50 城本地索引 + Open-Meteo geocoding 缓存。
//! SQLite 落盘（exe 同目录 world_cache.sqlite）；中文/英文/拼音/首字母匹配。

use std::path::PathBuf;
use std::sync::OnceLock;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::data::world_cities;

/// 城市条目（内置 50 城 / geocoding 结果 / 用户自定义城市共用）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldCityEntry {
    pub name: String,
    pub name_en: String,
    pub pinyin: String,
    pub initials: String,
    pub country: String,
    pub admin1: String,
    pub tz: String,
    pub lat: f64,
    pub lon: f64,
}

/// 搜索缓存（SQLite；geocode 结果 + 自定义城市）
pub struct WorldSearch {
    path: PathBuf,
}

impl Default for WorldSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldSearch {
    /// 生产路径：exe 同目录 world_cache.sqlite（与 weather_cache/ 同级）
    pub fn new() -> Self {
        Self::with_path(crate::util::paths::exe_dir().join("world_cache.sqlite"))
    }

    /// 测试注入路径
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// 内置 50 城 + 自定义城市的本地匹配（中文/英文/拼音/首字母）
    pub fn local_matches(&self, query: &str) -> Vec<WorldCityEntry> {
        let q = normalize_query(query);
        if q.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<WorldCityEntry> = builtin_entries()
            .iter()
            .filter(|e| matches_entry(e, &q))
            .cloned()
            .collect();
        out.extend(
            self.custom_cities()
                .into_iter()
                .filter(|e| matches_entry(e, &q)),
        );
        out.sort_by_key(|a| search_rank(a, &q));
        out.dedup_by(|a, b| a.name == b.name);
        out
    }

    /// 名称 → 内置或自定义条目
    pub fn entry_by_name(&self, name: &str) -> Option<WorldCityEntry> {
        if let Some(e) = builtin_entries().iter().find(|e| e.name == name) {
            return Some(e.clone());
        }
        self.custom_by_name(name)
    }

    /// 缓存命中：返回该 query 的 geocoding 结果
    pub fn cached(&self, query: &str) -> Option<Vec<WorldCityEntry>> {
        let q = normalize_query(query);
        if q.is_empty() {
            return None;
        }
        let conn = self.open().ok()?;
        let has: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM geo_cache WHERE query = ?1 LIMIT 1",
                params![q],
                |row| row.get(0),
            )
            .ok();
        has?;
        let mut stmt = conn
            .prepare(
                "SELECT name,name_en,pinyin,initials,country,admin1,tz,lat,lon \
                 FROM geo_result WHERE query=?1 ORDER BY rank",
            )
            .ok()?;
        let rows = stmt
            .query_map(params![q], row_to_entry)
            .ok()?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        if rows.is_empty() { None } else { Some(rows) }
    }

    /// 写入 geocoding 结果（事务；失败仅日志）
    pub fn store_candidates(&self, query: &str, results: &[WorldCityEntry]) {
        let q = normalize_query(query);
        if q.is_empty() || results.is_empty() {
            return;
        }
        let Ok(mut conn) = self.open() else {
            return;
        };
        let Ok(tx) = conn.transaction() else {
            tracing::warn!("世界城市缓存事务开启失败");
            return;
        };
        let now = epoch_secs();
        let _ = tx.execute("DELETE FROM geo_result WHERE query=?1", params![q]);
        let _ = tx.execute(
            "INSERT OR REPLACE INTO geo_cache(query,fetched_at) VALUES(?1,?2)",
            params![q, now],
        );
        for (rank, e) in results.iter().enumerate() {
            let _ = tx.execute(
                "INSERT OR REPLACE INTO geo_result(
                    query,rank,name,name_en,pinyin,initials,country,admin1,tz,lat,lon
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    q,
                    rank as i64,
                    e.name,
                    e.name_en,
                    e.pinyin,
                    e.initials,
                    e.country,
                    e.admin1,
                    e.tz,
                    e.lat,
                    e.lon
                ],
            );
        }
        if let Err(e) = tx.commit() {
            tracing::warn!("世界城市缓存写入失败: {e}");
        }
    }

    /// 用户自定义城市（geocoding 选中后落盘）
    pub fn custom_cities(&self) -> Vec<WorldCityEntry> {
        let Ok(conn) = self.open() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT name,name_en,pinyin,initials,country,admin1,tz,lat,lon \
             FROM custom_city ORDER BY created_at",
        ) else {
            return Vec::new();
        };
        stmt.query_map([], row_to_entry)
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default()
    }

    /// 自定义城市按名查
    pub fn custom_by_name(&self, name: &str) -> Option<WorldCityEntry> {
        let conn = self.open().ok()?;
        conn.query_row(
            "SELECT name,name_en,pinyin,initials,country,admin1,tz,lat,lon \
             FROM custom_city WHERE name=?1",
            params![name],
            row_to_entry,
        )
        .ok()
    }

    /// 新增/覆盖自定义城市
    pub fn add_custom(&self, entry: &WorldCityEntry) -> bool {
        let Ok(conn) = self.open() else {
            return false;
        };
        conn.execute(
            "INSERT OR REPLACE INTO custom_city(
                name,name_en,pinyin,initials,country,admin1,tz,lat,lon,created_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                entry.name,
                entry.name_en,
                entry.pinyin,
                entry.initials,
                entry.country,
                entry.admin1,
                entry.tz,
                entry.lat,
                entry.lon,
                epoch_secs()
            ],
        )
        .is_ok()
    }

    /// 删除自定义城市
    pub fn remove_custom(&self, name: &str) {
        if let Ok(conn) = self.open() {
            let _ = conn.execute("DELETE FROM custom_city WHERE name=?1", params![name]);
        }
    }

    fn open(&self) -> rusqlite::Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS geo_cache(
                query TEXT PRIMARY KEY,
                fetched_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS geo_result(
                query TEXT NOT NULL,
                rank INTEGER NOT NULL,
                name TEXT NOT NULL,
                name_en TEXT NOT NULL DEFAULT '',
                pinyin TEXT NOT NULL DEFAULT '',
                initials TEXT NOT NULL DEFAULT '',
                country TEXT NOT NULL DEFAULT '',
                admin1 TEXT NOT NULL DEFAULT '',
                tz TEXT NOT NULL,
                lat REAL NOT NULL,
                lon REAL NOT NULL,
                PRIMARY KEY(query, rank)
             );
             CREATE TABLE IF NOT EXISTS custom_city(
                name TEXT PRIMARY KEY,
                name_en TEXT NOT NULL DEFAULT '',
                pinyin TEXT NOT NULL DEFAULT '',
                initials TEXT NOT NULL DEFAULT '',
                country TEXT NOT NULL DEFAULT '',
                admin1 TEXT NOT NULL DEFAULT '',
                tz TEXT NOT NULL,
                lat REAL NOT NULL,
                lon REAL NOT NULL,
                created_at INTEGER NOT NULL
             );",
        )?;
        Ok(conn)
    }
}

/// 内置 50 城条目（拼音/英文别名只生成一次）
fn builtin_entries() -> &'static [WorldCityEntry] {
    static ENTRIES: OnceLock<Vec<WorldCityEntry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        world_cities::all()
            .iter()
            .map(|c| {
                let (pinyin, initials) = world_cities::pinyin_of(c.name);
                WorldCityEntry {
                    name: c.name.to_string(),
                    name_en: world_cities::english_of(c.name).unwrap_or("").to_string(),
                    pinyin,
                    initials,
                    country: String::new(),
                    admin1: String::new(),
                    tz: c.tz.to_string(),
                    lat: c.lat,
                    lon: c.lon,
                }
            })
            .collect()
    })
}

/// 内置城市按名查（世界页 hero/list 渲染用）
pub fn builtin_entry(name: &str) -> Option<WorldCityEntry> {
    builtin_entries().iter().find(|e| e.name == name).cloned()
}

/// IANA 时区是否可解析（自定义城市落库前校验）
pub fn valid_timezone(tz: &str) -> bool {
    tz.parse::<chrono_tz::Tz>().is_ok()
}

/// Open-Meteo geocoding URL（调用方用天气/代理通道取 JSON）
pub fn geocode_url(query: &str) -> String {
    let mut url = reqwest::Url::parse("https://geocoding-api.open-meteo.com/v1/search")
        .expect("geocoding base URL 合法");
    url.query_pairs_mut()
        .append_pair("name", query)
        .append_pair("count", "8")
        .append_pair("language", "zh")
        .append_pair("format", "json");
    url.to_string()
}

/// 解析 geocoding 响应；缺失 name/tz/lat/lon 的条目跳过
pub fn parse_geocode(value: &Value) -> Vec<WorldCityEntry> {
    let mut out = Vec::new();
    let Some(results) = value.get("results").and_then(Value::as_array) else {
        return out;
    };
    for item in results {
        let Some(name) = item.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(tz) = item.get("timezone").and_then(Value::as_str) else {
            continue;
        };
        let Some(lat) = item.get("latitude").and_then(Value::as_f64) else {
            continue;
        };
        let Some(lon) = item.get("longitude").and_then(Value::as_f64) else {
            continue;
        };
        let (pinyin, initials) = world_cities::pinyin_of(name);
        let name_en = if name.is_ascii() {
            name.to_string()
        } else {
            pinyin.clone()
        };
        out.push(WorldCityEntry {
            name: name.to_string(),
            name_en,
            pinyin,
            initials,
            country: item
                .get("country")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            admin1: item
                .get("admin1")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            tz: tz.to_string(),
            lat,
            lon,
        });
    }
    out
}

/// 搜索串归一：小写、去空白/撇号/连字符/点（拼音/英文/中文共用）
pub fn normalize_query(query: &str) -> String {
    query
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '\'' | '-' | '.' | '’'))
        .collect()
}

fn matches_entry(e: &WorldCityEntry, q: &str) -> bool {
    normalize_query(&e.name).contains(q)
        || normalize_query(&e.name_en).contains(q)
        || normalize_query(&e.pinyin).contains(q)
        || normalize_query(&e.initials).starts_with(q)
}

/// 排序：名称/英文/拼音前缀优先，再按包含
fn search_rank(e: &WorldCityEntry, q: &str) -> u8 {
    let n = normalize_query(&e.name);
    let en = normalize_query(&e.name_en);
    let py = normalize_query(&e.pinyin);
    if n.starts_with(q) || en.starts_with(q) || py.starts_with(q) {
        0
    } else if n.contains(q) || en.contains(q) || py.contains(q) {
        1
    } else {
        2
    }
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorldCityEntry> {
    Ok(WorldCityEntry {
        name: row.get(0)?,
        name_en: row.get(1)?,
        pinyin: row.get(2)?,
        initials: row.get(3)?,
        country: row.get(4)?,
        admin1: row.get(5)?,
        tz: row.get(6)?,
        lat: row.get(7)?,
        lon: row.get(8)?,
    })
}

fn epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_search(tag: &str) -> (WorldSearch, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("accel_world_search_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("临时目录");
        (WorldSearch::with_path(dir.join("world_cache.sqlite")), dir)
    }

    #[test]
    fn pinyin_and_english_aliases() {
        assert_eq!(
            world_cities::pinyin_of("北京"),
            ("beijing".to_string(), "bj".to_string())
        );
        assert_eq!(world_cities::english_of("东京"), Some("Tokyo"));
        assert_eq!(world_cities::english_of("纽约"), Some("New York"));
    }

    #[test]
    fn local_search_supports_cn_en_pinyin_initials() {
        let (search, dir) = temp_search("local");
        for q in [
            "北京", "beijing", "Beijing", "bj", "tokyo", "dongjing", "dj",
        ] {
            assert!(
                search
                    .local_matches(q)
                    .iter()
                    .any(|e| e.name == "北京" || e.name == "东京"),
                "query {q} should hit a built-in city"
            );
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn geocode_parse_and_cache_roundtrip() {
        let (search, dir) = temp_search("cache");
        let body = serde_json::json!({"results": [{
            "name": "苏州", "latitude": 31.3, "longitude": 120.6,
            "timezone": "Asia/Shanghai", "country": "中国", "admin1": "江苏"
        }]});
        let results = parse_geocode(&body);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tz, "Asia/Shanghai");
        search.store_candidates("suzhou", &results);
        let cached = search.cached("suzhou").expect("缓存应命中");
        assert_eq!(cached[0].name, "苏州");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn custom_city_add_and_remove() {
        let (search, dir) = temp_search("custom");
        let entry = WorldCityEntry {
            name: "测试城".to_string(),
            name_en: "Test City".to_string(),
            pinyin: "ceshicheng".to_string(),
            initials: "csc".to_string(),
            country: "测试国".to_string(),
            admin1: String::new(),
            tz: "Asia/Shanghai".to_string(),
            lat: 1.0,
            lon: 2.0,
        };
        assert!(search.add_custom(&entry));
        assert_eq!(search.custom_by_name("测试城"), Some(entry));
        search.remove_custom("测试城");
        assert!(search.custom_by_name("测试城").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}

// ===== src/world_search.rs 说明区 =====
// 输入：搜索串 / geocoding JSON / 自定义城市条目；输出：WorldCityEntry 列表/缓存
// 逻辑步骤：50 城本地索引（中文/英文/拼音/首字母）→ 未命中查 SQLite geo_cache →
//   调用方用 WeatherClient 代理通道取 geocoding JSON → parse_geocode → 落库
// 设计理由（PL028）：内置 50 城不跑网络；任意城市只在首次搜索请求 geocoding，
//   结果落 exe 同目录 SQLite，下次命中缓存；拼音由 pinyin crate 本地生成。
// 异常处理：SQLite 打开/写入失败仅日志并降级空；geocoding 缺字段条目跳过；
//   无结果返回空列表，不 panic。
// 关联配置：world_cache.sqlite（exe 同目录）；消费方：interface/world page
