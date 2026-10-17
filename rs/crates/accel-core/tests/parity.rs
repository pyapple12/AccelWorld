//! 夹具驱动集成测试（golden parity：Rust 实现 vs Python 0.6.0.0 行为快照）
//! PL011.04 导出 rs/fixtures/*.json；各域断言随 PL011.05-.13 逐模块补入
//! 落位说明：原计划 rs/tests/（虚拟 manifest 根级不编译），移入 core crate——
//! 夹具断言的正是 core 行为，归属语义也更正确（偏移登记 z.plan 完成记录）

// ===== tests/parity.rs 说明区 =====
// 消费方式：include_str!/serde 反序列化为强类型夹具结构后逐条断言
// 夹具路径：../../../fixtures/*.json（rs/fixtures/，git 入库）
// 后续各域断言分模块函数落位于本文件（time/config/weather；lunar 属 PL012）

#[test]
fn parity_harness_online() {
    // 夹具目录接线检查：README 随 git 入库，include 链路通 = 夹具路径契约成立
    // （真实四类夹具文件随 PL011.04 导出，届时本文件补入各域断言）
    let readme: &str = include_str!("../../../fixtures/README.md");
    assert!(!readme.is_empty());
}

/// 天气域 parity（PL011.11）：61 样本 accept/reject 决策 + 值 + 展示文本对 Python 快照
mod weather_parity {
    use accel_core::weather::{WeatherErr, format_weather_info, parse_open_meteo};
    use serde_json::Value;

    const WEATHER_SAMPLES: &str = include_str!("../../../fixtures/weather_samples.json");

    fn fixture() -> Value {
        serde_json::from_str(WEATHER_SAMPLES).expect("weather_samples.json 解析失败")
    }

    #[test]
    fn all_samples_match_python() {
        let binding = fixture();
        let samples = binding["samples"].as_array().expect("samples 应为数组");
        assert_eq!(samples.len(), 61);
        let (mut ok_n, mut reject_n) = (0, 0);
        for s in samples {
            let name = s["name"].as_str().expect("name 应为字符串");
            match parse_open_meteo(&s["raw"]) {
                Ok(w) => {
                    assert_eq!(
                        s["expect"]["ok"],
                        Value::Bool(true),
                        "样本 {name} Python 拒绝而 Rust 接受"
                    );
                    let d = &s["expect"]["data"];
                    // 数值字段位级比较（Python int 62 与 Rust 62.0 数值相等）
                    assert_eq!(
                        w.temperature,
                        d["temperature"].as_f64().expect("temperature"),
                        "{name}"
                    );
                    assert_eq!(
                        w.humidity,
                        d["humidity"].as_f64().expect("humidity"),
                        "{name}"
                    );
                    assert_eq!(
                        w.wind_speed,
                        d["wind_speed"].as_f64().expect("wind"),
                        "{name}"
                    );
                    assert_eq!(
                        w.apparent_temperature,
                        d["apparent_temperature"].as_f64().expect("apparent"),
                        "{name}"
                    );
                    // 展示三字段逐字符 parity（含未知码回退与 bool 码命中语义）
                    assert_eq!(w.weather, d["weather"].as_str().expect("weather"), "{name}");
                    assert_eq!(
                        w.description,
                        d["description"].as_str().expect("desc"),
                        "{name}"
                    );
                    assert_eq!(w.icon, d["icon"].as_str().expect("icon"), "{name}");
                    // weather_code 仅数值形态断言（Python 动态类型渗漏归一偏移，登记 z.plan）
                    if let (Some(code), true) = (
                        d["weather_code"].as_i64(),
                        s["raw"]["current"]["weather_code"].is_number(),
                    ) {
                        assert_eq!(w.weather_code, code, "{name}");
                    }
                    ok_n += 1;
                }
                Err(e) => {
                    assert_eq!(
                        s["expect"]["ok"],
                        Value::Bool(false),
                        "样本 {name} Python 接受而 Rust 拒绝: {e}"
                    );
                    reject_n += 1;
                }
            }
        }
        // 计数锚点：35 接受（正常 2 + 码表 28 + 边缘 5）+ 26 拒绝（结构 4 + 字段 10 + 错型 12）
        assert_eq!((ok_n, reject_n), (35, 26));
    }

