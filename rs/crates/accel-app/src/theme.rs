//! 主题设计系统（Python qss/token 体系的 Rust 对等物，PL013.08）
//! Theme 装载 ui.json 全 token → egui Visuals 深浅两套 apply；色值 #AARRGGBB 解析

use accel_core::static_config::Static;
use egui::{Color32, Context, Visuals};
use serde_json::Value;

/// 深浅两态 token 集（field.{dark,light} 域镜像；色值非预乘 rgba 0..1——shader 直传语义）
#[derive(Debug, Clone, Copy)]
pub struct FieldTokens {
    /// 背板上/下渐变（含 alpha）
    pub top: [f32; 4],
    pub bottom: [f32; 4],
    /// 光场双晕（色 + 位置/半径，uv 口径）
    pub glow_a: [f32; 4],
    pub glow_a_x: f32,
    pub glow_a_y: f32,
    pub glow_a_r: f32,
    pub glow_b: [f32; 4],
    pub glow_b_x: f32,
    pub glow_b_y: f32,
    pub glow_b_r: f32,
}

impl FieldTokens {
    /// 装载指定态（"dark"/"light"；field 为三级嵌套 field.{mode}.{key}，pointer 直取）
    pub fn load(mode: &str) -> FieldTokens {
        let s = Static::get();
        let str_at = |key: &str| {
            let ptr = format!("/field/{mode}/{key}");
            match s.ui.pointer(&ptr).and_then(serde_json::Value::as_str) {
                Some(v) => v.to_string(),
                None => panic!("ui.json 缺嵌套键: {ptr}"),
            }
        };
        let f64_at = |key: &str| {
            let ptr = format!("/field/{mode}/{key}");
            match s.ui.pointer(&ptr).and_then(serde_json::Value::as_f64) {
                Some(v) => v as f32,
                None => panic!("ui.json 缺嵌套键: {ptr}"),
            }
        };
        let color = |key: &str| parse_aarrggbb(&str_at(key)).unwrap_or([0.0; 4]);
        FieldTokens {
            top: color("top"),
            bottom: color("bottom"),
            glow_a: color("glow_a"),
            glow_a_x: f64_at("glow_a_x"),
            glow_a_y: f64_at("glow_a_y"),
            glow_a_r: f64_at("glow_a_r"),
            glow_b: color("glow_b"),
            glow_b_x: f64_at("glow_b_x"),
            glow_b_y: f64_at("glow_b_y"),
            glow_b_r: f64_at("glow_b_r"),
        }
    }
}

/// 主题（ui.json token 装载 + egui Visuals 应用）
pub struct Theme {
    /// 三态偏好（auto=跟随系统 / light / dark）
    pub pref: &'static str,
    pub dark: FieldTokens,
    pub light: FieldTokens,
    /// 系统深浅（A005 #30：auto 态生效色由它决定，与 egui ThemePreference::System 同源）
    system_is_dark: bool,
}

impl Theme {
    /// 全量装载（ui.json → 结构化 token）
    pub fn load(pref: &str) -> Theme {
        Theme {
            pref: match pref {
                "light" | "dark" => match pref {
                    "light" => "light",
                    _ => "dark",
                },
                _ => "auto",
            },
            dark: FieldTokens::load("dark"),
            light: FieldTokens::load("light"),
            system_is_dark: true,
        }
    }

    /// 注入系统深浅（A005 #30：auto 态的 GL token 与 egui 的 System 偏好必须同源）
    pub fn with_system(mut self, system_is_dark: bool) -> Self {
        self.system_is_dark = system_is_dark;
        self
    }

    /// 生效态 token（auto → 随注入的系统深浅；显式态直取）
    pub fn active(&self) -> &FieldTokens {
        match self.pref {
            "light" => &self.light,
            "dark" => &self.dark,
            _ if self.system_is_dark => &self.dark,
            _ => &self.light,
        }
    }

    /// 深色判定（Visuals 选择与文字色基准；auto 随系统）
    pub fn is_dark(&self) -> bool {
        match self.pref {
            "light" => false,
            "dark" => true,
            _ => self.system_is_dark,
        }
    }

