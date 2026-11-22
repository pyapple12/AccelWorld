//! 世界时钟城市搜索（PL028）：291 城 SQLite 主表（预置 db）+ Open-Meteo geocoding 缓存。
//! SQLite 落盘（exe 同目录 world_cache.sqlite）；中文/英文/拼音/首字母匹配。

use std::path::PathBuf;
use std::sync::OnceLock;

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::data::world_cities;

/// 城市条目（内置 291 城 / geocoding 结果 / 用户自定义城市共用）
#[derive(Debug, Clone, PartialEq)]
pub struct WorldCityEntry {
    pub name: String,
    pub name_en: String,
    pub pinyin: String,
    pub initials: String,
    pub country: String,
    pub country_code: String,
    pub admin1: String,
    pub tz: String,
    pub lat: f64,
    pub lon: f64,
    /// GeoNames 地点类型（PPLC/PPLA/PPL/AIRP…；搜索过滤用）
    pub feature_code: String,
    /// 人口（搜索过滤 + 弹层显示）
    pub population: i64,
}

/// 预置城市库（291 城 city 表；首次启动写到 exe 同目录）
const SEED_DB: &[u8] = include_bytes!("../../../assets/world_cache.sqlite");
/// 预置 city 行数（旧库检测）
const CITY_SEED_COUNT: i64 = 291;

/// 城市主表内存快照（来源 SQLite `city` 表；启动时加载）
static CITIES: OnceLock<Vec<WorldCityEntry>> = OnceLock::new();

/// 全局城市快照（未初始化时为空）
fn global_cities() -> &'static [WorldCityEntry] {
    CITIES.get_or_init(Vec::new)
}

/// 旧 50 城优先序（IANA → 城市名迁移时优先选这些，避免扩表后首匹配漂移）
const PREFERRED: &[&str] = &[
    "北京",
    "上海",
    "广州",
    "深圳",
    "杭州",
    "成都",
    "武汉",
    "西安",
    "重庆",
    "南京",
    "香港",
    "台北",
    "东京",
    "大阪",
    "首尔",
    "新加坡",
    "曼谷",
    "吉隆坡",
    "雅加达",
    "马尼拉",
    "孟买",
    "新德里",
    "迪拜",
    "利雅得",
    "伦敦",
    "巴黎",
    "柏林",
    "罗马",
    "马德里",
    "阿姆斯特丹",
    "苏黎世",
    "莫斯科",
    "伊斯坦布尔",
    "雅典",
    "纽约",
    "洛杉矶",
    "旧金山",
    "芝加哥",
    "多伦多",
    "温哥华",
    "墨西哥城",
    "圣保罗",
    "布宜诺斯艾利斯",
    "悉尼",
    "墨尔本",
    "珀斯",
    "奥克兰",
    "开罗",
    "约翰内斯堡",
    "内罗毕",
];

/// IANA 时区串 → 城市名（旧存档迁移；旧 50 城优先，再全表首匹配）
pub fn name_of_tz(tz: &str) -> Option<&'static str> {
    for name in PREFERRED {
        if let Some(c) = global_cities()
            .iter()
            .find(|c| c.name == *name && c.tz == tz)
        {
            return Some(c.name.as_str());
        }
    }
    global_cities()
        .iter()
        .find(|c| c.tz == tz)
        .map(|c| c.name.as_str())
}

/// 全部城市名（默认 pins 校验/测试用）
pub fn all_names() -> Vec<&'static str> {
    global_cities().iter().map(|c| c.name.as_str()).collect()
}

/// 名称 → 城市（SQLite 快照）
pub fn by_name(name: &str) -> Option<&'static WorldCityEntry> {
    global_cities().iter().find(|c| c.name == name)
}

/// 名称 → IANA 时区
pub fn tz_of(name: &str) -> Option<&'static str> {
    by_name(name).map(|c| c.tz.as_str())
}

/// 搜索缓存（SQLite；城市主表 + geocode 结果 + 自定义城市）
pub struct WorldSearch {
    path: PathBuf,
    cities: Vec<WorldCityEntry>,
}

