//! 农历/干支/节气/节日（modules/chinese_calendar.py 的 Rust 对等物）
//! lunar-rust 直调六字段 + 本地三表（时辰/财神/西式节日）+ holidays.json 三级兜底
//! 出入清单见 z.plan 修订二·PL012 B'（2026-10-16 实测 749 条比对定案）

use std::collections::HashMap;
use std::sync::OnceLock;

use chrono::{DateTime, Datelike, Local, NaiveDate, Timelike};
use lunar_rust::lunar::LunarRefHelper;
use lunar_rust::solar::SolarRefHelper;
use serde::Deserialize;

// 时辰映射（Python SHI_CHEN 同表：23-24 与 0-1 同属子时）
const SHI_CHEN: [(u32, u32, &str); 13] = [
    (0, 1, "子时"),
    (1, 3, "丑时"),
    (3, 5, "寅时"),
    (5, 7, "卯时"),
    (7, 9, "辰时"),
    (9, 11, "巳时"),
    (11, 13, "午时"),
    (13, 15, "未时"),
    (15, 17, "申时"),
    (17, 19, "酉时"),
    (19, 21, "戌时"),
    (21, 23, "亥时"),
    (23, 24, "子时"),
];

// 财神方位（公历月 → (方向, 卦位)；Python Cai_SHEN_DIRECTION 导出）
const CAI_SHEN_DIRECTION: [(&str, &str); 12] = [
    ("正北", "坎位"),
    ("东北", "艮位"),
    ("正东", "震位"),
    ("东南", "巽位"),
    ("正南", "离位"),
    ("西南", "坤位"),
    ("正西", "兑位"),
    ("西北", "乾位"),
    ("正北", "坎位"),
    ("东北", "艮位"),
    ("正东", "震位"),
    ("正南", "离位"),
];

// 西式/公历节日表（Python CUSTOM_HOLIDAYS 同表，T3 兜底）
const CUSTOM_HOLIDAYS: [((u32, u32), &str); 13] = [
    ((1, 1), "元旦"),
    ((2, 14), "情人节"),
    ((3, 8), "妇女节"),
    ((3, 12), "植树节"),
    ((4, 1), "愚人节"),
    ((5, 1), "劳动节"),
    ((5, 4), "青年节"),
    ((6, 1), "儿童节"),
    ((7, 1), "建党节"),
    ((8, 1), "建军节"),
    ((9, 10), "教师节"),
    ((10, 1), "国庆节"),
    ((12, 25), "圣诞节"),
];

// 星期中文映射（chinese_date 格式化）
const WEEKDAY_NAMES: [&str; 7] = [
    "星期一",
    "星期二",
    "星期三",
    "星期四",
    "星期五",
    "星期六",
    "星期日",
];

// 节假日表（T2：chinese-calendar [2024,2026] 全量导出，中文名已内嵌）
const HOLIDAYS_JSON: &str = include_str!("../../../config/static/holidays.json");

/// 农历信息聚合（Python LunarInfo dataclass 镜像）
#[derive(Debug, Clone, PartialEq)]
pub struct LunarInfo {
    /// 天干地支年（如"丙午年"）
    pub lunar_year: String,
    /// 生肖
    pub shengxiao: String,
    /// 农历月（中文，含闰前缀）
    pub lunar_month: String,
    /// 农历日（中文）
    pub lunar_day: String,
    /// 时辰
    pub shichen: String,
    /// 月相（如"蛾眉月"）
    pub yue_phase: String,
    /// 节气（无则空字符串）
    pub jieqi: String,
    /// 公历节日（无则空字符串）
    pub public_holiday: String,
    /// 财神方向
    pub cai_shen_dir: String,
    /// 八卦方位
    pub position: String,
}

// holidays.json 结构：{"2026": {"01-01": "元旦", ...}}
#[derive(Deserialize)]
struct HolidaysDoc(HashMap<String, HashMap<String, String>>);

static HOLIDAYS: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();

/// holidays 数据是否覆盖指定年份（FIX012.08 纯函数可测——运维触发信号：
/// 2027 安排公布后重跑导出脚本前的静默缺失不再无日志）
fn holidays_covers_year(table: &HashMap<String, HashMap<String, String>>, year: i32) -> bool {
    table.contains_key(&year.to_string())
}

