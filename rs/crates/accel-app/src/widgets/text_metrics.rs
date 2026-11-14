//! 全局文字盒高（PL021）：盒高 = 字号 + 0.3px 顶部余量。
//! 不再扫描字体集、不再按实际字形取最大；数字槽宽仍按 0–9 最大 advance 缓存。

use egui::{Color32, Context, FontId};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use super::text_ink::InkKind;

/// 盒高顶部余量：运行期字形增高不超过此值时，盒边界不动、不裁切
pub const RESERVE: f32 = 0.3;

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

/// 角色盒高：固定为字号 + 0.3px；不再测量实际字形
pub fn height(_ctx: &Context, font: &FontId, _kind: InkKind) -> f32 {
    font.size + RESERVE
}

/// 字体实际行高（egui 布局行高；组件高度公式统一取这里）
pub fn line_height(ctx: &Context, font: &FontId) -> f32 {
    ctx.fonts_mut(|f| f.row_height(font))
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
// 输入：Context + FontId；输出：固定盒高 / 数字最大槽宽
// 逻辑步骤：height 直接返回 font.size + 0.3；digit_cell 首次量 0–9 最大 advance 后缓存
// 设计理由（PL021）：全量代表集扫描会把不同字形的顶/底并集当成一个字高（首+g=27.2），
//   且 12px 中文实际都在 11.6px 内；按字号固定 M=size、留 0.3px 顶部余量即可稳定且不裁切
// 异常处理：锁中毒取回内部数据；槽宽下限 = 字号×0.5
// 关联依赖：无字体集扫描；消费方：pages/*、widgets/digits
