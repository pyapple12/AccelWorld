//! 设置页（PL017）：不再是右下三行“关于”浮窗；作为右侧抽屉页与时钟/四页同级。
//! 分区：外观（主题只读）/ 动效（减少动效 switch）/ 关于（版本/数据源/字体）。

use crate::bus::UiState;
use crate::widgets::{icons, switch};

/// 设置页正文（页头由 pages::Pages 统一绘制）
pub fn ui(ui: &mut egui::Ui, state: &mut UiState) {
    let ink = crate::theme::ink();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            section(ui, "外观", "当前主题");
            row_static(ui, "主题", "子夜星图（深色）");
            ui.add_space(crate::theme::spacing("lg"));

            section(ui, "动效", "关闭抽屉、悬停、按压与闹钟脉冲的过渡");
            let clicked = ui
                .horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("减少动效")
                                .font(crate::theme::font_medium(crate::theme::font_size("body")))
                                .color(ink.body),
                        );
                        ui.label(
                            egui::RichText::new("直取终态，不做过渡")
                                .font(crate::theme::font_regular(crate::theme::font_size("meta")))
                                .color(ink.muted),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        switch::show(
                            ui,
                            egui::Id::unique("settings_reduce_motion"),
                            state.reduce_motion,
                        )
                    })
                    .inner
                })
                .inner;
            if clicked {
                state.reduce_motion = !state.reduce_motion;
            }
            ui.add_space(crate::theme::spacing("lg"));

            section(ui, "关于", "版本与数据来源");
            row_static(
                ui,
                "版本",
                accel_core::static_config::Static::get().version(),
            );
            row_static(ui, "数据来源", "Open-Meteo（免费天气接口）");
            row_static(ui, "字体", "MiSans（小米，免费商用）/ Bahnschrift");
            ui.add_space(crate::theme::spacing("lg"));
            ui.label(
                egui::RichText::new("加速世界 · 子夜星图")
                    .font(crate::theme::font_regular(crate::theme::font_size("meta")))
                    .color(ink.meta),
            );
        });
}

/// 分区标题 + 说明 + 底部发丝
fn section(ui: &mut egui::Ui, title: &str, desc: &str) {
    let ink = crate::theme::ink();
    ui.label(
        egui::RichText::new(title)
            .font(crate::theme::font_semibold(crate::theme::font_size(
                "section",
            )))
            .color(ink.strong),
    );
    ui.label(
        egui::RichText::new(desc)
            .font(crate::theme::font_regular(crate::theme::font_size("meta")))
            .color(ink.muted),
    );
    ui.add_space(8.0);
    ui.painter().line_segment(
        [
            egui::pos2(ui.cursor().left(), ui.cursor().top()),
            egui::pos2(ui.max_rect().right(), ui.cursor().top()),
        ],
        egui::Stroke::new(1.0, crate::theme::border_color()),
    );
    ui.add_space(10.0);
}

/// 只读行：左标签 14，右值 14 muted（右对齐）
fn row_static(ui: &mut egui::Ui, label: &str, value: &str) {
    let ink = crate::theme::ink();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .font(crate::theme::font_regular(crate::theme::font_size("body")))
                .color(ink.body),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // 图标只作“数据来源”一行的轻提示；版本/字体保持纯文本
            if label == "数据来源" {
                icons::draw(
                    ui.painter(),
                    egui::pos2(ui.cursor().right() - 8.0, ui.cursor().center().y),
                    14.0,
                    icons::Icon::Cloud,
                    ink.meta,
                );
                ui.add_space(22.0);
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new(value)
                        .font(crate::theme::font_regular(crate::theme::font_size("body")))
                        .color(ink.muted),
                )
                .truncate(),
            )
            .on_hover_text(value);
        });
    });
}

// ===== src/pages/settings.rs 说明区 =====
// 输入：UiState（reduce_motion）；输出：reduce_motion 翻转 + 只读信息
// 逻辑步骤：ScrollArea → 外观（只读）→ 动效（自定义 switch）→ 关于（版本/数据源/字体）
// 设计理由（PL017）：原 `egui::Window` title_bar(false) 只有三行、无标题/关闭，像
//   调试浮窗；改为与四页同构的抽屉页（页头由 Pages 统一给），设置成为真正的页面
// 异常处理：无 panic 面；版本经 Static 单源
// 关联依赖：theme/widgets(switch/icons)；消费方：pages::Pages::ui
