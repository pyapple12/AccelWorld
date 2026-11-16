//! 运行时盒子探针（ACCELWORLD_LAYOUT_PROBE=1）：打印盒子、基线、形状矩形、光晕圆心与实体墨迹。
//! 盒高/下空白/dy 查 font_profile；形状用实际绘制矩形；墨迹取 alpha > 96。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use egui::{Align2, Color32, FontId, Pos2, Rect, Ui};

use crate::widgets::text_ink::{self, InkKind};

/// 布局盒探针是否开启（缓存一次，正常模式每帧零 env 查询）
pub fn layout_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("ACCELWORLD_LAYOUT_PROBE").is_ok())
}

/// 每个 tag 每 0.5s 最多打一次，避免每帧刷屏
fn due(ui: &Ui, tag: &'static str) -> bool {
    static MAP: OnceLock<Mutex<HashMap<&'static str, u64>>> = OnceLock::new();
    let slot = (ui.input(|i| i.time) * 2.0) as u64;
    let map = MAP.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().unwrap();
    if map.get(tag).copied() == Some(slot) {
        return false;
    }
    map.insert(tag, slot);
    true
}

/// 实体墨迹每 tag 每 0.5s 最多打一次（与 [GLOW] 独立节流）
fn due_ink(ui: &Ui, tag: &'static str) -> bool {
    static MAP: OnceLock<Mutex<HashMap<&'static str, u64>>> = OnceLock::new();
    let slot = (ui.input(|i| i.time) * 2.0) as u64;
    let map = MAP.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().unwrap();
    if map.get(tag).copied() == Some(slot) {
        return false;
    }
    map.insert(tag, slot);
    true
}

/// 文字盒：默认盒底对齐，打印盒顶、盒底、盒高与盒内基线
pub fn text(
    ui: &Ui,
    tag: &'static str,
    text: &str,
    font: &FontId,
    kind: InkKind,
    pos: Pos2,
    align: Align2,
) {
    if !layout_on() || !due(ui, tag) {
        return;
    }
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE);
    let w = galley.rect.width();
    let ink = text_ink::measure(ui, text, font, kind);
    let x = if align == Align2::RIGHT_TOP {
        pos.x - w
    } else if align == Align2::CENTER_TOP {
        pos.x - w * 0.5
    } else {
        pos.x
    };
    let baseline = pos.y + ink.baseline;
    let box_bottom = baseline + ink.dy + ink.bottom;
    let box_top = box_bottom - ink.h;
    eprintln!(
        "[BOX] {tag} x={:.2} y={:.2} w={:.2} h={:.2} box_bottom={:.2} baseline={:.2}",
        x, box_top, w, ink.h, box_bottom, baseline
    );
}

/// 形状：打印实际绘制矩形
pub fn rect(ui: &Ui, tag: &'static str, rect: Rect) {
    if !layout_on() || !due(ui, tag) {
        return;
    }
    eprintln!(
        "[RECT] {tag} x={:.2} y={:.2} w={:.2} h={:.2}",
        rect.left(),
        rect.top(),
        rect.width(),
        rect.height()
    );
}

/// GPU 光晕：打印圆心、bloom 与 alpha（与文字盒同一套 layout probe）
pub fn glow(ui: &Ui, tag: &'static str, center: Pos2, bloom: f32, alpha: u8) {
    if !layout_on() || !due(ui, tag) {
        return;
    }
    eprintln!(
        "[GLOW] {tag} center=({:.2},{:.2}) bloom={:.2} alpha={alpha}",
        center.x, center.y, bloom
    );
}

/// 实体墨迹：alpha > 96 的上下沿与中线，按 CENTER_TOP 锚点换算到屏幕
pub fn ink(ui: &Ui, tag: &'static str, text: &str, font: &FontId, pos: Pos2, align: Align2) {
    if !layout_on() || !due_ink(ui, tag) {
        return;
    }
    let Some((left, right, top, bottom)) = text_ink::solid_ink_bounds(ui, text, font) else {
        return;
    };
    let w = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
        .rect
        .width();
    let anchor_x = if align == Align2::RIGHT_TOP {
        pos.x - w
    } else if align == Align2::CENTER_TOP {
        pos.x - w * 0.5
    } else {
        pos.x
    };
    let left = anchor_x + left;
    let right = anchor_x + right;
    let top = pos.y + top;
    let bottom = pos.y + bottom;
    eprintln!(
        "[INK] {tag} x={left:.2} right={right:.2} top={top:.2} bottom={bottom:.2} center=({:.2},{:.2}) w={:.2} h={:.2}",
        (left + right) * 0.5,
        (top + bottom) * 0.5,
        right - left,
        bottom - top
    );
}
