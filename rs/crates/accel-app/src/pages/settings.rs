//! 设置页（PL017）：不再是右下三行“关于”浮窗；作为右侧抽屉页与时钟/四页同级。
//! 分区：外观（主题只读）/ 动效（减少动效 switch）/ 关于（版本/数据源/字体）。

use crate::bus::{Bus, UiState};
use crate::widgets::{field, icons, switch};
use accel_core::types::FrontendCommand;

/// 设置页正文（页头由 pages::Pages 统一绘制）
pub fn ui(ui: &mut egui::Ui, state: &mut UiState, bus: &Bus) {
    let ink = crate::theme::ink();
    // 盒高查 font_profile（不是当前字符串）
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
    let field_h = field::fixed_height(ui);
    let brand_h = meta_h + crate::theme::spacing_f32("sm");
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut stack = crate::widgets::box_stack::BoxStack::new(ui);

            // 外观
            let rect = stack.next(ui, section_h);
            section_box(ui, rect, "外观", "当前主题", &ink);
            let rect = stack.next(ui, row_h);
            row_box(
                ui,
                rect,
                "主题",
                "子夜星图（深色）",
                &ink,
                false,
                crate::widgets::text_ink::InkKind::Cjk,
            );

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

            // 网络（PL027）：天气代理开关 + 代理服务器地址
            let rect = stack.next(ui, section_h);
            section_box(ui, rect, "网络", "天气拉取与代理", &ink);
            let rect = stack.next(ui, switch_h);
            let toggled = ui
                .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    switch_row(
                        ui,
                        rect,
                        "使用代理服务器",
                        "关闭后天气只直连；打开后直连失败走代理",
                        "settings_weather_use_proxy",
                        state.prefs.weather_use_proxy,
                        &ink,
                    )
                })
                .inner;
            if toggled {
                state.prefs.weather_use_proxy = !state.prefs.weather_use_proxy;
                bus.send(FrontendCommand::SetWeatherProxy {
                    enabled: state.prefs.weather_use_proxy,
                    server: state.prefs.weather_proxy.clone(),
                });
            }
            let rect = stack.next(ui, field_h);
            let proxy_resp = ui
                .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    field::show_fixed(
                        ui,
                        "代理服务器",
                        "留空=跟随系统代理",
                        &mut state.prefs.weather_proxy,
                        rect.width(),
                    )
                })
                .inner;
            // 单行 TextEdit 回车先 surrender_focus，同帧 has_focus 已 false；
            // 用 lost_focus 才能收到回车提交（与倒计时/世界页同款）
            if proxy_resp.lost_focus() {
                bus.send(FrontendCommand::SetWeatherProxy {
                    enabled: state.prefs.weather_use_proxy,
                    server: state.prefs.weather_proxy.clone(),
                });
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
                    crate::widgets::text_ink::InkKind::Digits,
                ),
                (
                    "数据来源",
                    "Open-Meteo（免费天气接口）".to_string(),
                    true,
                    crate::widgets::text_ink::InkKind::Cjk,
                ),
                (
                    "字体",
                    "MiSans / Barlow Medium（OFL）".to_string(),
                    false,
                    crate::widgets::text_ink::InkKind::Cjk,
                ),
            ];
            for (label, value, cloud, kind) in rows {
                let rect = stack.next(ui, row_h);
                row_box(ui, rect, label, &value, &ink, cloud, kind);
            }

            let rect = stack.next(ui, brand_h);
            let brand_box = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), meta_h));
            let brand_ink = crate::widgets::text_ink::measure(
                ui,
                "天宫时计 · 子夜星图",
                &meta_font,
                crate::widgets::text_ink::InkKind::Cjk,
            );
            ui.painter().text(
                egui::pos2(
                    brand_box.left(),
                    brand_ink.top_for_bottom(brand_box.bottom()),
                ),
                egui::Align2::LEFT_TOP,
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
    let title_ink = crate::widgets::text_ink::measure(
        ui,
        title,
        &section_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    ui.painter().text(
        egui::pos2(
            title_box.left(),
            title_ink.top_for_bottom(title_box.bottom()),
        ),
        egui::Align2::LEFT_TOP,
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
    let desc_ink = crate::widgets::text_ink::measure(
        ui,
        desc,
        &desc_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    ui.painter().text(
        egui::pos2(desc_box.left(), desc_ink.top_for_bottom(desc_box.bottom())),
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
    value_kind: crate::widgets::text_ink::InkKind,
) {
    let painter = ui.painter().with_clip_rect(rect);
    let label_font = crate::theme::font_regular(crate::theme::font_size("body"));
    let value_font = crate::theme::font_regular(crate::theme::font_size("meta"));
    let text_h = crate::widgets::text_metrics::height(
        ui.ctx(),
        &label_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    let text_bottom = rect.center().y + text_h / 2.0;
    let label_ink = crate::widgets::text_ink::measure(
        ui,
        label,
        &label_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    painter.text(
        egui::pos2(rect.left(), label_ink.top_for_bottom(text_bottom)),
        egui::Align2::LEFT_TOP,
        label,
        label_font,
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
    let value_ink = crate::widgets::text_ink::measure(ui, value, &value_font, value_kind);
    painter.text(
        egui::pos2(right, value_ink.top_for_bottom(text_bottom)),
        egui::Align2::RIGHT_TOP,
        value,
        value_font,
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
    switch_row(
        ui,
        rect,
        "减少动效",
        "直取终态，不做过渡",
        "settings_reduce_motion",
        state.reduce_motion,
        ink,
    )
}

/// 通用开关行：标题 + 描述 + 右侧开关（PL027 复用给天气代理）
fn switch_row(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    title: &str,
    desc: &str,
    id: &str,
    value: bool,
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
    let title_ink = crate::widgets::text_ink::measure(
        ui,
        title,
        &title_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    ui.painter().text(
        egui::pos2(
            title_box.left(),
            title_ink.top_for_bottom(title_box.bottom()),
        ),
        egui::Align2::LEFT_TOP,
        title,
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
    let desc_ink = crate::widgets::text_ink::measure(
        ui,
        desc,
        &desc_font,
        crate::widgets::text_ink::InkKind::Cjk,
    );
    ui.painter().text(
        egui::pos2(desc_box.left(), desc_ink.top_for_bottom(desc_box.bottom())),
        egui::Align2::LEFT_TOP,
        desc,
        desc_font,
        ink.muted,
    );
    let sw_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 36.0, rect.center().y),
        egui::vec2(switch::width(), switch::height()),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sw_rect));
    switch::show(&mut child, egui::Id::unique(id), value)
}

// ===== src/pages/settings.rs 说明区 =====
// 输入：UiState（reduce_motion / weather 代理偏好）；输出：UI 偏好与 SetWeatherProxy 命令
// 逻辑步骤：ScrollArea → 外观（只读）→ 动效（switch）→ 网络（代理开关 + 地址）→ 关于（版本/数据源/字体）
// 设计理由（PL017）：原 `egui::Window` title_bar(false) 只有三行、无标题/关闭，像
//   调试浮窗；改为与四页同构的抽屉页（页头由 Pages 统一给），设置成为真正的页面
// 异常处理：无 panic 面；版本经 Static 单源
// 关联依赖：theme/widgets(switch/icons)；消费方：pages::Pages::ui
