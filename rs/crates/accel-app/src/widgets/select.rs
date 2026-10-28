//! 自绘下拉（PL019）：替换 egui 原生 ComboBox——32px 触发器 + 锚定菜单，
//! 与 Field 同边框/圆角；键盘 Space/Enter/↑↓/Esc 可达；菜单 180ms 自触发点展开。

use egui::{Align2, Color32, Id, Rect, Sense, Stroke};

use super::icons::{self, Icon};

#[derive(Clone, Copy, Default)]
struct State {
    open: bool,
    cursor: usize,
}

/// 绘制下拉；返回本次选中的项（点击/回车）。`current` 为显示文本，
/// `options` 为候选项，`width` 为触发器宽度（含边框）。
pub fn show(
    ui: &mut egui::Ui,
    id: Id,
    current: &str,
    options: &[String],
    width: f32,
) -> Option<String> {
    let ink = crate::theme::ink();
    let state_id = id.with("state");
    let mut st = ui
        .data(|d| d.get_temp::<State>(state_id))
        .unwrap_or_default();
    let was_open = st.open;

    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 32.0), Sense::click());
    let hovered = resp.hovered();
    ui.painter().rect_filled(
        rect,
        egui::epaint::CornerRadius::same(8),
        crate::theme::input_bg(),
    );
    ui.painter().rect_stroke(
        rect,
        egui::epaint::CornerRadius::same(8),
        Stroke::new(
            1.0,
            if hovered || st.open {
                crate::theme::border_strong_color()
            } else {
                crate::theme::input_border_color()
            },
        ),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        current,
        crate::theme::font_regular(crate::theme::font_size("body")),
        ink.body,
    );
    icons::draw(
        ui.painter(),
        egui::pos2(rect.right() - 14.0, rect.center().y),
        16.0,
        Icon::ChevronDown,
        if hovered || st.open {
            ink.body
        } else {
            ink.muted
        },
    );
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            egui::epaint::CornerRadius::same(9),
            Stroke::new(1.5, ink.knob),
            egui::StrokeKind::Inside,
        );
    }
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    // 键盘：聚焦时 Space/Enter 开合；展开时 ↑↓ 移动、Enter 选中、Esc 关闭
    let (enter, space, esc, up, down) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Space),
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
        )
    });
    if resp.has_focus() && (enter || space) {
        st.open = !st.open;
        st.cursor = options.iter().position(|o| o == current).unwrap_or(0);
    }
    if st.open {
        if esc {
            st.open = false;
        } else {
            if up && !options.is_empty() {
                st.cursor = st.cursor.saturating_sub(1);
            }
            if down && !options.is_empty() {
                st.cursor = (st.cursor + 1).min(options.len() - 1);
            }
            if enter && !options.is_empty() {
                let chosen = options[st.cursor.min(options.len() - 1)].clone();
                st.open = false;
                ui.data_mut(|d| d.insert_temp(state_id, st));
                return Some(chosen);
            }
        }
    }

    let mut chosen: Option<String> = None;
    let mut menu_rect = Rect::NOTHING;
    if st.open {
        let row_h = 30.0;
        let menu_h = (options.len() as f32 * row_h + 8.0).min(240.0);
        let screen = ui.ctx().input(|i| i.viewport_rect());
        let below = rect.bottom() + 4.0;
        let y = if below + menu_h <= screen.bottom() - 8.0 {
            below
        } else {
            (rect.top() - 4.0 - menu_h).max(screen.top() + 8.0)
        };
        // 展开动效：180ms，位移 4px + 透明度（reduce 直取终态）
        let t = if crate::theme::reduce_motion(ui.ctx()) {
            1.0
        } else {
            ui.ctx().animate_bool_with_time(id.with("anim"), true, 0.18)
        };
        let menu = egui::Area::new(id.with("menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(rect.left(), y + 4.0 * (1.0 - t)))
            .show(ui.ctx(), |ui| {
                ui.set_opacity(t);
                egui::Frame::new()
                    .fill(crate::theme::card_bg())
                    .stroke(Stroke::new(1.0, crate::theme::border_strong_color()))
                    .corner_radius(egui::epaint::CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.set_width((width - 8.0).max(80.0));
                        egui::ScrollArea::vertical()
                            .max_height(232.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (i, opt) in options.iter().enumerate() {
                                    let (row, r) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), row_h),
                                        Sense::click(),
                                    );
                                    let selected = opt == current;
                                    let hot = r.hovered() || (st.open && i == st.cursor);
                                    if hot {
                                        ui.painter().rect_filled(
                                            row,
                                            egui::epaint::CornerRadius::same(6),
                                            crate::theme::hover_bg(),
                                        );
                                    }
                                    ui.painter().text(
                                        egui::pos2(row.left() + 8.0, row.center().y),
                                        Align2::LEFT_CENTER,
                                        opt,
                                        crate::theme::font_regular(crate::theme::font_size("body")),
                                        if selected { ink.strong } else { ink.body },
                                    );
                                    if selected {
                                        icons::draw(
                                            ui.painter(),
                                            egui::pos2(row.right() - 12.0, row.center().y),
                                            14.0,
                                            Icon::Check,
                                            crate::theme::accent_color(),
                                        );
                                    }
                                    if r.clicked() {
                                        chosen = Some(opt.clone());
                                    }
                                }
                            });
                    });
            });
        menu_rect = menu.response.rect;
    }

    if let Some(c) = chosen {
        st.open = false;
        st.cursor = options.iter().position(|o| o == &c).unwrap_or(0);
        ui.data_mut(|d| d.insert_temp(state_id, st));
        return Some(c);
    }
    // 点外关闭（仅在展开前已打开时判定，避免开门这一击立即关闭）
    if was_open && st.open {
        let clicked_out = ui.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !menu_rect.contains(p) && !rect.contains(p))
        });
        if clicked_out {
            st.open = false;
        }
    }
    if st.open {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(16));
    }
    ui.data_mut(|d| d.insert_temp(state_id, st));
    None
}