    #[test]
    fn format_texts_match_python() {
        // 展示文本三态逐字符对齐（有城市/无城市/失败文案）
        let v = fixture();
        let raw = v["samples"]
            .as_array()
            .expect("samples")
            .iter()
            .find(|s| s["name"] == "code_2")
            .expect("code_2 样本应存在")["raw"]
            .clone();
        let w = parse_open_meteo(&raw).expect("code_2 应解析成功");
        assert_eq!(
            format_weather_info(Some(&w), "北京"),
            v["format_with_city"].as_str().expect("city")
        );
        assert_eq!(
            format_weather_info(Some(&w), ""),
            v["format_without_city"].as_str().expect("no city")
        );
        assert_eq!(
            format_weather_info(None, "北京"),
            v["format_none"].as_str().expect("none")
        );
    }

    #[test]
    fn rejection_reasons_match_python_stages() {
        // 拒绝样本的阶段语义对齐：Python 日志文案 ↔ Rust 错误变体（抽样锚定）
        let binding = fixture();
        let samples = binding["samples"].as_array().expect("samples");
        let stage_of = |name: &str| -> WeatherErr {
            let raw = samples
                .iter()
                .find(|s| s["name"] == name)
                .expect("样本应存在")["raw"]
                .clone();
            parse_open_meteo(&raw).expect_err("该样本应被拒绝")
        };
        assert_eq!(stage_of("top_not_dict"), WeatherErr::NotObject);
        assert_eq!(stage_of("current_not_dict"), WeatherErr::CurrentNotObject);
        assert_eq!(
            stage_of("missing_temperature_2m"),
            WeatherErr::MissingField("temperature_2m")
        );
        assert_eq!(
            stage_of("null_apparent_temperature"),
            WeatherErr::NullField("apparent_temperature")
        );
        assert_eq!(
            stage_of("badtype_str_temperature_2m"),
            WeatherErr::BadType("temperature_2m")
        );
    }
}

/// 农历域 parity（PL012.02/.03）：全年 365 天十字段 + info_str + 中文日期 + 时辰探针
mod lunar_parity {
    use accel_core::chinese_calendar::{
        get_chinese_date, get_chinese_lunar_calendar, get_lunar_info,
    };
    use chrono::{Datelike, Local, NaiveDate, TimeZone};
    use serde_json::Value;

    const LUNAR_SAMPLES: &str = include_str!("../../../fixtures/lunar_samples.json");

    fn local_noon(date_str: &str) -> chrono::DateTime<Local> {
        let d = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").expect("日期应可解析");
        Local
            .from_local_datetime(&d.and_hms_opt(12, 0, 0).expect("正午合法"))
            .single()
            .expect("本地时区无歧义")
    }

    #[test]
    fn full_year_all_ten_fields() {
        // 全年 365 天 × LunarInfo 十字段逐一全等（干支/生肖/月/日/时辰/月相/节气/节日/财神/方位）
        let v: Value = serde_json::from_str(LUNAR_SAMPLES).expect("解析失败");
        let days = v["days"].as_array().expect("days 数组");
        assert_eq!(days.len(), 365);
        for row in days {
            let date = row["date"].as_str().expect("date");
            let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("日期解析");
            let info = get_chinese_lunar_calendar(d.year(), d.month(), d.day(), 12);
            let e = &row["expect"];
            assert_eq!(
                info.lunar_year,
                e["lunar_year"].as_str().expect("y"),
                "{date}"
            );
            assert_eq!(
                info.shengxiao,
                e["shengxiao"].as_str().expect("sx"),
                "{date}"
            );
            assert_eq!(
                info.lunar_month,
                e["lunar_month"].as_str().expect("m"),
                "{date}"
            );
            assert_eq!(
                info.lunar_day,
                e["lunar_day"].as_str().expect("d"),
                "{date}"
            );
            assert_eq!(info.shichen, e["shichen"].as_str().expect("sc"), "{date}");
            assert_eq!(
                info.yue_phase,
                e["yue_phase"].as_str().expect("yx"),
                "{date}"
            );
            assert_eq!(info.jieqi, e["jieqi"].as_str().expect("jq"), "{date}");
            assert_eq!(
                info.public_holiday,
                e["public_holiday"].as_str().expect("ph"),
                "{date} 节日三级兜底不一致"
            );
            assert_eq!(
                info.cai_shen_dir,
                e["cai_shen_dir"].as_str().expect("cs"),
                "{date}"
            );
            assert_eq!(info.position, e["position"].as_str().expect("po"), "{date}");
        }
    }

