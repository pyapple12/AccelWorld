//! 字体组合参数表（PL023.37）：按“中文字体 + 数字字体”读取栅格、dy、trim 与盒高。
//! 表在 ui.json `font_profile`；代码只算公式，不写死任何字号数值。

use std::sync::OnceLock;

use accel_core::static_config::Static;
use serde_json::Value;

/// 单档参数：盒高、上下空白、三类字的 dy（全部由表算出）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeProfile {
    pub size: f32,
    pub dy_cjk: f32,
    pub dy_latin: f32,
    pub dy_digit: f32,
    pub top: f32,
    pub bottom: f32,
    pub h: f32,
}

/// 当前激活的字体组合
pub struct Profile {
    grid: f32,
    sizes: Vec<SizeProfile>,
}

impl Profile {
    /// 按字号取档：精确匹配；动态缩放字号取最近档
    pub fn for_size(&self, size: f32) -> SizeProfile {
        if let Some(p) = self.sizes.iter().find(|p| (p.size - size).abs() < 0.01) {
            return *p;
        }
        let mut best = self.sizes[0];
        for p in &self.sizes {
            if (p.size - size).abs() < (best.size - size).abs() {
                best = *p;
            }
        }
        best
    }

    /// 表内全部档位（单测与探针用）
    pub fn sizes(&self) -> &[SizeProfile] {
        &self.sizes
    }

    /// 栅格单位
    pub fn grid(&self) -> f32 {
        self.grid
    }
}

fn read_f32(path: &str) -> f32 {
    match Static::get().ui.pointer(path).and_then(Value::as_f64) {
        Some(v) => v as f32,
        None => panic!("ui.json 缺 font_profile 键: {path}"),
    }
}

fn read_i64(value: &Value, key: &str) -> i64 {
    match value.get(key).and_then(Value::as_i64) {
        Some(v) => v,
        None => panic!("ui.json 缺 font_profile.sizes 键: {key}"),
    }
}

fn build() -> Profile {
    let static_cfg = Static::get();
    let active = match static_cfg
        .ui
        .pointer("/font_profile/active")
        .and_then(Value::as_str)
    {
        Some(v) => v,
        None => panic!("ui.json 缺 font_profile.active"),
    };
    let base = format!("/font_profile/profiles/{active}");
    let grid = read_f32(&format!("{base}/grid"));
    let top_ratio = read_f32(&format!("{base}/top_ratio"));
    let bottom_ratio = read_f32(&format!("{base}/bottom_ratio"));
    let sizes = match static_cfg
        .ui
        .pointer(&format!("{base}/sizes"))
        .and_then(Value::as_object)
    {
        Some(v) => v,
        None => panic!("ui.json 缺 {base}/sizes"),
    };
    let mut list = Vec::with_capacity(sizes.len());
    for value in sizes.values() {
        let size = read_i64(value, "size") as f32;
        let trim = read_i64(value, "trim_grids") as f32;
        list.push(SizeProfile {
            size,
            dy_cjk: read_i64(value, "dy_cjk_grids") as f32 * grid,
            dy_latin: read_i64(value, "dy_latin_grids") as f32 * grid,
            dy_digit: read_i64(value, "dy_digit_grids") as f32 * grid,
            top: top_ratio * trim,
            bottom: bottom_ratio * trim,
            h: size - (grid - top_ratio - bottom_ratio) * trim,
        });
    }
    list.sort_by(|a, b| a.size.total_cmp(&b.size));
    Profile { grid, sizes: list }
}

/// 单例入口：首次调用解析表，此后零开销
pub fn get() -> &'static Profile {
    static P: OnceLock<Profile> = OnceLock::new();
    P.get_or_init(build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_tokens_are_closed() {
        for (size, want_h) in [
            (12.0, 11.5),
            (16.0, 15.5),
            (20.0, 19.5),
            (28.0, 27.5),
            (40.0, 39.0),
            (72.0, 70.0),
        ] {
            let p = get().for_size(size);
            assert_eq!(p.size, size);
            assert!(
                (p.h - want_h).abs() < 1e-5,
                "{size} -> {} want {want_h}",
                p.h
            );
            let n = p.top / 0.2;
            assert!((n - n.round()).abs() < 1e-5, "{size} n={n}");
        }
    }

    #[test]
    fn dynamic_size_uses_nearest() {
        assert_eq!(get().for_size(56.3).size, 56.0);
        assert_eq!(get().for_size(70.0).size, 72.0);
    }
}

// ===== src/font_profile.rs 说明区 =====// 输入：ui.json `font_profile`（active + profiles[combo]）
// 输出：按字号查到的 SizeProfile（h/top/bottom/dy）与栅格单位
// 逻辑步骤：读 active 组合 → grid/top_ratio/bottom_ratio → 遍历 sizes →
//   dy = grids × grid；h = size − (grid − top_ratio − bottom_ratio) × trim_grids
// 设计理由（PL023.37）：数值只进表；公式共用；换字体只换表
// 异常处理：缺键/错型 panic（构建产物损坏级），不回退错表
// 关联配置：ui.json font_profile；消费方：text_metrics / text_ink
