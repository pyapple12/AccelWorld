# AccelWorld 项目分析报告

> 2026-10-17 重做：项目已由 Python（PyQt6 + Fluent Widgets）重构为 Rust（egui + eframe/glow），本报告按 Rust 现役实现改写；Python 版分析随 `.archived/` 归档留档。
> 2026-10-23 UIR 更新：界面推倒重塑——玻璃管线/Acrylic 整体废弃，实心主题 + 侧栏导航（本文 3.7 已按实心界面装配重写为现役口径）。

---

## 1. 项目概述

cargo workspace 双 crate 桌面应用：基于加速倍率的时间膨胀时钟（egui 中文界面为主）。

```
[输入]   用户设定加速倍率（rate_min 1.0 - rate_max 20.0，来自 config/static/base.json）
[引擎]   accel-core::time_dilation 按倍率换算：加速时钟 1 秒 = 现实 1/rate 秒
[展示]   accel-app（egui + eframe/glow）：侧栏导航五页 + 设置浮窗 + 实心主题（单强调色）
[周边]   农历干支节气 / 世界时钟 / 倒计时 / 天气（Open-Meteo）/ 闹钟 / 托盘 / 全局热键
[持久化] 用户配置 config/user_config.json（exe 同目录，FIX007.15 基准；倍率/主题/城市/时区/倒计时/窗口几何/闹钟）
```

- **时间膨胀**：`TimeDilation { rate, day_hours }`，标准一天总秒数 × rate = 加速一天总秒数，整数运算拆分时分秒；`now` 全参数化（无内部时钟，测试可注入）
- **中华元素**：lunar-rust 农历/干支/生肖/节气 + holidays.json 法定节假日 + 财神方位本地表
- **天气**：Open-Meteo 免费 API，17 城市统一表驱动，WMO 天气代码映射，TTL 缓存 + 重试
- **闹钟**：多闹钟（上限 max_alarms）、预设/自定义铃声、系统通知、去重键触发 + `Clock` 抽象（VirtualClock 可拨钟测试）
- **系统集成**：tray-icon 托盘菜单树、global-hotkey 全局热键、WinRT toast 通知（失败一律降级不崩）

## 2. 目录结构与模块职责

```
rs/                              # cargo workspace 根
  ci.ps1                         # 四关卡本地 CI（fmt / clippy / test / 依赖树零 egui）
  fixtures/                      # golden 夹具（Python 0.6.0.0 行为快照，parity 基准）
  config/static/                 # 应用静态配置（零硬编码）
    base.json                    # 应用参数 + version（版本单一来源）
    ui.json                      # 字体/布局 tokens/颜色
    holidays.json                # 法定节假日（历年导出）
    world_cities.json            # 世界时钟城市表（名称/坐标/IANA 时区；天气候选另用 rs/data/cities.json）
  crates/accel-core/             # 纯业务 crate（零 GUI 依赖，CI 机器强制）
    src/interface.rs             # AppInterface（七域契约）+ FrontendCommand/BackendEvent 总线 + spawn
    src/types.rs                 # 协议类型转出（TimeInfo/WeatherData/Alarm/DTO/ThemePref）
    src/time_dilation.rs         # 时间膨胀引擎（公式对齐 Python，秒级农历缓存）
    src/chinese_calendar.rs      # 农历/干支/生肖/节气/节日（lunar-rust + 常量表 + holidays.json）
    src/weather.rs               # 天气解析层 + 传输层（reqwest rustls + 按城 TTL 缓存 + 重试）
    src/alarm.rs                 # Alarm 模型 + AlarmManager + 触发状态机 + Clock 抽象
    src/settings.rs              # 用户配置读写（serde + 字段白名单过滤 + 原子写）
    src/static_config.rs         # Static 单例（include_str 嵌入 + OnceLock + 域访问器）
    src/audio.rs                 # rodio 预设/自定义铃声播放
    src/data/                    # 三数据表加载器（cities/timezones/weather_codes + world_cities）
    src/util/                    # file_io（原子写/白名单）/ retry（泛型重试）
  crates/accel-app/              # egui 前端 crate
    src/main.rs / lib.rs         # 入口 + 编译期版本（compiled_version）
    src/window.rs                # 主窗装配器（AppHandle：drain→侧栏→页→平台→节奏 + 截图/几何探针）
    src/bus.rs                   # 信号总线（UiState 快照 + drain/apply + 命令出口）
    src/theme.rs                 # 设计系统（ui.json colors 实心色板 → Visuals 深色单板，仅深色定案）
    src/logging.rs               # tracing 每日滚动 + panic hook 崩溃栈
    src/pages/                   # 五页（clock/countdown/world/weather/alarm）+ 设置浮窗
    src/widgets/                 # 自绘控件（digits/icons/icon_button/chip/slider）
    src/tools/                   # 展示运算纯函数（clock/countdown/alarm_text）
    src/platform/                # 系统集成（tray/notify/hotkey，非 Windows 桩化）
  assets/                        # 托盘图标等内嵌资源
  logs/                          # 运行日志（每日独立文件，gitignore）
```