    #[test]
    fn full_year_info_str_and_chinese_date() {
        // info_str 逐字符全等（拼装契约 + 无"拜"回归）与中文日期全等
        let v: Value = serde_json::from_str(LUNAR_SAMPLES).expect("解析失败");
        for row in v["days"].as_array().expect("days") {
            let date = row["date"].as_str().expect("date");
            let now = local_noon(date);
            assert_eq!(
                get_lunar_info(now),
                row["lunar_info_str"].as_str().expect("str"),
                "{date}"
            );
            assert_eq!(
                get_chinese_date(now),
                row["chinese_date"].as_str().expect("cd"),
                "{date}"
            );
        }
    }

    #[test]
    fn shichen_probe_24_hours() {
        // 同日 24 小时时辰档全等（含 23 时双档子时边界）
        let v: Value = serde_json::from_str(LUNAR_SAMPLES).expect("解析失败");
        for probe in v["shichen_probe"].as_array().expect("probe") {
            let hour = probe["hour"].as_u64().expect("hour") as u32;
            let info = get_chinese_lunar_calendar(2026, 9, 23, hour);
            assert_eq!(
                info.shichen,
                probe["expect_shichen"].as_str().expect("sc"),
                "hour={hour}"
            );
        }
    }
}

/// 节日派生 parity（PL014.07）：year_chips 对 lunar-python 夹具全年断言 + 春节推导
mod festival_parity {
    use accel_core::chinese_calendar::{next_spring_festival, year_chips};
    use chrono::NaiveDate;

    const FESTIVAL_DERIVE: &str = include_str!("../../../fixtures/festival_derive.json");

    fn fixture() -> serde_json::Value {
        serde_json::from_str(FESTIVAL_DERIVE).expect("festival_derive.json 解析失败")
    }

    #[test]
    fn year_chips_match_python_snapshot() {
        // 夹具 = lunar-python 同规则全年快照（法定+传统合并、传统覆盖）
        // 以两年各一时点派生，断言：返回条目 ∈ 夹具（三年并集）且名称一致
        let v = fixture();
        let years = ["2026", "2027", "2028"];
        for iso_now in ["2026-06-01", "2027-06-01"] {
            let now = NaiveDate::parse_from_str(iso_now, "%Y-%m-%d").expect("now 解析");
            for (d, name) in year_chips(now) {
                let key = d.format("%Y-%m-%d").to_string();
                // 条目可能来自次年轮（跨 365 天窗口）——三年夹具并集校验
                let matched = years.iter().any(|y| {
                    v[*y]
                        .as_object()
                        .expect("年份对象")
                        .get(&key)
                        .and_then(|n| n.as_str())
                        == Some(name.as_str())
                });
                assert!(matched, "{iso_now} 派生 {key}/{name} 不在夹具（三年并集）");
            }
        }
    }

    #[test]
    fn year_chips_covers_future_365() {
        // 完备性：夹具中 [now, now+365] 窗口内条目必在派生结果（无漏节日）
        let now = NaiveDate::parse_from_str("2026-06-01", "%Y-%m-%d").expect("now");
        let chips: std::collections::BTreeMap<String, String> = year_chips(now)
            .into_iter()
            .map(|(d, n)| (d.format("%Y-%m-%d").to_string(), n))
            .collect();
        let v = fixture();
        let mut missing = 0;
        let horizon = now + chrono::Duration::days(365);
        for table in v.as_object().expect("对象").values() {
            for (iso, name) in table.as_object().expect("表") {
                if let Ok(d) = NaiveDate::parse_from_str(iso, "%Y-%m-%d") {
                    let in_window = d >= now && d <= horizon;
                    if in_window && chips.get(iso).map(|s| s.as_str()) != name.as_str() {
                        missing += 1;
                        println!("缺: {iso}/{name}（got {:?}）", chips.get(iso));
                    }
                }
            }
        }
        assert_eq!(missing, 0, "未来 365 天窗口内夹具节日不应缺席");
    }

