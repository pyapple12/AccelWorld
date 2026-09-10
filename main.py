#!/usr/bin/env python3
# 加速世界 - 主程序入口文件（CLI/GUI 统一分发，用法示例见 --help epilog，S10.12 F3 去重）
# PL001（plan#UI2.0）：本文件只做装配——GUI 分支构造 AppInterface 注入 main_gui；
# 静态参数经接口静态访问器读取（零 config import）；CLI 分支直连后端 main_cli
# （CLI 属后端自足入口，不涉接口，plan#UI2.0 合理例外）
import argparse
import logging
import sys
from typing import Any, Dict

from utils.file_utils import get_project_root
from utils.logger import setup_logging
from utils.monitor import install_crash_handler, install_excepthook, install_qt_message_handler
from interface import AppInterface
from modules.time_dilation import main_cli
from ui.main_window import main_gui


def _resolve_log_level(base: Dict[str, Any]) -> int:
    # 从静态配置解析日志级别（base["log_level"]，如 "INFO"/"DEBUG"），非法值回退 INFO（FIX001.24）
    level = logging.getLevelName(str(base.get("log_level", "INFO")).upper())
    return level if isinstance(level, int) else logging.INFO


def main() -> None:
    # 静态配置（倍率范围/默认值/日志路径等参数来源，经接口静态访问器，无实例副作用）
    base = AppInterface.get_app_static()

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

    # 核心参数（0.1 步进说明：生效值按 0.1 粒度吸附，FIX001.22）
    parser.add_argument(
        "--rate",
        "-R",
        type=float,
        default=None,
        help=f"时间膨胀倍率（{base['rate_min']}-{base['rate_max']}，步进 0.1，默认{base['default_rate']}）",
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

    # 初始化统一日志（日志目录/保留天数/级别从静态配置传入，utils 层零业务依赖 S10.4 D1）
    # 时机在参数解析后：--version/--help 等即刻返回的路径不产生日志文件副作用（FIX001.24）
    setup_logging(
        level=_resolve_log_level(base),
        log_dir=get_project_root() / base["logs_dir"],
        backup_days=int(base["log_backup_days"]),
    )

    # 装配运行监控三层（未捕获异常/Qt 警告/原生崩溃统一进日志，T002）
    # 时机在参数解析后：--version/--help 等即刻返回的路径不产生崩溃栈文件
    install_excepthook()
    install_qt_message_handler()
    install_crash_handler(get_project_root() / base["logs_dir"])

    # 判断运行模式（run_cli 一行别名已内联，S10.11 C4）
    if args.cli:
        # 运行命令行界面（顶层 import，无模块会 import main，按需加载收益不存在）
        if args.rate is not None:
            main_cli(rate=args.rate)
        else:
            main_cli()
    else:
        # 运行图形界面（装配 AppInterface 注入 GUI，plan#UI2.0 三大块装配点）
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

        main_gui(interface=AppInterface(), **gui_args)


if __name__ == "__main__":
    main()


# ===== main.py 函数/常量说明 =====
# 版本号：单一来源在 config/static/base.json（base["version"]），main.py 的 --version/description
#   经 AppInterface.get_app_static() 静态访问器读取（版本迁移方案，代码零硬编码版本字符串）
# main() -> None: 主程序入口
#   输入：命令行参数（argparse）
#   逻辑步骤：经接口静态访问器读取静态配置 → 解析参数
#            （--gui/--cli/--rate/--theme/--city/--hidden/--version）
#            → 验证 --rate 范围 → 初始化日志（级别/目录/保留期来自静态配置，FIX001.24 后移）
#            → 装配运行监控三层（T002：异常钩子/Qt 警告/崩溃栈）
#            → 分发 CLI（main_cli(rate=...)，直连后端例外）或 GUI（构造 AppInterface 注入
#            main_gui，plan#UI2.0 三大块装配点）
#   设计理由：入口只做装配不含业务；GUI 与后端经 interface 契约隔离（动 UI 不伤后端）；
#   get_app_static 为 staticmethod，argparse/--version 期读取零实例副作用（FIX001.24 保持）
#   异常处理：rate 越界打印错误并 sys.exit(1)；监控装配失败（OSError）按严格抛错暴露
#   关联配置：utils/logger.py 日志初始化；utils/monitor.py 运行监控；
#     modules/time_dilation.py CLI；interface/app_interface.py 应用接口；ui/main_window.py GUI
