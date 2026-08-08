# 统一日志配置模块
# 提供根日志初始化，所有模块的 logging.getLogger(__name__) 自动继承统一 handler

import logging
import sys

# 日志格式（时间/级别/模块/消息）
_LOG_FORMAT = "%(asctime)s %(levelname)s [%(name)s] %(message)s"

# 默认日志级别
_DEFAULT_LEVEL = logging.INFO

# 根 logger 配置标记（避免重复添加 handler）
_setup_done = False


def setup_logging(level: int = _DEFAULT_LEVEL) -> None:
    # 配置根 logger：控制台 + 文件双 handler，只执行一次
    global _setup_done
    if _setup_done:
        return

    root = logging.getLogger()
    root.setLevel(level)

    # 控制台 handler（stderr，中文输出需终端 UTF-8）
    console_handler = logging.StreamHandler(sys.stderr)
    console_handler.setFormatter(logging.Formatter(_LOG_FORMAT))
    root.addHandler(console_handler)

    # 文件 handler（写入用户配置目录旁的日志文件）
    try:
        from pathlib import Path

        log_dir = Path.home() / ".config" / "accelworld"
        log_dir.mkdir(parents=True, exist_ok=True)
        file_handler = logging.FileHandler(log_dir / "app.log", encoding="utf-8")
        file_handler.setFormatter(logging.Formatter(_LOG_FORMAT))
        root.addHandler(file_handler)
    except OSError:
        # 日志文件写入失败时降级为仅控制台输出
        pass

    _setup_done = True


def get_logger(name: str) -> logging.Logger:
    # 获取带模块名的 logger，根配置初始化后自动继承 handler
    setup_logging()
    return logging.getLogger(name)


# ===== utils/logger.py 函数/常量说明 =====
# setup_logging(level): 初始化根 logger（控制台+文件双 handler）
#   输入：日志级别 int；输出：None
#   逻辑步骤：设置根级别 → 添加控制台 handler → 尝试添加文件 handler（失败降级）
#   设计理由：logging 的 handler 属于根 logger，各模块 getLogger 自动继承，一处配置全局生效
#   异常处理：文件 handler 创建失败仅降级，不阻断程序
#   关联配置：日志文件写入 ~/.config/accelworld/app.log
# get_logger(name): 获取模块 logger
#   输入：模块名 str；输出：logging.Logger
#   设计理由：首次调用时完成根配置，保证任何模块打日志都有输出（修复 D12 日志静默问题）
#   关联配置：由 main.py 入口在启动时调用 setup_logging()