/// 菜单放置（纯函数，测试用）：返回左上角 y——下方放得下就下方，否则上方，
/// 两端钳进屏幕（8px 边距）
pub fn menu_y(trigger: Rect, screen: Rect, menu_h: f32) -> f32 {
    let below = trigger.bottom() + 4.0;
    if below + menu_h <= screen.bottom() - 8.0 {
        below
    } else {
        (trigger.top() - 4.0 - menu_h).max(screen.top() + 8.0)
    }
}

/// 菜单文字色（选中/未选中）——供调用方保持同一语义
pub fn text_color(selected: bool) -> Color32 {
    if selected {
        crate::theme::ink().strong
    } else {
        crate::theme::ink().body
    }
}

// ===== src/widgets/select.rs 说明区 =====
// 输入：Ui + 稳定 Id + 当前文本 + 选项 + 宽度；输出：本次选中项
// 逻辑步骤：触发器（32px/边框/ChevronDown）→ 点击或 Space/Enter 开 →
//   Area 菜单（下方优先/空间不足上翻、最高 240 滚动、180ms 淡入）→ 点击/回车选中
// 设计理由（PL019）：原生 ComboBox 与自绘 Field 边框/高度/字体不一致；本组件
//   复用 input_bg/input_border/border_strong，使世界/天气与表单同语言
// 异常处理：空选项菜单只显示 8px 内边距；越屏由 menu_y 钳制
// 关联依赖：theme/ink/icons(ChevronDown/Check)；消费方：pages/world、pages/weather

#[cfg(test)]
mod tests {
    use super::menu_y;
    use egui::{Rect, pos2, vec2};

    #[test]
    fn menu_flips_above_near_screen_bottom() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 500.0));
        let trigger = Rect::from_min_size(pos2(100.0, 440.0), vec2(200.0, 32.0));
        let y = menu_y(trigger, screen, 200.0);
        assert!(y + 200.0 <= 500.0);
        assert!(y < trigger.top());
    }

    #[test]
    fn menu_stays_inside_top_edge() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 500.0));
        let trigger = Rect::from_min_size(pos2(100.0, 10.0), vec2(200.0, 32.0));
        let y = menu_y(trigger, screen, 400.0);
        assert!(y >= 8.0);
    }
}
