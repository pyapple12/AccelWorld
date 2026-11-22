# AccelWorld 项目说明

Rust 桌面应用：基于加速倍率的时间膨胀时钟（egui + eframe/glow 实心桌面 UI，中文界面为主）。2026-10-16 起由 Python（PyQt6 + Fluent Widgets）重构为 Rust，Python 版全量归档 `.archived/`（git 忽略，仅供 golden 夹具再生）。2026-10-23 UI 推倒重塑（UIR）：玻璃管线/Acrylic/透明窗口整体废弃；2026-10-24 UIR4 开创性重构：单环星盘主仪恒驻（今四针）+ 底部仪器坞 + 抽屉页（侧栏导航废除）；2026-10-28 PL017 结构重排：悬浮带标签坞 + 抽屉三段式 + 盘下读数 + 短倍率条 + 设置抽屉 + 三档字重（V0.6.3.1）；2026-10-28 PL018 边界补强：退出自愈 + 落点校准 + 首帧门控 + `egui_flex` 裁决（V0.6.3.2）。2026-09-10 起接入 DeepTransHub 工作流体系（文档四件套 + Commit 提交规范 + `.agents/skills/` 项目级技能）。

## 运行与验证

- 全部命令在 `rs/` 目录（cargo workspace 根）下执行；工具链见 `rs/rust-toolchain.toml`（channel = stable）
- **统一入口四关卡**：`pwsh rs/ci.ps1`（fmt --check → clippy `-D warnings` → `cargo test --workspace` → core 依赖树零 egui/eframe）；`.github/workflows/rs-ci.yml` 复用同一命令集
- 全量测试：`cargo test --workspace`（core 单测 + app 单测）
- GUI 冒烟（真窗出血，会弹窗）：`cargo run -p accel-app`；UI 截图探针（逐页 PNG 后自动退出）：`$env:ACCELWORLD_UI_SHOT="<目录>"; cargo run -p accel-app`
- 探针环境变量（正常模式零影响）：`ACCELWORLD_FPS_PROBE=<秒>` 静置测帧；`ACCELWORLD_UI_SHOT_GEOMETRY=WxH` 固定测试几何；`ACCELWORLD_TEST_NO_SAVE=1` 退出不写窗口存档；`ACCELWORLD_TEST_POS="x,y"` + `ACCELWORLD_EXPECT_POS="x,y"` 落点对照；`ACCELWORLD_EARLY_SHOT_MS=<ms>` 早期帧；`ACCELWORLD_TEST_CLOSE_LOSS=n` 丢弃前 n 次 Close（验证退出自愈）；`ACCELWORLD_LAYOUT_PROBE=1` 布局盒/光晕/实体墨迹 `[BOX]/[RECT]/[GLOW]/[INK]`；`ACCELWORLD_TEXT_PROBE=1` 文字盒/基线/alpha 墨迹 `[TEXT]`；`ACCELWORLD_TEXT_MATRIX=1` 字体下探矩阵 `[GLYPH]/[GLYPHMAX]`（可用 `ACCELWORLD_TEXT_PROBE_TEXT` / `ACCELWORLD_TEXT_PROBE_SIZES` 覆盖样本与字号）
- **版本单一来源**：`rs/config/static/base.json` 的 `version` 字段（以该字段为准，不在文档写死当前值）；app 侧 `build.rs` 编译期注入 `ACCELWORLD_VERSION`，窗口标题/关于页/托盘 tooltip 一律经运行期 `Static::get().version()` 读取，代码中不得出现版本字符串
- **时间口径**：程序面向本地用户，所有用户可见时间与日志一律本地时区（`chrono::Local` / `ChronoLocal` / `LocalDailyWriter` 本地日期滚动）；世界时钟按 IANA 时区显示属跨时区功能，内部 UTC 仅作换算中间值。禁止使用 tracing-subscriber 默认 UTC 时间戳或 tracing-appender 的 UTC 日滚
- **版本体系**：四段式纯数字 `X.Y.Z.P`（无 `ver ` 前缀）；`feat`/`fix`/`refactor`/`perf` 提交前 bump patch 位；bump 以 **HEAD 已提交版本**为基准，同一批未提交改动只加一次，不得在工作区已 bump 的值上再加一；历史存量 `ver 0.4x` 三段式仅存于历史文档与提交记录，不回溯改写

