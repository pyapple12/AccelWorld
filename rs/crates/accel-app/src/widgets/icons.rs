//! 矢量图标库（PL017 Lucide 24 网格归一）：painter path 直接绘制，零字体/零 emoji
//! 依赖；线宽按 24 网格 2px 随尺寸缩放（钳 [1.25,2.0]），颜色走调用方语义色；
//! 几何按 Lucide 线稿归一（圆/线/弧采样），不引运行时 SVG/字体依赖。

use egui::{Color32, Painter, Pos2, Stroke, Vec2};

/// 图标集（侧栏五页 + 通用操作 + 昼夜/天气现象指示；20 变体）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// 时钟（表盘 + 指针）
    Clock,
    /// 沙漏（倒计时）
    Hourglass,
    /// 地球（世界时钟）
    Globe,
    /// 云（天气：阴/默认）
    Cloud,
    /// 雨（云 + 落雨丝）
    Rain,
    /// 雪（云 + 雪点）
    Snow,
    /// 雾（三道横线）
    Fog,
    /// 雷暴（云 + 闪电）
    Storm,
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
    /// 对勾（PL016.04：删除两步确认的武装态）
    Check,
    /// 关闭（PL017：抽屉/设置页头）
    Close,
    /// 定位/基准（PL017：世界时钟基准标记）
    Pin,
    /// 风（PL016.09：天气要素行风力段）
    Wind,
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

/// 云形折线（底线 + 三段半圆凸包；Cloud/Rain/Snow/Storm 共用几何，升腾排布由调用方定）
fn cloud_line(painter: &Painter, center: Pos2, r: f32, st: Stroke) {
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

/// 图标绘制：以 center 为中心、边长 size 的方形域内描线（线宽随尺寸缩放）
pub fn draw(painter: &Painter, center: Pos2, size: f32, icon: Icon, color: Color32) {
    // Lucide 24 网格：默认 2px / 24；随尺寸缩放，小尺寸钳 1.25，大尺寸不超过 2.0
    let sw = (size * 2.0 / 24.0).clamp(1.25, 2.0);
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
            cloud_line(painter, center, r, st);
        }
        Icon::Rain => {
            // 云升 + 三道斜落雨丝（A008 #5：现象图标随天气码）
            cloud_line(
                painter,
                egui::pos2(center.x, center.y - r * 0.22),
                r * 0.78,
                st,
            );
            for i in 0..3 {
                let x = center.x - r * 0.34 + r * 0.34 * i as f32;
                painter.line_segment(
                    [
                        egui::pos2(x, center.y + r * 0.28),
                        egui::pos2(x - r * 0.10, center.y + r * 0.62),
                    ],
                    st,
                );
            }
        }
        Icon::Snow => {
            // 云升 + 三粒雪点
            cloud_line(
                painter,
                egui::pos2(center.x, center.y - r * 0.22),
                r * 0.78,
                st,
            );
            for i in 0..3 {
                let x = center.x - r * 0.34 + r * 0.34 * i as f32;
                painter.circle_filled(egui::pos2(x, center.y + r * 0.48), sw * 0.95, color);
            }
        }
        Icon::Fog => {
            // 三道横雾线（中长上下短，雾的通用记法）
            for (i, w) in [1.00_f32, 0.72, 0.86].iter().enumerate() {
                let y = center.y - r * 0.34 + r * 0.34 * i as f32;
                let half = r * 0.62 * w;
                painter.line_segment(
                    [
                        egui::pos2(center.x - half, y),
                        egui::pos2(center.x + half, y),
                    ],
                    st,
                );
            }
        }
        Icon::Storm => {
            // 云升 + 折线闪电
            cloud_line(
                painter,
                egui::pos2(center.x, center.y - r * 0.22),
                r * 0.78,
                st,
            );
            painter.add(egui::Shape::line(
                vec![
                    egui::pos2(center.x + r * 0.12, center.y + r * 0.16),
                    egui::pos2(center.x - r * 0.14, center.y + r * 0.50),
                    egui::pos2(center.x + r * 0.08, center.y + r * 0.50),
                    // A009 #17：末端收至 0.80r——加半线宽后仍满足「四周 ≥1px」留边纪律
                    egui::pos2(center.x - r * 0.14, center.y + r * 0.80),
                ],
                st,
            ));
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
            // Lucide settings 线稿：12 齿短辐条 + 内圈 + 中心孔
            let gr = r * 0.58;
            painter.circle_stroke(center, gr, st);
            for i in 0..12 {
                let a = std::f32::consts::TAU * (i as f32 / 12.0);
                let (s, c) = a.sin_cos();
                painter.line_segment(
                    [
                        egui::pos2(center.x + gr * c, center.y + gr * s),
                        egui::pos2(center.x + r * 0.86 * c, center.y + r * 0.86 * s),
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
            painter.circle_stroke(center, r * 0.36, st);
            for i in 0..8 {
                let a = std::f32::consts::FRAC_PI_4 * i as f32;
                let (s, c) = a.sin_cos();
                painter.line_segment(
                    [
                        egui::pos2(center.x + r * 0.56 * c, center.y + r * 0.56 * s),
                        egui::pos2(center.x + r * 0.90 * c, center.y + r * 0.90 * s),
                    ],
                    st,
                );
            }
        }
        Icon::Moon => {
            // Lucide moon 线稿：外弧 + 内弧闭合成细月牙（不再用 0.42r 厚弧）
            let mut pts = arc_points(
                center,
                r * 0.62,
                std::f32::consts::FRAC_PI_3,
                std::f32::consts::TAU - std::f32::consts::FRAC_PI_3,
                20,
            );
            let inner_center = egui::pos2(center.x + r * 0.20, center.y - r * 0.10);
            let inner = arc_points(
                inner_center,
                r * 0.46,
                std::f32::consts::TAU - std::f32::consts::FRAC_PI_3,
                std::f32::consts::FRAC_PI_3,
                20,
            );
            pts.extend(inner);
            painter.add(egui::Shape::closed_line(pts, st));
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
        Icon::Check => {
            // PL016.04：对勾（删除两步确认武装态）——短臂下折 + 长臂上扬
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.44, center.y + 0.02 * r),
                    egui::pos2(center.x - r * 0.10, center.y + r * 0.34),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.10, center.y + r * 0.34),
                    egui::pos2(center.x + r * 0.46, center.y - r * 0.30),
                ],
                st,
            );
        }
        Icon::Close => {
            // 两对角交叉（圆头端点由 egui 线段默认方头近似；尺寸小不影响识读）
            let arm = r * 0.52;
            painter.line_segment(
                [
                    egui::pos2(center.x - arm, center.y - arm),
                    egui::pos2(center.x + arm, center.y + arm),
                ],
                st,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x + arm, center.y - arm),
                    egui::pos2(center.x - arm, center.y + arm),
                ],
                st,
            );
        }
        Icon::Pin => {
            // 圆头 + 斜杆 + 底点（简化的定位钉）
            painter.circle_stroke(center, r * 0.34, st);
            painter.line_segment(
                [
                    egui::pos2(center.x, center.y + r * 0.34),
                    egui::pos2(center.x, center.y + r * 0.78),
                ],
                st,
            );
            painter.circle_filled(center, sw * 0.85, color);
        }
        Icon::Wind => {
            // PL016.09：风——三道横流 + 上两道右端上卷（arc 采样与库同语言）
            // PL016.18：线距 0.24r→0.35r（对齐 Lucide 光学网格疏密）——14-18px
            // 小尺寸下原间距减线宽仅 ~0.4px 视觉读作「≈」（复审视觉实测）
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.50, center.y - r * 0.30),
                    egui::pos2(center.x + r * 0.16, center.y - r * 0.30),
                ],
                st,
            );
            let pts = arc_points(
                egui::pos2(center.x + r * 0.16, center.y - r * 0.48),
                r * 0.18,
                std::f32::consts::FRAC_PI_2,
                0.0,
                8,
            );
            painter.add(egui::Shape::line(pts, st));
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.30, center.y + r * 0.05),
                    egui::pos2(center.x + r * 0.24, center.y + r * 0.05),
                ],
                st,
            );
            let pts = arc_points(
                egui::pos2(center.x + r * 0.24, center.y - r * 0.13),
                r * 0.18,
                std::f32::consts::FRAC_PI_2,
                0.0,
                8,
            );
            painter.add(egui::Shape::line(pts, st));
            painter.line_segment(
                [
                    egui::pos2(center.x - r * 0.44, center.y + r * 0.42),
                    egui::pos2(center.x + r * 0.30, center.y + r * 0.42),
                ],
                st,
            );
        }
    }
}

