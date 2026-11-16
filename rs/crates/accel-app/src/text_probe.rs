//! 文字探针（默认零影响，环境变量激活）：
//! - `ACCELWORLD_TEXT_PROBE=1`：打印实际文字盒、基线、字体度量、alpha 墨迹底
//! - `ACCELWORLD_TEXT_MATRIX=1`：启动时打印字体下探矩阵（模拟）
//! - `ACCELWORLD_TEXT_PROBE_TEXT` / `ACCELWORLD_TEXT_PROBE_SIZES`：矩阵样本与字号
//!
//! alpha 阈值固定三档：8（淡边）/32（可见）/96（实心）；不再用 bbox 做结论。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use egui::epaint::text::Glyph;
use egui::{Align2, Color32, FontId, Pos2, Rect, Ui};

use crate::widgets::text_ink::{self, InkKind};

/// 淡边阈值
pub const ALPHA_FAINT: u8 = 8;
/// 可见阈值
pub const ALPHA_VISIBLE: u8 = 32;
/// 实心阈值
pub const ALPHA_SOLID: u8 = 96;
/// 固定三档
const THRESHOLDS: [u8; 3] = [ALPHA_FAINT, ALPHA_VISIBLE, ALPHA_SOLID];

/// 逐行文字探针是否开启（缓存一次）
pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("ACCELWORLD_TEXT_PROBE").is_ok())
}

/// 字体下探矩阵是否开启（缓存一次）
pub fn matrix_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("ACCELWORLD_TEXT_MATRIX").is_ok())
}

/// 每个 tag 每 0.5s 最多打一次，避免每帧刷屏
fn due(ui: &Ui, tag: &'static str) -> bool {
    static MAP: OnceLock<Mutex<HashMap<&'static str, u64>>> = OnceLock::new();
    let slot = (ui.input(|i| i.time) * 2.0) as u64;
    let map = MAP.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().unwrap_or_else(|e| e.into_inner());
    if map.get(tag).copied() == Some(slot) {
        return false;
    }
    map.insert(tag, slot);
    true
}

/// 从字体图集读单个 glyph 的三档 alpha 上下沿（相对基线，单位 points）
fn alpha_bounds(atlas: &egui::ColorImage, glyph: &Glyph) -> [Option<(f32, f32)>; 3] {
    let uv = glyph.uv_rect;
    let cols = usize::from(uv.max[0].saturating_sub(uv.min[0]));
    let rows = usize::from(uv.max[1].saturating_sub(uv.min[1]));
    if cols == 0 || rows == 0 || uv.size.y <= 0.0 {
        return [None; 3];
    }
    let row_h = uv.size.y / rows as f32;
    let mut out: [Option<(f32, f32)>; 3] = [None; 3];
    for r in 0..rows {
        let mut max_a = 0u8;
        for c in 0..cols {
            let px = atlas[(usize::from(uv.min[0]) + c, usize::from(uv.min[1]) + r)];
            max_a = max_a.max(px.a());
        }
        for (k, threshold) in THRESHOLDS.iter().enumerate() {
            if max_a > *threshold {
                let top = uv.offset.y + r as f32 * row_h;
                let bottom = uv.offset.y + (r + 1) as f32 * row_h;
                out[k] = Some(match out[k] {
                    Some((old_top, _)) => (old_top.min(top), bottom),
                    None => (top, bottom),
                });
            }
        }
    }
    out
}

/// 单行文字探针：盒、基线、字体下沿、三档 alpha 墨迹底
#[allow(clippy::too_many_arguments)]
pub fn line(
    ui: &Ui,
    tag: &'static str,
    text: &str,
    font: &FontId,
    _kind: InkKind,
    pos: Pos2,
    align: Align2,
    box_rect: Rect,
) {
    if !on() || !due(ui, tag) {
        return;
    }
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE);
    let ink = text_ink::measure(ui, text, font, InkKind::Cjk);
    let baseline = pos.y + ink.baseline;
    let (face_ascent, face_height) = galley
        .rows
        .first()
        .and_then(|r| {
            r.glyphs
                .first()
                .map(|g| (g.font_face_ascent, g.font_face_height))
        })
        .unwrap_or((0.0, 0.0));
    let face_descent = (face_height - face_ascent).max(0.0);
    let atlas = ui.ctx().fonts(|f| f.image());
    let mut alpha_top = [f32::INFINITY; 3];
    let mut alpha_bottom = [f32::NEG_INFINITY; 3];
    if let Some(row) = galley.rows.first() {
        for glyph in &row.glyphs {
            for (k, bounds) in alpha_bounds(&atlas, glyph).iter().enumerate() {
                if let Some((top, bottom)) = bounds {
                    alpha_top[k] = alpha_top[k].min(baseline + top);
                    alpha_bottom[k] = alpha_bottom[k].max(baseline + bottom);
                }
            }
        }
    }
    let _ = (pos.x, align);
    eprintln!(
        "[TEXT] {tag} box=({:.2},{:.2},{:.2},{:.2}) baseline={:.2} face=({:.2},{:.2},{:.2}) desc_line={:.2} a8=({:.2},{:.2},{:.2}) a32=({:.2},{:.2},{:.2}) a96=({:.2},{:.2},{:.2})",
        box_rect.left(),
        box_rect.top(),
        box_rect.width(),
        box_rect.height(),
        baseline,
        face_ascent,
        face_height,
        face_descent,
        baseline + face_descent,
        alpha_top[0],
        alpha_bottom[0],
        box_rect.bottom() - alpha_bottom[0],
        alpha_top[1],
        alpha_bottom[1],
        box_rect.bottom() - alpha_bottom[1],
        alpha_top[2],
        alpha_bottom[2],
        box_rect.bottom() - alpha_bottom[2],
    );
}

