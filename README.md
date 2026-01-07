# AccelWorld

[![Version](https://img.shields.io/badge/Version-ver%200.20-blue.svg)](accelworld_calc.py)
[![Python](https://img.shields.io/badge/Python-3.8+-green.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[English](README.md) | [中文](README_CN.md)

---

## Overview

A time dilation clock application that calculates custom daily hours based on an acceleration expansion rate.

基于时间膨胀倍率的自定义小时制时间显示应用。

## Features 功能特性

- **Time Dilation Calculation**: Calculate custom time based on a configurable acceleration rate
- **Dual Interface**: Support both command-line and graphical interfaces (PyQt6)
- **Real-time Clock**: Display both standard and accelerated time simultaneously
- **Chinese Date Support**: Show Chinese format dates and lunar calendar information
- **Customizable Rate**: Adjust acceleration rate from 1.0x to 20.0x

- **时间膨胀计算**: 根据可配置的时间膨胀倍率计算自定义时间
- **双界面支持**: 同时支持命令行界面和图形界面 (PyQt6)
- **实时时钟**: 同时显示标准时间和加速时间
- **中文日期支持**: 显示中文格式日期和农历信息
- **可调倍率**: 支持 1.0x 到 20.0x 的加速倍率调节

## Installation 安装

```bash
pip install -r requirements.txt
```

## Usage 使用

### Graphical Interface 图形界面

```bash
python accelworld.py
```

### Command Line 命令行

```bash
python accelworld.py --rate 2.0
```

## Project Structure 项目结构

```
AccelWorld/
├── accelworld.py          # Main entry point 主入口文件
├── accelworld_calc.py     # Core calculation logic 核心计算逻辑
├── accelworld_date.py     # Date and lunar calendar utilities 日期和农历工具
├── accelworld_gui.py      # PyQt6 graphical interface PyQt6图形界面
├── requirements.txt       # Python dependencies Python依赖
├── pyproject.toml         # Project configuration 项目配置
└── README.md              # This file 本文件
```

## Dependencies 依赖

- PyQt6 >= 6.6.0
- lunar-python >= 0.0.9
- chinese-calendar >= 1.8.0

## License 许可证

MIT License