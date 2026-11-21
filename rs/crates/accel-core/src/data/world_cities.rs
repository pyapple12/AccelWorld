//! 城市名 → 拼音/首字母（PL028）；城市主表已迁 `world_cache.sqlite` 的 `city` 表。
//! 本模块只保留纯文本工具，不持有任何城市数据。

/// 中文城市名 → (全拼, 首字母串)（本地搜索/缓存迁移用；不依赖网络）
pub fn pinyin_of(name: &str) -> (String, String) {
    use pinyin::ToPinyin;
    let mut full = String::new();
    let mut initials = String::new();
    for p in name.to_pinyin().flatten() {
        full.push_str(p.plain());
        initials.push_str(p.first_letter());
    }
    (full, initials)
}

// ===== 说明区 =====
// 输入：中文城市名
// 输出：(全拼, 首字母串)
// 设计理由：城市主数据在 SQLite `city` 表（预置 world_cache.sqlite），本模块不再
//   include_str JSON；拼音只服务 geocoding 结果与旧缓存迁移
// 异常处理：无 IO/DB，纯字符串处理
// 关联依赖：pinyin crate；消费方：world_search / settings 迁移