/// 便捷入口：只给绘制位置与盒底，内部按盒高构造盒矩形
#[allow(clippy::too_many_arguments)]
pub fn boxed_line(
    ui: &Ui,
    tag: &'static str,
    text: &str,
    font: &FontId,
    kind: InkKind,
    pos: Pos2,
    align: Align2,
    box_bottom: f32,
) {
    if !on() {
        return;
    }
    let ink = text_ink::measure(ui, text, font, kind);
    let w = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
        .rect
        .width();
    let x = if align == Align2::RIGHT_TOP {
        pos.x - w
    } else if align == Align2::CENTER_TOP {
        pos.x - w * 0.5
    } else {
        pos.x
    };
    let box_rect = Rect::from_min_size(Pos2::new(x, box_bottom - ink.h), egui::vec2(w, ink.h));
    line(ui, tag, text, font, kind, pos, align, box_rect);
}

/// 启动时最多跑一次的字体下探矩阵
pub fn maybe_run_matrix(ctx: &egui::Context) {
    if !matrix_on() {
        return;
    }
    static RAN: AtomicBool = AtomicBool::new(false);
    if RAN.swap(true, Ordering::Relaxed) {
        return;
    }
    run_matrix(ctx);
}

/// 字体下探矩阵：逐字打印 alpha 三档下探，再给每档最大值
fn run_matrix(ctx: &egui::Context) {
    let painter = egui::Painter::new(
        ctx.clone(),
        egui::LayerId::background(),
        Rect::from_min_size(Pos2::ZERO, egui::vec2(2400.0, 1200.0)),
    );
    let text = std::env::var("ACCELWORLD_TEXT_PROBE_TEXT")
        .unwrap_or_else(|_| "我们额度jhit赢了什么情况".to_owned());
    let sizes = std::env::var("ACCELWORLD_TEXT_PROBE_SIZES")
        .ok()
        .map(|raw| {
            raw.split(',')
                .filter_map(|s| s.trim().parse::<f32>().ok())
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec![12.0, 16.0, 20.0, 28.0, 40.0, 72.0]);
    let ppp = ctx.pixels_per_point();
    eprintln!("[GLYPH] matrix ppp={ppp:.3} text=\"{text}\" sizes={sizes:?}");
    for size in sizes {
        for (kind, font) in [
            ("regular", crate::theme::font_regular(size)),
            ("semibold", crate::theme::font_semibold(size)),
            ("digits", crate::widgets::digits::digit_font(size)),
        ] {
            let galley = painter.layout_no_wrap(text.clone(), font.clone(), Color32::WHITE);
            let atlas = ctx.fonts(|f| f.image());
            let mut max = [0.0_f32; 3];
            for glyph in galley.rows.iter().flat_map(|r| r.glyphs.iter()) {
                let bounds = alpha_bounds(&atlas, glyph);
                let v = bounds.map(|b| b.map_or(f32::NAN, |(_, bottom)| bottom));
                let t = bounds.map(|b| b.map_or(f32::NAN, |(top, _)| top));
                if v.iter().all(|x| x.is_nan()) {
                    continue;
                }
                for k in 0..3 {
                    if v[k].is_finite() {
                        max[k] = max[k].max(v[k]);
                    }
                }
                eprintln!(
                    "[GLYPH] size={size:.1} kind={kind} ch={:?} U+{:04X} a8={:.2} a32={:.2} a96={:.2} t96={:.2}",
                    glyph.chr, glyph.chr as u32, v[0], v[1], v[2], t[2]
                );
            }
            eprintln!(
                "[GLYPHMAX] size={size:.1} kind={kind} a8={:.2} a32={:.2} a96={:.2}",
                max[0], max[1], max[2]
            );
        }
    }
}

// ===== src/text_probe.rs 说明区 =====
// 输入：Ui/Context + 文案 + FontId + 盒 rect；输出：实际文字盒、字体度量、alpha 墨迹上下沿
// 逻辑步骤：layout_no_wrap → 字体图集 image → 每个 glyph 的 uv_rect 逐行 max alpha →
//   超过 8/32/96 的最后一行 → 相对基线 → 加上基线得到绝对墨迹底
// 设计理由（PL023）：默认布局不读 bbox；探针直接读真实栅格 alpha，淡边/可见/实心三档分开，
//   避免把位图包围盒当可见墨迹；矩阵用于换字号/DPI 前的回归模拟
// 异常处理：空文本/空格无 glyph 时输出 inf/nan；矩阵每进程最多一次
// 关联配置：ACCELWORLD_TEXT_PROBE / ACCELWORLD_TEXT_MATRIX /
//   ACCELWORLD_TEXT_PROBE_TEXT / ACCELWORLD_TEXT_PROBE_SIZES；消费方：pages/clock
