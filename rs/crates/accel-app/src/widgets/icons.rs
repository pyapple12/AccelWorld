//! 矢量图标库（UIR 重塑新增）：painter path 直接绘制，零字体/零 emoji 依赖
//! 跨设备字体回落不再影响图标观感；线宽随尺寸缩放，颜色走调用方语义色

use egui::{Color32, Painter, Pos2, Stroke, Vec2};

/// 图标集（Dock 五页 + 通用操作 + 昼夜指示）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// 时钟（表盘 + 指针）
    Clock,
    /// 沙漏（倒计时）
    Hourglass,
    /// 地球（世界时钟）
    Globe,
    /// 云（天气）
    Cloud,
    /// 铃铛（闹钟）
    Bell,
    /// 齿轮（设置）
    Gear,
    /// 加号
    Plus,
    /// 减号
    Minus,
    /// 垃圾桶（删除）
    Trash,
    /// 太阳（昼）
    Sun,
    /// 月牙（夜）
    Moon,
    /// 循环箭头（刷新）
    Refresh,
}

/// 圆弧折线采样（图标内圆弧共用）：a0→a1 弧度逆时针 segs 段
fn arc_points(center: Pos2, r: f32, a0: f32, a1: f32, segs: usize) -> Vec<Pos2> {
    (0..=segs)
        .map(|i| {
            let a = a0 + (a1 - a0) * (i as f32 / segs as f32);
            egui::pos2(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect()
}

/// 图标绘制：以 center 为中心、边长 size 的方形域内描线（线宽随尺寸缩放）
pub fn draw(painter: &Painter, center: Pos2, size: f32, icon: Icon, color: Color32) {
    let sw = (size * 0.09).max(1.2);
    let st = Stroke::new(sw, color);
    let r = size / 2.0;
    match icon {
        Icon::Clock => {
            painter.circle_stroke(center, r * 0.86, st);
            // 指针：12 点→中心→3 点（两段折线）
            painter.line_segment([center, egui::pos2(center.x, center.y - r * 0.52)], st);
            painter.line_segment(
                [center, egui::pos2(center.x + r * 0.38, center.y + r * 0.16)],
                st,
            );
        }
        Icon::Hourglass => {
            // 上下横杠 + 交叉双三角（漏斗颈在中心）
            let w = r * 0.78;
            let top = center.y - r * 0.82;
            let bot = center.y + r * 0.82;
            painter.line_segment(
                [egui::pos2(center.x - w, top), egui::pos2(center.x + w, top)],
                st,
            );
            painter.line_segment(
                [egui::pos2(center.x - w, bot), egui::pos2(center.x + w, bot)],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x - w * 0.86, top + sw),
                    egui::pos2(center.x + w * 0.30, center.y),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x + w * 0.86, top + sw),
                    egui::pos2(center.x - w * 0.30, center.y),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x + w * 0.30, center.y),
                    egui::pos2(center.x + w * 0.86, bot - sw),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x - w * 0.30, center.y),
                    egui::pos2(center.x - w * 0.86, bot - sw),
                ],
                st,
            );
        }
        Icon::Globe => {
            painter.circle_stroke(center, r * 0.86, st);
            // 赤道线
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.86, center.y),
                    egui::pos2(center.x + r * 0.86, center.y),
                ],
                st,
            );
            // 经线椭圆（两条纵向弧：±60° 摆幅近似）
            let rx = r * 0.42;
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a =
                        -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * (i as f32 / 24.0);
                    egui::pos2(center.x + rx * a.cos(), center.y + r * 0.86 * a.sin())
                })
                .collect();
            painter.add(egui::Shape::line(pts, st));
        }
        Icon::Cloud => {
            // 底线 + 三段半圆凸包（左小/中大/右小），单条折线描边
            let y = center.y + r * 0.30;
            let left = egui::pos2(center.x - r * 0.62, y);
            let right = egui::pos2(center.x + r * 0.62, y);
            let mut pts = Vec::new();
            pts.push(left);
            // 左凸包（半圆向上，半径 0.26）
            pts.extend(arc_points(
                egui::pos2(left.x + r * 0.26, y),
                r * 0.26,
                std::f32::consts::PI,
                0.0,
                10,
            ));
            // 中凸包（半圆向上，半径 0.40）
            pts.extend(arc_points(
                egui::pos2(left.x + r * 0.60, y),
                r * 0.40,
                std::f32::consts::PI,
                0.0,
                12,
            ));
            // 右凸包（半圆向上，半径 0.26）
            pts.extend(arc_points(
                egui::pos2(right.x - r * 0.26, y),
                r * 0.26,
                std::f32::consts::PI,
                0.0,
                10,
            ));
            pts.push(right);
            painter.add(egui::Shape::line(pts, st));
        }
        Icon::Bell => {
            // 钟 dome（上弧）+ 两侧裙线 + 底沿 + 钟舌
            let top = egui::pos2(center.x, center.y - r * 0.72);
            let rim_y = center.y + r * 0.52;
            let mut dome = arc_points(
                center,
                r * 0.62,
                std::f32::consts::PI,
                std::f32::consts::TAU,
                16,
            );
            dome.insert(0, top);
            painter.add(egui::Shape::line(dome, st));
            let rim_l = egui::pos2(center.x - r * 0.62, rim_y);
            let rim_r = egui::pos2(center.x + r * 0.62, rim_y);
            painter.line_segment([rim_l, rim_r], st);
            painter.line_segment(
                [
                    egui::pos2(rim_l.x, rim_y - sw),
                    egui::pos2(center.x - r * 0.30, center.y - r * 0.22),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(rim_r.x, rim_y - sw),
                    egui::pos2(center.x + r * 0.30, center.y - r * 0.22),
                ],
                st,
            );
            painter.circle_filled(egui::pos2(center.x, rim_y + sw * 1.6), sw * 1.1, color);
        }
        Icon::Gear => {
            // 外圈 + 8 齿短辐条 + 中心孔
            let gr = r * 0.58;
            painter.circle_stroke(center, gr, st);
            for i in 0..8 {
                let a = std::f32::consts::FRAC_PI_4 * i as f32;
                let (s, c) = a.sin_cos();
                painter.line_segment(
                    [
                        egui::pos2(center.x + gr * c, center.y + gr * s),
                        egui::pos2(center.x + r * 0.88 * c, center.y + r * 0.88 * s),
                    ],
                    st,
                );
            }
            painter.circle_stroke(center, gr * 0.30, st);
        }
        Icon::Plus => {
            let arm = r * 0.62;
            painter.line_segment(
                [
                    egui::pos2(center.x - arm, center.y),
                    egui::pos2(center.x + arm, center.y),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x, center.y - arm),
                    egui::pos2(center.x, center.y + arm),
                ],
                st,
            );
        }
        Icon::Minus => {
            let arm = r * 0.62;
            painter.line_segment(
                [
                    egui::pos2(center.x - arm, center.y),
                    egui::pos2(center.x + arm, center.y),
                ],
                st,
            );
        }
        Icon::Trash => {
            // 桶盖横线 + 提钮 + 收腰梯形桶身（两侧斜线 + 底线）
            let lid_y = center.y - r * 0.42;
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.62, lid_y),
                    egui::pos2(center.x + r * 0.62, lid_y),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.20, lid_y - sw),
                    egui::pos2(center.x - r * 0.12, center.y - r * 0.72),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x + r * 0.20, lid_y - sw),
                    egui::pos2(center.x + r * 0.12, center.y - r * 0.72),
                ],
                st,
            );
            let bot_y = center.y + r * 0.78;
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.50, lid_y + sw),
                    egui::pos2(center.x - r * 0.38, bot_y),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x + r * 0.50, lid_y + sw),
                    egui::pos2(center.x + r * 0.38, bot_y),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.38, bot_y),
                    egui::pos2(center.x + r * 0.38, bot_y),
                ],
                st,
            );
        }
        Icon::Sun => {
            painter.circle_filled(center, r * 0.34, color);
            for i in 0..8 {
                let a = std::f32::consts::FRAC_PI_4 * i as f32;
                let (s, c) = a.sin_cos();
                painter.line_segment(
                    [
                        egui::pos2(center.x + r * 0.50 * c, center.y + r * 0.50 * s),
                        egui::pos2(center.x + r * 0.86 * c, center.y + r * 0.86 * s),
                    ],
                    st,
                );
            }
        }
        Icon::Moon => {
            // 厚弧月牙：开口朝右的 C 形粗弧（stroke 宽 = 0.42r，弧段 120°→420°）
            let crescent = arc_points(
                center,
                r * 0.58,
                std::f32::consts::FRAC_PI_3 * 2.0,
                std::f32::consts::TAU + std::f32::consts::FRAC_PI_3 * 2.0,
                24,
            );
            painter.add(egui::Shape::line(crescent, Stroke::new(r * 0.42, color)));
        }
        Icon::Refresh => {
            // 开口圆弧 + 箭头（两短线收口）
            let rr = r * 0.66;
            let a0 = -std::f32::consts::FRAC_PI_2 + 0.5;
            let a1 = std::f32::consts::PI + 1.1;
            let pts = arc_points(center, rr, a0, a1, 20);
            painter.add(egui::Shape::line(pts, st));
            let tip = egui::pos2(center.x + rr * a1.cos(), center.y + rr * a1.sin());
            let inward = Vec2::normalized(egui::vec2(tip.x - center.x, tip.y - center.y));
            let tangent = egui::vec2(-inward.y, inward.x);
            let ah = sw * 2.6;
            painter.line_segment([tip, tip - inward * ah + tangent * ah], st);
            painter.line_segment([tip, tip - inward * ah - tangent * ah], st);
        }
    }
}

