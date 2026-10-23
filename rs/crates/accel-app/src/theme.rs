//! 主题设计系统（UIR 重塑：实心桌面主题，玻璃 token 体系已删）
//! ui.json colors.* 深色单板 → egui Visuals 应用 + 语义色/字号/几何取值
//! （A008 #2 定案仅深色：浅色体系与三态偏好切换已删，ThemePref 配置域仅存档）

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use accel_core::static_config::Static;
use egui::{Color32, Context, Visuals};
use serde_json::Value;

/// 应用到 egui 上下文（A008 #2 定案仅深色）：偏好钉深 + 双槽位都注入深色
/// Visuals——即便 egui 主题被切到 Light 槽也是同款深色，浅色体系无分叉残留
pub fn apply(ctx: &Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
    ctx.set_visuals_of(egui::Theme::Light, dark_visuals());
}

/// colors.{key} → Color32（#RRGGBB；缺键/非法回退灰度兜底，仅防御路径）
/// 结果缓存（token 编译期嵌入，运行期恒定）
pub fn color32_at(key: &str, fallback_gray: u8) -> Color32 {
    static C: OnceLock<Mutex<HashMap<String, Color32>>> = OnceLock::new();
    let cache_key = format!("{key}#{fallback_gray}");
    cached(&C, &cache_key, || {
        let ptr = format!("/colors/{key}");
        Static::get()
            .ui
            .pointer(&ptr)
            .and_then(Value::as_str)
            .and_then(parse_hex)
            .unwrap_or(Color32::from_gray(fallback_gray))
    })
}

/// "#RRGGBB" / "#AARRGGBB" → 预乘 Color32（egui 0.37 全预乘口径；非法串返 None）
pub fn parse_hex(text: &str) -> Option<Color32> {
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
    // 预乘换算（非预乘色值 → egui 预乘存储）
    let lut = |v: u8| ((v as u32 * a as u32 + 127) / 255) as u8;
    Some(Color32::from_rgba_premultiplied(lut(r), lut(g), lut(b), a))
}

/// 强调色（colors.primary；选中态/当前值/主操作唯一用色）
pub fn accent_color() -> Color32 {
    color32_at("primary", 0xE4)
}

/// 金底上的前景色（colors.on_accent——对比度锚定）
pub fn on_accent_color() -> Color32 {
    color32_at("on_accent", 0x1A)
}

/// 语义色等级（UIR 实心主题：strong=主文本 muted=次级 soft=辅助 dim=弱化
/// card=卡面文本 knob=滑钮 danger=危险 info=信息）
#[derive(Debug, Clone, Copy)]
pub struct Ink {
    pub strong: Color32,
    pub muted: Color32,
    pub soft: Color32,
    pub dim: Color32,
    pub card: Color32,
    pub knob: Color32,
    pub danger: Color32,
}

/// 装载语义色（colors.* 逐键；整组一次装载后缓存）
pub fn ink() -> Ink {
    static CACHE: OnceLock<Ink> = OnceLock::new();
    *CACHE.get_or_init(|| Ink {
        strong: color32_at("text_strong", 232),
        muted: color32_at("text_muted", 154),
        soft: color32_at("text_soft", 169),
        dim: color32_at("text_dim", 98),
        card: color32_at("text_card", 213),
        knob: color32_at("knob", 242),
        danger: color32_at("danger", 228),
    })
}

/// 卡面底色（colors.card_bg）
pub fn card_bg() -> Color32 {
    color32_at("card_bg", 31)
}

/// 卡面悬停底色（colors.faint_bg）
pub fn hover_bg() -> Color32 {
    color32_at("faint_bg", 38)
}

/// 描边色（colors.border）
pub fn border_color() -> Color32 {
    color32_at("border", 44)
}

/// 侧栏底色（colors.sidebar_bg）
pub fn sidebar_bg() -> Color32 {
    color32_at("sidebar_bg", 18)
}

/// 字号 token → 逻辑点（scale.{key}；结果缓存——编译期字面量键，运行期恒定）
pub fn font_size(key: &str) -> f32 {
    static C: OnceLock<Mutex<HashMap<String, f32>>> = OnceLock::new();
    cached(&C, key, || Static::get().ui_nested_i64("scale", key) as f32)
}

/// 键值 token 缓存：消除每帧 pointer 解析与键串分配（UIR 沿用 FIX006.10 模式）
pub(crate) fn cached<T: Clone + 'static>(
    cache: &'static OnceLock<Mutex<HashMap<String, T>>>,
    key: &str,
    load: impl FnOnce() -> T,
) -> T {
    let mut m = cache
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("token 缓存锁中毒");
    if let Some(v) = m.get(key) {
        return v.clone();
    }
    let v = load();
    m.insert(key.to_string(), v.clone());
    v
}