impl Default for WorldSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldSearch {
    /// 生产路径：exe 同目录 world_cache.sqlite（与 weather_cache/ 同级）
    pub fn new() -> Self {
        let search = Self::with_path(crate::util::paths::exe_dir().join("world_cache.sqlite"));
        search.migrate_custom_cities();
        search
    }

    /// 测试/指定路径：确保预置库存在，加载 city 表快照
    pub fn with_path(path: PathBuf) -> Self {
        let mut search = Self {
            path,
            cities: Vec::new(),
        };
        search.cities = search.load_cities_or_seed();
        // 空快照不写入全局缓存——否则一次失败会让 CITIES 永久为空
        if !search.cities.is_empty() {
            let _ = CITIES.set(search.cities.clone());
        }
        search
    }

    /// 初始化并读取城市表；任何失败都记 error（不再静默空列表）
    fn load_cities_or_seed(&self) -> Vec<WorldCityEntry> {
        if let Err(e) = self.ensure_seed_db() {
            tracing::error!("世界城市库初始化失败: {}（{e}）", self.path.display());
        }
        match self.load_cities() {
            Ok(list) if !list.is_empty() => list,
            Ok(_) => {
                tracing::error!("世界城市库为空: {}", self.path.display());
                Vec::new()
            }
            Err(e) => {
                tracing::error!("世界城市库读取失败: {}（{e}）", self.path.display());
                Vec::new()
            }
        }
    }

