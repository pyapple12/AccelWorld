# AccelWorld —— 加速世界世界钟

[![Version](https://img.shields.io/badge/Version-0.5.3.0-blue.svg)](config/static/base.json)
[![Python](https://img.shields.io/badge/Python-3.10+-green.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

---

## 项目简介

基于时间膨胀倍率的自定义小时制时钟应用。根据设定的加速倍率（1.0x - 20.0x），实时显示加速后的时间，同时保留标准时间对照。灵感来自《加速世界》，界面以中文为主，提供农历、生肖、节气等中华传统文化元素。

> 当前状态：S1-S10 重构完成；两轮审计修复完成；**UI 2.0 完成（0.5.0.0），UI 打磨进行中（0.5.2.1 真机观感返工）**——Fluent 重写 + 接口层架构 + 多页导航 + Acrylic + 真渐变玻璃光场 + 环形表盘与城市矩阵；任务清单见 `x.progress.md`，已知问题见 `y.problems.md`，版本路线见 `m.milestone.md`。

## 目录

- [特性](#特性)
- [快速开始](#快速开始)
- [使用说明](#使用说明)
- [项目结构](#项目结构)
- [依赖](#依赖)
- [贡献指南](#贡献指南)
- [常见问题](#常见问题)
- [许可证](#许可证)

## 项目展示

![项目截图](docs/images/demo.png)

启动应用程序即可查看同时显示标准时间和加速时间的实时时钟。

---

## 特性

- **时间膨胀计算**：根据可配置的加速倍率（1.0x - 20.0x）计算自定义时间流速
- **双界面支持**：同时支持命令行界面和图形界面（PyQt6 + Fluent Widgets）
- **实时时钟**：加速时间大字英雄区 + 标准时间对照 + 进度条可视化
- **侧栏多页导航**：时钟 / 倒计时 / 世界时钟 / 天气 / 闹钟 独立页面，设置页置底
- **农历信息**：显示天干地支年、生肖、时辰、月相、节气、公历节日、拜财神方向
- **世界时钟**：支持查看北京、东京、首尔、伦敦、巴黎、纽约、洛杉矶、悉尼等城市时间
- **倒计时功能**：支持设置目标时间倒计时显示
- **天气显示**：支持北京、上海、广州、深圳等 18 个主要城市天气（使用 Open-Meteo API）
- **闹钟功能**：支持设置多个闹钟，预设/自定义铃声，系统通知提醒
- **三态主题**：跟随系统（实时切换）/ 浅色 / 深色，设置页选择器或 Ctrl+T 循环
- **Acrylic 材质**：Windows 11 毛玻璃背板（不支持环境自动降级实底）
- **系统托盘**：支持隐藏到托盘后台运行，Fluent 风格右键菜单
- **配置持久化**：自动保存加速倍率、主题、城市、时区、闹钟等设置

## 快速开始

### 环境要求

- Python 3.10 或更高版本
- pip（Python 包管理器）

### 安装

```bash
# 克隆仓库
git clone https://github.com/pyapple12/AccelWorld.git
cd AccelWorld

# 安装依赖
pip install -r requirements.txt
```

## 使用说明

### 图形界面

```bash
# 默认启动（图形界面）
python main.py

# 指定加速倍率启动
python main.py --rate 3.0

# 启动后隐藏到托盘
python main.py --hidden

# 深色主题启动
python main.py --theme dark

# 指定城市启动
python main.py --city 上海
```

### 命令行模式

```bash
# 进入命令行交互模式
python main.py --cli

# 命令行模式指定倍率
python main.py --cli --rate 2.0
```

### 命令行参数

| 参数              | 说明                              |
| ----------------- | --------------------------------- |
| `--gui`           | 运行图形界面（默认）              |
| `--cli`           | 运行命令行界面                    |
| `--rate`, `-R`    | 加速倍率（1.0 - 20.0，默认：2.0） |
| `--theme`, `-T`   | 主题：`light` 或 `dark`           |
| `--city`, `-C`    | 默认显示城市                      |
| `--hidden`        | 启动后隐藏到托盘                  |
| `--version`, `-V` | 显示版本信息                      |

### GUI 操作说明

- **侧栏导航**：左侧边栏切换 时钟 / 倒计时 / 世界时钟 / 天气 / 闹钟 / 设置 六个页面
- **调节倍率**：时钟页拖动滑杆或输入数值后点击"应用加速"，也可点击工作/专注/睡眠预设
- **切换主题**：设置页选择 跟随系统/浅色/深色，或快捷键 Ctrl+T 三态循环；跟随系统时随 Windows 深浅色实时切换
- **毛玻璃**：Windows 11 下窗口自动启用 Acrylic 材质（壁纸渗透，失焦短暂收起属系统行为）
- **选择城市**：天气页从下拉框选择城市查看天气
- **设置倒计时**：倒计时页输入目标时间（支持 `YYYY-MM-DD`、`YYYY-MM-DD HH:MM` 或 `YYYY-MM-DD HH:MM:SS` 格式）
- **查看世界时钟**：世界时钟页从下拉框选择时区
- **托盘操作**：双击托盘图标显示窗口，右键弹出 Fluent 菜单，点击关闭按钮隐藏到托盘
- **快捷键**：Ctrl+S 保存 / Ctrl+Q 退出 / Ctrl+T 切换主题

## 项目结构

```
AccelWorld/
├── main.py                    # 主入口：装配 AppInterface → GUI（注入接口）/ CLI 分发
├── interface/                 # 接口层（UI 访问后端的唯一契约，无 Qt 依赖）
│   ├── app_interface.py       # AppInterface：时钟/倍率/天气/闹钟/时区/配置/几何七域方法
│   └── types.py               # 类型转出（TimeInfo/WeatherData/Alarm/PresetSound/UiPreferences）
├── modules/                   # 业务核心层（无 GUI 依赖，可独立测试）
│   ├── time_dilation.py       # 时间膨胀算法与 CLI 实时钟
│   ├── chinese_calendar.py    # 农历、干支、生肖、节气、节日
│   ├── weather_service.py     # 天气服务（Open-Meteo API，30 分钟缓存 + 重试）
│   └── alarm_service.py       # 闹钟模型与匹配逻辑（音频播放已迁 ui/audio_player.py）
├── config/
│   ├── settings.py            # 用户配置读写（UserConfig）
│   ├── user_config.json       # 用户配置（生成于项目内）
│   └── static/                # 应用静态配置（参数/UI 常量，json 驱动，零硬编码）
│       ├── config.json        # 引导映射表
│       ├── base.json          # 应用参数
│       ├── ui.json            # 字体/布局 tokens/颜色
│       └── static_config.py   # StaticConfig + get_static_config() 单例
├── logs/                      # 运行日志（app-*.log 每日文件 + crash-*.log 崩溃栈）
├── ui/                        # GUI 层（Fluent Widgets，零后端 import）
│   ├── main_window.py         # 主窗口装配器（FluentWindow 六导航页 + 三态主题）
│   ├── backdrop.py            # Acrylic 毛玻璃背板（DWM，Win11）
│   ├── system_tray.py         # 系统托盘（自绘图标/RoundMenu 菜单/原生通知）
│   ├── alarm_dialog.py        # 闹钟编辑对话框（MessageBoxBase）
│   ├── audio_player.py        # 闹钟音频播放（自定义铃声/异步分发）
│   ├── tools/                 # UI 层纯函数运算（倒计时解析/进度换算/文案格式化）
│   └── panels/                # 7 个面板（时钟/日期/倒计时/世界时钟/天气/闹钟/设置）
├── data/                      # 静态数据
│   ├── cities.py              # 城市经纬度表
│   ├── timezones.py           # 时区表（含夏令时标注）
│   └── weather_codes.py       # WMO 天气代码映射表
├── utils/                     # 通用工具
│   ├── logger.py              # 统一日志配置（每日独立文件）
│   ├── file_utils.py          # JSON 读写 + 缓存单例 + 项目根定位
│   ├── dataclass_utils.py     # dataclass 反序列化通用工具
│   └── retry.py               # 泛型重试函数
├── tests/                     # pytest 单元测试（54 用例）
├── requirements.txt           # Python 依赖列表
├── pyproject.toml             # 项目配置
├── AGENTS.md                  # Agent 协作规范（工程原则/代码规范/Commit 提交规范）
├── w.study.md                 # 项目分析报告（工作流四件套）
├── x.progress.md              # 任务清单（工作流四件套）
├── y.problems.md              # 已知问题（工作流四件套）
├── z.plan.md                  # 方案记录与审计附录（工作流四件套）
├── m.milestone.md             # 版本里程碑清单
├── LICENSE                    # GPL-3.0 许可证
└── README.md                  # 本文件
```

## 依赖

| 包名             | 版本      | 说明                           |
| ---------------- | --------- | ------------------------------ |
| PyQt6            | >= 6.6.0  | GUI 框架                       |
| lunar-python     | >= 0.0.9  | 农历、天干地支、生肖、节气计算 |
| chinese-calendar | >= 1.8.0  | 中国节假日信息                 |
| pytz             | >= 2024.1 | 世界时钟时区处理               |

## 贡献指南

欢迎各类贡献，包括 Bug 修复、功能开发、文档优化等。

### 贡献流程

1. Fork 本仓库
2. 创建特性分支（`git checkout -b feature/xxx`）
3. 提交修改（`git commit -m "feat: 新增xxx功能"`）
4. 推送分支（`git push origin feature/xxx`）
5. 发起 Pull Request

详细规范见 [CONTRIBUTING.md](./CONTRIBUTING.md)

## 常见问题

### Q1：加速倍率设置为多少最合适？

A：默认值为 2.0x，可根据个人需求在 1.0x 到 20.0x 之间调节。例如 2.0x 表示加速世界的一天等于现实 12 小时。建议从默认值开始，逐步调整找到适合自己的节奏。

### Q2：关闭程序后设置会丢失吗？

A：不会。程序会自动保存加速倍率、主题、城市、时区等设置到配置文件（`config/user_config.json`，位于项目目录内），下次启动时会自动恢复。

### Q3：支持查看哪些城市的天气？

A：支持北京、上海、广州、深圳、杭州、成都、武汉、南京、西安、重庆、天津、苏州、长沙、青岛、厦门、香港、台北等 18 个主要城市。天气数据来自 Open-Meteo 免费 API，无需 API Key。

### Q4：如何让程序在后台运行？

A：启动时使用 `--hidden` 参数，程序将直接隐藏到系统托盘运行。点击托盘图标可重新显示窗口，关闭窗口会最小化到托盘而非退出程序。

### Q5：倒计时功能支持哪些时间格式？

A：支持三种格式：

- `2025-12-31`（仅日期）
- `2025-12-31 23:59`（日期+小时分钟）
- `2025-12-31 23:59:59`（完整日期时间）

### Q6：世界时钟支持哪些时区？

A：支持北京、东京、首尔、伦敦、巴黎、纽约、洛杉矶、悉尼等 8 个常用时区，可通过下拉框快速切换。

### Q7：农历信息显示哪些内容？

A：显示天干地支年、生肖、当前时辰、月相、节气（如有）、公历节日（如有）、拜财神方向等丰富的中华传统文化元素。

## 维护者

- 作者：[pyapple12](https://github.com/pyapple12)
- 邮箱：takechance_bao@188.com

## 许可证

本项目基于 [GNU GPL v3 许可证](./LICENSE) 开源，允许自由使用、修改及分发，但必须保留源代码并以相同许可证发布。
