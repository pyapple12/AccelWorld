//! lunar-rust API 面清点探针（PL012.01）：30 代表日逐字段与 Python 夹具并列比对
//! 输出供三分法裁决：直调 / 薄实现+导表 / 纯本地
//! 运行：cargo run -p accel-core --example lunar_compare

use lunar_rust::lunar::LunarRefHelper;
use lunar_rust::solar::SolarRefHelper;

const LUNAR_SAMPLES: &str = include_str!("../../../fixtures/lunar_samples.json");

#[derive(serde::Deserialize)]
struct DayRow {
    date: String,
    expect: Expect,
}

#[derive(serde::Deserialize)]
struct Expect {
    lunar_year: String,
    shengxiao: String,
    lunar_month: String,
    lunar_day: String,
    jieqi: String,
    yue_phase: String,
    public_holiday: String,
}

fn main() {
    let v: serde_json::Value = serde_json::from_str(LUNAR_SAMPLES).expect("夹具解析失败");
    let days: Vec<DayRow> = serde_json::from_value(v["days"].clone()).expect("days 反序列化失败");
    assert_eq!(days.len(), 365);

    // 代表日抽取：节气日全取（≤24）/ 农历月初 / 农历月末 / 节日日 / 年首年末 / 普通日补足
    let mut picked: Vec<usize> = Vec::new();
    let mut prev_month: Option<&str> = None;
    let mut prev_day: Option<&str> = None;
    for (i, d) in days.iter().enumerate() {
        let e = &d.expect;
        if !e.jieqi.is_empty() {
            picked.push(i);
        }
        if Some(e.lunar_month.as_str()) != prev_month {
            picked.push(i);
            prev_month = Some(e.lunar_month.as_str());
        }
        if Some(e.lunar_day.as_str()) != prev_day {
            if e.lunar_day == "廿九" || e.lunar_day == "三十" || e.lunar_day == "廿八" {
                picked.push(i);
            }
            prev_day = Some(e.lunar_day.as_str());
        }
        if !e.public_holiday.is_empty() {
            picked.push(i);
        }
    }
    picked.push(0);
    picked.push(days.len() - 1);
    // 普通日均匀补 8 个
    for k in 0..8 {
        picked.push(k * 365 / 8 + 3);
    }
    picked.sort_unstable();
    picked.dedup();

    let mut mismatches = 0usize;
    let mut checked = 0usize;
    for &i in &picked {
        let d = &days[i];
        let parts: Vec<&str> = d.date.split('-').collect();
        let (y, m, dd): (i64, i64, i64) = (
            parts[0].parse().unwrap(),
            parts[1].parse().unwrap(),
            parts[2].parse().unwrap(),
        );
        let lunar = lunar_rust::solar::from_ymd(y, m, dd).get_lunar();
        let e = &d.expect;
        let mut cmp = |name: &str, rust: String, expect: &str| {
            checked += 1;
            let ok = rust == expect;
            if !ok {
                mismatches += 1;
            }
            println!(
                "{} | {:<10} rust={:<14} py={:<14} {}",
                d.date,
                name,
                format!("{rust:?}"),
                format!("{expect:?}"),
                if ok { "OK" } else { "DIFF" }
            );
        };
        // 直调字段（含 Python 拼接后缀的还原比对）
        cmp(
            "ganzhi",
            lunar.get_year_in_gan_zhi(),
            e.lunar_year.trim_end_matches('年'),
        );
        cmp("shengxiao", lunar.get_year_sheng_xiao(), &e.shengxiao);
        cmp(
            "lunar_month",
            lunar.get_month_in_chinese(),
            e.lunar_month.trim_end_matches('月'),
        );
        cmp("lunar_day", lunar.get_day_in_chinese(), &e.lunar_day);
        cmp(
            "yue_xiang",
            lunar.get_yue_xiang(),
            e.yue_phase.trim_end_matches('月'),
        );
        cmp("jieqi", lunar.get_jie_qi(), &e.jieqi);
        let festivals = lunar.get_festivals();
        let fest_first = festivals.first().cloned().unwrap_or_default();
        cmp("festival_t1", fest_first.clone(), &e.public_holiday);
        if fest_first != e.public_holiday {
            // 三级兜底归属证据：lunar 无节日时 Python 走 chinese-calendar/自定义表
            println!(
                "         └ tier 分析：lunar_fests={festivals:?} py_final={:?}（可能是 T2/T3 来源）",
                e.public_holiday
            );
        }
    }
    println!(
        "\n抽样 {} 天，字段比对 {checked} 条，DIFF {mismatches} 条",
        picked.len()
    );
}

// ===== examples/lunar_compare.rs 说明区 =====
// 输入：rs/fixtures/lunar_samples.json（365 天期望值）；输出：stdout 每字段一行比对
// 逻辑步骤：代表日抽取（节气日全取/农历月初/月末廿八至三十/节日日/年首年末/均匀普通日）
//   → lunar-rust 逐字段调取 → 与夹具并列比对 → DIFF 行附带三级兜底归属分析
// 设计理由：PL012.01 API 面清点证据（107 日 × 7 字段 = 749 条比对，六字段直调/
//   节日三级兜底定案依据）——实现定案后的回归工具（lunar-rust 升级时复跑）
// 异常处理：夹具解析失败 expect 快速失败；无异常分支
// 关联依赖：lunar_rust/serde_json；运行：cargo run -p accel-core --example lunar_compare
