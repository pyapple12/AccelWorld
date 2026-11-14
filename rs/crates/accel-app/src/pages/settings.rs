//! 设置页（PL017）：不再是右下三行“关于”浮窗；作为右侧抽屉页与时钟/四页同级。
//! 分区：外观（主题只读）/ 动效（减少动效 switch）/ 关于（版本/数据源/字体）。

use crate::bus::UiState;
use crate::widgets::{icons, switch};

/// 设置页正文（页头由 pages::Pages 统一绘制）
pub fn ui(ui: &mut egui::Ui, state: &mut UiState) {
    let ink = crate::theme::ink();
    // 盒高按文字墨迹计算（不是行框）
    let section_font = crate::theme::font_semibold(crate::theme::font_size("section"));
    let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let body_font = crate::theme::font_regular(crate::theme::font_size("body"));
    let section_ink = crate::widgets::text_ink::measure(
        ui,
        "外观",
        &section_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let meta_ink = crate::widgets::text_ink::measure(
        ui,
        "当前",
        &meta_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let body_ink = crate::widgets::text_ink::measure(
        ui,
        "主题",
        &body_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let section_h = section_ink.h + 8.0 + meta_ink.h + 8.0 + 1.0;
    let row_h = body_ink.h + 16.0;
    let switch_h = body_ink.h + 4.0 + meta_ink.h + 16.0;
    let brand_h = meta_ink.h + 8.0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut stack = crate::widgets::box_stack::BoxStack::new(ui);

            // 外观
            let rect = stack.next(ui, section_h);
            section_box(ui, rect, "外观", "当前主题", &ink);
            let rect = stack.next(ui, row_h);
            row_box(ui, rect, "主题", "子夜星图（深色）", &ink, false);

            // 动效
            let rect = stack.next(ui, section_h);
            section_box(
                ui,
                rect,
                "动效",
                "关闭抽屉、悬停、按压与闹钟脉冲的过渡",
                &ink,
            );
            let rect = stack.slot(switch_h);
            let clicked = ui
                .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    switch_box(ui, rect, state, &ink)
                })
                .inner;
            if clicked {
                state.reduce_motion = !state.reduce_motion;
            }

            // 关于
            let rect = stack.next(ui, section_h);
            section_box(ui, rect, "关于", "版本与数据来源", &ink);
            let rows = [
                (
                    "版本",
                    accel_core::static_config::Static::get()
                        .version()
                        .to_string(),
                    false,
                ),
                ("数据来源", "Open-Meteo（免费天气接口）".to_string(), true),
                (
                    "字体",
                    "MiSans（小米，免费商用）/ Bahnschrift".to_string(),
                    false,
                ),
            ];
            for (label, value, cloud) in rows {
                let rect = stack.next(ui, row_h);
                row_box(ui, rect, label, &value, &ink, cloud);
            }

            let rect = stack.next(ui, brand_h);
            ui.painter().text(
                egui::pos2(rect.left(), rect.center().y),
                egui::Align2::LEFT_CENTER,
                "加速世界 · 子夜星图",
                meta_font,
                ink.meta,
            );
        });
}

/// 分区盒：标题 + 说明 + 底部发丝（按墨迹定位）
fn section_box(ui: &egui::Ui, rect: egui::Rect, title: &str, desc: &str, ink: &crate::theme::Ink) {
    let section_font = crate::theme::font_semibold(crate::theme::font_size("section"));
    let desc_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let section_ink = crate::widgets::text_ink::measure(
        ui,
        title,
        &section_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let desc_ink = crate::widgets::text_ink::measure(
        ui,
        desc,
        &desc_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    ui.painter().text(
        egui::pos2(rect.left(), rect.top() - section_ink.top_pad),
        egui::Align2::LEFT_TOP,
        title,
        section_font,
        ink.strong,
    );
    let desc_top = rect.top() + section_ink.h + 8.0;
    ui.painter().text(
        egui::pos2(rect.left(), desc_top - desc_ink.top_pad),
        egui::Align2::LEFT_TOP,
        desc,
        desc_font,
        ink.muted,
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom() - 1.0),
            egui::pos2(rect.right(), rect.bottom() - 1.0),
        ],
        egui::Stroke::new(1.0, crate::theme::border_color()),
    );
}

/// 只读行盒：左标签，右值（可选云图标）
fn row_box(
    ui: &egui::Ui,
    rect: egui::Rect,
    label: &str,
    value: &str,
    ink: &crate::theme::Ink,
    cloud: bool,
) {
    let painter = ui.painter().with_clip_rect(rect);
    painter.text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        crate::theme::font_regular(crate::theme::font_size("body")),
        ink.body,
    );
    let mut right = rect.right();
    if cloud {
        icons::draw(
            &painter,
            egui::pos2(rect.right() - 8.0, rect.center().y),
            14.0,
            icons::Icon::Cloud,
            ink.meta,
        );
        right -= 22.0;
    }
    painter.text(
        egui::pos2(right, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        value,
        crate::theme::font_regular(crate::theme::font_size("body")),
        ink.muted,
    );
}

/// 动效开关盒：左侧两行文字，右侧开关
fn switch_box(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &UiState,
    ink: &crate::theme::Ink,
) -> bool {
    let title_font = crate::theme::font_medium(crate::theme::font_size("body"));
    let desc_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let title_ink = crate::widgets::text_ink::measure(
        ui,
        "减少动效",
        &title_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let desc_ink = crate::widgets::text_ink::measure(
        ui,
        "直取终态，不做过渡",
        &desc_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let title_top = rect.top() + 8.0;
    ui.painter().text(
        egui::pos2(rect.left(), title_top - title_ink.top_pad),
        egui::Align2::LEFT_TOP,
        "减少动效",
        title_font,
        ink.body,
    );
    let desc_top = title_top + title_ink.h + 4.0;
    ui.painter().text(
        egui::pos2(rect.left(), desc_top - desc_ink.top_pad),
        egui::Align2::LEFT_TOP,
        "直取终态，不做过渡",
        desc_font,
        ink.muted,
    );
    let sw_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 36.0, rect.center().y),
        egui::vec2(switch::SWITCH_W, switch::SWITCH_H),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sw_rect));
    switch::show(
        &mut child,
        egui::Id::unique("settings_reduce_motion"),
        state.reduce_motion,
    )
}

// ===== src/pages/settings.rs 说明区 =====
// 输入：UiState（reduce_motion）；输出：reduce_motion 翻转 + 只读信息
// 逻辑步骤：ScrollArea → 外观（只读）→ 动效（自定义 switch）→ 关于（版本/数据源/字体）
// 设计理由（PL017）：原 `egui::Window` title_bar(false) 只有三行、无标题/关闭，像
//   调试浮窗；改为与四页同构的抽屉页（页头由 Pages 统一给），设置成为真正的页面
// 异常处理：无 panic 面；版本经 Static 单源
// 关联依赖：theme/widgets(switch/icons)；消费方：pages::Pages::ui