    /// 生效主题标识（"dark"/"light"——glass.{mode} 域取值；A005 #5）
    pub fn mode(&self) -> &'static str {
        if self.is_dark() { "dark" } else { "light" }
    }

    /// 生效态玻璃提亮（ui.json glass.{mode}.gl_lift，0-255 → 0..1；缺键兜底 0.10）
    /// A005 #5 修正：原读不存在的 /glass/gl_lift 恒走兜底，且深浅不分
    pub fn glass_lift(&self) -> f32 {
        let ptr = format!("/glass/{}/gl_lift", self.mode());
        Static::get()
            .ui
            .pointer(&ptr)
            .and_then(Value::as_f64)
            .map(|v| (v / 255.0) as f32)
            .unwrap_or(0.10)
    }

    /// 生效态玻璃 tint（ui.json glass.{mode}.gl_tint 的 rgb 0..1；解析失败兜底深色值）
    pub fn glass_tint(&self) -> [f32; 3] {
        let ptr = format!("/glass/{}/gl_tint", self.mode());
        Static::get()
            .ui
            .pointer(&ptr)
            .and_then(Value::as_str)
            .and_then(parse_aarrggbb)
            .map(|c| [c[0], c[1], c[2]])
            .unwrap_or([0.16, 0.14, 0.20])
    }

    /// 应用到 egui 上下文：双主题并存体系（0.37）——深浅各自 set_visuals_of +
    /// ThemePreference 三态（auto=System 跟随，egui 内建，替代 Python 手工跟随）
    pub fn apply(&self, ctx: &Context) {
        ctx.set_theme(match self.pref {
            "light" => egui::ThemePreference::Light,
            "dark" => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::System,
        });
        ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
        ctx.set_visuals_of(egui::Theme::Light, light_visuals());
    }
}

/// 深色 Visuals（对齐 Python qfw 深色观感：微黑底/白字/低对比弱文本）
/// A005 #9：底色/文字色全部取 colors.* token
fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.panel_fill = color32_at("panel_dark", 18);
    v.window_fill = color32_at("window_dark", 22);
    v.extreme_bg_color = color32_at("bg_dark", 12);
    v.faint_bg_color = Color32::from_white_alpha(6);
    v.override_text_color = Some(color32_at("text_strong", 235));
    v.widgets.noninteractive.fg_stroke.color = color32_at("text_strong", 235);
    v.selection.stroke.color = color32_at("selection_dark", 25);
    v.window_corner_radius = corner(radius_px("lg") as u8);
    v.menu_corner_radius = corner(radius_px("md") as u8);
    widget_rounding(&mut v, corner(8));
    v
}

/// 浅色 Visuals（对齐 Python 浅色观感：亮底/深字）
fn light_visuals() -> Visuals {
    let mut v = Visuals::light();
    v.panel_fill = color32_at("panel_light", 244);
    v.window_fill = color32_at("window_light", 250);
    v.extreme_bg_color = color32_at("bg_light", 255);
    v.override_text_color = Some(color32_at("text_light", 35));
    v.widgets.noninteractive.fg_stroke.color = color32_at("text_light", 35);
    v.window_corner_radius = corner(radius_px("lg") as u8);
    v.menu_corner_radius = corner(radius_px("md") as u8);
    widget_rounding(&mut v, corner(8));
    v
}

/// 交互态控件圆角统一（inactive/hovered/active 三态同值）
fn widget_rounding(v: &mut Visuals, r: egui::epaint::CornerRadius) {
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
}

/// 四角等值圆角（epaint CornerRadius：u8 四角）
fn corner(px: u8) -> egui::epaint::CornerRadius {
    egui::epaint::CornerRadius::same(px)
}

/// 字号 token → 逻辑点（scale.{key}：hero_time/page_title/city_time/std_time/slider_value）
pub fn font_size(key: &str) -> f32 {
    Static::get().ui_nested_i64("scale", key) as f32
}

/// 选中强调色（colors.primary token，非预乘 rgba——GL u_accent 与导航描边/文字单源）
/// A005 #23：token 为编译期嵌入常量，结果缓存于 OnceLock（消除每帧 pointer 解析）
pub fn accent_rgba() -> [f32; 4] {
    static CACHE: std::sync::OnceLock<[f32; 4]> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        Static::get()
            .ui
            .pointer("/colors/primary")
            .and_then(Value::as_str)
            .and_then(parse_aarrggbb)
            .unwrap_or([0.89, 0.70, 0.42, 1.0]) // 键损坏兜底（暖金近似值，仅防御路径）
    })
}

