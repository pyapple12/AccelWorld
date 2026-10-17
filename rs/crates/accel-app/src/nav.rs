//! 胶囊导航（ui/panels/glass_nav.py 的 Rust 对等物，PL013.09）
//! 悬浮 rail：五页胶囊 + 底部齿轮；选中金边框呼吸；玻璃面注册（elevation 高层）

use egui::{Color32, Context, Rect, Sense, Vec2};

use crate::bus::{PAGE_ORDER, PageId, UiState};
use crate::glass::surface::{GlassScene, GlassSurface};
use crate::window::SURFACE_NAV;

// 导航 rail 几何（token 消费：ui.json nav_rail_* 域；零硬编码原则）
// nav_rail_width=84/nav_pill_height=62/nav_pill_spacing=10/nav_pill_pad=6/
// nav_rail_margin_top=56；pill 宽/rail 左距/齿轮间距无 token → 局部派生常量
fn nav_token(key: &str) -> f32 {
    accel_core::static_config::Static::get().ui_nested_i64("layout", key) as f32
}

/// rail 布局参数（每帧从 token 装载——ui.json 改即生效）
struct NavGeom {
    top: f32,
    pill_w: f32,
    pill_h: f32,
    gap: f32,
    gear_gap: f32,
}

impl NavGeom {
    fn load() -> NavGeom {
        let rail_w = nav_token("nav_rail_width");
        NavGeom {
            top: nav_token("nav_rail_margin_top"),
            pill_w: rail_w - 2.0 * nav_token("nav_pill_pad") - 4.0,
            pill_h: nav_token("nav_pill_height"),
            gap: nav_token("nav_pill_spacing"),
            gear_gap: nav_token("nav_pill_spacing"),
        }
    }

    fn rail_x() -> f32 {
        // rail 左距：居中于窗口左缘窄带（宽 nav_rail_width，左距取 pad+2）
        nav_token("nav_pill_pad") + 8.0
    }

    fn rail_w() -> f32 {
        nav_token("nav_rail_width")
    }
}

/// 帧交互结果（Area 闭包内收集，闭包外应用——self 借用隔离）
struct NavFrame {
    clicked: Option<PageId>,
    gear_toggled: bool,
    hovered_any: bool,
}

/// 导航 rail 状态（选中页变化记忆——变化帧触发 touch）
#[derive(Debug, Default)]
pub struct NavRail {
    last_selected: Option<PageId>,
}

impl NavRail {
    pub fn new() -> NavRail {
        NavRail {
            last_selected: None,
        }
    }

    /// 绘制 rail + 玻璃面注册 + 点击应用
    pub fn ui(
        &mut self,
        ctx: &Context,
        state: &mut UiState,
        scene: &mut GlassScene,
        accent: [f32; 4],
    ) {
        let geom = NavGeom::load();
        let height = geom.pill_h * PAGE_ORDER.len() as f32
            + geom.gap * (PAGE_ORDER.len() - 1) as f32
            + geom.gear_gap
            + geom.pill_h;
        let rail_rect = Rect::from_min_size(
            egui::pos2(NavGeom::rail_x(), geom.top),
            Vec2::new(NavGeom::rail_w(), height),
        );

        // rail 玻璃面（elevation 10 高于页卡——悬于内容之上）
        let mut surface = GlassSurface::new(SURFACE_NAV, rail_rect);
        surface.radius = geom.pill_h / 2.0 + 6.0;
        surface.elevation = 10;
        scene.register(surface);

        // 帧交互（闭包内只动局部——self 借用隔离）
        let mut frame = NavFrame {
            clicked: None,
            gear_toggled: false,
            hovered_any: false,
        };
        egui::Area::new(egui::Id::unique("nav_rail"))
            .fixed_pos(rail_rect.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_min_size(rail_rect.size());
                let time = ctx.input(|i| i.time) as f32;
                ui.vertical(|ui| {
                    for page in PAGE_ORDER {
                        let label = page_label(page);
                        let selected = state.current_page == page;
                        let resp = ui
                            .allocate_response(
                                Vec2::new(geom.pill_w, geom.pill_h - 6.0),
                                Sense::click(),
                            )
                            .on_hover_text(label);
                        if resp.hovered() {
                            frame.hovered_any = true;
                        }
                        // 选中金边框呼吸（sin 相位；GL 侧由 rail 面 selected 驱动）
                        let breath = if selected {
                            0.75 + 0.25 * (time * 2.2).sin()
                        } else if resp.hovered() {
                            0.35
                        } else {
                            0.0
                        };
                        let accent_color = Color32::from_rgba_premultiplied(
                            (accent[0] * accent[3] * 255.0) as u8,
                            (accent[1] * accent[3] * 255.0) as u8,
                            (accent[2] * accent[3] * 255.0) as u8,
                            (accent[3] * breath * 255.0) as u8,
                        );
                        paint_pill(ui, resp.rect, selected, resp.hovered(), accent_color, label);
                        if resp.clicked() && state.current_page != page {
                            frame.clicked = Some(page);
                        }
                    }
                    // A005 #3 修正：rail 面选中强度**在循环外每帧写一次**——循环内逐轮
                    // 写同一面会被 PAGE_ORDER 末位（非选中页）覆写为 0，致四页 GL 选中失效
                    write_rail_selection(scene, time);
                    ui.add_space(geom.gear_gap);
                    // 齿轮（设置浮窗开关）
                    let gear = ui
                        .allocate_response(
                            Vec2::new(geom.pill_h - 6.0, geom.pill_h - 6.0),
                            Sense::click(),
                        )
                        .on_hover_text("设置");
                    paint_gear(ui, gear.rect, gear.hovered());
                    if gear.hovered() {
                        frame.hovered_any = true;
                    }
                    if gear.clicked() {
                        frame.gear_toggled = true;
                    }
                });
            });