    #[test]
    fn next_spring_festival_cases() {
        // 跨年用例 + 年中推导（夹具锚点：2026 春节 02-17、2027 春节 02-06）
        let d = |y: i32, m: u32, dd: u32| NaiveDate::from_ymd_opt(y, m, dd).expect("合法");
        assert_eq!(
            next_spring_festival(d(2026, 2, 10)),
            d(2026, 2, 17),
            "年前推导当年"
        );
        assert_eq!(
            next_spring_festival(d(2026, 2, 17)),
            d(2026, 2, 17),
            "当日含当日"
        );
        assert_eq!(
            next_spring_festival(d(2026, 2, 18)),
            d(2027, 2, 6),
            "年后切次年"
        );
        assert_eq!(
            next_spring_festival(d(2026, 12, 31)),
            d(2027, 2, 6),
            "跨年边界"
        );
        assert_eq!(
            next_spring_festival(d(2026, 6, 1)),
            d(2027, 2, 6),
            "年中推导次年"
        );
    }

    #[test]
    fn qingming_drift_spot_check() {
        // 清明节气年际漂移抽查：两年清明都派生（04-04/05 节气漂移带内）
        let now = NaiveDate::from_ymd_opt(2026, 1, 1).expect("合法");
        let chips = year_chips(now);
        let qingmings: Vec<String> = chips
            .iter()
            .filter(|(_, n)| n == "清明节")
            .map(|(d, _)| d.format("%Y-%m-%d").to_string())
            .collect();
        assert!(
            qingmings.contains(&"2026-04-05".to_string())
                || qingmings.contains(&"2026-04-04".to_string()),
            "清明应在 04-04/05（节气漂移带）：{qingmings:?}"
        );
        assert!(
            chips.iter().filter(|(_, n)| n == "清明节").count() >= 2,
            "两年清明都应派生"
        );
    }
}

/// 时间域 parity（PL011.10）：47 向量全量断言（42 合法 + 5 越界，位级浮点一致）
mod time_parity {
    use accel_core::time_dilation::TimeDilation;
    use chrono::{Local, NaiveDateTime, TimeZone};
    use serde_json::Value;

    const TIME_VECTORS: &str = include_str!("../../../fixtures/time_vectors.json");

    #[test]
    fn all_vectors_match_python() {
        let v: Value = serde_json::from_str(TIME_VECTORS).expect("time_vectors.json 解析失败");
        let vectors = v["vectors"].as_array().expect("vectors 应为数组");
        assert_eq!(vectors.len(), 47);
        let mut ok_checked = 0;
        let mut err_checked = 0;
        for vec in vectors {
            let rate = vec["rate"].as_f64().expect("rate 应为数值");
            match vec["std"].as_str() {
                Some(std_iso) => {
                    // %.f 匹配可选小数秒（Python isoformat 微秒为 0 时省略）
                    let naive = NaiveDateTime::parse_from_str(std_iso, "%Y-%m-%dT%H:%M:%S%.f")
                        .expect("std 应可解析");
                    let now = Local
                        .from_local_datetime(&naive)
                        .single()
                        .expect("夹具时刻在本地时区应无歧义");
                    let td = TimeDilation::new(rate).expect("夹具倍率应合法");
                    let info = td.time_info(now);
                    let expect = &vec["expect"];
                    assert_eq!(
                        info.custom_time,
                        expect["custom_time"]
                            .as_str()
                            .expect("custom_time 应为字符串"),
                        "rate={rate} std={std_iso} 加速时间不一致"
                    );
                    let hour: i64 = info
                        .custom_time
                        .split(':')
                        .next()
                        .expect("custom_time 应含时分量")
                        .parse()
                        .expect("时分量应为整数");
                    assert_eq!(
                        hour,
                        expect["custom_hour"]
                            .as_i64()
                            .expect("custom_hour 应为整数")
                    );
                    assert_eq!(
                        info.standard_datetime[..10],
                        std_iso[..10],
                        "日期前缀应一致"
                    );
                    assert_eq!(
                        info.day_hours,
                        expect["day_hours"].as_i64().expect("day_hours")
                    );
                    assert_eq!(
                        info.expanded_hours_per_day,
                        expect["expanded_hours_per_day"].as_f64().expect("expanded")
                    );
                    assert_eq!(
                        info.dilation_percentage,
                        expect["dilation_percentage"].as_f64().expect("percentage"),
                        "rate={rate} std={std_iso} 百分比不一致"
                    );
                    assert_eq!(
                        info.remaining_hours,
                        expect["remaining_hours"].as_f64().expect("remaining"),
                        "rate={rate} std={std_iso} 剩余小时不一致"
                    );
                    assert_eq!(info.tick_ms, expect["tick_ms"].as_i64().expect("tick_ms"));
                    ok_checked += 1;
                }
                None => {
                    // 越界倍率：错误文案逐字符对齐（error_type ValueError ↔ RateError 映射）
                    let expect = &vec["expect"];
                    assert_eq!(expect["ok"], Value::Bool(false));
                    let err = TimeDilation::new(rate).expect_err("越界倍率应拒绝");
                    assert_eq!(
                        err.to_string(),
                        expect["message"].as_str().expect("message 应为字符串"),
                        "rate={rate} 错误文案不一致"
                    );
                    err_checked += 1;
                }
            }
        }
        assert_eq!((ok_checked, err_checked), (42, 5));
    }
}

