//! 等宽数字位渲染（UIR 重塑新增，消化 y.problems#12）：逐字符固定槽宽绘制
//! 历史根因：Bahnschrift 为可变字体、数字 advance 不等；现数字族为 Barlow Medium，
//! 槽宽制保留——不依赖字体等宽性，槽宽取 0–9 最大 advance，冒号/点占窄槽，
//! 字母与 ° % / 等符号按实际 advance 补槽，逐字符居中绘制

use egui::{Align2, Color32, FontId, Painter, Rect, Vec2};

/// 数字字体（Barlow Medium 注入于 Monospace 槽位 0——window.rs install_fonts）
pub fn digit_font(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Monospace)
}

/// 单字符槽宽：数字满槽；冒号/句点/空格窄槽；连字符保留 2px 间隙；
/// 字母与单位符号（° % / 等）按 Barlow 实际 advance 补槽，避免被窄槽压到相邻字符
fn slot_w(painter: &Painter, font: &FontId, cell: f32, c: char) -> f32 {
    if c.is_ascii_digit() {
        return cell;
    }
    match c {
        ':' | '.' | ' ' => cell * 0.45,
        '-' => cell * 0.45 + 2.0,
        _ => painter
            .layout_no_wrap(c.to_string(), font.clone(), Color32::WHITE)
            .rect
            .width()
            .max(cell * 0.45),
    }
}

/// 槽宽基准：查启动指标表（0–9 最大字形宽），不随当前数字变化；
/// 表缺失时现场补测，下限 = 字号×0.5
pub fn cell_width(painter: &Painter, font: &FontId) -> f32 {
    super::text_metrics::digit_cell(painter.ctx(), font)
}

/// 总占用宽度（纯函数）：Σ 单字符槽宽——页面布局据此预留
pub fn text_width(painter: &Painter, text: &str, font: &FontId) -> f32 {
    let cell = cell_width(painter, font);
    text.chars().map(|c| slot_w(painter, font, cell, c)).sum()
}

/// 盒子底对齐绘制等宽数字串；返回盒子矩形
pub fn draw_bottom(
    painter: &Painter,
    box_bottom: f32,
    center_x: f32,
    text: &str,
    font: &FontId,
    color: Color32,
) -> Rect {
    let ink = metrics_for(painter, font);
    let y = ink.top_for_bottom(box_bottom);
    let total = draw_slots(painter, text, font, color, center_x, y);
    Rect::from_min_size(
        egui::pos2(center_x - total / 2.0, box_bottom - ink.h),
        Vec2::new(total, ink.h),
    )
}

/// 盒子中线对齐绘制等宽数字串（字墨中线 = 盒中线）；返回盒子矩形
pub fn draw_center(
    painter: &Painter,
    box_center: f32,
    center_x: f32,
    text: &str,
    font: &FontId,
    color: Color32,
) -> Rect {
    let ink = center_for(painter, text, font);
    let y = ink.top_for_center(box_center);
    let total = draw_slots(painter, text, font, color, center_x, y);
    Rect::from_min_size(
        egui::pos2(center_x - total / 2.0, box_center - ink.h / 2.0),
        Vec2::new(total, ink.h),
    )
}

/// 数字盒度量（字体基线 + 盒高；默认盒底路径，取数字类）
fn metrics_for(painter: &Painter, font: &FontId) -> super::text_ink::Ink {
    let galley = painter.layout_no_wrap("0".to_owned(), font.clone(), Color32::WHITE);
    super::text_ink::metrics_from_galley(&galley, font, super::text_ink::InkKind::Digits)
}

/// 数字明确居中度量（整串字墨并集；仅 draw_center 用）
fn center_for(painter: &Painter, text: &str, font: &FontId) -> super::text_ink::InkCenter {
    let probe = if text.is_empty() {
        "0".to_owned()
    } else {
        text.to_owned()
    };
    let galley = painter.layout_no_wrap(probe, font.clone(), Color32::WHITE);
    super::text_ink::center_from_galley(&galley, font, super::text_ink::InkKind::Digits)
}

