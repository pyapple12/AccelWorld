//! 主题设计系统（UIR2「子夜星图」：黛蓝星图底 × 暖米白字 × 雄黄金单强调）
//! ui.json token → egui Visuals 应用 + 语义色/字号/间距/圆角取值（cached 键缓存）
//! 对比度纪律：全部文本 token 对其渲染底 ≥4.5:1（.temp/contrast_check.awk 实测矩阵）

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

/// 不透明色 → 指定 alpha 的预乘色（PL016.06 动效：hover/脉冲按进度染透明度）
pub fn with_alpha(c: Color32, a: u8) -> Color32 {
    let lut = |v: u8| ((v as u32 * a as u32 + 127) / 255) as u8;
    Color32::from_rgba_premultiplied(lut(c.r()), lut(c.g()), lut(c.b()), a)
}

/// 强调色（colors.accent——雄黄金：选中态/当前值/主操作唯一用色）
pub fn accent_color() -> Color32 {
    color32_at("accent", 0xE8)
}

/// 金底上的前景色（colors.on_accent——对比度锚定 9.13:1 实测）
pub fn on_accent_color() -> Color32 {
    color32_at("on_accent", 0x22)
}

/// 语义色等级（UIR2 五层：strong=主文 body=正文/卡文 muted=次级
/// meta=刻度/元信息 danger=朱砂 knob=滑钮亮白——soft/card/dim 三层已并）
#[derive(Debug, Clone, Copy)]
pub struct Ink {
    pub strong: Color32,
    pub body: Color32,
    pub muted: Color32,
    pub meta: Color32,
    pub danger: Color32,
    pub success: Color32,
    pub knob: Color32,
}

/// 装载语义色（colors.* 逐键；整组一次装载后缓存）
pub fn ink() -> Ink {
    static CACHE: OnceLock<Ink> = OnceLock::new();
    *CACHE.get_or_init(|| Ink {
        strong: color32_at("text_strong", 236),
        body: color32_at("text_body", 207),
        muted: color32_at("text_muted", 154),
        meta: color32_at("text_meta", 138),
        danger: color32_at("danger", 238),
        success: color32_at("success", 95),
        knob: color32_at("knob", 242),
    })
}

/// 卡面底色（colors.card_bg）
pub fn card_bg() -> Color32 {
    color32_at("card_bg", 20)
}

/// 输入框填充（colors.input_bg——比卡面更深一档的“洞”）
pub fn input_bg() -> Color32 {
    color32_at("input_bg", 11)
}

/// 输入框描边（colors.input_border——对 input_bg ≥3:1，非文本可辨）
pub fn input_border_color() -> Color32 {
    color32_at("input_border", 85)
}

/// 强描边（colors.border_strong——可点击表面/控件边界，对卡面 ≥3:1）
pub fn border_strong_color() -> Color32 {
    color32_at("border_strong", 85)
}

/// 语义成功色（colors.success——开关 on，替代品牌金）
pub fn success_color() -> Color32 {
    color32_at("success", 95)
}

/// 星点色（colors.star——低对比背景纹理）
pub fn star_color() -> Color32 {
    color32_at("star", 42)
}

/// 卡面悬停底色（colors.hover_bg）
pub fn hover_bg() -> Color32 {
    color32_at("hover_bg", 26)
}

/// 描边色（colors.border——发丝线）
pub fn border_color() -> Color32 {
    color32_at("border", 35)
}

/// 侧栏底色（colors.sidebar_bg——比窗底更深一档）
pub fn sidebar_bg() -> Color32 {
    color32_at("sidebar_bg", 11)
}

/// 窗底色（colors.window_bg）
pub fn window_bg() -> Color32 {
    color32_at("window_bg", 15)
}

/// 弱轨底色（colors.track——距条轨/开关关态单源，原双键同值归一）
pub fn track_color() -> Color32 {
    color32_at("track", 48)
}

/// 星轨未过刻度色（colors.tick_idle——信息性图形件，对窗底 ≥3:1 实测 3.13）
pub fn tick_idle_color() -> Color32 {
    color32_at("tick_idle", 85)
}

/// 字号 token → 逻辑点（scale.{key}；结果缓存——编译期字面量键，运行期恒定）
pub fn font_size(key: &str) -> f32 {
    static C: OnceLock<Mutex<HashMap<String, f32>>> = OnceLock::new();
    cached(&C, key, || Static::get().ui_nested_i64("scale", key) as f32)
}

/// 中文字体族（Regular，Proportional 默认）——正文/标签
pub fn font_regular(size: f32) -> egui::FontId {
    egui::FontId::proportional(size)
}

/// 中文字体族（Medium 500）——次级标题/按钮/标签
pub fn font_medium(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("medium".into()))
}