// ===== src/widgets/icons.rs 说明区 =====
// 输入：painter + 中心点/尺寸/图标/颜色；输出：纯矢量描线图标
// 逻辑步骤：stroke 宽随 size 缩放（size×0.09，下限 1.2）→ 各图标以
//   circle/line/arc 采样折线组合（arc_points 24 段内半径弦差可忽略）
// 设计理由：UIR 重塑定案——原 ⚙＋－☀☾ 文本字符图标跨设备字体回落不一致，
//   且与玻璃质感割裂；矢量绘制零字体依赖、随侧栏缩放；
//   月牙用厚弧（stroke=0.42r）而非双圆布尔（epaint 无减法，路径解算过重）
// 异常处理：无 panic 面（纯几何）；size 过小时线宽下限 1.2 保可辨识
// 关联依赖：egui Painter/Shape；消费方：window.rs 侧栏/widgets::icon_button/各页
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
            Icon::Rain,
            Icon::Snow,
            Icon::Fog,
            Icon::Storm,
            Icon::Bell,
            Icon::Gear,
            Icon::Plus,
            Icon::Minus,
            Icon::Trash,
            Icon::Sun,
            Icon::Moon,
            Icon::Refresh,
            Icon::Check,
            Icon::Close,
            Icon::Pin,
            Icon::Wind,
        ] {
            for size in [16.0, 20.0, 24.0, 32.0] {
                draw(&painter, egui::pos2(50.0, 50.0), size, icon, Color32::WHITE);
            }
            draw(&painter, egui::pos2(0.0, 0.0), 4.0, icon, Color32::GRAY);
        }
    }
}