架构一句话：`accel-core（纯业务 + interface 契约 + 命令/事件总线）← accel-app（egui 前端）`，依赖单向；app 不直碰业务（一切经 interface 命令/事件），core 不感知 egui（CI 关卡 4 机器强制）；`main.rs` 只做装配与运行时挂载。

## 3. 核心设计模式

### 3.1 应用静态配置层 — 编译期嵌入 + 单例

```rust
// static_config.rs：include_str! 编译期嵌入 + OnceLock 惰性初始化
pub struct Static { base: Value, ui: Value }
impl Static {
    pub fn get() -> &'static Static { /* get_or_init */ }
    pub fn base_f64(&self, key: &str) -> f64 { /* 缺键 panic 含键名 */ }
}
```

- base.json / ui.json 经 `include_str!` 嵌入二进制，运行期零文件依赖（便携 exe 自包含）
- 缺键/错型 panic 快速失败（附中文键名），对齐 Python `RuntimeError` 语义而不静默兜底
- 全项目零硬编码：倍率范围/周期/窗口几何/字体颜色/日志路径等一律 `Static::get().base_*()`

### 3.2 用户配置 — serde + 字段白名单过滤

- `UserConfig` 字段名与 Python 逐字一致（`time_dilation_rate`/`theme`/`last_city`/…），保证旧配置可读
- 经 `Option<Value>` 中间层逐字段过滤：类型不符/越界回退默认值 + warning 日志（等价 Python `dataclass_from_dict`）
- 写入走 `write_atomic`（`.{pid}.tmp` + rename，防半截文件）；损坏配置转存 `.bak` 并回退默认
- 倒计时目标双格式兼容：旧单值字符串 → 迁移为列表首项并回写新格式

### 3.3 时间膨胀引擎 — 参数化 now + 秒级缓存

```rust
pub fn time_info(&self, now: DateTime<Local>) -> TimeInfo {
    // day_start = now 本地当日 00:00 → elapsed=(now-day_start)×rate
    // accel = elapsed mod (day_hours×3600)（day_hours>24 时钟面扩展 HHH:MM:SS）
    // remaining = day_hours×3600 − elapsed；tick_ms = min(clock_tick_ms, 1000/rate)
}
```

- `now` 全参数化（无内部时钟）：测试可注入固定时刻，消除时间依赖
- 农历/中文日期按标准秒级缓存，标准/加速时间每次现算（对齐 Python 0.4.7.1 修复语义）
- 47 条 time 向量夹具位级全等（含 110.00000000000001 类 ULP 值——启用 `serde_json` `float_roundtrip` 特性）

### 3.4 闹钟服务 — 去重键 + 状态机 + Clock 抽象

- `Alarm` serde 结构与 Python 持久化逐字一致；`normalize()` 梳理脏值（repeat_days 强转/越界剔除/去重，时间 HH:MM 校验）
- `AlarmManager`：add（上限+去重）/remove/replace/toggle/from_dict_list（截断+过期禁用）；`should_trigger_on` 状态机逐行对齐 Python（一次性顺延/星期命中/过期不触发）
- `trait Clock`：`RealClock`（chrono::Local）/ `VirtualClock`（可拨）——调度循环经 `spawn_with_clock` 注入，拨钟即测触发

### 3.5 天气服务 — TTL 缓存 + 泛型重试

```rust
// 城市名 → 坐标表 → 拼 URL（host 常量白名单）→ GET → parse（经 retry）→ 写缓存
pub async fn fetch(&self, city: &str, force: bool) -> Result<WeatherData, WeatherErr>
```

- `WeatherClient`：reqwest(rustls，禁重定向) + 按城缓存（仅存成功）+ `force` 清该城键 + 失败不缓存
- 解析层逐字段校验（存在→非 null→类型对→范围对），对 null/缺字段/错型整包 `Err`；hourly/daily 键缺失向后兼容空序列
- `trait IsRetryable` 区分网络类（可重试）与解析类（立即返回）；httpmock 五路径集成断言

### 3.6 前后端解耦 — interface 七域 + 命令/事件总线

- `AppInterface`：world/weather/settings 等各域持有（Arc），提供同步查询（拉）+ 命令/事件通道（推）
- `FrontendCommand`（UI 意图：SetRate/ApplyRate/AddAlarm/SetTheme/Quit…）与 `BackendEvent`（后端事实：Tick/WeatherUpdated/AlarmTriggered/ConfigSaved…）全变体一次定义
- `bus.rs`（app 侧）：每帧 `logic()` 首行 drain 全部事件 → `UiState::apply` 更新快照；命令经 `send` 统一出口
- 硬保障：CI 关卡 4 校验 core 依赖树零 egui/eframe（单向分层从约定升级为机器强制）

### 3.7 界面装配 — 子夜星图实心主题 + 自绘控件（UIR2 现役）

