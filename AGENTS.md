# AccelWorld 项目说明

Rust 桌面应用：基于加速倍率的时间膨胀时钟（egui + eframe/glow 实心桌面 UI，中文界面为主）。2026-10-16 起由 Python（PyQt6 + Fluent Widgets）重构为 Rust，Python 版全量归档 `.archived/`（git 忽略，仅供 golden 夹具再生）。2026-10-23 UI 推倒重塑（UIR）：玻璃管线/Acrylic/透明窗口整体废弃，实心主题 + 侧栏导航。2026-09-10 起接入 DeepTransHub 工作流体系（文档四件套 + Commit 提交规范 + `.agents/skills/` 项目级技能）。

## 运行与验证

- 全部命令在 `rs/` 目录（cargo workspace 根）下执行；工具链见 `rs/rust-toolchain.toml`（channel = stable）
- **统一入口四关卡**：`pwsh rs/ci.ps1`（fmt --check → clippy `-D warnings` → `cargo test --workspace` → core 依赖树零 egui/eframe）；`.github/workflows/rs-ci.yml` 复用同一命令集
- 全量测试：`cargo test --workspace`（core 单测 + 夹具 parity 集成 + app 单测）
- GUI 冒烟（真窗出血，会弹窗）：`cargo run -p accel-app`；UI 截图探针（逐页 PNG 后自动退出）：`$env:ACCELWORLD_UI_SHOT="<目录>"; cargo run -p accel-app`
- **版本单一来源**：`rs/config/static/base.json` 的 `version` 字段（当前 `0.6.1.2`）；app 侧 `build.rs` 编译期注入 `ACCELWORLD_VERSION`，窗口标题/关于页/托盘 tooltip 一律经运行期 `Static::get().version()` 读取，代码中不得出现版本字符串
- **版本体系**：四段式纯数字 `X.Y.Z.P`（无 `ver ` 前缀）；`feat`/`fix`/`refactor`/`perf` 提交前 bump patch 位；历史存量 `ver 0.4x` 三段式仅存于历史文档与提交记录，不回溯改写

## 环境陷阱

- 工作区为 Windows + MSVC 工具链（`stable-x86_64-pc-windows-msvc`）；首次配置需 VS Build Tools 的 `cl`/`link`
- `.venv` 与 `config/user_config.json`（Python 版）已随 Python 归档，不再作为验证通道；现役用户配置与日志均在 **exe 同目录**（FIX007.15 基准统一：配置 `config/user_config.json`、日志 `logs/`，基准取 `core::util::paths::exe_dir`——`cargo run` 下落 `rs/target/debug/`，打包产物即 exe 旁自包含）
- `rs/target/` 为构建缓存（体积可达数十 GB），可随时 `cargo clean` 清除；勿将其纳入版本控制
- pwsh 会话带 `-NoProfile`；输出中文前先 `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;`；搜索统一 `rg`

## 结构与约定

- cargo workspace 双 crate，依赖单向 `app → core`（CI 关卡 4 机器强制 core 零 egui/eframe）：
  - `rs/crates/accel-core`：纯业务（时间膨胀 / 农历干支节气 / 天气 / 闹钟 / 配置 / 数据表 / 通用工具 / 接口契约），零 GUI 依赖
  - `rs/crates/accel-app`：egui 前端（实心主题窗口装配 / 侧栏导航五页 + 设置浮窗 / 自绘控件 / 系统集成）
