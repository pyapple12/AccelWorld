//! 运行时盒子探针（ACCELWORLD_LAYOUT_PROBE=1）：打印文字盒与实际形状矩形。
//! 文字盒 = 字号 + 0.5；基线 = 盒底 − 0.2；形状用实际绘制矩形。

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
    let box_bottom = baseline + crate::widgets::text_metrics::BOTTOM_RESERVE;
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
