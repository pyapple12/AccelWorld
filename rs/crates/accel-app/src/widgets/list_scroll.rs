//! 抽屉列表滚动套件（PL028）：渐隐 + 金色三角 + 右侧金色浮钮 + 拖拽回写。
//! 倒计时与世界时钟共用；`scroll_kit` 只保留纯视觉原语。

/// 列表滚动状态（每页各一份；跨帧保留）
#[derive(Default)]
pub struct ListScrollState {
    /// 上一帧滚动偏移（判断是否在滚动；None = 还没采样过）
    pub last_offset: Option<f32>,
    /// 三角呼吸截止时刻（秒；滚动时向后刷新，停手后回到常亮）
    pub blink_until: f64,
    /// 浮钮拖拽时，下一帧要写回的滚动偏移
    pub override_offset: Option<f32>,
}

/// 显示一个带项目滚动视觉的列表；`add_contents` 只负责列表内容。
pub fn show(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id_salt: &str,
    probe_tag: &str,
    state: &mut ListScrollState,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        crate::widgets::scroll_kit::apply_style(ui);
        let mut area = egui::ScrollArea::vertical()
            .id_salt(id_salt)
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        if let Some(y) = state.override_offset.take() {
            area = area.vertical_scroll_offset(y);
        }
        let out = area.show(ui, |ui| {
            add_contents(ui);
            // 列表内容尾部留白：滚到底时最后一张卡不贴裁切线（边框完整）
            // 必须真实分配空间，add_space 只推游标，不进 ScrollArea 内容高度
            let pad = crate::theme::layout_f32("scroll_content_pad_bottom");
            let bottom = ui.min_rect().bottom();
            let spacer = egui::Rect::from_min_size(
                egui::pos2(ui.min_rect().left(), bottom),
                egui::vec2(ui.available_width().max(1.0), pad),
            );
            ui.allocate_rect(spacer, egui::Sense::hover());
        });
        state.override_offset = draw_scroll(
            ui,
            &out,
            rect,
            id_salt,
            probe_tag,
            &mut state.last_offset,
            &mut state.blink_until,
        );
    });
}