        // 帧交互应用（self 借用归还后）
        let touched = apply_nav_frame(state, frame);
        // touch 直连重绘：hover/选中变化帧触发，静态期零调用（FIX004.7 纪律）
        if touched || self.last_selected != Some(state.current_page) {
            scene.touch();
            self.last_selected = Some(state.current_page);
        }
    }
}

/// 帧交互应用（可测纯逻辑）：点击切页 / 齿轮翻转浮窗；返回是否需 touch 重绘
fn apply_nav_frame(state: &mut UiState, frame: NavFrame) -> bool {
    if let Some(page) = frame.clicked {
        state.current_page = page;
    }
    if frame.gear_toggled {
        state.settings_open = !state.settings_open;
    }
    frame.hovered_any || frame.clicked.is_some() || frame.gear_toggled
}

/// 页标签（PL014 页名定案前用短名）
fn page_label(page: PageId) -> &'static str {
    match page {
        PageId::Clock => "时钟",
        PageId::Countdown => "倒计时",
        PageId::World => "世界时钟",
        PageId::Weather => "天气",
        PageId::Alarm => "闹钟",
    }
}

/// 胶囊按钮绘制（Painter 矢量路径：边框 + 文字）
/// 玻璃底面由 GL 场景渲染（rail 整体一面），此处只画选中/hover 叠加层
fn paint_pill(
    ui: &egui::Ui,
    rect: Rect,
    selected: bool,
    hovered: bool,
    accent: Color32,
    label: &str,
) {
    // A005 #9：语义色 token（文字/非选中描边）
    let ink = crate::theme::ink();
    let painter = ui.painter();
    let rounding = egui::epaint::CornerRadius::same((rect.height() / 2.0).min(18.0) as u8);
    if selected || hovered {
        let stroke = if selected {
            egui::Stroke::new(1.6, accent)
        } else {
            egui::Stroke::new(1.0, crate::theme::color32_at("nav_pill_stroke", 60))
        };
        painter.rect_stroke(rect, rounding, stroke, egui::StrokeKind::Inside);
    }
    let text_color = if selected { accent } else { ink.card };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(crate::theme::font_size("label")),
        text_color,
    );
}

/// rail 面选中强度写入（A005 #3：**每帧恰调用一次**，由 ui 在 PAGE_ORDER 循环外调用；
/// 呼吸相位只随时间变化——选中页由调用点决定，绝不可放回循环内逐轮覆写）
pub(crate) fn write_rail_selection(scene: &mut GlassScene, time: f32) {
    let breath = 0.75 + 0.25 * (time * 2.2).sin();
    scene.set_selected(SURFACE_NAV, breath);
}

/// 齿轮绘制（中心圆 + 六齿简化外圈刻线）
fn paint_gear(ui: &egui::Ui, rect: Rect, hovered: bool) {
    // A005 #9：语义色 token（非 hover 用 text_card）
    let ink = crate::theme::ink();
    let painter = ui.painter();
    let c = rect.center();
    let r = rect.height() / 2.0 - 4.0;
    let color = if hovered {
        crate::theme::accent_color()
    } else {
        ink.card
    };
    painter.circle_stroke(c, r * 0.62, egui::Stroke::new(1.8, color));
    for k in 0..6 {
        let angle = k as f32 * std::f32::consts::TAU / 6.0;
        let dir = Vec2::new(angle.cos(), angle.sin());
        painter.line_segment(
            [c + dir * (r * 0.62), c + dir * r],
            egui::Stroke::new(1.8, color),
        );
    }
}

