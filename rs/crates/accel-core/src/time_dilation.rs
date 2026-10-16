//! 时间膨胀核心（modules/time_dilation.py 的 Rust 对等物）
//! 倍率域校验/加速时间推进/整数日长口径/tick 联动/剩余小时——公式与 Python 逐行对齐

use std::sync::Mutex;

use chrono::{DateTime, Datelike, Local, Timelike};
use thiserror::Error;

use crate::chinese_calendar::{get_chinese_date, get_lunar_info};
use crate::static_config::Static;
use crate::types::TimeInfo;

/// 倍率域校验错误（Python ValueError 对等；文案经 {:?} 浮点格式化对齐 str(float)）
#[derive(Debug, Error, PartialEq)]
pub enum RateError {
    #[error("时间膨胀倍率必须大于或等于{min:?}！")]
    BelowMin { min: f64 },
    #[error("时间膨胀倍率必须小于或等于{max:?}！")]
    AboveMax { max: f64 },
}

// 标准秒缓存键：(年, 月, 日, 时, 分, 秒)——农历/中文日期昂贵计算的同秒复用键
type SecondKey = (i32, u32, u32, u32, u32, u32);

/// 时间膨胀核心（Python AcceleratedWorld 镜像）
/// 时间全参数化注入（PL011 层面无 Clock 依赖，可测性优先）；
/// 农历/中文日期按标准秒缓存（Python _lunar_cache 对等，PL012 扩入）
#[derive(Debug)]
pub struct TimeDilation {
    rate: f64,
    /// 整数日长 int(24×rate)——钟面回绕模数与日长显示同口径（FIX003.4）
    day_hours: i64,
    /// (标准秒键, 中文日期, 农历信息) 缓存——同标准秒内直接复用
    lunar_cache: Mutex<Option<(SecondKey, String, String)>>,
}

impl PartialEq for TimeDilation {
    // 等值语义：仅业务字段（rate/day_hours）；秒级缓存是透明性能状态不参与比较
    fn eq(&self, other: &Self) -> bool {
        self.rate == other.rate && self.day_hours == other.day_hours
    }
}

impl TimeDilation {
    /// 构造并校验倍率域：[rate_min, rate_max]（默认值经 with_default_rate）
    pub fn new(rate: f64) -> Result<Self, RateError> {
        let base = Static::get();
        let min = base.base_f64("rate_min");
        let max = base.base_f64("rate_max");
        if rate < min {
            return Err(RateError::BelowMin { min });
        }
        if rate > max {
            return Err(RateError::AboveMax { max });
        }
        Ok(TimeDilation {
            rate,
            day_hours: (24.0 * rate) as i64,
            lunar_cache: Mutex::new(None),
        })
    }

    /// 默认倍率构造（base.default_rate 单源）
    pub fn with_default_rate() -> Self {
        Self::new(Static::get().base_f64("default_rate")).expect("default_rate 必在倍率域内")
    }

    /// 当前倍率
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// 整数日长（Python custom_hours_per_day 对等）
    pub fn day_hours(&self) -> i64 {
        self.day_hours
    }

    /// 刷新周期毫秒：基础 tick 与加速秒周期（1000/rate）取小，下钳 1
    /// （T001.1：加速秒边界不被 tick 周期错过）
    pub fn tick_interval_ms(&self) -> i64 {
        let base_tick_ms = Static::get().base_i64("clock_tick_ms");
        let accelerated_second_ms = (1000.0 / self.rate) as i64;
        std::cmp::max(1, std::cmp::min(base_tick_ms, accelerated_second_ms))
    }

    /// 加速时间信息快照（Python get_custom_time 全域对等：时间公式 + 农历秒级缓存）
    /// 输入注入时刻 now（本地墙钟）；公式逐行对齐：
    /// total = 墙钟秒（微秒并入小数）→ custom_total = total×rate →
    /// 时= floor(custom_total/3600) mod day_hours、分秒各自取模取整 →
    /// 剩余 = max(0, day_hours×3600 − custom_total)/3600
    pub fn time_info(&self, now: DateTime<Local>) -> TimeInfo {
        // 农历/中文日期按标准秒缓存（同秒直接复用，跨秒现算——Python 同语义）
        let second_key: SecondKey = (
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
        );
        let mut cache = self.lunar_cache.lock().expect("lunar 缓存锁中毒");
        let (chinese_date, lunar_info) = match &*cache {
            Some((key, cd, li)) if *key == second_key => (cd.clone(), li.clone()),
            _ => {
                let cd = get_chinese_date(now);
                let li = get_lunar_info(now);
                *cache = Some((second_key, cd.clone(), li.clone()));
                (cd, li)
            }
        };
        drop(cache);

        let micro = now.nanosecond() / 1_000;
        let total_seconds =
            (now.hour() as i64 * 3600 + now.minute() as i64 * 60 + now.second() as i64) as f64
                + micro as f64 / 1e6;
        let custom_total_seconds = total_seconds * self.rate;
        let custom_hour =
            ((custom_total_seconds / 3600.0).floor() as i64).rem_euclid(self.day_hours);
        let custom_minute = (((custom_total_seconds % 3600.0) / 60.0).floor()) as i64;
        let custom_second = (custom_total_seconds % 60.0).floor() as i64;
        let custom_time = format!("{custom_hour:02}:{custom_minute:02}:{custom_second:02}");
        let remaining_seconds = ((self.day_hours * 3600) as f64 - custom_total_seconds).max(0.0);
        TimeInfo {
            standard_datetime: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            custom_time,
            chinese_date,
            lunar_info,
            dilation_percentage: self.rate * 100.0,
            expanded_hours_per_day: self.day_hours as f64,
            remaining_hours: remaining_seconds / 3600.0,
            day_hours: self.day_hours,
            tick_ms: self.tick_interval_ms(),
        }
    }
}

