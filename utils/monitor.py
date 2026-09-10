# 运行监控模块（T002：Python 未捕获异常 / Qt 原生警告 / 原生崩溃 三层统一进日志）
# 由 main.py 启动时装配，CLI/GUI 双模式生效；解决 GUI 无控制台下错误静默问题（y.problems#4）

import faulthandler
import logging
import sys
import threading
from datetime import date
from pathlib import Path

# Qt 消息管道（PyQt6 为项目硬依赖，main.py 顶层已加载，此处顶层 import 不增载重）
from PyQt6.QtCore import QtMsgType, qInstallMessageHandler

# 崩溃日志前缀单源常量（与 logger 清理逻辑共用一份，FIX001.24）
from utils.logger import CRASH_LOG_PREFIX

# 监控专用 logger（继承根 logger 的控制台+每日文件双通道）
logger = logging.getLogger(__name__)

# Qt 消息级别 → Python 日志级别映射
_QT_LEVEL_MAP = {
    QtMsgType.QtDebugMsg: logging.DEBUG,
    QtMsgType.QtInfoMsg: logging.INFO,
    QtMsgType.QtWarningMsg: logging.WARNING,
    QtMsgType.QtCriticalMsg: logging.CRITICAL,
    QtMsgType.QtFatalMsg: logging.FATAL,
}

# faulthandler 需要文件句柄保持打开（句柄被 GC 关闭后崩溃栈无法落盘）
_crash_log_file = None


def _log_uncaught(exc_type: type, exc_value: BaseException, exc_tb, source: str) -> None:
    # 统一落盘未捕获异常：critical 级别 + 完整堆栈（exc_info 三元组直接透传）
    logger.critical(
        f"未捕获异常（{source}）: {exc_type.__name__}: {exc_value}",
        exc_info=(exc_type, exc_value, exc_tb),
    )


def install_excepthook() -> None:
    # 安装全局异常钩子：主线程 sys.excepthook + 子线程 threading.excepthook
    # 原 Python 默认行为把堆栈打到 stderr，GUI 运行下不可见（修复 T002.1）；
    # 链式保留既有钩子（FIX001.24：直接覆盖会静默丢弃先行装配的钩子）
    previous_sys_hook = sys.excepthook
    previous_thread_hook = threading.excepthook

    def _hook(exc_type, exc_value, exc_tb):
        _log_uncaught(exc_type, exc_value, exc_tb, "sys.excepthook")
        if previous_sys_hook is not None and previous_sys_hook is not sys.__excepthook__:
            previous_sys_hook(exc_type, exc_value, exc_tb)

    def _thread_hook(args):
        # threading.excepthook 收到单个 ExcInfo 参数（exc_type/exc_value/exc_traceback/thread）
        _log_uncaught(
            args.exc_type, args.exc_value, args.exc_traceback,
            f"threading/{getattr(args.thread, 'name', '?')}",
        )
        if (
            previous_thread_hook is not None
            and previous_thread_hook is not threading.__excepthook__
        ):
            previous_thread_hook(args)

    sys.excepthook = _hook
    threading.excepthook = _thread_hook


def install_qt_message_handler() -> None:
    # 安装 Qt 原生消息处理器：qWarning/qCritical/QSS 解析失败等按级别转发进日志（修复 T002.2）
    def _handler(msg_type, context, message):
        level = _QT_LEVEL_MAP.get(msg_type, logging.WARNING)
        origin = f" [{context.file}:{context.line}]" if context.file else ""
        logger.log(level, f"Qt: {message}{origin}")

    qInstallMessageHandler(_handler)


def install_crash_handler(log_dir: Path) -> None:
    # 启用 faulthandler：原生崩溃（段错误等）栈自动写入 logs/crash-YYYY-MM-DD.log（修复 T002.3）
    log_dir = Path(log_dir)
    log_dir.mkdir(parents=True, exist_ok=True)
    crash_path = log_dir / f"{CRASH_LOG_PREFIX}{date.today().isoformat()}.log"
    global _crash_log_file
    _crash_log_file = open(crash_path, "a", encoding="utf-8")
    faulthandler.enable(file=_crash_log_file)
    logger.info(f"原生崩溃栈监控已启用: {crash_path}")


# ===== utils/monitor.py 函数/常量说明 =====
# CRASH_LOG_PREFIX: 自 utils/logger.py 导入（前缀单源，FIX001.24）
# _QT_LEVEL_MAP: Qt 五级消息 → Python 日志级别映射
# _crash_log_file: faulthandler 崩溃栈文件句柄（模块级持有防 GC 关闭）
# _log_uncaught(exc_type, exc_value, exc_tb, source): 未捕获异常统一 critical 落盘（带堆栈）
# install_excepthook(): 安装主线程/子线程双异常钩子（T002.1）
#   设计理由：GUI 下 stderr 不可见，钩子把未捕获异常转进 logs/ 每日日志；
#   链式调用既有钩子（FIX001.24，跳过 Python 内置默认钩子防重复输出），
#   只补日志不改异常流（钩子返回后按 Python/Qt 原有语义继续）
#   异常处理：钩子内部不抛错（logger.critical 落盘失败仅影响日志不影响退出流程）
# install_qt_message_handler(): 安装 Qt 消息处理器（T002.2）
#   设计理由：QSS 解析失败、属性警告等 Qt 原生输出默认走 stderr/调试器，GUI 下不可见
#   关联依赖：PyQt6.QtCore.qInstallMessageHandler（进程级全局处理器）
# install_crash_handler(log_dir): 启用 faulthandler 崩溃栈落盘（T002.3）
#   逻辑：创建日志目录 → 打开 crash-YYYY-MM-DD.log（追加）→ faulthandler.enable
#   设计理由：原生崩溃（访问违例等）绕过 Python 异常体系，faulthandler 是标准库唯一落栈手段；
#   Windows 下可配合事件查看器（应用程序日志）查崩溃码
#   异常处理：目录创建失败（OSError）向上抛出——监控装配失败属启动期错误，按严格抛错暴露
#   关联配置：log_dir 由 main.py 从 base.json 的 logs_dir 传入