    /// 预置库：文件不存在则写出嵌入 db；旧库 city 行数不符则事务重建 city 表；
    /// 文件损坏/无法打开则用「临时文件 + rename」整体重建（自定义城市会丢失，日志明示）
    fn ensure_seed_db(&self) -> rusqlite::Result<()> {
        if !self.path.exists() {
            self.write_seed_file(&self.path)?;
            return Ok(());
        }
        match self.sync_seed_table() {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!(
                    "世界城市库不可用，按预置库重建（自定义城市将丢失）: {}（{e}）",
                    self.path.display()
                );
                self.rebuild_from_seed()
            }
        }
    }

    /// 写预置库字节（父目录不存在时补建）
    fn write_seed_file(&self, path: &std::path::Path) -> rusqlite::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }
        std::fs::write(path, SEED_DB)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    /// 行数不符时从嵌入 seed 重建 city 表：ATTACH 必须在事务外，DELETE/INSERT 包事务
    fn sync_seed_table(&self) -> rusqlite::Result<()> {
        let mut conn = self.open()?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM city", [], |r| r.get(0))?;
        if count == CITY_SEED_COUNT {
            return Ok(());
        }
        let tmp = self.path.with_extension("seed.tmp");
        self.write_seed_file(&tmp)?;
        if let Err(e) = conn.execute(
            "ATTACH DATABASE ?1 AS seed",
            params![tmp.to_string_lossy().to_string()],
        ) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        let result = (|| -> rusqlite::Result<()> {
            let tx = conn.transaction()?;
            tx.execute("DELETE FROM city", [])?;
            tx.execute("INSERT INTO city SELECT * FROM seed.city", [])?;
            tx.commit()
        })();
        let detach = conn.execute("DETACH DATABASE seed", []);
        let _ = std::fs::remove_file(&tmp);
        result?;
        detach?;
        Ok(())
    }

    /// 整体重建：先写同目录临时文件，再删除旧文件并 rename（Windows rename 不覆盖）
    fn rebuild_from_seed(&self) -> rusqlite::Result<()> {
        let tmp = self.path.with_extension("rebuild.tmp");
        self.write_seed_file(&tmp)?;
        if self.path.exists() {
            std::fs::remove_file(&self.path)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }
        std::fs::rename(&tmp, &self.path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    fn load_cities(&self) -> rusqlite::Result<Vec<WorldCityEntry>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT name,name_en,pinyin,initials,country,country_code,admin1,population,tz,lat,lon \
             FROM city ORDER BY sort_order",
        )?;
        let rows = stmt.query_map([], row_to_city)?;
        rows.collect()
    }

    /// 旧自定义城市补全：本地表同名覆盖；国内城市补国家代码与拼音英文名
    pub fn migrate_custom_cities(&self) {
        for mut e in self.custom_cities() {
            if let Some(b) = builtin_entry(&e.name) {
                let _ = self.add_custom(&b);
                continue;
            }
            let mut changed = false;
            if e.country_code.is_empty() && e.country == "中国" {
                e.country_code = "CN".to_string();
                changed = true;
            }
            if matches!(e.country_code.as_str(), "CN" | "HK" | "TW" | "MO") {
                let (pinyin, _) = world_cities::pinyin_of(&e.name);
                if e.name_en != pinyin {
                    e.name_en = pinyin;
                    changed = true;
                }
            }
            if changed {
                let _ = self.add_custom(&e);
            }
        }
    }

    /// 内置 291 城 + 自定义城市的本地匹配（中文/英文/拼音/首字母）
    pub fn local_matches(&self, query: &str) -> Vec<WorldCityEntry> {
        let q = normalize_query(query);
        if q.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<WorldCityEntry> = self
            .cities
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
        if let Some(e) = self.cities.iter().find(|e| e.name == name) {
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
                "SELECT name,name_en,pinyin,initials,country,country_code,admin1,tz,lat,lon,feature_code,population \
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
                    query,rank,name,name_en,pinyin,initials,country,country_code,admin1,tz,lat,lon,feature_code,population
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    q,
                    rank as i64,
                    e.name,
                    e.name_en,
                    e.pinyin,
                    e.initials,
                    e.country,
                    e.country_code,
                    e.admin1,
                    e.tz,
                    e.lat,
                    e.lon,
                    e.feature_code,
                    e.population
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
            "SELECT name,name_en,pinyin,initials,country,country_code,admin1,tz,lat,lon,feature_code,population \
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
            "SELECT name,name_en,pinyin,initials,country,country_code,admin1,tz,lat,lon,feature_code,population \
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
                name,name_en,pinyin,initials,country,country_code,admin1,tz,lat,lon,feature_code,population,created_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                entry.name,
                entry.name_en,
                entry.pinyin,
                entry.initials,
                entry.country,
                entry.country_code,
                entry.admin1,
                entry.tz,
                entry.lat,
                entry.lon,
                entry.feature_code,
                entry.population,
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
        // schema 迁移只跑一次：旧库 user_version=0 时补表/补列后写 1
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version >= 1 {
            return Ok(conn);
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS city(
                name TEXT PRIMARY KEY,
                name_en TEXT NOT NULL DEFAULT '',
                pinyin TEXT NOT NULL DEFAULT '',
                initials TEXT NOT NULL DEFAULT '',
                country TEXT NOT NULL DEFAULT '',
                country_code TEXT NOT NULL DEFAULT '',
                admin1 TEXT NOT NULL DEFAULT '',
                population INTEGER NOT NULL DEFAULT 0,
                tz TEXT NOT NULL,
                lat REAL NOT NULL,
                lon REAL NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS geo_cache(
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
                country TEXT NOT NULL DEFAULT '', country_code TEXT NOT NULL DEFAULT '', feature_code TEXT NOT NULL DEFAULT '', population INTEGER NOT NULL DEFAULT 0,
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
                country TEXT NOT NULL DEFAULT '', country_code TEXT NOT NULL DEFAULT '', feature_code TEXT NOT NULL DEFAULT '', population INTEGER NOT NULL DEFAULT 0,
                admin1 TEXT NOT NULL DEFAULT '',
                tz TEXT NOT NULL,
                lat REAL NOT NULL,
                lon REAL NOT NULL,
                created_at INTEGER NOT NULL
             );",
        )?;
        // 旧库补列（列已存在时报错，忽略即可）
        let _ = conn.execute(
            "ALTER TABLE geo_result ADD COLUMN country_code TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE custom_city ADD COLUMN country_code TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE geo_result ADD COLUMN feature_code TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE custom_city ADD COLUMN feature_code TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE geo_result ADD COLUMN population INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE custom_city ADD COLUMN population INTEGER NOT NULL DEFAULT 0",
            [],
        );
        conn.pragma_update(None, "user_version", 1)?;
        Ok(conn)
    }
}

/// 内置城市按名查（世界页 hero/list 渲染用；生产来自 SQLite 快照）
pub fn builtin_entry(name: &str) -> Option<WorldCityEntry> {
    global_cities().iter().find(|e| e.name == name).cloned()
}

/// IANA 时区是否可解析（自定义城市落库前校验）
pub fn valid_timezone(tz: &str) -> bool {
    tz.parse::<chrono_tz::Tz>().is_ok()
}

/// Open-Meteo geocoding URL（调用方用天气/代理通道取 JSON；language 决定返回名称语言）
pub fn geocode_url(query: &str, language: &str) -> String {
    let mut url = reqwest::Url::parse("https://geocoding-api.open-meteo.com/v1/search")
        .expect("geocoding base URL 合法");
    let count = crate::static_config::Static::get()
        .base_i64("world_geocode_count")
        .max(1);
    url.query_pairs_mut()
        .append_pair("name", query)
        .append_pair("count", &count.to_string())
        .append_pair("language", language)
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
        let country_code = item
            .get("country_code")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        // 国内城市英文名直接用拼音；国外表外城市不靠 Open-Meteo 补英文
        let name_en = if matches!(country_code.as_str(), "CN" | "HK" | "TW" | "MO") {
            pinyin.clone()
        } else if name.is_ascii() {
            name.to_string()
        } else {
            String::new()
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
            country_code,
            admin1: item
                .get("admin1")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            tz: tz.to_string(),
            lat,
            lon,
            feature_code: item
                .get("feature_code")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            population: item.get("population").and_then(Value::as_i64).unwrap_or(0),
        });
    }
    out
}