- **分层与镜像**（Python → Rust，完整对应见 z.plan 镜像映射表）：`interface.rs` 七域契约 + `FrontendCommand`/`BackendEvent` 命令事件总线是 UI **写操作**后端的唯一入口（UI 不直改业务状态）；app 经 `bus.rs` 每帧 drain 事件、按需发命令，core 不感知 egui。**契约边界（A005 #12 明确）**：只读访问允许直取——`static_config::Static`（渲染 token）、`types` 与纯常量 DTO（`alarm::PresetSound`、`settings::CountdownTarget`）、`data::{cities,world_cities}` 只读检索、`audio`（平台侧播放封装）；判定口径：**改变后端状态**才必须走总线，纯读取再经 interface 转一层属无价值间接
- **代码零硬编码**：业务参数（倍率范围/默认值/周期/窗口几何/颜色/日志路径等）全部从 `rs/config/static/*.json` 读取（`static_config.rs` 单例经 `include_str!` 编译期嵌入）；运行期文件一律 exe 同目录相对（FIX007.15）：用户配置 `config/user_config.json`、日志 `logs/`（每日独立文件）
- **注释房规**：`///` 中文文档注释（函数/方法，1-3 行，说明用途与核心逻辑）；模块头 `//!` 一行职责；每个 `.rs` 文件末尾保留 `// ===== 说明区 =====`（输入输出 / 逻辑步骤 / 设计理由 / 异常处理 / 关联配置）
- **视觉走查门**：UI 观感改动经 `ACCELWORLD_UI_SHOT` 截图探针（真窗逐页 PNG）自检再进用户桌面走查——功能探针抓不住纯视觉缺陷（UIR 前的玻璃时代为 render_probe 离屏断言，管线已废）
- **平台集成降级纪律**：托盘 / 全局热键 / 通知等系统能力构造失败一律降级日志不崩（行为无损），非 Windows 平台编译期桩化
- **视觉设计不得向测试环境妥协**：offscreen / 无头探针仅作功能与结构参考；视觉与材质的最终实测以用户桌面走查为准
- 提交信息规范见下文「Commit 提交规范」节；功能开发先走 OpenSpec 提案流程
- 工作流文档：`w.study.md` 项目分析 / `x.progress.md` 任务清单（PL011–PL015 + UIR，已完成在前、未完成在后）/ `y.problems.md` 已知问题 / `z.plan.md` 方案记录与审计附录（豁免定案清单 `A{NNN}` 递增）/ `m.milestone.md` 版本里程碑 / `t.hint.md` 经验复盘
- 项目级 Agent 技能在 `.agents/skills/`（audit-project / audit-report / progress-task / skillforge，gitignore 仅本地使用）
- 历史规划文档 `workingboard/` 已归档 `archived/workingboard`（gitignore）

## 工程原则（设计哲学，所有项目通用）

### 核心思想

- 以第一性原理思考问题：理解需求背后的真实目标，而非直接套用已有模式或技术方案。
- 优先解决本质问题，避免为假设中的未来需求提前设计复杂系统。
- 在保证长期可维护性的前提下，选择当前最简单、可靠、清晰的实现方案。

### 简洁与设计

- 遵循 KISS：优先选择简单直接的实现，避免不必要的复杂度。
- 遵循 DRY：避免重复逻辑，但不要为了消除少量重复而创建过度抽象。
- 遵循 SOLID 思想：职责清晰、降低模块耦合，提高可维护性和扩展能力。

### 架构

- 不长期保留废弃方案：优先删除过时代码，而不是增加兼容层、fallback 或临时迁移逻辑。
- 不进行未经验证的架构设计：避免提前引入抽象、配置和间接层。
- 从最小可工作的版本开始逐步演进，每次修改建立在已有可运行系统之上。
- 永远不要用未来可能需要的复杂性，牺牲当前产品的可用性。

### 代码质量

- 保持模块职责明确，避免一个模块承担过多职责。
- 优先使用成熟、稳定、维护良好的第三方库，而不是重复造轮子。
- 使用项目已有依赖解决问题之前，不要随意新增依赖。
- 在引入新方案前，先检查已有代码、依赖、文档和能力。
- 避免为了"看起来更优雅"而增加实际复杂度。

### 工程决策

- 优先选择长期可维护的方案，而不是只能临时运行的解决方案。
- 代码应该服务于业务目标，而不是为了展示技术复杂度。
- 如果简单方案已经满足需求，不要主动升级为复杂方案。

## 代码规范

### 注释规则

- 注释一律中文，使用 Rust 原生注释：`//` 行注释、`///` 文档注释（函数/方法/字段）、`//!` 模块头
- 每个函数/方法定义处用 `///` 说明用途与核心逻辑（1-3 行）；模块头 `//!` 一行职责
- 每个 `.rs` 文件末尾必须有 `// ===== 说明区 =====`，涵盖文件中所有函数/模块级常量：输入输出、逻辑步骤、设计理由（为什么这样做）、异常处理（Result/panic 策略）、关联的配置或外部依赖
- 禁止用大段注释替代可读代码；注释解释"为什么"，不复述"是什么"

### 代码约定

