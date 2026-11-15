//! 文字盒度量：盒高 = 字号 + 0.3（上）+ 0.2（下）；基线 = 盒底 − 0.2。
//! 基线取 galley 行排版基线；整串字墨 bbox 仅服务“明确居中”，不参与盒高。

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use egui::{Color32, FontId, Galley, Ui};

/// 字面类型：中文表意字 / 数字（Bahnschrift）——语义标签，盒高与基线不区分
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InkKind {
    Cjk,
    Digits,
}

/// 文字盒度量：盒高 = 字号 + 0.5；基线 = 盒底 − 0.2；不含任何字墨 bbox
#[derive(Clone, Copy)]
pub struct Ink {
    /// 盒高（= 字号 + 0.3 + 0.2）
    pub h: f32,
    /// galley 顶 → 行排版基线（字体度量，同一行所有字形共用）
    pub baseline: f32,
}

impl Ink {
    /// 目标基线 → galley 顶
    pub fn top_for_baseline(&self, baseline: f32) -> f32 {
        baseline - self.baseline
    }

    /// 盒底 → galley 顶：基线 = 盒底 − 0.2
    pub fn top_for_bottom(&self, bottom: f32) -> f32 {
        self.top_for_baseline(bottom - super::text_metrics::BOTTOM_RESERVE)
    }
}

/// 明确居中用的字墨中线（唯一读取 glyph bbox 的路径，不参与盒高）
#[derive(Clone, Copy)]
pub(crate) struct InkCenter {
    pub h: f32,
    pub baseline: f32,
    pub ink_top: f32,
    pub ink_bottom: f32,
}

impl InkCenter {
    /// 盒中心 → galley 顶：字墨中线 = 盒中线
    pub fn top_for_center(&self, center: f32) -> f32 {
        let ascent = (self.baseline - self.ink_top).max(0.0);
        let descent = (self.ink_bottom - self.baseline).max(0.0);
        let baseline = center + (ascent - descent) * 0.5;
        baseline - self.baseline
    }
}

/// 字体基线缓存键：字号 bits + 字族 hash + ppp bits
type FontKey = (u32, u64, u32);

fn font_key(ui: &Ui, font: &FontId) -> FontKey {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    font.family.hash(&mut h);
    (
        font.size.to_bits(),
        h.finish(),
        ui.ctx().pixels_per_point().to_bits(),
    )
}

fn baseline_cache() -> &'static Mutex<HashMap<FontKey, f32>> {
    static CACHE: OnceLock<Mutex<HashMap<FontKey, f32>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 字体行排版基线（按字号/字族缓存，默认盒底路径不再 layout 文本）
fn baseline_for(ui: &Ui, font: &FontId) -> f32 {
    let key = font_key(ui, font);
    if let Some(v) = baseline_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
        .copied()
    {
        return v;
    }
    let galley = ui
        .painter()
        .layout_no_wrap("0".to_owned(), font.clone(), Color32::WHITE);
    let baseline = galley
        .rows
        .first()
        .and_then(|r| r.glyphs.first().map(|g| r.pos.y + g.pos.y))
        .unwrap_or(font.size);
    baseline_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, baseline);
    baseline
}

/// 默认盒底路径：只取盒高 + 字体基线，不读文本、不读 bbox
pub fn measure(ui: &Ui, _text: &str, font: &FontId, _kind: InkKind) -> Ink {
    Ink {
        h: font.size + super::text_metrics::TOP_RESERVE + super::text_metrics::BOTTOM_RESERVE,
        baseline: baseline_for(ui, font),
    }
}

/// 明确居中路径：盒中心 → galley 顶；只有这里读整串字墨 bbox
pub fn top_for_center(ui: &Ui, text: &str, font: &FontId, _kind: InkKind, center: f32) -> f32 {
    let probe = if text.is_empty() { "0" } else { text };
    let galley = ui
        .painter()
        .layout_no_wrap(probe.to_owned(), font.clone(), Color32::WHITE);
    center_from_galley(&galley, font).top_for_center(center)
}

