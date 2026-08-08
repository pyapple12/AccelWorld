#!/usr/bin/env python3
"""
加速世界 - 主程序入口文件（CLI/GUI 统一分发，用法示例见 --help epilog，S10.12 F3 去重）
"""

import argparse
import sys

from config.static.static_config import get_static_config
from utils.file_utils import get_project_root


def main() -> None:
    """主程序入口函数：初始化日志、解析参数、分发 CLI/GUI"""
    # 静态配置（倍率范围/默认值/日志路径等参数来源）
    base = get_static_config().base

    # 初始化统一日志（日志目录/保留天数从静态配置传入，utils 层零业务依赖 S10.4 D1）
    from utils.logger import setup_logging

    setup_logging(
        log_dir=get_project_root() / base["logs_dir"],
        backup_days=int(base["log_backup_days"]),
    )

    parser = argparse.ArgumentParser(
        description=f"加速世界 - 时间膨胀时钟工具 {base['version']}",
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
        help=f"时间膨胀倍率（{base['rate_min']}-{base['rate_max']}，默认{base['default_rate']}）",
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

    # 其他参数
    parser.add_argument(
        "--version", "-V", action="version", version=f"%(prog)s {base['version']}"
    )

    args = parser.parse_args()

    # 验证 --rate 参数（范围与 GUI 滑杆/CLI 一致，来自静态配置）
    if args.rate is not None and not (
        base["rate_min"] <= args.rate <= base["rate_max"]
    ):
        print(f"错误: --rate 参数必须在 {base['rate_min']} 到 {base['rate_max']} 之间")
        print("例如: python main.py --rate 2.0")
        sys.exit(1)

    # 判断运行模式（run_cli 一行别名已内联，S10.11 C4）
    if args.cli:
        # 运行命令行界面
        from modules.time_dilation import main_cli

        if args.rate is not None:
            main_cli(rate=args.rate)
        else:
            main_cli()
    else:
        # 运行图形界面
        from ui.main_window import main_gui

        # 构建启动参数（可选参数推导式过滤 None，hidden 布尔单独处理）
        gui_args = {
            k: v
            for k, v in {
                "rate": args.rate,
                "theme": args.theme,
                "city": args.city,
            }.items()
            if v is not None
        }
        if args.hidden:
            gui_args["hidden"] = True

        main_gui(**gui_args)


if __name__ == "__main__":
    main()


# ===== main.py 函数/常量说明 =====
# 版本号：单一来源在 config/static/base.json（base["version"]），main.py 的 --version/description
#   及各 UI 显示均从静态配置读取（版本迁移方案，代码零硬编码版本字符串）
# main() -> None: 主程序入口
#   输入：命令行参数（argparse）
#   逻辑步骤：读取静态配置 → 初始化日志 → 解析参数（--gui/--cli/--rate/--theme/--city/--hidden/--version）
#            → 验证 --rate 范围 → 分发 CLI（main_cli(rate=...)）或 GUI（main_gui(**gui_args)）
#   设计理由：入口收编 CLI/GUI 分发；版本号从 base.json 读取（单一来源，代码零硬编码）
#   异常处理：rate 越界打印错误并 sys.exit(1)
#   关联配置：utils/logger.py 日志初始化；modules/time_dilation.py CLI；ui/main_window.py GUI
