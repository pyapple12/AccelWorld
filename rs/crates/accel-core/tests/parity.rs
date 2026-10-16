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

    #[test]
    fn defaults_match_python() {
        // 默认值快照全量相等（default_factory 单源语义的跨语言锚定）
        let expect: UserConfig = serde_json::from_value(fixture()["defaults"].clone()).unwrap();
        assert_eq!(UserConfig::default(), expect);
    }

    #[test]
    fn roundtrip_in_to_out_on_disk() {
        // 样本实例 → lenient 构造 → 序列化 == Python 落盘内容（含 alarms 透传）
        let rt = &fixture()["roundtrip"];
        let cfg = UserConfig::from_json_value(&rt["in"]);
        assert_eq!(cfg.to_json_value(), rt["out_on_disk"]);
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
        let dir = std::env::temp_dir().join(format!("accel_cfgparity_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("临时目录创建失败");
        let path = dir.join("user_config.json");
        std::fs::write(&path, cb["in_file"].as_str().expect("in_file 应为字符串"))
            .expect("写入失败");
        let restored = settings::load(&path).to_json_value();
        assert_eq!(restored, cb["default_restored"]);
        let bak = path.with_file_name("user_config.json.bak");
        assert!(bak.exists(), "应生成 .bak 转存");
        let bak_text = std::fs::read_to_string(&bak).expect("bak 应可读");
        assert_eq!(
            bak_text,
            cb["in_file"].as_str().expect("in_file 应为字符串")
        );
    }

    #[test]
    fn unknown_key_ignored() {
        // 未知键忽略且无副作用
        let uk = &fixture()["unknown_key_ignored"];
        let cfg = UserConfig::from_json_value(&{
            let mut payload = uk["out"].clone();
            payload["unknown_key"] = serde_json::json!({"x": 1});
            payload
        });
        assert_eq!(cfg.to_json_value(), uk["out"]);
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
