//! 应用静态配置单例（config/static/static_config.py 的 Rust 对等物）
//! include_str 编译期嵌入 rs/config/static/ 两 token 文件（与 Python 原文件零改写一致），
//! OnceLock 惰性单例 + 类型化访问器 + 缺键/错型 panic 快速失败（Python KeyError 等价）

use std::sync::OnceLock;

use serde_json::Value;

// 静态配置物理文件（路径即唯一结构约定：Python 版经 config.json 映射表 + __file__ 自定位，
// Rust 版编译期嵌入，映射表一层间接不再需要——偏移登记 z.plan）
const BASE_JSON: &str = include_str!("../../../config/static/base.json");
const UI_JSON: &str = include_str!("../../../config/static/ui.json");

// 静态配置聚合：base 业务参数域 + ui 视觉 token 域（对应 Python get_static_config().base/.ui）
pub struct Static {
    pub base: Value,
    pub ui: Value,
}

static STATIC: OnceLock<Static> = OnceLock::new();

impl Static {
    /// 单例入口：首次调用解析嵌入 JSON，此后零开销直取
    /// 解析失败属构建产物损坏级错误，panic 快速失败（不静默降级）
    pub fn get() -> &'static Static {
        STATIC.get_or_init(|| Static {
            base: serde_json::from_str(BASE_JSON).expect("base.json 解析失败（嵌入产物损坏）"),
            ui: serde_json::from_str(UI_JSON).expect("ui.json 解析失败（嵌入产物损坏）"),
        })
    }

    /// 显示版本单一来源（base.version，四段式 X.Y.Z.P）
    pub fn version(&self) -> &str {
        self.base_str("version")
    }

    // ===== base 域类型化访问器（键名由调用方给字面量，缺键/错型 panic 带键名）=====

    /// base 域取 f64（Python float(base[key]) 对等；int/float 均收）
    pub fn base_f64(&self, key: &str) -> f64 {
        match self.base.get(key).and_then(Value::as_f64) {
            Some(v) => v,
            None => panic!("base.json 缺键或非数值: {key}"),
        }
    }

    /// base 域取 i64（Python int(base[key]) 对等）
    pub fn base_i64(&self, key: &str) -> i64 {
        match self.base.get(key).and_then(Value::as_i64) {
            Some(v) => v,
            None => panic!("base.json 缺键或非整数: {key}"),
        }
    }

    /// base 域取字符串（Python str 直取对等）
    pub fn base_str(&self, key: &str) -> &str {
        match self.base.get(key).and_then(Value::as_str) {
            Some(v) => v,
            None => panic!("base.json 缺键或非字符串: {key}"),
        }
    }

    /// base 域取字符串数组（default_world_pins 等列表 token）
    pub fn base_str_list(&self, key: &str) -> Vec<&str> {
        match self.base.get(key).and_then(Value::as_array) {
            Some(arr) => arr
                .iter()
                .map(|v| {
                    v.as_str()
                        .unwrap_or_else(|| panic!("base.json {key} 含非字符串元素"))
                })
                .collect(),
            None => panic!("base.json 缺键或非数组: {key}"),
        }
    }

    // base 域取 bool / ui 域取 i64 / ui 嵌套取串（A005 #15：生产零调用，已删除——
    // 需要时用 `base`/`ui` 字段经 pointer 直取，勿再长出双轨访问器）
    // ===== ui 域类型化访问器 =====

    /// ui 域取字符串（字体族名等）
    pub fn ui_str(&self, key: &str) -> &str {
        match self.ui.get(key).and_then(Value::as_str) {
            Some(v) => v,
            None => panic!("ui.json 缺键或非字符串: {key}"),
        }
    }

    // ui 域其余访问器（A005 #15 + FIX007.10：顶层 ui_i64/ui_nested_str/ui_f64 与
    // 嵌套 ui_nested_f64 生产零调用已删除；需两级/浮点 token 时用 `ui` 字段
    // pointer 直取）

    /// ui 域嵌套对象取 i64
    pub fn ui_nested_i64(&self, group: &str, key: &str) -> i64 {
        match self
            .ui
            .get(group)
            .and_then(|g| g.get(key))
            .and_then(Value::as_i64)
        {
            Some(v) => v,
            None => panic!("ui.json 缺嵌套键或非整数: {group}.{key}"),
        }
    }
}