fn holidays_table() -> &'static HashMap<String, HashMap<String, String>> {
    HOLIDAYS.get_or_init(|| {
        let table: HashMap<String, HashMap<String, String>> =
            serde_json::from_str::<HolidaysDoc>(HOLIDAYS_JSON)
                .expect("holidays.json 解析失败（嵌入产物损坏）")
                .0;
        let year = chrono::Local::now().year();
        if !holidays_covers_year(&table, year) {
            tracing::warn!(
                "holidays 数据不含当年 {year}，法定节假日降级为空——年末安排公布后重跑导出脚本"
            );
        }
        table
    })
}

/// 年份节假日表（Python get_holiday_detail 对等数据面）
/// None = 年份超数据范围（优雅降级空表语义，对齐 NotImplementedError 捕获路径）
pub fn holidays(year: i32) -> Option<&'static HashMap<String, String>> {
    holidays_table().get(&year.to_string())
}

// ===== 节日派生（PL014.07：倒计时默认目标与节日 chips 数据源）=====

/// 传统节日农历规则表（农历月, 农历日, 名称；除夕单独按次年正月初一回退一天）
const LUNAR_FESTIVAL_RULES: [(i64, i64, &str); 8] = [
    (1, 1, "春节"),
    (1, 15, "元宵节"),
    (5, 5, "端午节"),
    (7, 7, "七夕节"),
    (7, 15, "中元节"),
    (8, 15, "中秋节"),
    (9, 9, "重阳节"),
    (12, 8, "腊八节"),
];

/// 农历日期 → 公历（lunar-rust Lunar 构造转 Solar）
fn lunar_ymd_to_solar(lunar_year: i64, lunar_month: i64, lunar_day: i64) -> NaiveDate {
    let solar = lunar_rust::lunar::from_ymd(lunar_year, lunar_month, lunar_day).get_solar();
    NaiveDate::from_ymd_opt(
        solar.get_year() as i32,
        solar.get_month() as u32,
        solar.get_day() as u32,
    )
    .expect("lunar 转公历日期必合法")
}

/// 某农历年的正月初一公历（春节）
pub fn spring_festival_of_lunar_year(lunar_year: i32) -> NaiveDate {
    lunar_ymd_to_solar(lunar_year as i64, 1, 1)
}

/// 下一个春节公历日期（now 当天含当日；已过本年春节则取次年）
pub fn next_spring_festival(now: NaiveDate) -> NaiveDate {
    let mut lunar_year = now.year();
    loop {
        let d = spring_festival_of_lunar_year(lunar_year);
        if d >= now {
            return d;
        }
        lunar_year += 1;
    }
}

/// 清明公历日期（lunar-rust 节气表；锚定当年任一日的 lunar 实例取全表）
fn qingming_of_gregorian_year(year: i32) -> Option<NaiveDate> {
    let lunar = lunar_rust::solar::from_ymd(year as i64, 6, 1).get_lunar();
    lunar.get_jie_qi_table().get("清明").and_then(|solar| {
        NaiveDate::from_ymd_opt(
            solar.get_year() as i32,
            solar.get_month() as u32,
            solar.get_day() as u32,
        )
    })
}

