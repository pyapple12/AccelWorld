//! 文字墨迹 bbox（盒子模型）：用字形位图的实际 bbox，而不是字体行框或比例估算。

use egui::{Color32, FontId, Ui};

/// 字面类型：中文表意字 / 数字（Bahnschrift）——仅在无位图时做回退估算
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InkKind {
    Cjk,
    Digits,
}

/// 文字墨迹度量（偏移相对 galley 顶）
#[derive(Clone, Copy)]
pub struct Ink {
    /// galley 行框高
    pub line_h: f32,
    /// 行框顶 → 墨迹顶
    pub top_pad: f32,
    /// 墨迹底 → 行框底
    pub bottom_pad: f32,
    /// 墨迹高
    pub h: f32,
    /// 墨迹中心 − 行框中心（正数：墨迹偏下）
    pub center_offset: f32,
}

/// 量一段文字的墨迹 bbox：优先取字形位图（uv_rect.offset/size），无位图时按字面比例回退
pub fn measure(ui: &Ui, text: &str, font: &FontId, kind: InkKind) -> Ink {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
    let line_h = galley.size().y;
    let size = font.size;
    let row = galley.rows.first();
    let glyph = row.and_then(|r| r.glyphs.first());
    let (ink_top, ink_bottom) = match (row, glyph) {
        (Some(r), Some(g)) => {
            let baseline = r.pos.y + g.pos.y;
            let uv = g.uv_rect;
            if uv.size.y > 0.0 {
                // 字形位图就是「实际看到的」边界（含抗锯齿像素）
                let top = baseline + uv.offset.y;
                (top, top + uv.size.y)
            } else {
                let (cap, below) = ratios(kind);
                (baseline - size * cap, baseline + size * below)
            }
        }
        _ => {
            let (cap, below) = ratios(kind);
            let h = size * (cap + below);
            (line_h / 2.0 - h / 2.0, line_h / 2.0 + h / 2.0)
        }
    };
    let h = (ink_bottom - ink_top).max(1.0);
    Ink {
        line_h,
        top_pad: ink_top,
        bottom_pad: (line_h - ink_bottom).max(0.0),
        h,
        center_offset: (ink_top + ink_bottom) / 2.0 - line_h / 2.0,
    }
}

/// 无位图时的字面比例回退：中文 0.86em + 0.03em 下沿，数字 0.62em
fn ratios(kind: InkKind) -> (f32, f32) {
    match kind {
        InkKind::Cjk => (0.86, 0.03),
        InkKind::Digits => (0.62, 0.0),
    }
}

// ===== src/widgets/text_ink.rs 说明区 =====
// 输入：Ui + 文案 + 字体 + 字面类型；输出：墨迹高与行框内偏移
// 逻辑步骤：layout_no_wrap 取 galley → 首 glyph 的 pos.y 即基线 → uv_rect.offset/size
//   给出字形位图的实际上下边界；无位图（空格等）时按字面比例回退
// 设计理由：盒子模型要求「盒顶=墨迹顶、盒底=墨迹底」；行框含 ascent/descent 留白，
//   比例估算又会差 1px，直接取字形位图才能让竖线、间距与可见像素严格同线
// 异常处理：空文本/无位图回退到行框中心 + 比例；h 下限 1px
// 关联依赖：egui Glyph.uv_rect（offset/size 单位为 points）；消费方：pages/clock、widgets/rate_strip