/// 列表滚动视觉件：上下 16px 渐隐、上下金色三角（滚动期呼吸、静止常亮）、
/// 卡片右缘 +8px 的 4px 金色浮钮；返回拖拽浮钮时下一帧要写回的滚动偏移。
fn draw_scroll(
    ui: &egui::Ui,
    out: &egui::scroll_area::ScrollAreaOutput<()>,
    list_rect: egui::Rect,
    id_salt: &str,
    probe_tag: &str,
    scroll_last_offset: &mut Option<f32>,
    scroll_blink_until: &mut f64,
) -> Option<f32> {
    let viewport = out.inner_rect;
    let eps = crate::theme::layout_f64("list_visible_eps");
    if viewport.height() <= eps {
        return None;
    }
    let offset = out.state.clamped_offset().y;
    let max_offset = (out.content_size.y - viewport.height()).max(0.0);
    let scrollable = max_offset > 1.0;
    if crate::layout_probe::layout_on() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static FRAME: AtomicU64 = AtomicU64::new(0);
        if FRAME.fetch_add(1, Ordering::Relaxed).is_multiple_of(30) {
            eprintln!(
                "[{}] viewport=({:.1},{:.1},{:.1},{:.1}) content=({:.1},{:.1}) scrollable={} offset={:.1} max={:.1}",
                probe_tag,
                viewport.left(),
                viewport.top(),
                viewport.width(),
                viewport.height(),
                out.content_size.x,
                out.content_size.y,
                scrollable,
                offset,
                max_offset
            );
        }
    }
    let at_top = offset <= eps;
    let at_bottom = offset >= max_offset - eps;

    // 滚动判定：偏移变化即刷新呼吸截止；停手后回到常亮
    let now = ui.input(|i| i.time);
    let blink_s = crate::theme::motion_ms("scroll_hint_blink_ms") as f64 / 1000.0;
    if let Some(prev) = *scroll_last_offset
        && (offset - prev).abs() > 0.1
    {
        *scroll_blink_until = now + blink_s;
    }
    *scroll_last_offset = Some(offset);
    let reduce = crate::theme::reduce_motion(ui.ctx());
    let blinking = scrollable && !reduce && now < *scroll_blink_until;
    if blinking {
        ui.ctx().request_repaint();
    }
    let base_alpha = crate::theme::layout_f32("scroll_hint_alpha_x100") / 100.0;
    // 1s 呼吸：1 → 0.35 → 1；静止时保持常亮，不闪
    let hint_alpha = if blinking {
        let phase = (now * std::f64::consts::TAU).cos() as f32;
        base_alpha * (0.675 + 0.325 * phase)
    } else {
        base_alpha
    };

    let fade = crate::theme::layout_f32("scroll_fade");
    let top_t = fade_t(ui, &format!("{id_salt}_fade_top"), scrollable && !at_top);
    let bottom_t = fade_t(
        ui,
        &format!("{id_salt}_fade_bottom"),
        scrollable && !at_bottom,
    );
    crate::widgets::scroll_kit::paint_fade(ui, viewport, fade * top_t, fade * bottom_t);
    crate::widgets::scroll_kit::paint_hints(
        ui,
        viewport,
        scrollable && !at_top,
        scrollable && !at_bottom,
        hint_alpha,
    );

    // 金色浮钮：卡片右缘 +8px（右边距 8/4/8 居中），4px 宽；拖拽命中域左右各放宽 6px
    if !scrollable {
        return None;
    }
    let gap = crate::theme::layout_f32("scroll_bar_gap");
    let width = crate::theme::layout_f32("scroll_bar_w");
    let track_h = viewport.height();
    let thumb_h = (track_h * (viewport.height() / out.content_size.y.max(1.0)))
        .max(crate::theme::layout_f32("scroll_handle_min"))
        .min(track_h);
    let travel = (track_h - thumb_h).max(0.0);
    let range = max_offset.max(1.0);
    let ratio = (offset / range).clamp(0.0, 1.0);
    let x = list_rect.right() + gap;
    let hit = egui::Rect::from_min_max(
        egui::pos2(x - 6.0, viewport.top()),
        egui::pos2(x + width + 6.0, viewport.bottom()),
    );
    let resp = ui
        .interact(
            hit,
            egui::Id::unique(format!("{id_salt}_thumb")),
            egui::Sense::drag(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let mut thumb_ratio = ratio;
    let mut next_override = None;
    if resp.dragged() && travel > 0.0 {
        let dy = ui.input(|i| i.pointer.delta().y);
        let new_offset = (offset + dy / travel * max_offset).clamp(0.0, max_offset);
        thumb_ratio = (new_offset / range).clamp(0.0, 1.0);
        next_override = Some(new_offset);
    }
    let y = viewport.top() + thumb_ratio * travel;
    let radius = ((width / 2.0).max(1.0)) as u8;
    ui.painter().rect_filled(
        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, thumb_h)),
        egui::epaint::CornerRadius::same(radius),
        crate::theme::accent_color(),
    );
    next_override
}

/// 渐隐显隐动画：出现走 scroll_fade_ms，收走边走 scroll_edge_ms；减少动效直接落位。
fn fade_t(ui: &egui::Ui, id: &str, visible: bool) -> f32 {
    let target = if visible { 1.0 } else { 0.0 };
    if crate::theme::reduce_motion(ui.ctx()) {
        return target;
    }
    let ms = if visible {
        crate::theme::motion_ms("scroll_fade_ms")
    } else {
        crate::theme::motion_ms("scroll_edge_ms")
    };
    ui.ctx()
        .animate_value_with_time(egui::Id::unique(id), target, ms / 1000.0)
}

// ===== src/widgets/list_scroll.rs 说明区 =====
// 输入：Ui + 列表视口矩形 + id_salt/probe_tag + ListScrollState + 内容闭包
// 逻辑步骤：apply_style 关自带渐隐 → ScrollArea（隐藏原生滑杆）→ 内容 →
//   渐隐/三角/浮钮绘制 → 浮钮拖拽回写 override_offset
// 设计理由（PL028）：倒计时已有完整滚动视觉；世界时钟需要同一套，抽成组件避免
//   复制 ~150 行。倒计时沿用原 id_salt 保持滚动位置；探针标签由调用方给。
// 异常处理：视口高 ≤0.5 跳过；不可滚动时不画浮钮；无 panic 面
// 关联配置：scroll_* / motion.scroll_* tokens；消费方：countdown/world
