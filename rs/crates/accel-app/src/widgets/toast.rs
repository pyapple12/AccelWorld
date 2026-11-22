//! 应用内 toast 堆叠（PL016.07，审核 #8）：右上角滑入卡片——入场 0.3s
//! ease-out 平移、驻留、末段 0.15s 滑出；错误红缘；主题 token 全源
//! PL017.19：<600 宽窗口用全宽 −32px，宽窗 260px；右上 × 可关闭
//! 替代 egui-notify（其维护线未跟进 egui 0.37——手写零依赖，动画与
//! 零忙转纪律自持）

use crate::bus::Toast;
use crate::widgets::icons::{self, Icon};
use crate::widgets::text_ink::{self, InkKind};

/// toast 卡片宽（纯函数）：<600 视口全宽 −32，宽窗 260；下限 180
pub fn toast_width(viewport_w: f32) -> f32 {
    if viewport_w < 600.0 {
        (viewport_w - 32.0).max(180.0)
    } else {
        260.0
    }
}

/// 绘制 toast 堆栈（每帧调用）；存活期自提频重绘（动画 16ms/驻留 0.25s）
pub fn paint(ctx: &egui::Context, toasts: &mut Vec<Toast>) {
    let ink = crate::theme::ink();
    let ttl = crate::bus::toast_ttl();
    let in_s = (crate::theme::motion_ms("toast_in_ms") / 1000.0).max(0.01);
    let out_s = (crate::theme::motion_ms("toast_out_ms") / 1000.0).max(0.01);
    let reduce = crate::theme::reduce_motion(ctx);
    toasts.retain(|t| t.born.elapsed().as_secs_f32() < ttl.as_secs_f32());
    if toasts.is_empty() {
        return;
    }

    // 提频决策：任一卡在入/出场相位 → 16ms；全部静驻 → 距最近到期 0.25s
    let mut animating = false;
    let mut youngest_alive = f32::INFINITY;
    for t in toasts.iter() {
        let age = t.born.elapsed().as_secs_f32();
        let remain = ttl.as_secs_f32() - age;
        youngest_alive = youngest_alive.min(remain);
        if age < in_s || remain < out_s {
            animating = true;
        }
    }
    let wake = if animating {
        0.016
    } else {
        youngest_alive.min(0.25)
    };
    ctx.request_repaint_after(std::time::Duration::from_secs_f32(wake));

    let width = toast_width(ctx.input(|i| i.viewport_rect().width()));
    let margin = 16.0;
    // FIX011.02：堆叠步距 token 单源（原 80.0 内联魔法数；88 预留单行卡间距）
    let step = crate::theme::layout_i64("toast_step") as f32;
    let mut dismiss: Option<usize> = None;
    for (i, toast) in toasts.iter().enumerate() {
        let age = toast.born.elapsed().as_secs_f32();
        let remain = ttl.as_secs_f32() - age;
        // 入场右缘外 24px 滑入；末段 0.15s 原路滑出（相位推导，无逐卡状态）
        let slide = if reduce {
            0.0
        } else if age < in_s {
            (1.0 - crate::tools::motion::ease_out_cubic(age / in_s)) * 24.0
        } else if remain < out_s {
            (1.0 - remain / out_s) * 24.0
        } else {
            0.0
        };
        let edge = ink.danger;
        let mut close = false;
        // FIX011.08：Id 锚出生时刻（单条生命周期内稳定——原 elapsed().as_nanos()
        // 每帧变化，依赖 egui Memory GC 兜底的反模式；Instant 实现 Hash 可直作键）
        egui::Area::new(egui::Id::unique(("accel_toast", i, toast.born)))
            .anchor(
                egui::Align2::RIGHT_TOP,
                [-(margin + slide), margin + i as f32 * step],
            )
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(crate::theme::card_bg())
                    .corner_radius(egui::epaint::CornerRadius::same(
                        crate::theme::radius_px("md") as u8,
                    ))
                    .stroke(egui::Stroke::new(1.0, crate::theme::border_color()))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        let content_w = width - 24.0;
                        let title_font = crate::theme::font_medium(crate::theme::font_size("body"));
                        let body_font = crate::theme::font_regular(crate::theme::font_size("meta"));
                        let title_h = crate::widgets::text_metrics::height(
                            ui.ctx(),
                            &title_font,
                            InkKind::Cjk,
                        );
                        let body_galley = ui.painter().layout(
                            toast.body.clone(),
                            body_font.clone(),
                            ink.muted,
                            content_w,
                        );
                        let body_rows = body_galley.rows.len().max(1);
                        let body_h = (body_rows as f32
                            * crate::font_profile::get().for_size(body_font.size).h)
                            .max(body_galley.size().y);
                        let gap = 5.0;
                        let content_h = title_h + gap + body_h;
                        let (content, _) = ui.allocate_exact_size(
                            egui::vec2(content_w, content_h),
                            egui::Sense::hover(),
                        );
                        // 左缘色条（状态不只靠颜色：色条 + 标题 + 正文三层冗余）
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(content.left() - 12.0, content.top() - 12.0),
                                egui::vec2(3.0, content.height() + 24.0),
                            ),
                            egui::epaint::CornerRadius::same(0),
                            edge,
                        );
                        // 标题盒：右侧留 20px 关闭键；文字盒底对齐
                        let title_box = egui::Rect::from_min_size(
                            content.min,
                            egui::vec2((content_w - 24.0).max(1.0), title_h),
                        );
                        let title_ink =
                            text_ink::measure(ui, &toast.title, &title_font, InkKind::Cjk);
                        ui.painter().with_clip_rect(title_box).text(
                            egui::pos2(
                                title_box.left(),
                                title_ink.top_for_bottom(title_box.bottom()),
                            ),
                            egui::Align2::LEFT_TOP,
                            &toast.title,
                            title_font,
                            ink.strong,
                        );
                        // 关闭：图标进 20×20 盒，与页头同一语言
                        let close_rect = egui::Rect::from_min_size(
                            egui::pos2(content.right() - 20.0, content.top()),
                            egui::vec2(20.0, 20.0),
                        );
                        let close_resp = ui.interact(
                            close_rect,
                            egui::Id::unique(("toast_close", i, toast.born)),
                            egui::Sense::click(),
                        );
                        icons::draw(
                            ui.painter(),
                            close_rect.center(),
                            14.0,
                            Icon::Close,
                            if close_resp.hovered() {
                                ink.strong
                            } else {
                                ink.meta
                            },
                        );
                        let keyboard = close_resp.has_focus()
                            && ui.input(|i| {
                                i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)
                            });
                        if close_resp.clicked() || keyboard {
                            close = true;
                        }
                        // 正文盒：盒底对齐；长文本按盒裁剪，不越过卡片
                        let body_box = egui::Rect::from_min_size(
                            egui::pos2(content.left(), content.top() + title_h + gap),
                            egui::vec2(content_w, body_h),
                        );
                        ui.painter()
                            .galley(body_box.left_top(), body_galley, ink.muted);
                    });
            });
        if close {
            dismiss = Some(i);
        }
    }
    if let Some(i) = dismiss {
        toasts.remove(i);
    }
}