## 环境陷阱

- 工作区为 Windows + MSVC 工具链（`stable-x86_64-pc-windows-msvc`）；首次配置需 VS Build Tools 的 `cl`/`link`
- `.venv` 与 `config/user_config.json`（Python 版）已随 Python 归档，不再作为验证通道；现役用户配置与日志均在 **exe 同目录**（FIX007.15 基准统一：配置 `config/user_config.json`、日志 `.logs/`，基准取 `core::util::paths::exe_dir`——`cargo run` 下落 `rs/target/debug/`，打包产物即 exe 旁自包含）
- `rs/target/` 为构建缓存（体积可达数十 GB），可随时 `cargo clean` 清除；勿将其纳入版本控制
- pwsh 会话带 `-NoProfile`；输出中文前先 `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;`；搜索统一 `rg`

## 结构与约定

- cargo workspace 双 crate，依赖单向 `app → core`（CI 关卡 4 机器强制 core 零 egui/eframe）：
  - `rs/crates/accel-core`：纯业务（时间膨胀 / 农历干支节气 / 天气 / 闹钟 / 配置 / 数据表 / 通用工具 / 接口契约），零 GUI 依赖
  - `rs/crates/accel-app`：egui 前端（实心主题窗口装配 / 星盘主仪 + 悬浮仪器坞 + 抽屉页（PageHeader/Scroll/Footer）+ 设置抽屉 / 自绘控件（field/switch/primary_button 等）/ Tabler 图标字族 / 反馈动效 / 系统集成 / 首帧门控显示）