/// 城市筛选：仅居民点；排除区/小地方/农场/废弃/县级乡级驻地；人口需达阈值
pub fn passes_city_filter(e: &WorldCityEntry, min_population: i64) -> bool {
    if !e.feature_code.starts_with("PPL") {
        return false;
    }
    if matches!(
        e.feature_code.as_str(),
        "PPLX" | "PPLL" | "PPLS" | "PPLF" | "PPLH" | "PPLQ" | "PPLW" | "PPLR" | "PPLA3" | "PPLA4"
    ) {
        return false;
    }
    e.population >= min_population
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

fn row_to_city(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorldCityEntry> {
    Ok(WorldCityEntry {
        name: row.get(0)?,
        name_en: row.get(1)?,
        pinyin: row.get(2)?,
        initials: row.get(3)?,
        country: row.get(4)?,
        country_code: row.get(5)?,
        admin1: row.get(6)?,
        population: row.get(7)?,
        tz: row.get(8)?,
        lat: row.get(9)?,
        lon: row.get(10)?,
        feature_code: String::new(),
    })
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorldCityEntry> {
    Ok(WorldCityEntry {
        name: row.get(0)?,
        name_en: row.get(1)?,
        pinyin: row.get(2)?,
        initials: row.get(3)?,
        country: row.get(4)?,
        country_code: row.get(5)?,
        admin1: row.get(6)?,
        tz: row.get(7)?,
        lat: row.get(8)?,
        lon: row.get(9)?,
        feature_code: row.get(10)?,
        population: row.get(11)?,
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

    /// 测试临时目录（项目根 .temp/，Drop 自动清理；不写用户临时目录）
    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp_dir(tag: &str) -> TempDir {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.temp/world_search")
            .join(format!("{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("临时目录");
        TempDir(dir)
    }

    fn temp_search(tag: &str) -> (WorldSearch, TempDir) {
        let dir = temp_dir(tag);
        (
            WorldSearch::with_path(dir.0.join("world_cache.sqlite")),
            dir,
        )
    }

    #[test]
    fn corrupt_db_is_rebuilt_from_seed() {
        // FIX013.08：损坏文件不再永久空表——临时文件 + rename 整体重建
        let dir = temp_dir("corrupt");
        let path = dir.0.join("world_cache.sqlite");
        std::fs::write(&path, b"this is not a sqlite database").expect("写坏库");
        let search = WorldSearch::with_path(path.clone());
        assert_eq!(
            search.cities.len(),
            CITY_SEED_COUNT as usize,
            "损坏库应按预置库重建"
        );
        assert_eq!(
            WorldSearch::with_path(path).cities.len(),
            CITY_SEED_COUNT as usize,
            "重建后的库应可再次正常打开"
        );
    }

    #[test]
    fn empty_city_table_is_resynced_from_seed() {
        // FIX013.08：库可打开但 city 表为空/行数不符 → 事务内从 seed 重建
        let dir = temp_dir("resync");
        let path = dir.0.join("world_cache.sqlite");
        let first = WorldSearch::with_path(path.clone());
        assert_eq!(first.cities.len(), CITY_SEED_COUNT as usize);
        let conn = rusqlite::Connection::open(&path).expect("打开城市库");
        conn.execute("DELETE FROM city", []).expect("清空 city");
        drop(conn);
        let second = WorldSearch::with_path(path);
        assert_eq!(
            second.cities.len(),
            CITY_SEED_COUNT as usize,
            "空 city 表应从 seed 恢复"
        );
    }

    #[test]
    fn schema_migration_sets_user_version_once() {
        // FIX013.28：DDL 只在 user_version<1 时执行，完成后写 1
        let dir = temp_dir("migrate");
        let path = dir.0.join("world_cache.sqlite");
        let _ = WorldSearch::with_path(path.clone());
        let conn = rusqlite::Connection::open(&path).expect("打开城市库");
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("读 user_version");
        assert_eq!(version, 1);
    }

    #[test]
    fn pinyin_and_english_aliases() {
        let (_search, _dir) = temp_search("aliases");
        assert_eq!(
            world_cities::pinyin_of("北京"),
            ("beijing".to_string(), "bj".to_string())
        );
        assert_eq!(builtin_entry("东京").expect("东京应在表").name_en, "Tokyo");
        assert_eq!(
            builtin_entry("纽约").expect("纽约应在表").name_en,
            "New York"
        );
    }

    #[test]
    fn local_search_supports_cn_en_pinyin_initials() {
        let (search, _dir) = temp_search("local");
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
    }

    #[test]
    fn geocode_parse_and_cache_roundtrip() {
        let (search, _dir) = temp_search("cache");
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
    }

    #[test]
    fn custom_city_add_and_remove() {
        let (search, _dir) = temp_search("custom");
        let entry = WorldCityEntry {
            name: "测试城".to_string(),
            name_en: "Test City".to_string(),
            pinyin: "ceshicheng".to_string(),
            initials: "csc".to_string(),
            country: "测试国".to_string(),
            country_code: "TT".to_string(),
            admin1: String::new(),
            feature_code: "PPLC".to_string(),
            population: 1_000_000,
            tz: "Asia/Shanghai".to_string(),
            lat: 1.0,
            lon: 2.0,
        };
        assert!(search.add_custom(&entry));
        assert_eq!(search.custom_by_name("测试城"), Some(entry));
        search.remove_custom("测试城");
        assert!(search.custom_by_name("测试城").is_none());
    }
}

// ===== src/world_search.rs 说明区 =====
// 输入：搜索串 / geocoding JSON / 自定义城市条目；输出：WorldCityEntry 列表/缓存
// 逻辑步骤：291 城 SQLite 主表（预置 db；中文/英文/拼音/首字母）→ 未命中查 geo_cache →
//   调用方用 WeatherClient 代理通道取 geocoding JSON → parse_geocode → 落库
// 设计理由（PL028）：城市主表启动从预置 world_cache.sqlite 加载；境内表外城市
//   才请求 geocoding，结果落 exe 同目录 SQLite；拼音由 pinyin crate 本地生成。
// 异常处理：SQLite 打开/写入失败仅日志并降级空；geocoding 缺字段条目跳过；
//   无结果返回空列表，不 panic。
// 关联配置：world_cache.sqlite（exe 同目录）；消费方：interface/world page
