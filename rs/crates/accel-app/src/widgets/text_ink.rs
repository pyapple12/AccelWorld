//! 文字墨迹与基线度量：墨迹 bbox 用于盒高参考，竖排定位统一走排版基线。
//! 基线取 galley 的 row.pos.y + glyph.pos.y，不受字形位图取整影响。

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
    /// 行框顶 → 基线（排版基线，不受字形位图取整影响）
    pub baseline: f32,
}

impl Ink {
    /// 目标基线 → galley 顶
    pub fn top_for_baseline(&self, baseline: f32) -> f32 {
        baseline - self.baseline
    }

    /// 行盒底 → galley 顶（盒高 = 字号 + RESERVE，基线在盒底上方 RESERVE）
    pub fn top_for_bottom(&self, bottom: f32) -> f32 {
        self.top_for_baseline(bottom - super::text_metrics::RESERVE)
    }

    /// 行盒中心 → galley 顶（盒高 = 字号 + RESERVE）
    pub fn top_for_center(&self, center: f32, font_size: f32) -> f32 {
        let baseline = center + (font_size - super::text_metrics::RESERVE) * 0.5;
        self.top_for_baseline(baseline)
    }
}

/// 量一段文字的墨迹 bbox：优先取字形位图（uv_rect.offset/size），无位图时按字面比例回退
pub fn measure(ui: &Ui, text: &str, font: &FontId, kind: InkKind) -> Ink {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
    ink_from_galley(&galley, font, kind)
}

/// 从已排版的 galley 提取墨迹（启动指标表与运行期 measure 共用）
pub(crate) fn ink_from_galley(galley: &egui::Galley, font: &FontId, kind: InkKind) -> Ink {
    let line_h = galley.size().y;
    let size = font.size;
    let row = galley.rows.first();
    let glyph = row.and_then(|r| r.glyphs.first());
    let (ink_top, ink_bottom, baseline) = match (row, glyph) {
        (Some(r), Some(g)) => {
            let baseline = r.pos.y + g.pos.y;
            let uv = g.uv_rect;
            if uv.size.y > 0.0 {
                // 字形位图就是「实际看到的」边界（含抗锯齿像素）
                let top = baseline + uv.offset.y;
                (top, top + uv.size.y, baseline)
            } else {
                let (cap, below) = ratios(kind);
                (baseline - size * cap, baseline + size * below, baseline)
            }
        }
        _ => {
            let (cap, below) = ratios(kind);
            let h = size * (cap + below);
            (line_h / 2.0 - h / 2.0, line_h / 2.0 + h / 2.0, line_h / 2.0)
        }
    };
    let h = (ink_bottom - ink_top).max(1.0);
    Ink {
        line_h,
        top_pad: ink_top,
        bottom_pad: (line_h - ink_bottom).max(0.0),
        h,
        center_offset: (ink_top + ink_bottom) / 2.0 - line_h / 2.0,
        baseline,
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
// 输入：Ui + 文案 + 字体 + 字面类型；输出：墨迹 bbox、行框偏移与排版基线
// 逻辑步骤：layout_no_wrap 取 galley → 首 glyph 的 pos.y 即基线 → uv_rect.offset/size
//   给出字形位图的上下边界；无位图（空格等）时按字面比例回退
// 设计理由（PL021/基线）：竖排定位统一用 top_for_baseline/top_for_bottom/
//   top_for_center 换算，基线来自排版数据，避免首字位图下沿留白不同导致混排错位；
//   墨迹 bbox 仅保留作盒高参考与回退
// 异常处理：空文本/无位图回退到行框中心 + 比例；h 下限 1px
// 关联依赖：egui Glyph.uv_rect（offset/size 单位为 points）；消费方：pages/*、widgets/*