/// 全年节日 chips 派生（PL014.07）：法定（holidays 当年+次年）+ 传统（农历规则 +
/// 清明节气 + 除夕回退）合并去重（传统覆盖同名法定日）→ 排序 → 截 [now, now+365]
/// 返回 (公历日, 名称) 升序列表；倒计时默认目标 = next_spring_festival（UI 派生不落盘）
pub fn year_chips(now: NaiveDate) -> Vec<(NaiveDate, String)> {
    let mut merged: std::collections::BTreeMap<NaiveDate, String> =
        std::collections::BTreeMap::new();
    for gregorian_year in [now.year(), now.year() + 1] {
        // 法定（holidays.json：中文名内嵌；超范围年静默无条目）
        if let Some(table) = holidays(gregorian_year) {
            for (md, name) in table {
                if let Ok(d) =
                    NaiveDate::parse_from_str(&format!("{gregorian_year}-{md}"), "%Y-%m-%d")
                {
                    merged.entry(d).or_insert_with(|| name.clone());
                }
            }
        }
        // 传统（农历年 = 公历年对齐：春节/腊八等跨年项由次年轮补齐）
        for (m, d, name) in LUNAR_FESTIVAL_RULES {
            merged.insert(
                lunar_ymd_to_solar(gregorian_year as i64, m, d),
                name.to_string(),
            );
        }
        // 清明（节气法，年际漂移随节气表）
        if let Some(qm) = qingming_of_gregorian_year(gregorian_year) {
            merged.insert(qm, "清明节".to_string());
        }
        // 除夕 = 次年正月初一 - 1 天（含腊月廿九/三十大小月差异）
        let eve = spring_festival_of_lunar_year(gregorian_year + 1).pred_opt();
        if let Some(eve) = eve {
            merged.insert(eve, "除夕".to_string());
        }
    }
    let horizon = now + chrono::Duration::days(365);
    merged
        .into_iter()
        .filter(|(d, _)| *d >= now && *d <= horizon)
        .collect()
}

/// 农历计算核心（Python get_chinese_lunar_calendar 对等）
/// 三级节日兜底：T1 lunar-rust 农历节日 → T2 holidays.json → T3 西式节日表
pub fn get_chinese_lunar_calendar(year: i32, month: u32, day: u32, hour: u32) -> LunarInfo {
    let lunar = lunar_rust::solar::from_ymd(year as i64, month as i64, day as i64).get_lunar();

    // 时辰：双档子时表匹配（Python 同序首中即取）
    let shichen = SHI_CHEN
        .iter()
        .find(|(start, end, _)| *start <= hour && hour < *end)
        .map(|(_, _, name)| *name)
        .unwrap_or("子时");

    // 节气（非节气日 lunar-rust 返空串，与 Python str(None→"") 语义一致）
    let jieqi = lunar.get_jie_qi();

    // 公历节日三级兜底
    let festivals = lunar.get_festivals();
    let mut public_holiday = festivals.first().cloned().unwrap_or_default();
    if public_holiday.is_empty()
        && let Some(table) = holidays(year)
    {
        public_holiday = table
            .get(&format!("{month:02}-{day:02}"))
            .cloned()
            .unwrap_or_default();
    }
    if public_holiday.is_empty() {
        public_holiday = CUSTOM_HOLIDAYS
            .iter()
            .find(|((m, d), _)| *m == month && *d == day)
            .map(|(_, name)| (*name).to_string())
            .unwrap_or_default();
    }

    // 财神方位（公历月 → (方向, 卦位)；Python Cai_SHEN_DIRECTION 导出；月索引越界兜底见下）
    let (cai_shen_dir, position) = CAI_SHEN_DIRECTION
        .get((month.saturating_sub(1)) as usize)
        .copied()
        .unwrap_or(("正北", "坎位")); // A005 #29：域外月（0/13+）不 panic，取首档兜底

    LunarInfo {
        lunar_year: format!("{}年", lunar.get_year_in_gan_zhi()),
        shengxiao: lunar.get_year_sheng_xiao(),
        lunar_month: format!("{}月", lunar.get_month_in_chinese()),
        lunar_day: lunar.get_day_in_chinese(),
        shichen: shichen.to_string(),
        yue_phase: format!("{}月", lunar.get_yue_xiang()),
        jieqi,
        public_holiday,
        cai_shen_dir: cai_shen_dir.to_string(),
        position: position.to_string(),
    }
}

/// 中文日期字符串（Python get_chinese_date 对等："YYYY年MM月DD日 星期X"）
pub fn get_chinese_date(now: DateTime<Local>) -> String {
    let weekday = WEEKDAY_NAMES[now.weekday().num_days_from_monday() as usize];
    now.format("%Y年%m月%d日").to_string() + " " + weekday
}

