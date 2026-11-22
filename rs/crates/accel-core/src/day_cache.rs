//! 农历日缓存（PL020）：宜忌/农历日期/中文日期按日持久化——启动读一次、跨日写一次。
//! 时辰不缓存：农历串存 `{时辰}` 占位，`time_info` 每次按墙钟小时替换。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::static_config::Static;

/// 单日缓存（文件内容）
#[derive(Serialize, Deserialize)]
pub struct DayCache {
    /// 缓存结构版本（缺字段的旧缓存视为过期，重算）
    #[serde(default)]
    pub version: u32,
    /// YYYY-MM-DD
    pub date: String,
    pub chinese_date: String,
    /// 农历展示模板：时辰位置存 `chinese_calendar::SHICHEN_PLACEHOLDER`
    pub lunar_info: String,
    pub yi: Vec<String>,
    pub ji: Vec<String>,
    #[serde(default)]
    pub jieqi: String,
    #[serde(default)]
    pub festival: String,
}

/// 当前缓存结构版本（2：时辰从缓存剥离为占位符；v1 直接判过期重算）
pub const CACHE_VERSION: u32 = 2;

/// 生产缓存路径（base.lunar_cache token + exe 同目录）
pub fn default_path() -> PathBuf {
    crate::util::paths::exe_dir().join(Static::get().base_str("lunar_cache"))
}

/// 读取指定路径的当日缓存；缺失/损坏/版本过旧/日期不匹配均返 None（调用方重算）
pub fn load(path: &Path, year: i32, month: u32, day: u32) -> Option<DayCache> {
    let text = std::fs::read_to_string(path).ok()?;
    let cache: DayCache = serde_json::from_str(&text).ok()?;
    if cache.version != CACHE_VERSION {
        return None;
    }
    let want = format!("{year:04}-{month:02}-{day:02}");
    (cache.date == want).then_some(cache)
}

/// 原子写指定路径的当日缓存；失败只记警告（缓存不是正确性依赖）
pub fn save(path: &Path, cache: &DayCache) {
    if let Some(dir) = path.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        tracing::warn!("农历日缓存目录创建失败: {e}");
        return;
    }
    let text = match serde_json::to_string_pretty(cache) {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!("农历日缓存序列化失败: {e}");
            return;
        }
    };
    // tmp 带 pid：多实例同日写同一路径时互不覆盖
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    if let Err(e) = std::fs::write(&tmp, text) {
        tracing::warn!("农历日缓存写入失败: {e}");
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        tracing::warn!("农历日缓存替换失败: {e}");
        let _ = std::fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.temp/day_cache");
        std::fs::create_dir_all(&dir).expect("创建 .temp 测试目录");
        dir.join(format!("{tag}_{}.json", std::process::id()))
    }

    fn sample(date: &str) -> DayCache {
        DayCache {
            version: CACHE_VERSION,
            date: date.to_string(),
            chinese_date: "2026年09月23日 星期三".to_string(),
            lunar_info: "丙午年（马年）八月十三{时辰} 月相：渐盈凸月 财神：正北方向（坎位）"
                .to_string(),
            yi: vec!["祈福".to_string()],
            ji: vec!["动土".to_string()],
            jieqi: "秋分".to_string(),
            festival: String::new(),
        }
    }

    #[test]
    fn roundtrip_and_date_guard() {
        let path = temp_path("roundtrip");
        save(&path, &sample("2026-09-23"));
        let hit = load(&path, 2026, 9, 23).expect("同日应命中");
        assert_eq!(hit.lunar_info, sample("2026-09-23").lunar_info);
        assert!(load(&path, 2026, 9, 24).is_none(), "跨日应判过期");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn version_mismatch_rejected() {
        let path = temp_path("version");
        let mut old = sample("2026-09-23");
        old.version = CACHE_VERSION - 1;
        save(&path, &old);
        assert!(load(&path, 2026, 9, 23).is_none(), "旧版本缓存应重算");
        let _ = std::fs::remove_file(path);
    }
}

// ===== src/day_cache.rs 说明区 =====
// 输入：缓存路径 / 年月日 / DayCache；输出：当日缓存 Option / 原子写入文件
// 逻辑步骤：load 读文件 → serde → 日期比对；save 建目录 → 写 pid.tmp → rename 替换
// 设计理由（PL020/FIX013.23）：宜忌每天只变一次，且 lunar-rust 每次调用克隆大表；
//   持久化后同一天反复启动也只算一次；时辰存占位符避免跨小时冻结
// 异常处理：读写/解析/序列化失败一律降级（返 None 或 warn），不阻塞时间推进
// 关联配置：base.lunar_cache（默认 config/lunar_day_cache.json）；消费方：time_dilation
