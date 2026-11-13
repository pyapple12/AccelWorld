//! 农历日缓存（PL020）：宜忌/农历串/中文日期按日持久化——启动读一次、跨日写一次。
//! 目的：同一天反复打开 app 不重算黄历（lunar-rust 的 day_yi/ji 每次克隆大表）。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 单日缓存（文件内容）
#[derive(Serialize, Deserialize)]
pub struct DayCache {
    /// 缓存结构版本（缺字段的旧缓存视为过期，重算）
    #[serde(default)]
    pub version: u32,
    /// YYYY-MM-DD
    pub date: String,
    pub chinese_date: String,
    pub lunar_info: String,
    pub yi: Vec<String>,
    pub ji: Vec<String>,
    #[serde(default)]
    pub jieqi: String,
    #[serde(default)]
    pub festival: String,
}

/// 当前缓存结构版本
pub const CACHE_VERSION: u32 = 1;

fn cache_path() -> PathBuf {
    crate::util::paths::exe_dir()
        .join("config")
        .join("lunar_day_cache.json")
}

/// 读取当日缓存；文件缺失/损坏/版本过旧/日期不匹配均返 None（调用方重算）
pub fn load(year: i32, month: u32, day: u32) -> Option<DayCache> {
    let text = std::fs::read_to_string(cache_path()).ok()?;
    let cache: DayCache = serde_json::from_str(&text).ok()?;
    if cache.version != CACHE_VERSION {
        return None;
    }
    let want = format!("{year:04}-{month:02}-{day:02}");
    (cache.date == want).then_some(cache)
}

/// 原子写当日缓存；失败只记警告（缓存不是正确性依赖）
pub fn save(cache: &DayCache) {
    let path = cache_path();
    if let Some(dir) = path.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        tracing::warn!("农历日缓存目录创建失败: {e}");
        return;
    }
    let Ok(text) = serde_json::to_string_pretty(cache) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp, text) {
        tracing::warn!("农历日缓存写入失败: {e}");
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        tracing::warn!("农历日缓存替换失败: {e}");
        let _ = std::fs::remove_file(&tmp);
    }
}

// ===== src/day_cache.rs 说明区 =====
// 输入：年月日 / DayCache；输出：当日缓存 Option / 磁盘文件 config/lunar_day_cache.json
// 逻辑步骤：load 读文件 → serde → 日期比对；save 建目录 → 写 .tmp → rename 原子替换
// 设计理由（PL020）：宜忌每天只变一次，且 lunar-rust 每次调用克隆大表；持久化后
//   同一天反复启动也只算一次，跨日才重写
// 异常处理：读写/解析失败一律降级（返 None 或记 warn），不阻塞时间推进
// 关联依赖：serde_json、crate::util::paths::exe_dir；消费方：time_dilation::time_info
