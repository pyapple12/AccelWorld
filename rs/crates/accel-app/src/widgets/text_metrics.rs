//! 全局文字盒高（PL023.37）：盒高按 font_profile 查表，不再写死常量。
//! 数字槽宽仍按 0–9 最大 advance 缓存。

use egui::{Color32, Context, FontId};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use super::text_ink::InkKind;

/// 数字槽宽缓存键：字号 bits + 字族 hash
type CellKey = (u32, u64);

#[derive(Default)]
struct Tables {
    cells: HashMap<CellKey, f32>,
}

static TABLES: OnceLock<Mutex<Tables>> = OnceLock::new();

/// 取全局表；锁中毒时取回内部数据，不让绘制路径 panic
fn tables() -> &'static Mutex<Tables> {
    TABLES.get_or_init(|| Mutex::new(Tables::default()))
}

/// 字族 hash（FontId 只实现 PartialEq，不能直接当 HashMap 键）
fn family_hash(font: &FontId) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    font.family.hash(&mut h);
    h.finish()
}

fn cell_key(font: &FontId) -> CellKey {
    (font.size.to_bits(), family_hash(font))
}

/// 角色盒高：Cjk 用中文盒高，Latin 用英文盒高，Digits 用数字盒高，Mixed 取两者较大
pub fn height(_ctx: &Context, font: &FontId, kind: InkKind) -> f32 {
    let p = crate::font_profile::get().for_size(font.size);
    match kind {
        InkKind::Cjk => p.h,
        InkKind::Latin => p.h_latin,
        InkKind::Digits => p.h_digit,
        InkKind::Mixed => p.h_latin.max(p.h_digit),
    }
}

/// 数字槽宽：取 0–9 的最大 advance，按字号/字族缓存
pub fn digit_cell(ctx: &Context, font: &FontId) -> f32 {
    let key = cell_key(font);
    if let Some(v) = tables()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .cells
        .get(&key)
        .copied()
    {
        return v;
    }
    let galley =
        ctx.fonts_mut(|f| f.layout_no_wrap("0123456789".to_owned(), font.clone(), Color32::WHITE));
    let w = galley
        .rows
        .iter()
        .flat_map(|r| r.glyphs.iter())
        .map(|g| g.advance_width)
        .fold(0.0, f32::max)
        .max(font.size * 0.5);
    tables()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .cells
        .insert(key, w);
    w
}

// ===== src/widgets/text_metrics.rs 说明区 =====
// 输入：Context + FontId；输出：查表盒高 / 数字最大槽宽
// 逻辑步骤：height 按字号查 font_profile；digit_cell 首次量 0–9 最大 advance 后缓存
// 设计理由（PL023.37）：盒高只由字体组合表决定，同字号恒等；字形真实下探允许画出盒底，
//   盒子不裁切；组件高度不再另立 line_height 第二套
// 异常处理：锁中毒取回内部数据；槽宽下限 = 字号×0.5
// 关联依赖：font_profile；消费方：pages/*、widgets/digits

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_height_matches_profile() {
        let ctx = Context::default();
        for (size, want) in [(12.0, 11.5), (16.0, 15.5), (20.0, 19.5), (72.0, 70.0)] {
            let font = FontId::proportional(size);
            let got = height(&ctx, &font, InkKind::Cjk);
            assert!((got - want).abs() < 1e-5, "{size} -> {got}, want {want}");
        }
    }
}
