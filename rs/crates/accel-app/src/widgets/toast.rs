//! 应用内 toast 堆叠（PL016.07，审核 #8）：右上角滑入卡片——入场 0.3s
//! ease-out 平移、驻留、末段 0.15s 滑出；错误红缘/信息金缘；主题 token 全源
//! 替代 egui-notify（其维护线未跟进 egui 0.37——手写零依赖，动画与
//! 零忙转纪律自持）

use crate::bus::{Toast, ToastKind};

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

    let width = 260.0;
    let margin = 16.0;
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
        let edge = match toast.kind {
            ToastKind::Error => ink.danger,
            ToastKind::Info => crate::theme::accent_color(),
        };
        egui::Area::new(egui::Id::unique((
            "accel_toast",
            i,
            toast.born.elapsed().as_nanos() as u64,
        )))
        .anchor(
            egui::Align2::RIGHT_TOP,
            [-(margin + slide), margin + i as f32 * 80.0],
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
                    ui.set_max_width(width - 24.0);
                    // 左缘色条（状态不只靠颜色：色条 + 标题 + 正文三层冗余）
                    let inner = ui.max_rect();
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(inner.left() - 12.0, inner.top() - 12.0),
                            egui::vec2(3.0, inner.height() + 24.0),
                        ),
                        egui::epaint::CornerRadius::same(0),
                        edge,
                    );
                    ui.label(
                        egui::RichText::new(&toast.title)
                            .size(crate::theme::font_size("body"))
                            .color(ink.strong),
                    );
                    ui.label(
                        egui::RichText::new(&toast.body)
                            .size(crate::theme::font_size("meta"))
                            .color(ink.muted),
                    );
                });
        });
    }
}

// ===== src/widgets/toast.rs 说明区 =====
// 输入：UiState.toasts（drain 入队/修剪）；输出：右上角堆叠卡片绘制 + 提频重绘
// 逻辑步骤：修剪过期（bus drain 已做，此处兜底）→ 提频决策（动画 16ms/驻留
//   0.25s）→ 逐卡 Area 定位绘制（入场/出场滑移按 born 相位推导，无逐卡状态）
// 设计理由（PL016.07）：审核建议 egui-notify——其发布线未覆盖 egui 0.37，
//   手写零依赖且动画参数/主题全走 ui.json motion 段（自持纪律）；出场选滑出
//   而非淡出（Frame 无 alpha 管线，滑出同样成立且实现干净）
// 异常处理：空队列零绘制零提频；reduce_motion 直出直隐（PL016.06 开关联动）
// 关联配置：motion.toast_{ttl,in,out}_ms；消费方：window.rs（每帧调用）
