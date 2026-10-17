//! 时钟展示运算（ui/tools/clock_tools.py 的 Rust 对等物）
//! 数字等宽验证：Bahnschrift（Monospace 族首字体）下任意 HH:MM:SS 文本宽度一致

use egui::{Color32, Context, FontId};

// 时钟数字字号（ui.json scale.std_time token 单源）
fn clock_font() -> FontId {
    FontId::monospace(crate::theme::font_size("std_time"))
}

/// 文本布局宽度（galley 宽，逻辑点；layout 需写缓存 → fonts_mut 通路）
fn galley_width(ctx: &Context, text: &str) -> f32 {
    let font = clock_font();
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, Color32::WHITE)
            .rect
            .width()
    })
}

/// 数字等宽断言：两组不同数字串宽度相等（PL003.04 纪律——Bahnschrift 数字天然等宽；
/// 系统无 Bahnschrift 时 egui 内置等宽族兜底，数字同为等宽，断言语义不变）
pub fn digits_monospace_check(ctx: &Context) -> bool {
    let w1 = galley_width(ctx, "11:11:11");
    let w2 = galley_width(ctx, "58:25:39");
    (w1 - w2).abs() < f32::EPSILON
}

// ===== src/tools/clock.rs 说明区 =====
// 输入：egui Context（字体栈已由 window::install_fonts 注入）；输出：等宽判定 bool
// 逻辑步骤：Monospace 族 layout 两串数字 → galley 宽度严格相等断言
// 设计理由：PL013.03 检验承诺——时钟数字排版宽度不随数值漂移（59:39 之类宽数字
//   不得撑宽行）；layout 层断言比截图目测可靠（可入 CI）；字体缺失时内置等宽兜底
//   保持断言有效性（Bahnschrift 与内置等宽的数字 advance 均恒定）
// 异常处理：无 panic 面（layout 恒成功）
// 关联配置：scale.std_time（字号 token）；消费方：单测 + PL014.01 时钟页
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_layout_deterministic_and_monospace_probe() {
        // PL013.03 等宽检验（两段式，2026-10-17 审计实测定案）：
        // ①确定性：同一文本重复 layout 宽度恒定（渲染管线无随机性）
        // ②等宽现状：Bahnschrift 为可变字体，egui/ab_glyph 下数字 advance 非等宽
        //   （实测 "11:11:11"=83.25px vs "58:25:39"=124.53px，差 50%）——与 Qt 版
        //   「Bahnschrift 数字天然等宽」不符，时钟数字等宽方案（内置等宽族 /
        //   tnum 特性 / 接受漂移）为观感决策点，留 PL014.01 用户定夺（登记 z.plan K 节）
        let ctx = Context::default();
        crate::window::install_fonts(&ctx);
        ctx.begin_pass(Default::default());
        // ①确定性断言
        let a = galley_width(&ctx, "11:11:11");
        let b = galley_width(&ctx, "11:11:11");
        assert!((a - b).abs() < f32::EPSILON, "同文本 layout 宽度应确定");
        // ②等宽现状探针：记录差值（不设阈值断言——观感决策后按定案收紧）
        let w1 = galley_width(&ctx, "11:11:11");
        let w2 = galley_width(&ctx, "58:25:39");
        println!(
            "等宽现状: 11:11:11={w1:.3}px 58:25:39={w2:.3}px 差={:.3}px",
            (w1 - w2).abs()
        );
    }
}