/// 农历展示文本（Python get_lunar_info 对等，拼装契约逐字符对齐）
/// `{年}（{生肖}年）{月}{日}{时辰} 月相：{X} 财神：{Y}方向（{Z}）`
/// 节气/节日不入串（FIX003.5 定案延续）；值段不含拆分标记字样（FIX004.3）
pub fn get_lunar_info(now: DateTime<Local>) -> String {
    let info = get_chinese_lunar_calendar(now.year(), now.month(), now.day(), now.hour());
    format!(
        "{}（{}年）{}{}{} 月相：{} 财神：{}方向（{}）",
        info.lunar_year,
        info.shengxiao,
        info.lunar_month,
        info.lunar_day,
        info.shichen,
        info.yue_phase,
        info.cai_shen_dir,
        info.position
    )
}

// ===== src/chinese_calendar.rs 说明区 =====
// 输入：公历年月日时（或 DateTime<Local>）；输出：LunarInfo / 农历展示串 / 中文日期串 /
//   节日派生群（PL014.07：spring_festival_of_lunar_year 春节公历日 /
//   next_spring_festival 未来最近春节 / year_chips 近期节日条 /
//   lunar_ymd_to_solar 农历转公历 / qingming_of_gregorian_year 清明节气日）
// 逻辑步骤：lunar-rust 直调六字段（干支/生肖/月/日/月相/节气）→ 时辰表/财神表/
//   西式节日表本地匹配 → 节日三级兜底（T1 农历节日 → T2 holidays.json → T3 表）；
//   节日派生：两年轮（当年+次年）扫法定 holidays 表合并 + LUNAR_FESTIVAL_RULES
//   全部 8 项农历规则 + 清明节气表取值 + 除夕回退（腊月最后一天），horizon 365
//   天过滤后按日升序出 chips（除夕回退 = 腊月最后一天，无「除夕」键年份走此路）
// 设计理由：三分法出入清单（z.plan B'）实测定案——lunar-rust 与 lunar-python 同源，
//   六字段 749 条比对全等直调；时辰/财神/西式节日三表纯移植；T2 数据文件化将
//   chinese-calendar 依赖固化为静态数据（年末维护改为重跑导出脚本，y.problems#7
//   春节日期部分根治——节假日随数据文件更新不再随依赖版本漂移）
// 降级路径：年份超 holidays 范围（>2026）→ T2 空表继续走 T3/空串，不 panic
//   （对齐 Python NotImplementedError 捕获）；lunar-rust 天文算法无年份上限
// 已知偏移：①T2 中文名在导出期经 HOLIDAY_TRANSLATION 中文化，运行时不需翻译表
//   （Python 运行时翻译）；②T1 优先级实测：除夕等农历节日覆盖 T2 跨度名（与
//   Python 三级顺序一致，夹具 2026-02-16 实证）
// 关联依赖：lunar_rust（SolarRefHelper/LunarRefHelper trait）；holidays.json（T2）
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn local(y: i32, m: u32, d: u32, h: u32) -> DateTime<Local> {
        let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)
            .expect("日期合法")
            .and_hms_opt(h, 0, 0)
            .expect("时刻合法");
        Local.from_local_datetime(&naive).single().expect("无歧义")
    }

    #[test]
    fn anchor_day_fields() {
        // 锚点日全字段（2026-09-23 12 时，夹具真值：秋分/丙午年马年八月十三午时/财神正北）
        let info = get_chinese_lunar_calendar(2026, 9, 23, 12);
        assert_eq!(info.lunar_year, "丙午年");
        assert_eq!(info.shengxiao, "马");
        assert_eq!(info.lunar_month, "八月");
        assert_eq!(info.lunar_day, "十三");
        assert_eq!(info.jieqi, "秋分");
        assert_eq!(info.shichen, "午时");
        assert_eq!(info.yue_phase, "渐盈凸月");
        assert_eq!(info.cai_shen_dir, "正北");
        assert_eq!(info.position, "坎位");
        assert_eq!(info.public_holiday, "");
    }

    #[test]
    fn lunar_info_str_shape() {
        // 拼装契约（夹具真值逐字符）：无"拜"前缀混入月相段（FIX004.3），节气/节日不入串
        let s = get_lunar_info(local(2026, 9, 23, 12));
        assert_eq!(
            s,
            "丙午年（马年）八月十三午时 月相：渐盈凸月 财神：正北方向（坎位）"
        );
        assert!(!s.contains("秋分"), "节气不得入串");
        assert!(!s.contains("拜"), "月相段不得混入「拜」字");
    }

    #[test]
    fn jieqi_boundary_plus_minus_one_day() {
        // 节气边界 ±1：当日有名、前后为空（2026 秋分 09-23）
        let at = get_chinese_lunar_calendar(2026, 9, 23, 12).jieqi;
        let before = get_chinese_lunar_calendar(2026, 9, 22, 12).jieqi;
        let after = get_chinese_lunar_calendar(2026, 9, 24, 12).jieqi;
        assert_eq!(at, "秋分");
        assert_eq!(before, "");
        assert_eq!(after, "");
    }

    #[test]
    fn degradation_year_2035_safe_path() {
        // 超范围年（2035 > holidays 数据上界）：全流程不 panic，T2 空 → T3/空兜底
        let info = get_chinese_lunar_calendar(2035, 1, 1, 12);
        assert!(!info.lunar_year.is_empty());
        assert_eq!(info.public_holiday, "元旦", "T3 元旦兜底应命中");
        let s = get_lunar_info(local(2035, 6, 15, 8));
        assert!(!s.is_empty(), "农历串应正常产出");
        assert!(holidays(2035).is_none(), "超范围年应返回 None");
    }

    #[test]
    fn out_of_domain_inputs_do_not_panic() {
        // A005 #29 回归：域外入参走安全路径（原实现 rem_euclid(0) / 数组越界会 panic）
        let info = get_chinese_lunar_calendar(2026, 13, 1, 12);
        assert_eq!(info.cai_shen_dir, "正北", "越界月取首档兜底");
        let info0 = get_chinese_lunar_calendar(2026, 0, 1, 12);
        assert_eq!(info0.cai_shen_dir, "正北");
    }

    #[test]
    fn holidays_covers_year_check() {
        // FIX012.08（A012 #8）验收：数据覆盖判定纯函数——真实嵌入表含 2026 不含 2027
        let table = holidays_table();
        assert!(holidays_covers_year(table, 2026), "数据上界年份应覆盖");
        assert!(
            !holidays_covers_year(table, 2027),
            "上界外年份应判不覆盖（触发装载期 warn）"
        );
    }

    #[test]
    fn holiday_tier_priority() {
        // 三级兜底优先级：T1 除夕覆盖 T2 春节跨度名（2026-02-16 夹具实证）
        let eve = get_chinese_lunar_calendar(2026, 2, 16, 12);
        assert_eq!(eve.public_holiday, "除夕");
        // T2 跨度日（02-15 非 T1 命中 → holidays.json 春节）
        let span = get_chinese_lunar_calendar(2026, 2, 15, 12);
        assert_eq!(span.public_holiday, "春节");
        // T3 西式节日（02-14 情人节）
        let western = get_chinese_lunar_calendar(2026, 2, 14, 12);
        assert_eq!(western.public_holiday, "情人节");
        // 普通日空串
        let ordinary = get_chinese_lunar_calendar(2026, 7, 15, 12);
        assert_eq!(ordinary.public_holiday, "");
    }

    #[test]
    fn holidays_table_known_entries() {
        // 已知节日锚点（2026：元旦/春节/清明/劳动/端午/中秋/国庆）
        let t = holidays(2026).expect("2026 应有数据");
        assert_eq!(t.get("01-01"), Some(&"元旦".to_string()));
        assert_eq!(t.get("02-17"), Some(&"春节".to_string()));
        assert_eq!(t.get("04-05"), Some(&"清明节".to_string()));
        assert_eq!(t.get("05-01"), Some(&"劳动节".to_string()));
        assert_eq!(t.get("06-19"), Some(&"端午节".to_string()));
        assert_eq!(t.get("09-25"), Some(&"中秋节".to_string()));
        assert_eq!(t.get("10-01"), Some(&"国庆节".to_string()));
        assert_eq!(t.len(), 33, "2026 年 33 个休假日");
        assert!(holidays(2023).is_none(), "2023 无数据应 None");
    }

    #[test]
    fn chinese_date_format() {
        // 中文日期格式（2026-09-23 是周三）
        assert_eq!(
            get_chinese_date(local(2026, 9, 23, 12)),
            "2026年09月23日 星期三"
        );
    }
}