- 命名：函数/变量/字段用 `snake_case`，类型/trait/枚举与变体用 `CamelCase`，常量/静态用 `UPPER_SNAKE_CASE`
- 可见性：模块内部私有项不加 `pub`（等价于 Python `_` 前缀），外部模块不应直接调用
- 错误处理：库层返回 `Result<T, E>`（错误类型用 `thiserror`）；产品路径避免 `unwrap`/`expect`，构建期/初始化期允许 panic 快速失败（附中文文案）
- 类型：公开 API 显式标注签名；用 `Option`/`Result` 表达空值与失败，不用哨兵值
- import 顺序：标准库 → 第三方 crate → 本 crate/workspace crate，每组之间空行
- 格式化：`cargo fmt`（`rs/rustfmt.toml`：4 空格缩进、行宽 100）；`cargo clippy --workspace --all-targets -- -D warnings` 零警告
- 字符串：字符串格式化优先用 `format!`/`write!`，避免 `+` 拼接
- 集合构建：优先 iterator 与 `collect` 而非手写可变循环
- 布尔/空值判断：用 `if x` / `if !x` / `if let Some(v)` / `x.is_none()`，避免 `== true` 之类比较
- 路径处理：强制 `std::path::Path`/`PathBuf`，禁止字符串拼接路径
- 测试：core 各模块内联 `#[cfg(test)]` 就近单测，夹具驱动集成测试放 `rs/crates/accel-core/tests/`；app 单测就近放置
- 临时文件：所有临时生成的脚本/文件必须写入项目根 `.temp/`（gitignore）；`.temp/` 探针脚本不受「必须用 edit」限制
- **文件修改必须用 edit 工具**：修改既有 `.rs`/`.md`/`.json` 一律用 edit 的精确 oldString/newString 替换，禁止用脚本做内容替换
- **版本单一来源**：见「运行与验证」节
- **全量回归输出**：`cargo test --workspace` 的结果行全量显示，禁止截断（会掩盖前段 FAIL）

## Commit 提交规范

- **标题行**：`<type>: V<版本>，<摘要>`——版本号与 `rs/config/static/base.json` 的 `version` 一致（四段式全写，如 `V0.6.1.2`）；摘要一句话概括核心
- **版本递增**：`feat`/`fix`/`refactor`/`perf` 提交前先 bump version（patch 位 +1）；`docs`/`test`/`style`/`chore` 不强制
- **type 全集**（conventional 风格）：`feat` 新功能 / `fix` 修复 / `refactor` 重构（行为不变）/ `perf` 性能 / `docs` 文档 / `test` 测试 / `style` 格式 / `build` 构建依赖 / `ci` CI / `chore` 杂项 / `revert` 回滚
- **正文**（改动大时可选）：`- ` 列表，按主题分组、分组下子条目缩进，每个功能块一行自然中文描述，每行 ≤ 100 字符
- **禁止项**：内部编号（A0.1/B1.5/FIX1.1/PL015.01 等任务编号）、验证/回归数字（如"全量回归 44 项通过"）、英文混排描述
- **提交范围**：一个版本的所有连带改动一次提交（源码 + 配置 + 文档同步）
- **流程**：由 AI 根据 `git status`/`git diff` 核对清单并草拟 commit 内容（git add 清单 + message）→ 用户审阅后自行执行 `git add`/`git commit`/`git push`（AI 不执行 git 写操作）

## Git 注意

- `.gitignore` 忽略 `CLAUDE.md`、`openspec/`、`.vscode/`、`.venv`、`archived/`、`.archived/`、`.opencode/`、`.agents/`、`.temp/`、`.history/`、`.mimosa/`、`rs/target/`、`logs/` —— 对这些文件的修改不会出现在 `git status` 中；`AGENTS.md` 与工作流文档（w/x/y/z/m/t）已纳入版本控制
- ~~`rs/config/user_config.json` 每次提交必须入库~~（FIX007.15 作废：运行期文件基准统一为 exe 同目录后，dev 落点在 `rs/target/debug/config/`——属 gitignore 的 `rs/target/`，不再入库；打包产物 exe 旁自包含，无库内副本诉求）

## 操作注意

- 执行命令前先检测当前 shell（Windows 下为 pwsh）：**搜索统一用 `rg`**（ripgrep，自动遵循 .gitignore，中文 UTF-8 正常；如 `rg -n "pattern" --type rust`，统计匹配数用 `rg -c`），无 rg 环境时回退 `Select-String`；目录操作使用 PowerShell 兼容命令（`Get-ChildItem` 替代 `ls` 等），避免 Linux-only 工具
- pwsh 会话带 `-NoProfile` 不加载 `$PROFILE`，输出中文前必须先设置编码：`[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;`
- 未经用户明确要求，不得擅自执行 `git add`、`git commit` 或任何其他 Git 写操作
- 视觉设计不得向测试环境妥协：offscreen/无头探针仅作功能与结构参考，桌面端一律按原设计实现
- **UI 行为变更固定连带清单**（A009 #3 定案，A010 #3 放宽）：凡界面行为/能力变更（主题、导航、热键、页面能力等），除代码与测试外必须同步检查：README「特性 + 使用说明 + 项目结构 + 常见问题」四节、`w.study.md` 主题/控件/路径相关表述（1/2/3.7 节）、`m.milestone.md`「后续版本」规划节、`rs/config/static/*.json` 死键——历史教训：FIX008 仅深色定案漏扫 README 五处与 w.study 三处（A009 #3/#4）