// ===== src/widgets/toast.rs 说明区 =====
// 输入：UiState.toasts（drain 入队/修剪）；输出：右上角堆叠卡片绘制 + 提频重绘
// 逻辑步骤：修剪过期（bus drain 已做，此处兜底）→ 提频决策（动画 16ms/驻留
//   0.25s）→ 逐卡 Area 定位绘制（入场/出场滑移按 born 相位推导，无逐卡状态）→
//   标题盒 14.5 + 5 + 正文盒（行数×12.5，不低于 galley 高）；关闭为 20×20 盒内 Tabler Close 图标
// 设计理由（PL016.07）：审核建议 egui-notify——其发布线未覆盖 egui 0.37，
//   手写零依赖且动画参数/主题全走 ui.json motion 段（自持纪律）；出场选滑出
//   而非淡出（Frame 无 alpha 管线，滑出同样成立且实现干净）
// 异常处理：空队列零绘制零提频；reduce_motion 直出直隐（PL016.06 开关联动）
// 关联配置：motion.toast_{ttl,in,out}_ms + layout.toast_step（FIX012.04 补记）；消费方：window.rs（每帧调用）
#[cfg(test)]
mod tests {
    use super::toast_width;

    #[test]
    fn toast_width_follows_viewport() {
        assert_eq!(toast_width(900.0), 260.0);
        assert_eq!(toast_width(520.0), 488.0);
        assert_eq!(toast_width(320.0), 288.0);
    }
}