/// 逐字符槽宽绘制；y 是 galley 顶；返回总宽
fn draw_slots(
    painter: &Painter,
    text: &str,
    font: &FontId,
    color: Color32,
    center_x: f32,
    y: f32,
) -> f32 {
    let cell = cell_width(painter, font);
    let total: f32 = text.chars().map(|c| slot_w(painter, font, cell, c)).sum();
    let mut cursor = center_x - total / 2.0;
    for c in text.chars() {
        let w = slot_w(painter, font, cell, c);
        painter.text(
            egui::pos2(cursor + w / 2.0, y),
            Align2::CENTER_TOP,
            c.to_string(),
            font.clone(),
            color,
        );
        cursor += w;
    }
    total
}

// ===== src/widgets/digits.rs 说明区 =====
// 输入：painter + 盒子底/盒中线 + 文案/字号/颜色；输出：逐字符固定槽宽的表盘数字
// 逻辑步骤：cell_width（「0」最大 advance，下限字号×0.5）→ slot_ratio（数字满槽/
//   冒号点窄槽 0.45）→ draw_bottom 以盒底−0.2 为基线；draw_center 以字墨中线对盒中线
// 设计理由（PL023）：数字也是文字盒；默认盒底对齐，明确居中才用字墨中线；
//   槽宽制与字体解耦（字体缺失回退仍有表盘节奏）；不再用 0.62/0.86/1.24 估算盒子
// PL029：字母与 ° % / 等非时间标点按实际 advance 补槽——原统一 0.45 窄槽会把
//   单位符号压到相邻数字/字母上（指标块 12.5°C / 43% / 11.0km/h）
// 异常处理：无 panic 面；空串返回零宽盒子
// 关联依赖：text_ink::metrics_from_galley / center_from_galley；消费方：五页所有
//   时间类大字（时钟 hero/标准时/世界走时/倒计时剩余/闹钟时刻）
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试样本字号（槽宽/盒高断言基准）
    const TEST_SIZE: f32 = 20.0;

    #[test]
    fn symbol_slots_cover_natural_advance() {
        // 数字满槽；冒号/句点/空格窄槽；° % / 与字母按实际 advance 补槽，
        // 不再被 0.45 窄槽压到相邻字符（PL029）
        let painter = test_painter();
        let font = digit_font(TEST_SIZE);
        let cell = cell_width(&painter, &font);
        let w = |s: &str| text_width(&painter, s, &font);
        assert!((w(":") - cell * 0.45).abs() < 1e-4);
        assert!(w("°") > cell * 0.45, "° 应补宽: {}", w("°"));
        assert!(w("%") > cell, "% 应补宽: {}", w("%"));
        assert!(w("/") > cell * 0.45, "/ 应补宽: {}", w("/"));
        assert!(w("m") > cell, "m 应按实际 advance: {}", w("m"));
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
        let font = digit_font(TEST_SIZE);
        let cell = cell_width(&painter, &font);
        assert!(cell >= TEST_SIZE * 0.5, "槽宽下限 = 字号×0.5");
        let w = text_width(&painter, "12:34", &font);
        assert!((w - cell * 4.45).abs() < 1e-4, "12:34 应为 4.45 槽: {w}");
        // 空串零宽
        assert!(text_width(&painter, "", &font).abs() < 1e-6);
    }

    fn test_painter() -> egui::Painter {
        let ctx = egui::Context::default();
        crate::window::install_fonts(&ctx);
        ctx.begin_pass(egui::RawInput::default());
        egui::Painter::new(
            ctx,
            egui::LayerId::background(),
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 100.0)),
        )
    }

    #[test]
    fn draw_bottom_and_center_use_box_values() {
        let painter = test_painter();
        let font = digit_font(TEST_SIZE);
        let want_h = crate::font_profile::get().for_size(TEST_SIZE).h_digit;
        let bottom = draw_bottom(&painter, 100.0, 50.0, "12", &font, Color32::WHITE);
        assert!((bottom.bottom() - 100.0).abs() < 1e-4);
        assert!((bottom.height() - want_h).abs() < 1e-4);
        let center = draw_center(&painter, 100.0, 50.0, "12", &font, Color32::WHITE);
        assert!((center.center().y - 100.0).abs() < 1e-4);
        assert!((center.height() - want_h).abs() < 1e-4);
    }
}
