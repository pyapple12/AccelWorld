//! 按宽截断单行文字（FIX013.31/32）：逐字符试排，超宽补省略号；返回是否截断。
//! 供 weather/countdown/world/alarm 共用，避免各自复制同一算法。

/// 按 max_w 截断并补「…」；返回值第二位表示是否发生截断（hover 全文的门控）
pub fn fit(ui: &egui::Ui, text: &str, font: &egui::FontId, max_w: f32) -> (String, bool) {
    if text.is_empty() {
        return (String::new(), false);
    }
    let width = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE)
            .rect
            .width()
    };
    if width(text) <= max_w {
        return (text.to_owned(), false);
    }
    let ell_w = width("…");
    let mut out = String::new();
    for ch in text.chars() {
        let mut probe = out.clone();
        probe.push(ch);
        if width(&probe) + ell_w > max_w {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    (out, true)
}

// ===== src/widgets/text_fit.rs 说明区 =====
// 输入：Ui、原文本、字体、可用宽；输出：(截断文本, 是否截断)
// 逻辑步骤：整串先量宽 → 未超宽原样返回 → 逐字符试排，留出「…」宽 → 补省略号
// 设计理由：FIX013.31/32 三处长文本缺陷共用一套算法；按字符切保证 CJK UTF-8 边界安全
// 异常处理：空串返回空；max_w ≤ 0 时结果只剩「…」；无 panic 面
// 关联配置：调用方传入各自盒宽；消费方：pages/weather、pages/countdown、pages/world、pages/alarm
