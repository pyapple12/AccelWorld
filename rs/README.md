# AccelWorld RS

时间膨胀时钟的 Rust 重构版（egui + eframe/wgpu），接替 Python 0.6.0.0。
重构总纲与任务切分见仓库根 `z.plan.md`（Rust 重构总纲节）与 `x.progress.md`（PL011–PL015）。

## 结构

cargo workspace 双 crate，依赖单向 `app → core`（CI 机器强制零反向）：

- `crates/accel-core`：纯业务（时间膨胀/农历/天气/闹钟/配置/数据表），零 GUI 依赖
- `crates/accel-app`：egui 前端（窗口/玻璃渲染/六页导航/系统集成），一切后端访问经 core::interface 命令/事件总线

验证：`pwsh rs/ci.ps1`（fmt / clippy / test / 依赖树四关卡）。

## 注释房规（Python 版 `#` 注释体系的 Rust 对等物）

- 每个函数/方法定义处用 `///` 中文文档注释，说明用途与核心逻辑（1-3 行）
- 模块头 `//!` 一行职责说明
- 每个 `.rs` 文件末尾保留 `// ===== 说明区 =====`，涵盖：输入输出、逻辑步骤、
  设计理由、异常处理（Result/panic 策略）、关联配置或外部依赖
- 注释一律中文；占位模块写明落位任务编号（如「PL013 落地」）

样例：

```rust
/// 取加速时间信息快照
/// 输入：注入时刻 now（测试可参数化）；输出：TimeInfo（钟面/日长/tick/剩余）
pub fn time_info(&self, now: DateTime<Local>) -> TimeInfo {
    ...
}

// ===== src/time_dilation.rs 说明区 =====
// ...
```

## 其他房规

- 版本单一来源：`rs/config/static/base.json` 的 `version` 字段（app 侧 build.rs 注入），
  代码中不得出现版本字符串
- 零硬编码：业务参数全部走 `rs/config/static/*.json` token（include_str 嵌入，
  static_config.rs 单例访问）
- 夹具 parity：`rs/fixtures/*.json` 为 Python 0.6.0.0 行为快照（golden），
  core 行为断言一律对夹具（`.archived/` 可再生）
