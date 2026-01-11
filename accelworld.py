#!/usr/bin/env python3
"""
加速世界 - 主程序入口文件

这是加速世界程序的统一入口点，允许用户选择运行命令行界面或图形界面。

使用方法：
  python accelworld.py                      # 运行图形界面（默认）
  python accelworld.py --gui                # 运行图形界面
  python accelworld.py --cli --rate 2.0     # 运行命令行界面，指定加速倍率
  python accelworld.py --hidden             # 启动后隐藏到托盘
  python accelworld.py --theme dark         # 使用暗色主题

版本：ver 0.42
"""

import sys
import argparse

# 导入程序版本号
from accelworld_calc import VERSION


def main():
    """主程序入口函数"""
    parser = argparse.ArgumentParser(
        description=f"加速世界 - 时间膨胀时钟工具 {VERSION}",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  python accelworld.py                      # 启动图形界面
  python accelworld.py --gui                # 启动图形界面
  python accelworld.py --cli --rate 3.0     # 启动命令行界面，倍率3.0
  python accelworld.py --hidden             # 启动并隐藏到托盘
  python accelworld.py --theme dark         # 使用暗色主题
  python accelworld.py --city 上海          # 默认显示上海天气
        """
    )

    # 界面模式选择
    mode_group = parser.add_mutually_exclusive_group()
    mode_group.add_argument("--gui", action="store_true", help="运行图形界面（默认）")
    mode_group.add_argument("--cli", action="store_true", help="运行命令行界面")

    # 核心参数
    parser.add_argument(
        "--rate", "-R",
        type=float,
        default=None,
        help="时间膨胀倍率（必须大于1.0，默认2.0）"
    )

    # GUI 专属参数
    parser.add_argument(
        "--theme", "-T",
        choices=["light", "dark"],
        default=None,
        help="指定主题：light（浅色）或 dark（深色）"
    )
    parser.add_argument(
        "--city", "-C",
        default=None,
        help="指定默认显示城市"
    )
    parser.add_argument(
        "--hidden", action="store_true",
        help="启动后隐藏到系统托盘"
    )
    parser.add_argument(
        "--minimized", action="store_true",
        help="启动后最小化到系统托盘"
    )

    # 其他参数
    parser.add_argument(
        "--version", "-V",
        action="version",
        version=f"%(prog)s {VERSION}"
    )

    args = parser.parse_args()

    # 验证 --rate 参数
    if args.rate is not None and args.rate < 1.0:
        print("错误: --rate 参数必须大于或等于 1.0")
        print("例如: python accelworld.py --rate 2.0")
        sys.exit(1)

    # 判断运行模式
    run_cli = args.cli or any(arg in sys.argv for arg in ["--cli", "-c"])

    if run_cli:
        # 运行命令行界面
        from accelworld_calc import main_cli
        if args.rate:
            sys.argv = [sys.argv[0], f"--rate={args.rate}"]
        else:
            sys.argv = [sys.argv[0]]
        main_cli()
    else:
        # 运行图形界面
        from accelworld_gui import main_gui

        # 构建启动参数
        gui_args = {}
        if args.rate:
            gui_args["rate"] = args.rate
        if args.theme:
            gui_args["theme"] = args.theme
        if args.city:
            gui_args["city"] = args.city
        if args.hidden or args.minimized:
            gui_args["hidden"] = True

        main_gui(**gui_args)


if __name__ == "__main__":
    main()
