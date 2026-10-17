# AccelWorld RS

时间膨胀时钟的 Rust 重构版（egui + eframe/wgpu），接替 Python 0.6.0.0。
重构总纲与任务切分见仓库根 `z.plan.md`（Rust 重构总纲节）与 `x.progress.md`（PL011–PL015）；
项目分析与房规见根 `w.study.md` 与 `AGENTS.md`。

## 结构

cargo workspace 双 crate，依赖单向 `app → core`（CI 机器强制零反向）：

- `crates/accel-core`：纯业务（时间膨胀/农历/天气/闹钟/配置/数据表/工具/接口契约），零 GUI 依赖
- `crates/accel-app`：egui 前端（窗口/玻璃渲染/五页导航 + 设置浮窗/自绘控件/系统集成），一切后端写操作经 core::interface 命令/事件总线（只读 token/DTO 允许直取，见 AGENTS 契约边界）

## 构建与运行

```powershell
cargo run -p accel-app                          # 开发运行（真窗）
cargo build --release                           # 发布构建 → target/release/accel-app.exe
cargo run -p accel-app --bin render_probe       # 玻璃离屏渲染断言探针
```

## 验证

`pwsh ../rs/ci.ps1`（或在仓库根 `pwsh rs/ci.ps1`）——四关卡：fmt / clippy / test / 依赖树零 egui。
单独跑：`cargo test --workspace`；`cargo clippy --workspace --all-targets -- -D warnings`。

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

- 版本单一来源：`rs/config/static/base.json` 的 `version` 字段（app 侧 `build.rs` 编译期注入
  `ACCELWORLD_VERSION`），代码中不得出现版本字符串
- 零硬编码：业务参数全部走 `rs/config/static/*.json` token（`include_str` 嵌入，
  `static_config.rs` 单例访问）
- 夹具 parity：`rs/fixtures/*.json` 为 Python 0.6.0.0 行为快照（golden），
  core 行为断言一律对夹具（`.archived/` 可再生）
- 平台集成降级：托盘/热键/通知构造失败一律降级日志不崩；非 Windows 平台编译期桩化
- 渲染质量门：玻璃观感/管线改动须过 `render_probe` 离屏断言再进人工走查
