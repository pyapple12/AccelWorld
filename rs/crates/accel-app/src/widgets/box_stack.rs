//! 抽屉页盒布局（PL021）：显式 y 游标 + 盒间距 token `layout.drawer_box_gap`。
//! 盒 = 区域矩形；盒内 padding 由调用方给，不再用行框/add_space 串联。

/// 抽屉页统一盒间距（layout.drawer_box_gap 单源）
pub fn gap() -> f32 {
    crate::theme::layout_f32("drawer_box_gap")
}

/// 垂直盒堆叠器：左/右边界固定，y 游标逐盒推进
pub struct BoxStack {
    left: f32,
    right: f32,
    y: f32,
    gap: f32,
}

impl BoxStack {
    /// 从 Ui 当前可用区顶部开始
    pub fn new(ui: &egui::Ui) -> Self {
        Self::with_gap(ui, gap())
    }

    /// 自定义盒间距
    pub fn with_gap(ui: &egui::Ui, gap: f32) -> Self {
        let r = ui.available_rect_before_wrap();
        Self {
            left: r.left(),
            right: r.right(),
            y: r.top(),
            gap,
        }
    }

    /// 分配下一个盒并登记到布局（ScrollArea 据此计算内容高度）
    pub fn next(&mut self, ui: &mut egui::Ui, h: f32) -> egui::Rect {
        let rect = self.slot(h);
        ui.allocate_rect(rect, egui::Sense::hover());
        rect
    }

    /// 只计算下一个盒 rect 并推进游标，不登记布局（供 scope_builder 自行登记）
    pub fn slot(&mut self, h: f32) -> egui::Rect {
        let rect = egui::Rect::from_min_size(
            egui::pos2(self.left, self.y),
            egui::vec2((self.right - self.left).max(0.0), h.max(0.0)),
        );
        self.y = rect.bottom() + self.gap;
        rect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawer_gap_reads_token() {
        // FIX013.35：抽屉盒间距只认 layout.drawer_box_gap
        assert_eq!(gap(), crate::theme::layout_f32("drawer_box_gap"));
    }
}

// ===== src/widgets/box_stack.rs 说明区 =====
// 输入：Ui 可用矩形 + 盒高；输出：下一个盒 rect，游标 bottom + drawer_box_gap
// 逻辑步骤：new 读 token 取间距 → slot 计算矩形并推进 → next 额外登记 allocate_rect
// 设计理由（PL021）：抽屉页统一盒模型——盒间距只由 rect 计算，盒内 padding 固定，
//   不再用 add_space/行框拼接；ScrollArea 用 allocate_rect 得到真实内容高度
// 异常处理：负宽/负高钳 0，无 panic 面
// 关联依赖：egui Rect/Ui；消费方：pages/countdown/world/weather/alarm/settings
