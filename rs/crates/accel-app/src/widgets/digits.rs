//! 等宽数字位渲染（UIR 重塑新增，消化 y.problems#12）：逐字符固定槽宽绘制
//! Bahnschrift 为可变字体、数字 advance 不等——时钟走秒抖动的根因；
//! 本控件不依赖字体等宽性：以「0」字形宽为满槽，冒号/点占窄槽，逐字符居中绘制

use egui::{Align2, Color32, FontId, Painter, Rect, Vec2};

/// 数字字体（Bahnschrift 注入于 Monospace 槽位 0——window.rs install_fonts）
pub fn digit_font(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Monospace)
}

/// 字符槽宽系数：数字/字母占满槽，冒号/句点/空白占窄槽（表盘排版惯例）
fn slot_ratio(c: char) -> f32 {
    if c.is_ascii_digit() || c.is_alphabetic() {
        1.0
    } else {
        0.45
    }
}

/// 槽宽基准（纯函数可测）：取「0」字形宽（数字最稳的宽度锚点），
/// 下限 = 字号×0.5（字体缺失回退 egui 内置时仍保有表盘间距）
pub fn cell_width(painter: &Painter, font: &FontId) -> f32 {
    let galley = painter.layout_no_wrap("0".to_string(), font.clone(), Color32::WHITE);
    galley.rect.width().max(font.size * 0.5)
}

/// 总占用宽度（纯函数）：Σ(槽宽×系数)——页面布局据此预留
pub fn text_width(painter: &Painter, text: &str, font: &FontId) -> f32 {
    let cell = cell_width(painter, font);
    text.chars().map(|c| cell * slot_ratio(c)).sum()
}

/// 居中绘制等宽数字串（时钟/倒计时/走时表盘共用）；返回占用矩形
pub fn draw_centered(
    painter: &Painter,
    center: egui::Pos2,
    text: &str,
    font: &FontId,
    color: Color32,
) -> Rect {
    let cell = cell_width(painter, font);
    let total: f32 = text.chars().map(|c| cell * slot_ratio(c)).sum();
    let mut cursor = center.x - total / 2.0;
    for c in text.chars() {
        let w = cell * slot_ratio(c);
        painter.text(
            egui::pos2(cursor + w / 2.0, center.y),
            Align2::CENTER_CENTER,
            c.to_string(),
            font.clone(),
            color,
        );
        cursor += w;
    }
    Rect::from_min_size(
        egui::pos2(center.x - total / 2.0, center.y - font.size * 0.62),
        Vec2::new(total, font.size * 1.24),
    )
}

// ===== src/widgets/digits.rs 说明区 =====
// 输入：painter + 中心点/文案/字号/颜色；输出：逐字符固定槽宽的表盘数字
// 逻辑步骤：cell_width（「0」字形宽，下限字号×0.5）→ slot_ratio（数字满槽/
//   冒号点窄槽 0.45）→ 逐字符 CENTER_CENTER 绘制于槽心
// 设计理由：y.problems#12 根治——Bahnschrift 数字 advance 不等，直接 layout
//   整串文本则走秒期间宽度抖动；槽宽制与字体解耦（字体缺失回退仍有表盘节奏）；
//   窄槽标点对齐机械钟/表盘惯例（冒号占位小、视觉重心稳定）
// 异常处理：无 panic 面；空串返回零宽矩形于 center
// 关联依赖：egui Painter.layout_no_wrap（单字符量宽实测）；消费方：五页所有
//   时间类大字（时钟 hero/标准时/世界走时/倒计时剩余/闹钟时刻）
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_ratio_matrix() {
        // 数字满槽；冒号/句点/空格窄槽（表盘节奏的可证锚点）
        assert_eq!(slot_ratio('0'), 1.0);
        assert_eq!(slot_ratio('9'), 1.0);
        assert_eq!(slot_ratio('A'), 1.0);
        assert_eq!(slot_ratio(':'), 0.45);
        assert_eq!(slot_ratio('.'), 0.45);
        assert_eq!(slot_ratio(' '), 0.45);
    }

    #[test]
    fn text_width_sums_slots() {
        // "12:34" = 4 满槽 + 1 窄槽 = 4.45 槽宽（几何自洽）；
        // 量宽前须注入字体（egui 默认上下文无字体——install_fonts 装载系统族）
        let ctx = egui::Context::default();
        crate::window::install_fonts(&ctx);
        // set_fonts 挂起到 begin_pass 才生效——测试无 run()，手动开一帧 pass
        ctx.begin_pass(egui::RawInput::default());
        let painter = egui::Painter::new(
            ctx,
            egui::LayerId::background(),
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 100.0)),
        );
        let font = digit_font(20.0);
        let cell = cell_width(&painter, &font);
        assert!(cell >= 10.0, "槽宽下限 = 字号×0.5");
        let w = text_width(&painter, "12:34", &font);
        assert!((w - cell * 4.45).abs() < 1e-4, "12:34 应为 4.45 槽: {w}");
        // 空串零宽
        assert!(text_width(&painter, "", &font).abs() < 1e-6);
    }
}
