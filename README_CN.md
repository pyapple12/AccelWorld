# AccelWorld

[![Version](https://img.shields.io/badge/Version-ver%200.11-blue.svg)](accelworld_calc.py)
[![Python](https://img.shields.io/badge/Python-3.8+-green.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[English](README.md) | [中文](README_CN.md)

---

## 概述

基于时间膨胀倍率的自定义小时制时间显示应用。

A time dilation clock application that calculates custom daily hours based on an acceleration expansion rate.

## 功能特性 Features

- **时间膨胀计算**: 根据可配置的时间膨胀倍率计算自定义时间
- **双界面支持**: 同时支持命令行界面和图形界面 (PyQt6)
- **实时时钟**: 同时显示标准时间和加速时间
- **中文日期支持**: 显示中文格式日期和农历信息
- **可调倍率**: 支持 1.0x 到 20.0x 的加速倍率调节

- **Time Dilation Calculation**: Calculate custom time based on a configurable acceleration rate
- **Dual Interface**: Support both command-line and graphical interfaces (PyQt6)
- **Real-time Clock**: Display both standard and accelerated time simultaneously
- **Chinese Date Support**: Show Chinese format dates and lunar calendar information
- **Customizable Rate**: Adjust acceleration rate from 1.0x to 20.0x

## 安装 Installation

```bash
pip install -r requirements.txt
```

## 使用 Usage

### 图形界面 Graphical Interface

```bash
python accelworld.py
```

### 命令行 Command Line

```bash
python accelworld.py --rate 2.0
```

## 项目结构 Project Structure

```
AccelWorld/
├── accelworld.py          # 主入口文件 Main entry point
├── accelworld_calc.py     # 核心计算逻辑 Core calculation logic
├── accelworld_date.py     # 日期和农历工具 Date and lunar calendar utilities
├── accelworld_gui.py      # PyQt6图形界面 PyQt6 graphical interface
├── requirements.txt       # Python依赖 Python dependencies
├── pyproject.toml         # 项目配置 Project configuration
└── README_CN.md           # 本文件 This file
```

## 依赖 Dependencies

- PyQt6 >= 6.6.0
- lunar-python >= 0.0.9
- chinese-calendar >= 1.8.0

## 许可证 License

MIT License
