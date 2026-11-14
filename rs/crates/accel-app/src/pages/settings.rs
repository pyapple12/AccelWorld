//! 设置页（PL017）：不再是右下三行“关于”浮窗；作为右侧抽屉页与时钟/四页同级。
//! 分区：外观（主题只读）/ 动效（减少动效 switch）/ 关于（版本/数据源/字体）。

use crate::bus::UiState;
use crate::widgets::{icons, switch};

/// 设置页正文（页头由 pages::Pages 统一绘制）
pub fn ui(ui: &mut egui::Ui, state: &mut UiState) {
    let ink = crate::theme::ink();
    // 盒高查启动指标表（角色最大墨迹高，不是当前字符串）
    let section_font = crate::theme::font_semibold(crate::theme::font_size("section"));
    let meta_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let body_font = crate::theme::font_regular(crate::theme::font_size("body"));
    let section_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &section_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let meta_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &meta_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let body_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &body_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let section_h = section_h
        + crate::theme::spacing_f32("sm")
        + meta_h
        + crate::theme::spacing_f32("sm")
        + crate::theme::layout_f32("field_border");
    let row_h = body_h + crate::theme::spacing_f32("lg");
    let switch_h =
        body_h + crate::theme::spacing_f32("xs") + meta_h + crate::theme::spacing_f32("lg");
    let brand_h = meta_h + crate::theme::spacing_f32("sm");
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
            let rect = stack.next(ui, switch_h);
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
            let brand_box = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), meta_h));
            ui.painter().text(
                brand_box.left_center(),
                egui::Align2::LEFT_CENTER,
                "天宫时计 · 子夜星图",
                meta_font,
                ink.meta,
            );
        });
}

/// 分区盒：标题盒 + 8 + 说明盒 + 8 + 发丝
fn section_box(ui: &egui::Ui, rect: egui::Rect, title: &str, desc: &str, ink: &crate::theme::Ink) {
    let section_font = crate::theme::font_semibold(crate::theme::font_size("section"));
    let desc_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let section_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &section_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let desc_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &desc_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let title_box = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), section_h));
    ui.painter().text(
        title_box.left_center(),
        egui::Align2::LEFT_CENTER,
        title,
        section_font,
        ink.strong,
    );
    let desc_box = egui::Rect::from_min_size(
        egui::pos2(
            rect.left(),
            rect.top() + section_h + crate::theme::spacing_f32("sm"),
        ),
        egui::vec2(rect.width(), desc_h),
    );
    ui.painter().text(
        desc_box.left_center(),
        egui::Align2::LEFT_CENTER,
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
    let font = crate::theme::font_regular(crate::theme::font_size("body"));
    painter.text(
        rect.left_center(),
        egui::Align2::LEFT_CENTER,
        label,
        font.clone(),
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
        font,
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
    let title_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &title_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let desc_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &desc_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let title_box = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.top() + crate::theme::spacing_f32("sm")),
        egui::vec2(rect.width(), title_h),
    );
    ui.painter().text(
        title_box.left_center(),
        egui::Align2::LEFT_CENTER,
        "减少动效",
        title_font,
        ink.body,
    );
    let desc_box = egui::Rect::from_min_size(
        egui::pos2(
            rect.left(),
            title_box.bottom() + crate::theme::spacing_f32("xs"),
        ),
        egui::vec2(rect.width(), desc_h),
    );
    ui.painter().text(
        desc_box.left_center(),
        egui::Align2::LEFT_CENTER,
        "直取终态，不做过渡",
        desc_font,
        ink.muted,
    );
    let sw_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 36.0, rect.center().y),
        egui::vec2(switch::width(), switch::height()),
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