// ===== src/time_dilation.rs 说明区 =====
// 输入：倍率 f64（构造期校验）+ 注入时刻 DateTime<Local>
// 输出：TimeInfo 快照 / tick 周期 / RateError（文案与 Python 逐字符对齐）
// 逻辑步骤：倍率域校验（min/max 来自 base.json）→ day_hours = int(24×rate) →
//   墙钟总秒（微秒并入）× rate → 时分秒取整分解（时按 day_hours 回绕）→ 剩余钳 0
// 设计理由：与 Python 版同为无状态纯计算（CLI/GUI/测试共用）；浮点表达式树保持
//   与 Python 同序（int+float 混算 → f64 同 IEEE 结果，夹具断言位级一致）；
//   {:?} 格式化浮点 = Python str(float)（1.0 → "1.0"），错误文案可逐字符 parity；
//   rem_euclid 与 Python % 在正模数下同语义（day_hours 恒正）
// 异常处理：域外倍率返 RateError（Python ValueError 对等，不 panic）；
//   default_rate 越界属 token 表损坏，expect 快速失败
// 已知偏移：农历缓存自 TimeDilation 内部承接（Python _lunar_cache 同构，PL012 落地——
//   原 PL011 注记「interface 层承接」改为模型内承接，与 Python 结构一致）
// 关联配置：rate_min/rate_max/default_rate/clock_tick_ms（base.json）
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    fn local_of(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32, micro: u32) -> DateTime<Local> {
        let naive = NaiveDate::from_ymd_opt(y, m, d)
            .expect("日期合法")
            .and_hms_micro_opt(h, mi, s, micro)
            .expect("时刻合法");
        Local
            .from_local_datetime(&naive)
            .single()
            .expect("夹具时刻在本地时区应无歧义")
    }

    #[test]
    fn rate_domain_validation() {
        // 边界内接受/边界值接受/越界拒绝（含文案）
        assert!(TimeDilation::new(1.0).is_ok());
        assert!(TimeDilation::new(20.0).is_ok());
        assert_eq!(
            TimeDilation::new(0.9),
            Err(RateError::BelowMin { min: 1.0 })
        );
        assert_eq!(
            TimeDilation::new(20.1),
            Err(RateError::AboveMax { max: 20.0 })
        );
        assert_eq!(
            TimeDilation::new(0.0).unwrap_err().to_string(),
            "时间膨胀倍率必须大于或等于1.0！"
        );
        assert_eq!(
            TimeDilation::new(100.0).unwrap_err().to_string(),
            "时间膨胀倍率必须小于或等于20.0！"
        );
    }

    #[test]
    fn integer_day_hours_dual_gauge() {
        // 整数日长口径：1.1 → 26（26.4 截断，FIX003.4 双口径回归位）
        assert_eq!(TimeDilation::new(1.0).unwrap().day_hours(), 24);
        assert_eq!(TimeDilation::new(1.1).unwrap().day_hours(), 26);
        assert_eq!(TimeDilation::new(2.0).unwrap().day_hours(), 48);
        assert_eq!(TimeDilation::new(20.0).unwrap().day_hours(), 480);
    }

    #[test]
    fn tick_follows_rate() {
        // tick = min(clock_tick_ms=100, int(1000/rate))：低速钳 100、高速 50
        assert_eq!(TimeDilation::new(1.0).unwrap().tick_interval_ms(), 100);
        assert_eq!(TimeDilation::new(1.1).unwrap().tick_interval_ms(), 100);
        assert_eq!(TimeDilation::new(10.0).unwrap().tick_interval_ms(), 100);
        assert_eq!(TimeDilation::new(20.0).unwrap().tick_interval_ms(), 50);
    }

    #[test]
    fn wraparound_at_rate_1_1_late_night() {
        // rate=1.1 深夜：23:00 → 25:18:00，剩余 0.7h（FIX003.4 回归锚点）
        let td = TimeDilation::new(1.1).unwrap();
        let info = td.time_info(local_of(2026, 9, 23, 23, 0, 0, 0));
        assert_eq!(info.custom_time, "25:18:00");
        assert_eq!(info.day_hours, 26);
        assert_eq!(info.expanded_hours_per_day, 26.0);
        assert_eq!(info.dilation_percentage, 110.00000000000001);
        assert_eq!(info.remaining_hours, 0.699999999999996);
        assert_eq!(info.standard_datetime, "2026-09-23 23:00:00");
    }

    #[test]
    fn high_rate_hours_exceed_day_wrap() {
        // rate=20 午间：小时远超 24 且按 480 回绕（12:34:56 → 251 时段）
        let td = TimeDilation::new(20.0).unwrap();
        let info = td.time_info(local_of(2026, 6, 15, 12, 34, 56, 123456));
        assert_eq!(info.custom_time, "251:38:42");
        assert_eq!(info.tick_ms, 50);
    }

    #[test]
    fn default_rate_from_token() {
        // 默认倍率单源：base.default_rate = 2.0
        let td = TimeDilation::with_default_rate();
        assert_eq!(td.rate(), 2.0);
        assert_eq!(td.day_hours(), 48);
    }
}
