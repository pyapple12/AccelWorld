#!/usr/bin/env python3
"""
加速世界 - 主程序入口文件

这是加速世界程序的统一入口点，允许用户选择运行命令行界面或图形界面。

使用方法：
  python main.py                          # 运行图形界面（默认）
  python main.py --gui                    # 运行图形界面
  python main.py --cli --rate 2.0         # 运行命令行界面，指定加速倍率
  python main.py --hidden                 # 启动后隐藏到托盘
  python main.py --theme dark             # 使用暗色主题

版本：ver 0.44
"""

import argparse
import sys

# 程序版本号（单一来源，所有模块从此处引用）
VERSION = "ver 0.44"


def main() -> None:
    """主程序入口函数：初始化日志、解析参数、分发 CLI/GUI"""
    # 初始化统一日志
    from utils.logger import setup_logging

    setup_logging()

    parser = argparse.ArgumentParser(
        description=f"加速世界 - 时间膨胀时钟工具 {VERSION}",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  python main.py                          # 启动图形界面
  python main.py --gui                    # 启动图形界面
  python main.py --cli --rate 3.0         # 启动命令行界面，倍率3.0
  python main.py --hidden                 # 启动并隐藏到托盘
  python main.py --theme dark             # 使用暗色主题
  python main.py --city 上海              # 默认显示上海天气
        """,
    )

    # 界面模式选择
    mode_group = parser.add_mutually_exclusive_group()
    mode_group.add_argument("--gui", action="store_true", help="运行图形界面（默认）")
    mode_group.add_argument("--cli", action="store_true", help="运行命令行界面")

    # 核心参数
    parser.add_argument(
        "--rate",
        "-R",
        type=float,
        default=None,
        help="时间膨胀倍率（必须大于1.0，默认2.0）",
    )

    # GUI 专属参数
    parser.add_argument(
        "--theme",
        "-T",
        choices=["light", "dark"],
        default=None,
        help="指定主题：light（浅色）或 dark（深色）",
    )
    parser.add_argument("--city", "-C", default=None, help="指定默认显示城市")
    parser.add_argument("--hidden", action="store_true", help="启动后隐藏到系统托盘")
    parser.add_argument(
        "--minimized", action="store_true", help="启动后最小化到系统托盘"
    )

    # 其他参数
    parser.add_argument(
        "--version", "-V", action="version", version=f"%(prog)s {VERSION}"
    )

    args = parser.parse_args()

    # 验证 --rate 参数
    if args.rate is not None and args.rate < 1.0:
        print("错误: --rate 参数必须大于或等于 1.0")
        print("例如: python main.py --rate 2.0")
        sys.exit(1)

    # 判断运行模式
    run_cli = args.cli or any(arg in sys.argv for arg in ["--cli", "-c"])

    if run_cli:
        # 运行命令行界面
        from modules.time_dilation import main_cli

        if args.rate is not None:
            main_cli(rate=args.rate)
        else:
            main_cli()
    else:
        # 运行图形界面
        from ui.main_window import main_gui

        # 构建启动参数
        gui_args = {}
        if args.rate is not None:
            gui_args["rate"] = args.rate
        if args.theme is not None:
            gui_args["theme"] = args.theme
        if args.city is not None:
            gui_args["city"] = args.city
        if args.hidden or args.minimized:
            gui_args["hidden"] = True

        main_gui(**gui_args)


if __name__ == "__main__":
    main()


# ===== main.py 函数/常量说明 =====
# VERSION: str，程序版本号单一来源（当前 ver 0.44），其他模块经 `from main import VERSION` 引用
# main() -> None: 主程序入口
#   输入：命令行参数（argparse）
#   逻辑步骤：初始化日志 → 解析参数（--gui/--cli/--rate/--theme/--city/--hidden/--version）
#            → 验证 --rate >= 1.0 → 分发 CLI（main_cli(rate=...)）或 GUI（main_gui(**gui_args)）
#   设计理由：入口收编 CLI/GUI 分发与 VERSION，模块间顶层 import 避免延迟导入
#   异常处理：rate 越界打印错误并 sys.exit(1)
#   关联配置：utils/logger.py 日志初始化；modules/time_dilation.py CLI；ui/main_window.py GUI