- **分层与镜像**（Python → Rust，完整对应见 z.plan 镜像映射表）：`interface.rs` 七域契约 + `FrontendCommand`/`BackendEvent` 命令事件总线是 UI **写操作**后端的唯一入口（UI 不直改业务状态）；app 经 `bus.rs` 每帧 drain 事件、按需发命令，core 不感知 egui。**契约边界（A005 #12 明确）**：只读访问允许直取——`static_config::Static`（渲染 token）、`types` 与纯常量 DTO（`alarm::PresetSound`、`settings::CountdownTarget`）、`data::world_cities` 与 `config/static/continents.json` 只读检索、`audio`（平台侧播放封装）；判定口径：**改变后端状态**才必须走总线，纯读取再经 interface 转一层属无价值间接
- **代码零硬编码**：业务参数（倍率范围/默认值/周期/窗口几何/颜色/日志路径等）全部从 `rs/config/static/*.json` 读取（`static_config.rs` 单例经 `include_str!` 编译期嵌入）；运行期文件一律 exe 同目录相对（FIX007.15）：用户配置 `config/user_config.json`、日志 `.logs/`（每日独立文件）
- **注释房规**：`///` 中文文档注释（函数/方法，1-3 行，说明用途与核心逻辑）；模块头 `//!` 一行职责；每个 `.rs` 文件末尾保留 `// ===== 说明区 =====`（输入输出 / 逻辑步骤 / 设计理由 / 异常处理 / 关联配置）
- **视觉走查门**：UI 观感改动经 `ACCELWORLD_UI_SHOT` 截图探针（真窗逐页 PNG）自检再进用户桌面走查——功能探针抓不住纯视觉缺陷（UIR 前的玻璃时代为 render_probe 离屏断言，管线已废）
- **布局定位染色规则**：同一布局连续几轮调整仍对不上时，停止盲调；先临时给对应排版盒染色 + 描边（蓝=块、黄=文字盒、红=图标盒/关键盒），真窗或截图确认盒边界与中线，定位后再删除临时色，不得把染色留在正式构建
- **平台集成降级纪律**：托盘 / 全局热键 / 通知等系统能力构造失败一律降级日志不崩（行为无损），非 Windows 平台编译期桩化
- **天气网络策略**：用户设置优先（设置 → 网络：代理开关 + 地址；关=仅直连，开+空=系统探测，开+地址=指定代理）；直连（显式 `no_proxy`）优先，直连网络失败且允许代理时改走代理；每次拉取前重读策略，开关/改地址无需重启；两通道都网络失败由 `interface` 排 `weather_fail_retry_delay_s`（默认 600s）自动重试。冷启动先读磁盘缓存上屏，首帧后才发起网络强制首拉（不得阻塞启动）。相关 token 全在 `base.json`（`weather_timeout_s` / `weather_connect_timeout_s` / `weather_retries` / `weather_fail_retry_delay_s` / `weather_auto_refresh_s` / `default_weather_use_proxy` / `default_weather_proxy`），不得在代码写死
- **世界时钟**：单 hero（城市名 + 大号 24h + 日期周几 + 时区偏移）+ 下方滚动列表；hero 城市不在列表重复，hero 不能直接删，先 pin 新城市；291 城 SQLite 主表（预置 world_cache.sqlite）中文/英文/拼音/首字母索引（12 中国 + 279 境外）；国内表外城市走 Open-Meteo geocoding（英文名用拼音），境外只用本地城市表；城市主表、搜索结果与自定义城市均落 exe 同目录 `world_cache.sqlite`；`SetBaseTimezone`/`last_timezone`/`default_timezone` 已退役，不再新增“基准”状态
- **天气城市**：无独立城市下拉/刷新——天气城市 = 世界页 hero；`default_city`（上海）只作无 hero 时兜底，启动读到 hero 后立即替换并强制拉取；运行期 hero 变更即切城拉取；程序打开期间每 `weather_auto_refresh_s`（1800s）自动刷新。旧 17 城天气坐标表与 `SetWeatherCity` 已退役，天气坐标从 SQLite `city` 表/自定义城市条目解析
- **视觉设计不得向测试环境妥协**：offscreen / 无头探针仅作功能与结构参考；视觉与材质的最终实测以用户桌面走查为准
- 提交信息规范见下文「Commit 提交规范」节；功能开发先走 OpenSpec 提案流程
- 工作流文档：`w.study.md` 项目分析 / `x.progress.md` 任务清单（PL011–PL018 + UIR/FIX，已完成在前、未完成在后）/ `y.problems.md` 已知问题 / `z.plan.md` 方案记录与审计附录（豁免定案清单 `A{NNN}` 递增）/ `m.milestone.md` 版本里程碑 / `t.hint.md` 经验复盘
- **UI 正式文档**：`u.UI_GLOSSARY.md` UI 术语表（区域/控件/状态主称 + 代码锚点，目测对账单一入口）/ `u.UI_MAP.html` UI 标注指南（六页结构图 + 编号索引 + 状态词 + 令牌速查，浏览器直接打开）
- 项目级 Agent 技能在 `.agents/skills/`（工作流 4 件：audit-project / audit-report / progress-task / skillforge；UI 5 件：better-interface / better-colors / better-layout / better-typography / better-ui；设计与评审 8 件：taste-skill / game-ui-ux / game-feel / diagram-design / performance-optimization / save-systems / input-systems / dimensional-drawing；gitignore 仅本地使用）
- 历史规划文档 `workingboard/` 已归档 `archived/workingboard`（gitignore）

## 文字盒与盒子对齐（强制基准，2026-11-15 定案，不得更改）