// ===== src/widgets/icons.rs 说明区 =====
// 输入：painter + 中心点/尺寸/图标/颜色；输出：纯矢量描线图标
// 逻辑步骤：stroke 宽随 size 缩放（size×0.09，下限 1.2）→ 各图标以
//   circle/line/arc 采样折线组合（arc_points 24 段内半径弦差可忽略）
// 设计理由：UIR 重塑定案——原 ⚙＋－☀☾ 文本字符图标跨设备字体回落不一致，
//   且与玻璃质感割裂；矢量绘制零字体依赖、随主题语义色、随 Dock 缩放；
//   月牙用厚弧（stroke=0.42r）而非双圆布尔（epaint 无减法，路径解算过重）
// 异常处理：无 panic 面（纯几何）；size 过小时线宽下限 1.2 保可辨识
// 关联依赖：egui Painter/Shape；消费方：widgets::dock/widgets::icon_button/各页
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_points_endpoints_exact() {
        // 端点精确落在起止角上（图标几何的可证锚点）
        let c = egui::pos2(10.0, 10.0);
        let pts = arc_points(c, 5.0, 0.0, std::f32::consts::PI, 8);
        assert_eq!(pts.len(), 9);
        assert!((pts[0].x - 15.0).abs() < 1e-5 && (pts[0].y - 10.0).abs() < 1e-5);
        assert!((pts[8].x - 5.0).abs() < 1e-5 && (pts[8].y - 10.0).abs() < 1e-5);
    }

    #[test]
    fn all_icons_draw_without_panic() {
        // 全图标冒烟：任意尺寸/位置不 panic（纯 Shape 几何，无状态）
        let ctx = egui::Context::default();
        let painter = egui::Painter::new(
            ctx,
            egui::LayerId::background(),
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0)),
        );
        for icon in [
            Icon::Clock,
            Icon::Hourglass,
            Icon::Globe,
            Icon::Cloud,
            Icon::Bell,
            Icon::Gear,
            Icon::Plus,
            Icon::Minus,
            Icon::Trash,
            Icon::Sun,
            Icon::Moon,
            Icon::Refresh,
        ] {
            draw(&painter, egui::pos2(50.0, 50.0), 20.0, icon, Color32::WHITE);
            draw(&painter, egui::pos2(0.0, 0.0), 4.0, icon, Color32::GRAY);
        }
    }
}