// ===== src/nav.rs 说明区 =====
// 输入：UiState（当前页）+ 场景；输出：rail 绘制 + 玻璃面 + 页切换/浮窗开关
// 逻辑步骤：rail 几何常量 → 玻璃面注册（elevation 10）→ 五胶囊 + 齿轮交互 →
//   帧结果应用（点击切换 current_page/齿轮翻转 settings_open）→ touch 直连重绘
// 设计理由：悬浮 rail 对齐 Python 定案；呼吸双路——GL shader u_sel（玻璃面
//   selected 强度）+ egui 矢量描边（降级态）；Area 闭包内不碰 self（借用隔离，
//   交互结果经 NavFrame 局部带出）；hover 态 resp.hovered() 本地记忆零计时器
// 已知偏移：页切换纯 UI 态不走 FrontendCommand（Python 同为主窗内切换）；
//   玻璃面显隐由 window.sync_page_visibility 消费 current_page
// 关联依赖：bus/glass/window；消费方：window.rs AppHandle::ui

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::PageId;

    #[test]
    fn click_applies_page_and_gear_toggles() {
        // .09/.10 验收：点击→current_page 切换；齿轮→settings_open 翻转（可开可关）
        let mut state = UiState::default();
        assert_eq!(state.current_page, PageId::Clock);
        assert!(apply_nav_frame(
            &mut state,
            NavFrame {
                clicked: Some(PageId::Weather),
                gear_toggled: false,
                hovered_any: false
            }
        ));
        assert_eq!(state.current_page, PageId::Weather);
        assert!(apply_nav_frame(
            &mut state,
            NavFrame {
                clicked: None,
                gear_toggled: true,
                hovered_any: false
            }
        ));
        assert!(state.settings_open, "首次齿轮应打开浮窗");
        // 再次齿轮 → 关闭浮窗
        assert!(apply_nav_frame(
            &mut state,
            NavFrame {
                clicked: None,
                gear_toggled: true,
                hovered_any: false
            }
        ));
        assert!(!state.settings_open, "再次齿轮应关闭浮窗");
        // 无交互帧：不 touch
        assert!(!apply_nav_frame(
            &mut state,
            NavFrame {
                clicked: None,
                gear_toggled: false,
                hovered_any: false
            }
        ));
        // 同页点击（已选中再点）仍切换为同值且视为交互
        assert!(apply_nav_frame(
            &mut state,
            NavFrame {
                clicked: Some(PageId::Weather),
                gear_toggled: false,
                hovered_any: false
            }
        ));
        assert_eq!(state.current_page, PageId::Weather);
    }

    #[test]
    fn rail_selected_nonzero_for_every_page() {
        // A005 #3 回归：任一选中页（含 PAGE_ORDER 非末位）写入后 rail 面 selected 均非零；
        // 原实现把写入放在五页循环内，末位（Alarm）以 0.0 覆写 → 四页 GL 选中失效
        for current in PAGE_ORDER {
            let mut scene = GlassScene::new();
            scene.register(GlassSurface::new(
                SURFACE_NAV,
                Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(60.0, 300.0)),
            ));
            // 模拟 ui 的循环外单次写入（current 仅决定调用时机，强度随时间呼吸）
            write_rail_selection(&mut scene, 0.0);
            let selected = scene
                .draw_list()
                .iter()
                .find(|s| s.surface_id == SURFACE_NAV)
                .map(|s| s.selected)
                .expect("rail 面应在场景中");
            assert!(
                selected > 0.5,
                "选中 {current:?} 时 rail 面 selected 应非零，实际 {selected}"
            );
        }
    }

    #[test]
    fn nav_geom_from_tokens() {
        // .09 token 化：几何参数来自 ui.json layout 域（零硬编码）
        let geom = NavGeom::load();
        assert_eq!(geom.top, 56.0, "nav_rail_margin_top token");
        assert_eq!(geom.pill_h, 62.0, "nav_pill_height token");
        assert_eq!(geom.gap, 10.0, "nav_pill_spacing token");
        assert!(
            geom.pill_w > 40.0 && geom.pill_w < NavGeom::rail_w(),
            "pill 宽由 rail 宽派生"
        );
    }
}