/// 配置域 parity（PL011.09）：默认值/往返/损坏转存/脏值回退对 Python 快照全量断言
mod config_parity {
    use accel_core::settings::{self, UserConfig};
    use serde_json::Value;

    const CONFIG_BEHAVIOR: &str = include_str!("../../../fixtures/config_behavior.json");

    fn fixture() -> Value {
        serde_json::from_str(CONFIG_BEHAVIOR).expect("config_behavior.json 解析失败")
    }

    /// Python 键值子集断言（PL014.06 schema 演进适配）：Rust 输出是 Python 快照的
    /// 向后兼容超集（新增 countdown_targets 键）——夹具每键值仍须逐一相等
    fn assert_python_keys_subset(expect: &Value, actual: &Value, ctx: &str) {
        let (Some(expect_obj), Some(actual_obj)) = (expect.as_object(), actual.as_object()) else {
            panic!("{ctx}: 双方均应为对象");
        };
        for (k, v) in expect_obj {
            assert_eq!(actual_obj.get(k), Some(v), "{ctx} 键 {k} 不一致");
        }
    }

    #[test]
    fn defaults_match_python() {
        // 默认值快照：Python 键子集全等（新增键 countdown_targets 为 PL014.06 演进）
        let expect: UserConfig = serde_json::from_value(fixture()["defaults"].clone()).unwrap();
        assert_python_keys_subset(
            &fixture()["defaults"],
            &UserConfig::default().to_json_value(),
            "defaults",
        );
        assert_eq!(UserConfig::default(), expect);
    }

    #[test]
    fn roundtrip_in_to_out_on_disk() {
        // 样本实例 → lenient 构造 → 序列化：Python 落盘键子集全等（含 alarms 透传）
        let rt = &fixture()["roundtrip"];
        let cfg = UserConfig::from_json_value(&rt["in"]);
        assert_python_keys_subset(&rt["out_on_disk"], &cfg.to_json_value(), "roundtrip");
        assert_eq!(rt["saved_ok"], Value::Bool(true));
    }

    #[test]
    fn dirty_fields_fall_back_like_python() {
        // 22 条脏值用例逐条：{field: bad} → Rust 装载字段值 == Python 实测字段值
        let defaults = UserConfig::default().to_json_value();
        for case in fixture()["dirty_fields"]
            .as_array()
            .expect("dirty_fields 应为数组")
        {
            let field = case["field"].as_str().expect("field 应为字符串");
            let mut payload = defaults.clone();
            payload[field] = case["bad_value"].clone();
            let loaded = UserConfig::from_json_value(&payload).to_json_value();
            assert_eq!(
                loaded[field], case["expect_field_value"],
                "脏值用例不一致: {field} <- {:?}",
                case["bad_value"]
            );
            assert_eq!(
                case["bak_created"],
                Value::Bool(false),
                "脏值（可解析）不应转存"
            );
        }
    }