/// 中文字体族（Semibold 600）——页面标题/分区标题/强调
pub fn font_semibold(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("semibold".into()))
}

/// 布局 token → f32（layout.{key}）
pub fn layout_f32(key: &str) -> f32 {
    layout_i64(key) as f32
}

/// 窗口最大内尺寸（layout.window_max_*）
pub fn window_max_size() -> egui::Vec2 {
    egui::vec2(
        layout_f32("window_max_width"),
        layout_f32("window_max_height"),
    )
}

/// 页面头部高度（layout.page_header）
pub fn page_header_height() -> f32 {
    layout_f32("page_header")
}

/// 页面页脚高度（layout.page_footer）
pub fn page_footer_height() -> f32 {
    layout_f32("page_footer")
}

/// sRGB 相对亮度（WCAG；对比度测试用）
pub fn relative_luminance(c: Color32) -> f32 {
    let lin = |v: u8| {
        let s = v as f32 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// WCAG 对比度（1..21；同色/非法不 panic）
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// 间距 token → 逻辑点（spacing.{key}——UIR2 新域：页面/卡内间距不再写魔法数）
pub fn spacing(key: &str) -> f32 {
    static C: OnceLock<Mutex<HashMap<String, f32>>> = OnceLock::new();
    cached(&C, key, || {
        Static::get().ui_nested_i64("spacing", key) as f32
    })
}

/// 动效时长 token → 秒（motion.{key}——PL016.06 三级动效单源；0.001 精度足够）
pub fn motion_ms(key: &str) -> f32 {
    static C: OnceLock<Mutex<HashMap<String, f32>>> = OnceLock::new();
    cached(&C, key, || {
        Static::get().ui_nested_i64("motion", key) as f32
    })
}

/// 动效弹簧参数（motion.drawer_bouncy_{hz_x10,zeta_x100}——ui_nested_i64 整型
/// 单源，读取侧除回实际值）——(频率 Hz, 阻尼比 ζ<1 过冲)
pub fn drawer_spring_params() -> (f32, f32) {
    (
        motion_ms("drawer_bouncy_hz_x10") / 10.0,
        motion_ms("drawer_bouncy_zeta_x100") / 100.0,
    )
}

/// 减少动效开关读取（PL016.06：每帧由 window 写入 ctx data，控件层读取；
/// 缺省 false——关是默认态，开是用户显式选择）
pub fn reduce_motion(ctx: &egui::Context) -> bool {
    ctx.data(|d| {
        d.get_temp::<bool>(egui::Id::unique("accel.reduce_motion"))
            .unwrap_or(false)
    })
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

/// 抽屉全开宽度（FIX011.03：比例与钳界 layout token 单源——原 0.46/[280,420]
/// 魔法数三处内联；纯函数可测）
pub fn drawer_width(avail_w: f32) -> f32 {
    let ratio = layout_i64("drawer_width_ratio_x100") as f32 / 100.0;
    (avail_w * ratio).clamp(
        layout_i64("drawer_width_min") as f32,
        layout_i64("drawer_width_max") as f32,
    )
}

/// 圆角（ui.json radius.*；结果缓存）
pub fn radius_px(key: &str) -> f32 {
    static C: OnceLock<Mutex<HashMap<String, f32>>> = OnceLock::new();
    cached(&C, key, || {
        Static::get().ui_nested_i64("radius", key) as f32
    })
}

/// 深色 Visuals（子夜星图：黛蓝底/暖米白字；控件填充走色板）
fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.panel_fill = window_bg();
    v.window_fill = card_bg();
    v.extreme_bg_color = input_bg();
    v.faint_bg_color = hover_bg();
    v.override_text_color = Some(ink().strong);
    // 输入框占位等弱文本对齐 muted 层（egui dark 默认 3.06:1 不达标——UIR2 二轮实测）
    v.weak_text_color = Some(ink().muted);
    v.selection.stroke.color = color32_at("selection", 42);
    v.widgets.noninteractive.bg_fill = card_bg();
    v.widgets.inactive.bg_fill = card_bg();
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, input_border_color());
    v.widgets.hovered.bg_fill = hover_bg();
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, border_strong_color());
    v.widgets.active.bg_fill = color32_at("selection", 42);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent_color());
    v.widgets.inactive.fg_stroke.color = ink().strong;
    v.widgets.hovered.fg_stroke.color = ink().strong;
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
// 输入：ui.json colors/scale/spacing/radius/layout token；输出：Visuals 应用 +
//   语义色/字号/间距/圆角/色值取值（全部实心色，无透明无 GL）
// 逻辑步骤：apply（仅深色定案：偏好钉深 + 双槽位同款深色 Visuals）→
//   color32_at/ink/font_size/spacing/layout/radius 取值（cached 键缓存）
// 设计理由（UIR2 子夜星图）：旧板两处硬伤实测在案——text_dim 卡面 2.85:1
//   失效（正文级信息低于 AA）、text_soft 与 text_muted 仅差 1.28:1 同职重叠；
//   新板五层文本各距 ≥1.1:1 梯度且全矩阵 ≥4.5:1（.temp/contrast_check.awk）；
//   色键按角色命名（accent/track/hover_bg），ramp 单源（ring/slider 双键归一）
// 异常处理：token 缺键由 static_config panic（启动期暴露）；色值非法降级灰度
// 关联配置：ui.json colors.*/scale.*/spacing.*/radius.*/layout.*；消费方：window/pages/widgets
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_tokens_match_ui_json() {
        // 色板锚点：雄黄金 + 黛蓝星图底（UIR2 子夜星图定值）
        assert_eq!(color32_at("accent", 0), parse_hex("#E8B544").expect("合法"));
        assert_eq!(
            color32_at("window_bg", 0),
            parse_hex("#0F1420").expect("合法")
        );
        // 语义色整组
        let ink = ink();
        assert_eq!(ink.strong, color32_at("text_strong", 0));
    }

    #[test]
    fn parse_hex_matrix() {
        // 6 位不透明 / 8 位带 alpha / 非法降级
        assert_eq!(
            parse_hex("#E8B544"),
            Some(Color32::from_rgb(0xE8, 0xB5, 0x44))
        );
        assert!(parse_hex("#80E8B544").is_some());
        assert_eq!(parse_hex("zzz"), None);
        assert_eq!(parse_hex("#12345"), None);
    }

    #[test]
    fn dark_only_apply() {
        // 仅深色定案——apply 后 egui 主题钉深；浅色体系无分叉可回退
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

    #[test]
    fn text_ink_tiers_are_distinct() {
        // UIR2 验收：五层文本色两两不同（旧板 soft/muted 重叠治理）
        let ink = ink();
        let tiers = [ink.strong, ink.body, ink.muted, ink.meta];
        for i in 0..tiers.len() {
            for j in (i + 1)..tiers.len() {
                assert_ne!(tiers[i], tiers[j], "文本层级 {i}/{j} 不得同值");
            }
        }
    }

    #[test]
    fn control_borders_meet_non_text_contrast() {
        // PL017：输入框/可点击表面边界 ≥3:1（非文本 1.4.11 口径）
        let input = contrast_ratio(input_border_color(), input_bg());
        assert!(input >= 3.0, "输入框边界 {input:.2}:1 < 3:1");
        let strong = contrast_ratio(border_strong_color(), card_bg());
        assert!(strong >= 3.0, "强描边 {strong:.2}:1 < 3:1");
        let track = contrast_ratio(color32_at("track", 48), window_bg());
        assert!(track >= 1.6, "轨道为弱轨（非控件唯一边界）：{track:.2}");
        let text = contrast_ratio(ink().strong, accent_color());
        assert!(
            text >= 1.5,
            "文字不应直接压在金底（记录值 {text:.2}）；用 on_accent"
        );
        let on_accent = contrast_ratio(on_accent_color(), accent_color());
        assert!(on_accent >= 4.5, "金底前景 {on_accent:.2} < 4.5:1");
    }

    #[test]
    fn semantic_colors_are_not_the_accent() {
        // PL017：金只做数据/当前值；成功/焦点不得复用 accent
        assert_ne!(success_color(), accent_color());
        assert_ne!(ink().knob, accent_color());
    }

    #[test]
    fn spacing_scale_monotonic() {
        // UIR2 验收：间距阶严格递增（xs<sm<md<lg<xl<xxl）
        let names = ["xs", "sm", "md", "lg", "xl", "xxl"];
        for w in names.windows(2) {
            assert!(
                spacing(w[0]) < spacing(w[1]),
                "间距阶 {} 应小于 {}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn drawer_width_ratio_and_clamp() {
        // FIX012.10（A012 #10）锚定：比例换算与钳界（ui.json layout 域 token）——
        // 宽窗走钳界上限、窄窗走钳界下限、中段走比例
        let max = layout_i64("drawer_width_max") as f32;
        let min = layout_i64("drawer_width_min") as f32;
        let ratio = layout_i64("drawer_width_ratio_x100") as f32 / 100.0;
        assert_eq!(drawer_width(2000.0), max, "宽窗应钳上限");
        assert_eq!(drawer_width(400.0), min, "窄窗应钳下限");
        let mid = 1000.0;
        assert!(
            (drawer_width(mid) - (mid * ratio).clamp(min, max)).abs() < 1e-4,
            "中段应等于比例值"
        );
    }
}
