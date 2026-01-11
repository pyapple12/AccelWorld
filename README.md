# AccelWorld —— 加速世界世界钟

[![Version](https://img.shields.io/badge/Version-ver%200.41-blue.svg)](accelworld_calc.py)
[![Python](https://img.shields.io/badge/Python-3.8+-green.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

---

## 概述

基于时间膨胀倍率的自定义小时制时钟应用。根据设定的加速倍率（1.0x - 20.0x），实时显示加速后的时间，同时保留标准时间对照。灵感来自《加速世界》，界面支持中英文双语显示（GUI 以中文为主），提供农历、生肖、节气等中华传统文化元素。

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
- **双界面支持**：同时支持命令行界面和图形界面（PyQt6）
- **实时时钟**：同时显示标准时间和加速时间，带进度条可视化
- **农历信息**：显示天干地支年、生肖、时辰、月相、节气、公历节日、拜财神方向
- **世界时钟**：支持查看北京、东京、首尔、伦敦、巴黎、纽约、洛杉矶、悉尼等城市时间
- **倒计时功能**：支持设置目标时间倒计时显示
- **天气显示**：支持北京、上海、广州、深圳等 18 个主要城市天气（使用 Open-Meteo API）
- **闹钟功能**：支持设置多个闹钟，预设/自定义铃声，系统通知提醒
- **主题切换**：支持浅色和深色主题一键切换
- **系统托盘**：支持隐藏到托盘后台运行，关闭按钮最小化到托盘而非退出
- **配置持久化**：自动保存加速倍率、主题、城市、时区、闹钟等设置

## 快速开始

### 环境要求

- Python 3.8 或更高版本
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
python accelworld.py

# 指定加速倍率启动
python accelworld.py --rate 3.0

# 启动后隐藏到托盘
python accelworld.py --hidden

# 启动后最小化到托盘
python accelworld.py --minimized

# 深色主题启动
python accelworld.py --theme dark

# 指定城市启动
python accelworld.py --city 上海
```

### 命令行模式

```bash
# 进入命令行交互模式
python accelworld.py --cli

# 命令行模式指定倍率
python accelworld.py --cli --rate 2.0
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
| `--minimized`     | 启动后最小化到托盘                |
| `--version`, `-V` | 显示版本信息                      |

### GUI 操作说明

- **调节倍率**：拖动滑杆或输入数值后点击"应用加速"
- **切换主题**：点击天气区域的 🌙/☀️ 按钮
- **选择城市**：从下拉框选择城市查看天气
- **设置倒计时**：输入目标时间（支持 `YYYY-MM-DD`、`YYYY-MM-DD HH:MM` 或 `YYYY-MM-DD HH:MM:SS` 格式）
- **查看世界时钟**：从下拉框选择时区
- **托盘操作**：双击托盘图标显示窗口，点击关闭按钮隐藏到托盘

## 项目结构

```
AccelWorld/
├── accelworld.py            # 主入口文件，命令行参数解析
├── accelworld_calc.py       # 核心计算逻辑，时间膨胀算法
├── accelworld_date.py       # 日期和农历工具，天干地支、生肖、节气
├── accelworld_gui.py        # PyQt6图形界面，主窗口和组件
├── accelworld_config.py     # 配置文件管理，JSON格式持久化
├── accelworld_weather.py    # 天气服务，Open-Meteo API集成
├── accelworld_alarm.py      # 闹钟管理，多闹钟/自定义铃声/系统通知
├── requirements.txt         # Python依赖列表
├── pyproject.toml           # 项目配置
├── LICENSE                  # GPL-3.0 许可证
└── README.md                # 本文件
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

A：不会。程序会自动保存加速倍率、主题、城市、时区等设置到配置文件（`~/.config/accelworld/config.json`），下次启动时会自动恢复。

### Q3：支持查看哪些城市的天气？

A：支持北京、上海、广州、深圳、杭州、成都、武汉、南京、西安、重庆、天津、苏州、长沙、青岛、厦门、香港、台北等 18 个主要城市。天气数据来自 Open-Meteo 免费 API，无需 API Key。

### Q4：如何让程序在后台运行？

A：启动时使用 `--hidden` 或 `--minimized` 参数，程序将直接隐藏到系统托盘运行。点击托盘图标可重新显示窗口，关闭窗口会最小化到托盘而非退出程序。

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

## 贡献指南

欢迎各类贡献（Bug 修复、功能开发、文档优化），流程如下：

1. Fork 本仓库
2. 创建特性分支（`git checkout -b feature/xxx`）
3. 提交修改（`git commit -m "feat: 新增xxx功能"`）
4. 推送分支（`git push origin feature/xxx`）
5. 发起 Pull Request

详细规范见 [CONTRIBUTING.md]（待创建）