- **唯一参照系是盒子。** 文字、星盘、按钮、胶囊等一切 UI 元素都盒子化；盒子可以套盒子，但最多一层（容器盒 + 子盒），子盒内只放文字/图形，不再继续套盒。
- **文字盒**：盒高按字体组合参数表计算：`n = (字号 − 最大实体墨迹) / 栅格`，`盒高 = 字号 − (栅格 − 上空白 − 下空白) × n`；`盒顶 = 盒底 − 盒高`；`盒中线 = 盒顶 + 盒高 / 2`；`盒内基线 = 盒底 − 下空白`。上空白 = `0.2n`、下空白 = `0.1n`，当前栅格 = 0.8px。
- **盒底不是裁切边界。** 下空白只是排版余量；英文 g/p/y 等真实字形下探允许画出盒底，盒子与父盒都不得因此裁切文字。
- **图标盒不是裁切边界。** 图标盒 20×20 / 22×22 只是定位盒；字形光栅化后超出盒子的部分（含淡边、渐隐像素）允许画出盒外，父盒与 `clip_rect` 不得裁掉。
- **盘内数字例外。** 星盘 12/3/6/9 属图形刻度，不建文字盒、不查 `font_profile` 的盒高与 dy；字号 = `20 × dial_scale`，按实际墨迹 bbox 中线对极坐标落点。
- **星盘等比。** 盘内几何、字号、线宽、尾长与光晕按 `dial_d / 320` 缩放；指针长度固定为 320 下的 60 / 90 / 110 / 100 再乘 scale；AM/PM 胶囊固定不缩放，当前半日文字金色、填充不变；金圈厚度 2–10×scale，跨日 1s 平滑不消失。
- **GPU 光晕 quad。** `paint_dot` 的 pad 取 `4.6 × bloom + 2`，且必须保证 `quad_top ≥ 窗口内容区上沿`；否则实际渲染圆心会向下挤。`[GLOW]` 只是 CPU 参数，不代表最终渲染位置。
- **默认垂直对齐 = 盒底对盒底。** 盒子与盒子只比较盒子数值；文字基线只在盒子内部画字，**不参与外部对齐**。混排一行时，行盒高 = 各文字盒高的最大值，各子盒底边对齐行盒底边；各盒内部基线自然同为 `盒底 − 下空白`，这是结果，不是对齐依据。
- **明确说明“居中”时 = 文字自己的字墨中线对盒子中线。** 文字是主动方，盒子中线是目标；混排时每个不同字号各自拿自己的字墨中线对同一盒子中线，允许基线不同。禁止用 `中线 + 盒高/2 − 下空白` 冒充居中（那只是底对齐的另一种写法）。组件高度可以因删除 `line_height` 变矮；居中只由字墨中线对盒中线决定，不得用 padding 补偿高度来伪造居中；居中不对就是居中算法错。
- **盒子间对齐只允许盒子数值**：顶部 `a.top = b.top`；垂直中心 `a.center.y = b.center.y`；底部 `a.bottom = b.bottom`；左 `a.left = b.left`；水平中心 `a.center.x = b.center.x`；右 `a.right = b.right`。不得出现字墨 bbox、基线、字号、cap height、0.62/0.86/1.24 等比例作为对齐参数。
- **字宽防抖沿用现行办法**：数字槽宽取 0–9 最大 advance，按字号/字族缓存，下限 `字号 × 0.5`；数字 1.0 槽，冒号/句点/空格 0.45 槽，连字符 0.45 槽 +2px；字母与 `°` `%` `/` 等单位符号按字体实际 advance 补槽（PL029，防单位符号挤字）。
- **原生控件也必须进盒。** TextEdit、原生 label、图表标注等外层必须有明确盒子；原生尺寸可以不变，但位置、对齐、间距只走盒子数值。
- **非文字元素同样盒子化。** 星盘、按钮、胶囊等与文字盒同级；凡只规定下限的尺寸，不设上限（例如星盘盒只保底 120）。
- **星盘盒**：正方形，边长 = `min(剩余高, 页宽 − 2×82)`，下限 120、无上限；`82 = 内容边→药丸 10 + 药丸 52 + 药丸→盘盒 20`。宽度不足时以宽度定边长，高度跟随，不得按高度无限放大撑破内容。
- **实现口径**：`text_metrics::height()` 与 `text_ink` 的盒高/基线一律查字体组合参数表，不写死常量；`text_ink` 只提供字体基线偏移，以及“明确居中”所需的**整串字墨 bbox 并集**，不得用首字位图 bbox 决定盒高；组件高度不得再用 `line_height` 另立一套。
- **历史文档例外**：`x.progress.md`、`z.plan.md`、`m.milestone.md` 中已完成/已归档的旧记录保持原样；现行文档（`u.UI_GLOSSARY.md`、`w.study.md`、`u.UI_MAP.html`、README、代码注释与说明区）必须与本规范一致。

### 字体组合参数表与三线基线（强制基准，2026-11-16 定案，不得更改）