// ===== src/static_config.rs 说明区 =====
// 输入：rs/config/static/{base,ui}.json（结构承自 Python 原文件——PL011 时点
//   sha256 一致，此后独立演进）
// 输出：Static 单例 + base 域全型访问器（f64/i64/str/str_list——A009 #11：bool
//   访问器已随 A005 #15 删除，此处清单订正）+ ui 域
//   str/ui_nested_i64（FIX007.10：ui 域零调用 f64 族访问器已删，余走 pointer 直取）
// 逻辑步骤：include_str 编译期嵌入 → OnceLock get_or_init 首用解析 → 访问器按键取值
// 设计理由：token 零硬编码原则的 Rust 承接——业务参数不进代码；访问器 panic 带键名
//   对应 Python RuntimeError/KeyError 快速失败语义（启动期暴露而非运行期静默）；
//   Python 版 config.json 映射表是 __file__ 自定位的结构约定，Rust include_str 路径
//   即等价约定，映射层删除（不保留废弃间接层）
// 异常处理：JSON 解析失败/缺键/错型一律 panic（构建产物或 token 表损坏，不可恢复）；
//   可选消费（如未来新增的开关型 token）由调用方经 base.get 自行取 Value 判断，
//   不在本层提供 Option 变体（避免双轨语义）
// 关联依赖：serde_json；消费方：settings（默认值单源）/time_dilation（倍率域）/
//   weather（TTL/重试参数）/ app theme·window（视觉与几何 token）
#[cfg(test)]
mod tests {
    use super::Static;

    #[test]
    fn version_single_source() {
        // 版本单一来源契约：四段式纯数字形态（值本体只在 base.json——代码零版本字符串；
        // 编译期注入与运行期读取的取值 parity 由 accel-app lib.rs 单测锚定）
        let v = Static::get().version();
        let segs: Vec<&str> = v.split('.').collect();
        assert_eq!(segs.len(), 4, "版本应为四段式 X.Y.Z.P，实际 {v}");
        assert!(
            segs.iter()
                .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())),
            "各段必须为纯数字，实际 {v}"
        );
    }

    #[test]
    fn base_domain_tokens() {
        // 真实 token 抽样：倍率域/时钟 tick/天气参数/默认偏好
        let s = Static::get();
        assert_eq!(s.base_f64("rate_min"), 1.0);
        assert_eq!(s.base_f64("rate_max"), 20.0);
        assert_eq!(s.base_f64("default_rate"), 2.0);
        assert_eq!(s.base_i64("clock_tick_ms"), 100);
        assert_eq!(s.base_i64("weather_cache_ttl"), 1800);
        assert_eq!(s.base_i64("weather_retries"), 3);
        assert_eq!(s.base_f64("weather_retry_delay"), 1.0);
        assert_eq!(s.base_str("default_theme"), "auto");
        assert_eq!(s.base_str("default_city"), "北京");
        assert_eq!(s.base_str("default_timezone"), "Asia/Shanghai");
        assert_eq!(s.base_i64("max_alarms"), 10);
    }

    #[test]
    fn base_str_list_token() {
        // default_world_pins：8 个城市名（FIX007.2 域统一），首尾锚定
        let pins = Static::get().base_str_list("default_world_pins");
        assert_eq!(pins.len(), 8);
        assert_eq!(pins[0], "北京");
        assert_eq!(pins[7], "首尔");
    }

    #[test]
    fn ui_domain_tokens() {
        // ui 域真实 token 抽样：字体族/字号/实心色板（UIR 后 colors 域为唯一色彩源）
        let s = Static::get();
        assert_eq!(s.ui_str("font_family"), "Microsoft YaHei");
        assert_eq!(s.ui_str("font_family_digits"), "Bahnschrift");
        assert_eq!(s.ui_nested_i64("scale", "hero_time"), 64);
        let window_bg = s.ui.pointer("/colors/window_bg").and_then(|v| v.as_str());
        assert_eq!(window_bg, Some("#171A21"));
    }

    #[test]
    #[should_panic(expected = "缺键或非数值")]
    fn missing_base_key_panics() {
        // 缺键快速失败：panic 带键名（Python KeyError 对等）
        Static::get().base_f64("no_such_key");
    }

    #[test]
    #[should_panic(expected = "缺键或非字符串")]
    fn wrong_type_base_key_panics() {
        // 错型快速失败：数值键按字符串取 → panic
        Static::get().base_str("rate_min");
    }

    #[test]
    #[should_panic(expected = "缺嵌套键")]
    fn missing_nested_ui_key_panics() {
        // ui 嵌套缺键快速失败
        Static::get().ui_nested_i64("scale", "no_such");
    }
}