/// 语义色等级（ui.json colors.* → Color32；A005 #9：页面/控件不再写字面色值）
#[derive(Debug, Clone, Copy)]
pub struct Ink {
    pub strong: Color32,
    pub muted: Color32,
    pub soft: Color32,
    pub dim: Color32,
    pub card: Color32,
    pub knob: Color32,
    pub danger: Color32,
    /// 信息色（降水概率等辅助数据）
    pub info: Color32,
}

/// colors.{key} → Color32（#RRGGBB / #AARRGGBB 均收；缺键/非法回退灰度兜底）
pub fn color32_at(key: &str, fallback_gray: u8) -> Color32 {
    let ptr = format!("/colors/{key}");
    Static::get()
        .ui
        .pointer(&ptr)
        .and_then(Value::as_str)
        .and_then(parse_aarrggbb)
        .map(|c| {
            Color32::from_rgba_premultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (c[3] * 255.0) as u8,
            )
        })
        .unwrap_or(Color32::from_gray(fallback_gray))
}

/// 装载语义色（colors.* 逐键；缺键/非法回退灰度兜底，仅防御路径）
pub fn ink() -> Ink {
    Ink {
        strong: color32_at("text_strong", 235),
        muted: color32_at("text_muted", 200),
        soft: color32_at("text_soft", 170),
        dim: color32_at("text_dim", 120),
        card: color32_at("text_card", 225),
        knob: color32_at("knob", 240),
        danger: color32_at("danger", 228),
        info: color32_at("info", 140),
    }
}

/// 数值型 layout token（A005 #9：进度环径/滑轨高等几何参数单源）
pub fn layout_i64(key: &str) -> i64 {
    Static::get().ui_nested_i64("layout", key)
}

/// 页面圆角（ui.json radius.lg；A005 #9：页卡/浮窗圆角单源）
pub fn radius_px(key: &str) -> f32 {
    Static::get().ui_nested_i64("radius", key) as f32
}

/// 强调色 Color32 形态（egui 矢量描边/文字用；非预乘 → 预乘换算；A005 #23 同源缓存）
pub fn accent_color() -> Color32 {
    static CACHE: std::sync::OnceLock<Color32> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        let [r, g, b, a] = accent_rgba();
        Color32::from_rgba_premultiplied(
            (r * a * 255.0) as u8,
            (g * a * 255.0) as u8,
            (b * a * 255.0) as u8,
            (a * 255.0) as u8,
        )
    })
}

/// "#AARRGGBB" / "#RRGGBB" → 非预乘 rgba f32（Python QColor 同格式；非法串返 None 降级）
/// A005 执行补录：原实现只收 8 位，而 ui.json 的 colors.* 实为 6 位 #RRGGBB →
/// 强调色/语义色 token 全部静默走兜底；现 6 位按不透明（alpha=FF）解析
/// egui 0.37 Color32 移除非预乘构造——token 层直接以 f32 数组承载（shader 直传）
pub fn parse_aarrggbb(text: &str) -> Option<[f32; 4]> {
    let hex = text.strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let (a, r, g, b) = match hex.len() {
        8 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        ),
        6 => (
            0xFF,
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        ),
        _ => return None,
    };
    Some([
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ])
}

