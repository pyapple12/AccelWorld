#!/usr/bin/env python3
"""
加速世界 - 主程序入口文件

这是加速世界程序的统一入口点，允许用户选择运行命令行界面或图形界面。

使用方法：
  python accelworld.py [--rate RATE] [-R RATE]
    运行命令行界面，指定加速倍率
  python accelworld.py
    运行图形界面

版本：ver 0.20
"""

import sys
import argparse

# 导入程序版本号
from accelworld_calc import VERSION

def main():
    """主程序入口函数"""
    parser = argparse.ArgumentParser(description=f"加速世界 - 时间膨胀时钟工具 {VERSION}")
    parser.add_argument(
        "--rate", "-R", 
        type=float, 
        default=2.0, 
        help="时间膨胀倍率（必须大于1.0，默认2.0）"
    )
    
    args = parser.parse_args()
    
    if len(sys.argv) > 1:
        # 如果有命令行参数，运行命令行界面
        from accelworld_calc import main_cli
        # 传递参数给命令行界面
        sys.argv = [sys.argv[0]] + [f"--rate={args.rate}"]
        main_cli()
    else:
        # 否则运行图形界面
        from accelworld_gui import main_gui
        main_gui()

if __name__ == "__main__":
    main()