/// 数值型 layout token（结果缓存）
pub fn layout_i64(key: &str) -> i64 {
    static C: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
    cached(&C, key, || Static::get().ui_nested_i64("layout", key))
}

// A009 #14：layout_vec2（chip_size 双值槽）已删——chip 宽自适应后仅消费高度，
// token 改单值 layout.chip_height（零半死槽位）

/// 圆角（ui.json radius.*；结果缓存）
pub fn radius_px(key: &str) -> f32 {
    static C: OnceLock<Mutex<HashMap<String, f32>>> = OnceLock::new();
    cached(&C, key, || {
        Static::get().ui_nested_i64("radius", key) as f32
    })
}

/// 深色 Visuals（实心主题：暗底/亮字/低对比弱文本；控件填充走色板）
fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.panel_fill = color32_at("window_bg", 23);
    v.window_fill = color32_at("card_bg", 31);
    v.extreme_bg_color = color32_at("sidebar_bg", 18);
    v.faint_bg_color = color32_at("faint_bg", 38);
    v.override_text_color = Some(color32_at("text_strong", 232));
    v.selection.stroke.color = color32_at("selection", 46);
    v.widgets.noninteractive.bg_fill = color32_at("card_bg", 31);
    v.widgets.inactive.bg_fill = color32_at("card_bg", 31);
    v.widgets.hovered.bg_fill = color32_at("faint_bg", 38);
    v.widgets.active.bg_fill = color32_at("selection", 46);
    v.widgets.inactive.fg_stroke.color = color32_at("text_strong", 232);
    v.widgets.hovered.fg_stroke.color = color32_at("text_strong", 232);
    v.window_corner_radius = corner(radius_px("lg") as u8);
    v.menu_corner_radius = corner(radius_px("md") as u8);
    widget_rounding(&mut v, corner(radius_px("sm") as u8));
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

// ===== src/theme.rs 说明区 =====
// 输入：ui.json colors/scale/layout/radius token；输出：Visuals 应用 + 语义色/
//   字号/几何/色值取值（全部实心色，无透明无 GL）
// 逻辑步骤：apply（A008 #2 定案仅深色：偏好钉深 + 双槽位同款深色 Visuals）→
//   color32_at/ink/font_size/layout/radius 取值（cached 键缓存，运行期恒定）
// 设计理由（UIR 重塑定案）：玻璃拟态整体废弃——半透明面在高 DPI/动态背景上
//   对比度不可控（截图探针实证文字淹没）；实体色板 + 单强调色纪律（金=选中/
//   当前值/主操作，余皆灰阶）是可用性优先的桌面语法；parse_hex 直接产预乘
//   Color32（0.37 全预乘口径，替换旧 rgba f32 管线中间层）
// 异常处理：token 缺键由 static_config panic（启动期暴露）；色值非法降级灰度
// 关联配置：ui.json colors.*/scale.*/layout.*/radius.*；消费方：window/pages/widgets
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_tokens_match_ui_json() {
        // 色板锚点：主色金 + 深底（UIR 实心主题定值）
        assert_eq!(
            color32_at("primary", 0),
            parse_hex("#E4B36A").expect("合法")
        );
        assert_eq!(
            color32_at("window_bg", 0),
            parse_hex("#171A21").expect("合法")
        );
        // 语义色整组
        let ink = ink();
        assert_eq!(ink.strong, color32_at("text_strong", 0));
    }

    #[test]
    fn parse_hex_matrix() {
        // 6 位不透明 / 8 位带 alpha / 非法降级
        assert_eq!(
            parse_hex("#E4B36A"),
            Some(Color32::from_rgb(0xE4, 0xB3, 0x6A))
        );
        assert!(parse_hex("#80E4B36A").is_some());
        assert_eq!(parse_hex("zzz"), None);
        assert_eq!(parse_hex("#12345"), None);
    }

    #[test]
    fn dark_only_apply() {
        // A008 #2 验收：仅深色定案——apply 后 egui 主题钉深；
        // 浅色体系（light_visuals/Theme 三态）已删，无分叉可回退
        let ctx = Context::default();
        apply(&ctx);
        assert_eq!(ctx.theme(), egui::Theme::Dark);
    }

    #[test]
    fn color32_at_same_key_different_fallback_isolated() {
        // 缺键路径下同 key 不同兜底互不污染（缓存键含兜底灰——FIX007.8 延续）
        let a = color32_at("no_such_color_key", 10);
        let b = color32_at("no_such_color_key", 200);
        assert_eq!(a, Color32::from_gray(10));
        assert_eq!(b, Color32::from_gray(200), "同 key 不同兜底不得固化首值");
    }
}