- **三线定义**：排版基线 `P` 是字体文件给定的原始基线；优化基线 `O` 是中文实体墨迹底上移到英文/数字排版基线后的线；盒子基线 `B` 是盒底向上留出下空白后的锚点。
- **三线关系**：`P_cjk + dy_cjk = P_latin + dy_latin = P_digit + dy_digit = O = B`。当前 `dy_latin = dy_digit = 0`，公式保留三项，未来表里填值即生效，不改代码。
- **基线定位**：`B = 盒底 − 下空白`，`O = B`，`P = O − dy`；渲染用 `P` 定位 galley，`B` 只做对齐目标，不直接绘制。
- **三个写死常量**：`grid = 0.8`、`top_ratio = 0.2`、`bottom_ratio = 0.1`；`step = grid − top_ratio − bottom_ratio`，不得再立第四个常量。
- **四个实测值**（程序从字体逐档量出，属字体事实；量错一个，整表失效）：
  - `m_cjk` 中文最大墨迹：全量程序汉字、regular、`alpha > 96`；口径 = 字符集墨迹上缘到基线的最大距离 + 基线下最大下探，即最大墨迹总高。
  - `m_latin` 英文最大墨迹：`a–z A–Z` 整字剔除 `g/j/p/q/y`、regular、`alpha > 96`；口径 = 剔除后字符集的最大墨迹（上缘；下探为 0）。
  - `m_digit` 数字最大墨迹：`0–9`、regular 数字字体、`alpha > 96`；口径 = 0–9 最大墨迹（上缘；下探为 0）。
  - `dy` 中文下探：同一份中文 `alpha > 96` 数据的基线下最大下探。
- **表内只存栅格数**：`cjk_ink_grids = m_cjk / grid`、`latin_ink_grids = m_latin / grid`、`digit_ink_grids = m_digit / grid`、`dy_cjk_grids = dy / grid`。
- **派生公式**：中文 `trim_grids = size / grid − cjk_ink_grids`；`h_cjk = size − step × trim_grids`；`top = top_ratio × trim_grids`；`bottom = bottom_ratio × trim_grids`。英文/数字按各自最大墨迹：`raw = m + 0.2 + 0.1`；`h = ceil(raw / step) × step`；`pad = h − raw`；按 0.1px 单位 `u = round(pad / 0.1)`，`top_extra = floor(u / 2) × 0.1`，`bottom_extra = pad − top_extra`；`bottom_class = 0.1 + bottom_extra`；`dy_latin = dy_digit = 0`。
- **白边分配**：英文/数字基础白边固定上 `0.2`、下 `0.1`；取整余量 `pad` 按 `0.1 → 下`、`0.2/0.4 → 上下均分`、`0.3 → 上 0.1 / 下 0.2` 分配。中文不使用这套固定白边，仍按 `trim_grids` 缩放，并保留 `dy_cjk`。
- **三线**：`B = 盒底 − bottom`，`O = B`，`P = O − dy`；`dy = dy_cjk_grids × grid`；英文、数字、混排 `dy = 0`。
- **测量口径**：全量程序字符（静态文案 + `ui.json` 去重汉字 + `a–z A–Z 0–9`）、regular、`alpha > 96` 实体；不用 ascender，不用 bbox，不用淡边；125% DPI 下 `grid = 0.8px`；最大墨迹取全字符集最大值，中位只作参照；数字单独以 `0–9` 测量 `m_digit`，不并入 `m_latin`。
- **新字体组合**：换字体 = 新增 profile 表，重新跑矩阵测 `m_cjk` / `m_latin` / `m_digit` / `dy`，重算全部栅格；公式与三线关系不变。
- **数字字形平替例外**：仅更换数字族字形、`latin_ink` 仍由 MiSans 决定、`digit_ink` 仍按同一 `0–9` 口径、最大 advance 差 <0.5% 时，可保留原 profile 数值与 `dy`；须在 `z.plan` 登记只读测算（PL026 Barlow Medium）。
- **经验**：英文墨迹上缘不是等比（12px 8.8、16px 12.0），必须每档实测；`m_cjk`、`m_latin`、`m_digit` 必须整除 `grid`，出现小数说明测量口径有误。

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
- 测试：core 各模块内联 `#[cfg(test)]` 就近单测；app 单测就近放置
- 临时文件：所有临时生成的脚本/文件/截图必须写入项目根 `.temp/`（gitignore）；**禁止写入用户目录**（`%TEMP%`、`C:/Users/<用户>/AppData/Local/Temp/opencode/` 等）；`.temp/` 探针脚本不受「必须用 edit」限制
- **文件修改必须用 edit 工具**：修改既有 `.rs`/`.md`/`.json` 一律用 edit 的精确 oldString/newString 替换，禁止用脚本做内容替换
- **版本单一来源**：见「运行与验证」节
- **全量回归输出**：`cargo test --workspace` 的结果行全量显示，禁止截断（会掩盖前段 FAIL）