- `window.rs`：`AppHandle`（logic：drain/探针/退出序列 + ui：侧栏 Panel → CentralPanel 当前页 → 设置浮窗）
- `theme.rs`：ui.json `colors.*` 深色实心单板（仅深色定案）→ egui Visuals + 语义色（strong/body/muted/meta 五层，对比度全矩阵 ≥4.5:1 实测）/字号/间距（spacing 域）/圆角访问器（cached 键缓存）
- `widgets/`：digits（等宽数字位——数字串逐字符槽宽绘制，根治 Bahnschrift 非等宽抖动）/ icons（矢量图标零字体依赖）/ icon_button / chip（单行安静徽标）/ slider
- 签名元素：星轨刻度（时钟页日进度 48 道刻度标尺——已过段金色/未至段弱轨，整点高半点矮）
- 高 DPI 纪律：面/文本坐标一律逻辑点；标题栏底色经 DWMWA_CAPTION_COLOR 恒钉侧栏深色（仅深色定案）
- 走查门：`ACCELWORLD_UI_SHOT` 截图探针（真窗逐页 PrintWindow PNG）+ 用户桌面终审
- 历史：玻璃渲染（WGSL 管线 + GlassScene + Acrylic）已整体废弃删除——半透明面对比度不可控（UIR 定案，留档见 git 历史）；UIR 初代「Calm Instrument」灰石板金调随 UIR2 子夜星图重塑（2026-10-24）

### 3.8 日志 — 每日滚动 + 崩溃栈

- `tracing_subscriber` + `tracing_appender::rolling::daily("logs","accel")`（对齐 Python 每日独立文件）
- panic hook → `logs/crash-YYYY-MM-DD.log`（含 location + backtrace）
- 级别读 base.json `log_level` token；后台任务 spawn 统一 join 监控（panic 记 error 不静默）

## 4. 代码风格观察

| 方面     | 实际情况                                                                          |
| -------- | --------------------------------------------------------------------------------- |
| 命名风格 | 函数/变量/字段 `snake_case`，类型/trait/枚举 `CamelCase`，常量 `UPPER_SNAKE_CASE` |
| 注释语言 | 中文；`///` 文档注释（1-3 行）+ `//!` 模块头 + 文件末 `// ===== 说明区`           |
| 错误处理 | 库层 `Result<T, E>` + `thiserror`；构建/初始化期 panic 快速失败（中文文案）       |
| 类型     | 公开 API 显式签名；`Option`/`Result` 表达空值与失败                               |
| import   | 三段顺序（标准库 → 第三方 → 本地），顶层 import                                   |
| 配置格式 | JSON；静态 `config/static/*.json`（include_str 嵌入）+ 用户配置 serde 反序列化    |
| 路径处理 | 强制 `std::path::Path`/`PathBuf`                                                  |
| 格式化   | `cargo fmt`（4 空格、行宽 100）；`cargo clippy -D warnings` 零警告                |
| 测试     | core 内联 `#[cfg(test)]` 就近单测 + `crates/accel-core/tests/` 夹具 parity 集成   |
| 临时文件 | `.temp/`（gitignore）                                                             |

## 5. 与 DeepTransHub 对比

| 维度       | AccelWorld RS                                            | DeepTransHub                              |
| ---------- | -------------------------------------------------------- | ----------------------------------------- |
| 应用形态   | Rust GUI（egui + eframe/glow）                           | CLI 菜单 + subprocess 调度脚本            |
| 配置管理   | `config/static/` 静态层 + serde 用户配置                 | `config/static/` 静态层 + key.json        |
| 配置加载   | `Static::get()` 单例（include_str + OnceLock）           | `get_static_config()` 单例                |
| 上下文聚合 | `TimeInfo`/`LunarInfo`/`WeatherData`/`UserConfig` 四 DTO | `_XxxContext` 四工具各自 dataclass        |
| 重试       | `util/retry.rs` 泛型 `retry()` + `IsRetryable`           | `utils/error_handler.py` `retry_call()`   |
| 并发模型   | tokio 后台 runtime + mpsc 命令/事件总线                  | 串行 + Ctrl+Q 文件信号优雅退出            |
| 数据存储   | JSON 用户配置 + data/ 静态数据表                         | Excel 状态机 + SQLite 词汇库              |
| 日志系统   | tracing 每日滚动 + panic hook 崩溃栈                     | 按天+序号 usage 日志 + 分析工具链         |
| 代码规范   | AGENTS.md 成文（`///`+`//!`+说明区房规）                 | AGENTS.md 成文（同源，DeepTransHub 更早） |
| 工作流体系 | 2026-09-10 接入（本文档交接）                            | w/x/y/z 四件套 + skills + 审计流程        |

## 6. 与 Python 版的 parity 基准

- Python 版（0.6.0.0）冻结为 golden 基准：`rs/fixtures/*.json` 为行为快照（time/config/weather/lunar/alarm 五类），core 行为断言一律对夹具
- 夹具可由 `.archived/` 内 Python 副本再生（`export_fixtures.py`，sha256 与入库值一致）
- 镜像映射表（Python → Rust）见 `z.plan.md`「总纲修订一」；执行偏移与豁免清单见 z.plan 各 PL 完成记录节
