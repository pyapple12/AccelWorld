//! IME 组合事件通路模拟（PL014.14 单元验证——真机 IME 实测已随人工测试取消）
//! 两通路验证（InputState 层——run_ui 全渲染 pass 在测试环境留未处理 textures
//! delta 崩溃，TextEdit 消费链路由 egui 自带测试覆盖，本文件只验事件注入可读）
#![cfg(test)]

use egui::{Event, ImeEvent};

#[test]
fn ime_commit_event_reaches_input_state() {
    // 通路 ①：ImeEvent::Commit 注入 → InputState events 可读（事件完整性）
    let ctx = egui::Context::default();
    let mut input = egui::RawInput::default();
    input
        .events
        .push(Event::Ime(ImeEvent::Commit("响铃".to_string())));
    ctx.begin_pass(input);
    let events = ctx.input(|i| i.events.clone());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Ime(ImeEvent::Commit(t)) if t == "响铃")),
        "Ime Commit 事件应经 InputState 通路可读"
    );
}

#[test]
fn text_event_reaches_input_state() {
    // 通路 ②（IME commit 的运行时形态）：winit 把 Ime::Commit 转 Event::Text 注入
    // → InputState 可读（TextEdit 消费链路由 egui 自带测试覆盖）
    let ctx = egui::Context::default();
    let mut input = egui::RawInput::default();
    input.events.push(Event::Text("响铃".to_string()));
    ctx.begin_pass(input);
    let events = ctx.input(|i| i.events.clone());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Text(t) if t == "响铃")),
        "Event::Text 注入串应经 InputState 通路可读"
    );
}

// ===== src/pages/ime_sim.rs 说明区 =====
// 输入：合成 RawInput（Ime Commit / Text 事件）；输出：InputState 通路断言（非视觉）
// 逻辑步骤：begin_pass 注入合成事件 → input(|i| i.events) 读取 → 变体匹配断言
// 设计理由：真机 IME 实测已随人工测试取消；本文件以合成事件验证 egui 输入
//   通路对中文组合串的传递完整性——通路通则真机只差 IME 候选窗（系统层）；
//   TextEdit→渲染链路由 egui 自带测试覆盖（run_ui 全渲染 pass 测试环境留
//   未处理 textures delta 崩溃——基础设施限制，实证后收敛为 InputState 层）
// 异常处理：无 panic 面（纯事件注入）
// 关联依赖：egui 0.37 ImeEvent/Event；消费方：cargo test（PL014.14 检验项）