/// 由 galley 构造盒度量；不读任何字号/字形 bbox
pub(crate) fn metrics_from_galley(galley: &Galley, font: &FontId) -> Ink {
    let baseline = galley
        .rows
        .first()
        .and_then(|r| r.glyphs.first().map(|g| r.pos.y + g.pos.y))
        .unwrap_or(font.size);
    Ink {
        h: font.size + super::text_metrics::TOP_RESERVE + super::text_metrics::BOTTOM_RESERVE,
        baseline,
    }
}

/// 由 galley 构造“明确居中”的字墨中线；整串 glyph uv_rect 并集
pub(crate) fn center_from_galley(galley: &Galley, font: &FontId) -> InkCenter {
    let baseline = galley
        .rows
        .first()
        .and_then(|r| r.glyphs.first().map(|g| r.pos.y + g.pos.y))
        .unwrap_or(font.size);
    let (mut ink_top, mut ink_bottom) = (f32::INFINITY, f32::NEG_INFINITY);
    if let Some(row) = galley.rows.first() {
        for g in &row.glyphs {
            let uv = g.uv_rect;
            if uv.size.y > 0.0 {
                let top = baseline + uv.offset.y;
                ink_top = ink_top.min(top);
                ink_bottom = ink_bottom.max(top + uv.size.y);
            }
        }
    }
    if !ink_top.is_finite() || !ink_bottom.is_finite() {
        // 空串/纯空格：无可见字墨；用名义 em 盒做居中回退
        ink_top = baseline - font.size;
        ink_bottom = baseline;
    }
    InkCenter {
        h: font.size + super::text_metrics::TOP_RESERVE + super::text_metrics::BOTTOM_RESERVE,
        baseline,
        ink_top,
        ink_bottom,
    }
}

// ===== src/widgets/text_ink.rs 说明区 =====
// 输入：字体/文本；输出：盒高 + 行排版基线（默认）；明确居中分支另算字墨中线
// 逻辑步骤：盒高 = 字号 + 0.3 + 0.2 → 基线按 (字号, 字族, ppp) 缓存 →
//   top_for_bottom；只有 top_for_center 才 layout 文本并取整串 uv_rect 并集
// 设计理由（PL023）：默认盒底路径不读任何 bbox；bbox 只服务“明确居中”的视觉对中，
//   不参与盒高、不参与盒子对齐；基线是行排版基线，不是首位图 bbox
// 异常处理：空串/纯空格居中回退名义 em 盒；h 恒为字号+0.5
// 关联配置：text_metrics::TOP_RESERVE / BOTTOM_RESERVE；消费方：pages/*、widgets/*

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个盒度量：基线距 galley 顶 11
    fn sample() -> Ink {
        Ink {
            h: 14.5,
            baseline: 11.0,
        }
    }

    #[test]
    fn bottom_baseline_is_bottom_minus_reserve() {
        let ink = sample();
        let galley_top = ink.top_for_bottom(100.0);
        assert!((galley_top + ink.baseline - 99.8).abs() < 1e-5);
    }

    #[test]
    fn center_puts_ink_center_on_box_center() {
        let ink = InkCenter {
            h: 14.5,
            baseline: 11.0,
            ink_top: 2.0,
            ink_bottom: 14.0,
        };
        let box_center = 200.0;
        let galley_top = ink.top_for_center(box_center);
        let ink_center = galley_top + (ink.ink_top + ink.ink_bottom) * 0.5;
        assert!((ink_center - box_center).abs() < 1e-5);
    }

    #[test]
    fn mixed_boxes_share_baseline_at_shared_bottom() {
        let mut small = sample();
        small.h = 12.5;
        let mut large = sample();
        large.h = 18.5;
        let bottom = 300.0;
        let small_baseline = small.top_for_bottom(bottom) + small.baseline;
        let large_baseline = large.top_for_bottom(bottom) + large.baseline;
        assert!((small_baseline - large_baseline).abs() < 1e-5);
        assert!((small_baseline - (bottom - 0.2)).abs() < 1e-5);
    }
}