    #[test]
    fn corrupt_backup_behavior() {
        // 损坏转存：原文进 .bak + 默认值恢复（本地临时目录复刻 Python 用例）
        let cb = &fixture()["corrupt_backup"];
        // A005 #26：用例末尾清理临时目录（原实现反复运行残留）
        let dir = std::env::temp_dir().join(format!("accel_cfgparity_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("临时目录创建失败");
        let path = dir.join("user_config.json");
        std::fs::write(&path, cb["in_file"].as_str().expect("in_file 应为字符串"))
            .expect("写入失败");
        let restored = settings::load(&path).to_json_value();
        assert_python_keys_subset(&cb["default_restored"], &restored, "corrupt_restored");
        let bak = path.with_file_name("user_config.json.bak");
        assert!(bak.exists(), "应生成 .bak 转存");
        let bak_text = std::fs::read_to_string(&bak).expect("bak 应可读");
        assert_eq!(
            bak_text,
            cb["in_file"].as_str().expect("in_file 应为字符串")
        );
        // A005 #26：清理现场（连跑两次不留残留）
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_key_ignored() {
        // 未知键忽略且无副作用（Python 键子集全等）
        let uk = &fixture()["unknown_key_ignored"];
        let cfg = UserConfig::from_json_value(&{
            let mut payload = uk["out"].clone();
            payload["unknown_key"] = serde_json::json!({"x": 1});
            payload
        });
        assert_python_keys_subset(&uk["out"], &cfg.to_json_value(), "unknown_key");
    }
}

/// 数据三表 parity（PL011.06）：条目数与全量值对 Python 导出夹具逐一相等
mod data_parity {
    use accel_core::data;
    use serde_json::Value;

    const DATA_TABLES: &str = include_str!("../../../fixtures/data_tables.json");

    #[test]
    fn tables_match_python_snapshot() {
        let v: Value = serde_json::from_str(DATA_TABLES).expect("data_tables.json 解析失败");
        let cities = v["cities"].as_object().expect("cities 应为对象");
        assert_eq!(cities.len(), data::cities::all().len());
        for (name, ll) in cities {
            let arr = ll.as_array().expect("城市坐标应为二元数组");
            let lat = arr[0].as_f64().expect("lat 应为数值");
            let lon = arr[1].as_f64().expect("lon 应为数值");
            assert_eq!(
                data::cities::get(name),
                Some((lat, lon)),
                "城市 {name} 坐标不一致"
            );
        }
        let tzs = v["timezones"].as_array().expect("timezones 应为数组");
        assert_eq!(tzs.len(), data::timezones::all().len());
        for (i, pair) in tzs.iter().enumerate() {
            let display = pair[0].as_str().expect("显示名应为字符串");
            let iana = pair[1].as_str().expect("IANA 应为字符串");
            let e = &data::timezones::all()[i];
            assert_eq!(
                (e.display, e.iana),
                (display, iana),
                "时区表第 {i} 行不一致"
            );
        }
        let codes = v["weather_codes"]
            .as_object()
            .expect("weather_codes 应为对象");
        assert_eq!(codes.len(), data::weather_codes::all().len());
        for (k, info) in codes {
            let code: i64 = k.parse().expect("码键应为整数");
            let rust = data::weather_codes::lookup(code);
            assert_eq!(rust.name, info["name"].as_str().expect("name 应为字符串"));
            assert_eq!(rust.icon, info["icon"].as_str().expect("icon 应为字符串"));
            assert_eq!(
                rust.description,
                info["description"]
                    .as_str()
                    .expect("description 应为字符串")
            );
        }
        let unk = &v["unknown_weather"];
        let r = data::weather_codes::unknown();
        assert_eq!(r.name, unk["name"].as_str().expect("name 应为字符串"));
        assert_eq!(r.icon, unk["icon"].as_str().expect("icon 应为字符串"));
        assert_eq!(
            r.description,
            unk["description"].as_str().expect("description 应为字符串")
        );
    }
}
