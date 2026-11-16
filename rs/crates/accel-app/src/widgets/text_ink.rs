//! 文字盒度量（PL023.37）：盒高、上/下空白与 dy 全部查 font_profile。
//! 基线取 galley 行排版基线；整串字墨 bbox 仅服务“明确居中”，不参与盒高。

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use egui::{Color32, FontId, Galley, Ui};

/// 字面类型：中文 / 纯英文 / 纯数字 / 英文数字混排——决定取哪一类 dy
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InkKind {
    Cjk,
    Latin,
    Digits,
    Mixed,
}

/// 文字盒度量：盒高与下空白查表；不含任何字墨 bbox
#[derive(Clone, Copy)]
pub struct Ink {
    /// 盒高（表内算出）
    pub h: f32,
    /// galley 顶 → 行排版基线（字体度量，同一行所有字形共用）
    pub baseline: f32,
    /// 盒底 → 盒子基线 B
    pub bottom: f32,
    /// 排版基线上移量：中文 dy > 0，英文/数字当前 0
    pub dy: f32,
}

impl Ink {
    /// 目标基线 → galley 顶
    pub fn top_for_baseline(&self, baseline: f32) -> f32 {
        baseline - self.baseline
    }

    /// 盒底 → galley 顶：B = 盒底 − bottom，P = B − dy
    pub fn top_for_bottom(&self, bottom: f32) -> f32 {
        self.top_for_baseline(bottom - self.bottom - self.dy)
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

/// 由字号与字面类型选 dy：中文取 dy_cjk，英文取 dy_latin，数字取 dy_digit，
/// 混排取两者较大值（当前英文/数字/混排均为 0）
fn dy_for(font: &FontId, kind: InkKind) -> f32 {
    let p = crate::font_profile::get().for_size(font.size);
    match kind {
        InkKind::Cjk => p.dy_cjk,
        InkKind::Latin => p.dy_latin,
        InkKind::Digits => p.dy_digit,
        InkKind::Mixed => p.dy_latin.max(p.dy_digit),
    }
}

/// 默认盒底路径：查表盒高 + 字体基线，不读文本、不读 bbox
pub fn measure(ui: &Ui, _text: &str, font: &FontId, kind: InkKind) -> Ink {
    let p = crate::font_profile::get().for_size(font.size);
    let h = match kind {
        InkKind::Cjk => p.h,
        InkKind::Latin | InkKind::Digits | InkKind::Mixed => p.h_latin,
    };
    Ink {
        h,
        baseline: baseline_for(ui, font),
        bottom: p.bottom,
        dy: dy_for(font, kind),
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
    let p = crate::font_profile::get().for_size(font.size);
    let baseline = galley
        .rows
        .first()
        .and_then(|r| r.glyphs.first().map(|g| r.pos.y + g.pos.y))
        .unwrap_or(font.size);
    Ink {
        h: p.h,
        baseline,
        bottom: p.bottom,
        dy: p.dy_cjk,
    }
}

/// 由 galley 构造“明确居中”的字墨中线；整串 glyph uv_rect 并集
pub(crate) fn center_from_galley(galley: &Galley, font: &FontId) -> InkCenter {
    let p = crate::font_profile::get().for_size(font.size);
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
        h: p.h,
        baseline,
        ink_top,
        ink_bottom,
    }
}

// ===== src/widgets/text_ink.rs 说明区 =====
// 输入：字体/文本/InkKind；输出：盒高 + 行排版基线 + 下空白 + dy
// 逻辑步骤：查 font_profile → 盒高/下空白/dy → 基线按 (字号, 字族, ppp) 缓存 →
//   top_for_bottom = 基线 − (盒底 − bottom − dy)；只有 top_for_center 才读整串 uv_rect 并集
// 设计理由（PL023.37）：默认盒底路径不读任何 bbox；三线 P + dy = O = B；
//   bbox 只服务“明确居中”的视觉对中，不参与盒高、不参与盒子对齐
// 异常处理：空串/纯空格居中回退名义 em 盒；缺表 panic
// 关联配置：ui.json font_profile；消费方：pages/*、widgets/*

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个盒度量：基线距 galley 顶 11
    fn sample() -> Ink {
        Ink {
            h: 14.5,
            baseline: 11.0,
            bottom: 0.1,
            dy: 1.6,
        }
    }

    #[test]
    fn bottom_baseline_is_bottom_minus_bottom_and_dy() {
        let ink = sample();
        let galley_top = ink.top_for_bottom(100.0);
        assert!((galley_top + ink.baseline - (100.0 - 0.1 - 1.6)).abs() < 1e-5);
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
        small.h = 11.5;
        small.bottom = 0.1;
        small.dy = 0.8;
        let mut large = sample();
        large.h = 19.5;
        large.bottom = 0.1;
        large.dy = 1.6;
        let bottom = 300.0;
        let small_baseline = small.top_for_bottom(bottom) + small.baseline;
        let large_baseline = large.top_for_bottom(bottom) + large.baseline;
        assert!((small_baseline - (bottom - 0.1 - 0.8)).abs() < 1e-5);
        assert!((large_baseline - (bottom - 0.1 - 1.6)).abs() < 1e-5);
    }
}