## Commit 提交规范

- **标题行**：`<type>: V<版本>，<摘要>`——版本号与 `rs/config/static/base.json` 的 `version` 一致（四段式全写，如 `V0.6.1.2`）；摘要一句话概括核心
- **版本递增**：`feat`/`fix`/`refactor`/`perf` 提交前先 bump version（patch 位 +1）；`docs`/`test`/`style`/`chore` 不强制
- **type 全集**（conventional 风格）：`feat` 新功能 / `fix` 修复 / `refactor` 重构（行为不变）/ `perf` 性能 / `docs` 文档 / `test` 测试 / `style` 格式 / `build` 构建依赖 / `ci` CI / `chore` 杂项 / `revert` 回滚
- **正文**（改动大时可选）：`- ` 列表，按主题分组、分组下子条目缩进，每个功能块一行自然中文描述，每行 ≤ 100 字符
- **正文范围**：只写代码与行为变化；文档同步（README / AGENTS / w.study / UI 文档等）不单独列条，文档随版本提交
- **禁止项**：内部编号（A0.1/B1.5/FIX1.1/PL015.01 等任务编号）、验证/回归数字（如"全量回归 44 项通过"）、英文混排描述
- **提交范围**：一个版本的所有连带改动一次提交（源码 + 配置 + 文档同步）
- **流程**：由 AI 根据 `git status`/`git diff` 核对清单并草拟 commit 内容（git add 清单 + message）→ 用户审阅后自行执行 `git add`/`git commit`/`git push`（AI 不执行 git 写操作）

## Git 注意

- `.gitignore` 忽略所有以 `.` 开头的目录（`.*/`，唯一例外 `.github/` 保留 CI）、`CLAUDE.md`、`openspec/`、`rs/target/` —— 对这些文件的修改不会出现在 `git status` 中；`AGENTS.md`、工作流文档（w/x/y/z/m/t）与 UI 正式文档（`u.UI_GLOSSARY.md` / `u.UI_MAP.html`）已纳入版本控制
- ~~`rs/config/user_config.json` 每次提交必须入库~~（FIX007.15 作废：运行期文件基准统一为 exe 同目录后，dev 落点在 `rs/target/debug/config/`——属 gitignore 的 `rs/target/`，不再入库；打包产物 exe 旁自包含，无库内副本诉求）

## 操作注意

- 执行命令前先检测当前 shell（Windows 下为 pwsh）：**搜索统一用 `rg`**（ripgrep，自动遵循 .gitignore，中文 UTF-8 正常；如 `rg -n "pattern" --type rust`，统计匹配数用 `rg -c`），无 rg 环境时回退 `Select-String`；目录操作使用 PowerShell 兼容命令（`Get-ChildItem` 替代 `ls` 等），避免 Linux-only 工具
- pwsh 会话带 `-NoProfile` 不加载 `$PROFILE`，输出中文前必须先设置编码：`[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;`
- 未经用户明确要求，不得擅自执行 `git add`、`git commit` 或任何其他 Git 写操作
- **临时内容落点**：探针截图、临时脚本、测试输出一律写项目根 `.temp/`，不得写入用户目录（`%TEMP%`、`C:/Users/<用户>/AppData/Local/Temp/opencode/` 等）；任务结束清理不再需要的临时文件
- 视觉设计不得向测试环境妥协：offscreen/无头探针仅作功能与结构参考，桌面端一律按原设计实现
- **首帧门控**（PL018.06）：窗口以 `with_visible(false)` 创建，完成 1 帧或 250ms 看门狗后才显示；探针截图在 1.6s 后采集不受影响，若改启动时序须回归“截图矩阵/隐藏驻留/正常启动”三条路径
- **UI 行为变更固定连带清单**（A009 #3 定案，A010 #3 放宽）：凡界面行为/能力变更（主题、导航、热键、页面能力等），除代码与测试外必须同步检查：README「特性 + 使用说明 + 项目结构 + 常见问题」四节、`w.study.md` 主题/控件/路径相关表述（1/2/3.7 节）、`m.milestone.md`「后续版本」规划节、`rs/config/static/*.json` 死键——历史教训：FIX008 仅深色定案漏扫 README 五处与 w.study 三处（A009 #3/#4）
