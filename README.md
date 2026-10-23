# AccelWorld —— 加速世界世界钟

[![Version](https://img.shields.io/badge/Version-0.6.1.1-blue.svg)](rs/config/static/base.json)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](rs/rust-toolchain.toml)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

---

## 项目简介

基于时间膨胀倍率的自定义小时制时钟应用。根据设定的加速倍率（1.0x - 20.0x），实时显示加速后的时间，同时保留标准时间对照。灵感来自《加速世界》，界面以中文为主，提供农历、生肖、节气等中华传统文化元素。

> 当前技术栈：**Rust**（egui + eframe/glow 实心桌面 UI）。0.6.0.0 为原 Python（PyQt6 + Fluent Widgets）版本的定版，本仓库已在同版本号段内完成引擎重构（PL011–PL015）与 UI 推倒重塑（UIR，玻璃管线整体废弃），Python 版归档于 `.archived/`（git 忽略，仅供 golden 夹具再生）。
> 任务清单见 `x.progress.md`，已知问题见 `y.problems.md`，版本路线见 `m.milestone.md`，方案与审计见 `z.plan.md`。

## 目录

- [特性](#特性)
- [快速开始](#快速开始)
- [使用说明](#使用说明)
- [项目结构](#项目结构)
- [依赖](#依赖)
- [贡献指南](#贡献指南)
- [常见问题](#常见问题)
- [许可证](#许可证)

## 特性

- **时间膨胀计算**：根据可配置的加速倍率（1.0x - 20.0x）计算自定义时间流速
- **实时时钟**：加速时间大字英雄区（等宽数字位不抖动）+ 标准时间对照 + 日进度细条
- **实心桌面 UI**：侧栏导航 + 深浅两套实心主题 + 矢量图标（零字体依赖）+ 单强调色纪律
- **侧栏导航**：时钟 / 倒计时 / 世界时钟 / 天气 / 闹钟 五页 + 设置浮窗
- **农历信息**：天干地支年、生肖、时辰、月相、节气、公历节日、拜财神方向
- **世界时钟**：城市矩阵走时 + 昼夜指示，支持增删城市与基准时区切换
- **倒计时功能**：多目标列表，法定/传统节日自动派生
- **天气显示**：Open-Meteo 免费 API，当前 / 逐小时（24 点）/ 七日预报，单城市可换（城市选择器，选择即持久化）
- **闹钟功能**：多闹钟、预设/自定义铃声、一键快捷添加、系统通知提醒
- **三态主题**：跟随系统 / 浅色 / 深色，设置浮窗选择或全局热键 Ctrl+T 循环
- **系统集成**：系统托盘菜单树（显示/隐藏/退出，关闭按钮隐藏驻留）+ 全局热键（Ctrl+S 保存 / Ctrl+Q 退出 / Ctrl+T 主题）
- **配置持久化**：倍率、主题、城市、时区、倒计时目标、闹钟等自动保存

## 快速开始

### 环境要求

- Rust stable 工具链（见 `rs/rust-toolchain.toml`）
- Windows：MSVC 链接器（VS Build Tools，含 C++ 工作负载）

### 构建与运行

```powershell
cd rs

# 开发运行（真窗）
cargo run -p accel-app

# 发布构建
cargo build --release        # 产物 rs/target/release/accel-app.exe
```

### 验证

```powershell
# 四关卡 CI：格式 / clippy / 全量测试 / core 依赖树零 GUI
pwsh rs/ci.ps1

# 仅测试
cargo test --workspace

# UI 截图探针（真窗逐页 PNG 后自动退出，输出到指定目录）
$env:ACCELWORLD_UI_SHOT=".temp/ui_shots"; cargo run -p accel-app
```

## 使用说明

### 界面操作

- **导航**：左侧边栏切换 时钟 / 倒计时 / 世界时钟 / 天气 / 闹钟；底部「设置」打开设置浮窗
- **调节倍率**：时钟页拖动滑杆（拖动即生效，松手落盘），支持 0.1x 步进
- **切换主题**：设置浮窗选择 跟随系统/浅色/深色，或热键 Ctrl+T 三态循环；跟随系统时随 Windows 深浅色实时切换
- **世界时钟**：点击城市磁贴切换基准时区，加号添加城市、悬停减号移除（至少保留一个）
- **倒计时**：加号新增目标（名称 + 日期时间），减号移除；默认目标为下一个春节
- **天气**：城市选择器切换查看城市（选择即持久化），刷新按钮强制拉取
- **闹钟**：快捷按钮（15/30/60/240 分钟）或自定义添加（时间/标签/重复星期），行卡开关与删除
- **托盘**：关闭按钮隐藏到托盘后台驻留；右键弹出菜单（显示窗口 / 隐藏到托盘 / 退出），双击托盘图标还原
- **全局热键**：Ctrl+S 保存 / Ctrl+Q 退出 / Ctrl+T 主题循环（窗口失焦亦生效）

### 配置与日志

- 用户配置：`rs/config/user_config.json`（倍率/主题/城市/时区/倒计时/闹钟/窗口几何）
- 运行日志：`rs/logs/`（按日独立文件 + 崩溃栈）

## 项目结构

```
rs/                              # cargo workspace 根（全部命令在此执行）
├── Cargo.toml                   # [workspace] members = ["crates/*"] + 统一依赖版本
├── rust-toolchain.toml / rustfmt.toml / clippy.toml
├── ci.ps1                       # 四关卡本地 CI（fmt / clippy / test / 依赖树）
├── README.md                    # Rust 工程房规与结构说明
├── fixtures/                    # golden 夹具（Python 0.6.0.0 行为快照，parity 基准）
├── config/static/               # 应用静态配置（零硬编码 token）
│   ├── base.json                # 应用参数 + version（版本单一来源）
│   ├── ui.json                  # 字体/布局 tokens/颜色
│   ├── holidays.json            # 法定节假日（逐年导出）
│   └── world_cities.json        # 世界时钟城市表（名称/坐标/IANA 时区；天气候选另用 rs/data/cities.json）
├── crates/
│   ├── accel-core/              # 纯业务（零 GUI 依赖，CI 强制）
│   │   └── src/                 # interface/types/time_dilation/chinese_calendar/weather/
│   │                            #   alarm/settings/static_config/audio/data/util
│   └── accel-app/               # egui 前端
│       └── src/                 # window/bus/theme/logging/main.rs
│           ├── pages/           # 五页（clock/countdown/world/weather/alarm）+ 设置浮窗
│           ├── widgets/         # 自绘控件（digits/icons/icon_button/switch/chip/slider）
│           ├── tools/           # 展示运算纯函数
│           └── platform/        # 系统集成（tray/notify/hotkey）
├── assets/                      # 托盘图标等内嵌资源
└── logs/                        # 运行日志（gitignore）
```

## 依赖

主要 crates（完整清单见 `rs/Cargo.toml`）：

| 领域     | 依赖                                                              |
| -------- | ----------------------------------------------------------------- |
| GUI      | eframe(glow) / egui / image                                       |
| 后端     | tokio / reqwest(rustls) / chrono / chrono-tz / lunar_rust / rodio |
| 序列化   | serde / serde_json                                                |
| 日志     | tracing / tracing-subscriber / tracing-appender                   |
| 系统集成 | tray-icon / global-hotkey / tauri-winrt-notification / windows    |
| 测试     | httpmock                                                          |

## 贡献指南

欢迎各类贡献，包括 Bug 修复、功能开发、文档优化等。

1. Fork 本仓库
2. 创建特性分支（`git checkout -b feature/xxx`）
3. 提交前跑 `pwsh rs/ci.ps1` 四关卡全绿
4. 提交修改（提交信息规范见 `AGENTS.md`）
5. 推送分支并发起 Pull Request

## 常见问题

### Q1：加速倍率设置为多少最合适？

A：默认值为 2.0x，可在 1.0x 到 20.0x 之间调节。例如 2.0x 表示加速世界的一天等于现实 12 小时。建议从默认值开始逐步调整。

### Q2：关闭程序后设置会丢失吗？

A：不会。倍率、主题、城市、时区、闹钟等设置自动保存到 `rs/config/user_config.json`，下次启动自动恢复。

### Q3：支持查看哪些城市的天气？

A：覆盖国内外 17 个主要城市（`rs/data/cities.json`）。天气数据来自 Open-Meteo 免费 API，无需 API Key。世界时钟页可另选 50 城看走时（`rs/config/static/world_cities.json`）。

### Q4：如何让程序在后台运行？

A：点击窗口关闭按钮即隐藏到系统托盘驻留；托盘右键菜单或双击图标可还原窗口，菜单"退出"才真正结束进程。

### Q5：倒计时支持哪些目标？

A：任意名称 + 日期时间目标，另自动派生当年/次年法定节假日与传统节日；未设置时默认指向上一个/下一个春节。

## 维护者

- 作者：[pyapple12](https://github.com/pyapple12)
- 邮箱：takechance_bao@188.com

## 许可证

本项目基于 [GNU GPL v3 许可证](./LICENSE) 开源，允许自由使用、修改及分发，但必须保留源代码并以相同许可证发布。