// ===== src/theme.rs 说明区 =====
// 输入：ui.json field/scale 等 token；输出：FieldTokens（渲染 uniforms 源）+ Visuals 应用
// 逻辑步骤：load（#AARRGGBB 解析 + 嵌套 f64 取值）→ active()（三态）→ apply(ctx)
// 设计理由：Python 版主题三件（qss 深浅、field 渐变光场、gl_lift）在 Rust 拆为
//   theme.rs（Visuals/文字色）与 glass uniform 源（FieldTokens 直传 painter）；
//   三态偏好经 SetTheme 命令持久化（interface 已接线），Theme::pref 由 UiState 回推重建
// 异常处理：token 缺键由 static_config panic（启动期暴露）；色值非法降级透明
//   （field 域全键齐备，防御路径）
// 关联配置：ui.json field.*（dark/light 全键）；消费方：window.rs（apply/传 token）、
//   glass/painter.rs（FieldTokens → uniforms）
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_aarrggbb_matrix() {
        // 真实 token（ui.json field.dark.top）+ 非法形态
        let got = parse_aarrggbb("#230D1322").expect("合法色");
        assert!((got[0] - 0x0D as f32 / 255.0).abs() < 1e-6);
        assert!((got[1] - 0x13 as f32 / 255.0).abs() < 1e-6);
        assert!((got[2] - 0x22 as f32 / 255.0).abs() < 1e-6);
        assert!((got[3] - 0x23 as f32 / 255.0).abs() < 1e-6);
        assert_eq!(parse_aarrggbb("#FFF"), None);
        assert_eq!(parse_aarrggbb("#GGGGGGGG"), None);
        assert_eq!(parse_aarrggbb("no-hash"), None);
    }

    #[test]
    fn load_tokens_match_ui_json() {
        // field.dark 真实 token 锚点（与 ui.json 逐值相等）
        let dark = FieldTokens::load("dark");
        assert_eq!(dark.top, parse_aarrggbb("#230D1322").unwrap());
        assert!((dark.glow_a_x - 0.84).abs() < 1e-6);
        assert!((dark.glow_a_y - 0.0).abs() < 1e-6);
        assert!((dark.glow_a_r - 0.38).abs() < 1e-6);
    }

    #[test]
    fn glass_tokens_follow_mode() {
        // A005 #5 回归：gl_lift/gl_tint 按生效主题取自 glass.{mode}（原读 /glass/gl_lift 恒兜底）
        let dark = Theme::load("dark");
        assert_eq!(dark.mode(), "dark");
        assert!(
            (dark.glass_lift() - 26.0 / 255.0).abs() < 1e-6,
            "dark gl_lift token = 26"
        );
        let light = Theme::load("light");
        assert_eq!(light.mode(), "light");
        assert!(
            (light.glass_lift() - 34.0 / 255.0).abs() < 1e-6,
            "light gl_lift token = 34"
        );
        assert_ne!(
            dark.glass_lift(),
            light.glass_lift(),
            "深浅 gl_lift 必须可区分"
        );
        let t = light.glass_tint();
        assert!(
            t.iter().all(|c| (0.0..=1.0).contains(c)) && t.iter().any(|c| *c > 0.0),
            "tint 应为有效 rgb 0..1，实际 {t:?}"
        );
    }

    #[test]
    fn theme_three_state_and_apply() {
        // 三态装载 + 生效态切换（auto→dark 兜底；light→light）
        let theme = Theme::load("auto").with_system(true);
        assert!(theme.is_dark(), "auto + 系统深色 → 深色");
        assert_eq!(theme.active().top, FieldTokens::load("dark").top);
        // A005 #30 回归：auto + 系统浅色 → 走浅色 token（与 egui System 同源）
        let auto_light = Theme::load("auto").with_system(false);
        assert!(!auto_light.is_dark(), "auto + 系统浅色 → 浅色");
        assert_eq!(auto_light.active().top, FieldTokens::load("light").top);
        assert!(
            (auto_light.glass_lift() - 34.0 / 255.0).abs() < 1e-6,
            "auto 浅色下 gl_lift 取 light 档"
        );
        let light = Theme::load("light");
        assert!(!light.is_dark());
        assert_eq!(light.active().top, FieldTokens::load("light").top);

        // apply 后 ctx 双主题断言（0.37 样式体系：两套并存 + 偏好三态；
        // 无 theme_preference getter——生效主题经 theme() 断言）
        let ctx = Context::default();
        Theme::load("light").apply(&ctx);
        assert_eq!(ctx.theme(), egui::Theme::Light);
        assert!(!ctx.style_of(egui::Theme::Light).visuals.dark_mode);
        Theme::load("dark").apply(&ctx);
        assert_eq!(ctx.theme(), egui::Theme::Dark);
        assert!(ctx.style_of(egui::Theme::Dark).visuals.dark_mode);
        Theme::load("auto").apply(&ctx);
        let _ = ctx.theme(); // System 跟随：值随环境，合法性由 light/dark 两态覆盖
    }
}
