# 统一日志配置模块
# 提供根日志初始化，所有模块的 logging.getLogger(__name__) 自动继承统一 handler
# S9.5 定案：日志集中项目内 logs/，每天独立文件 app-YYYY-MM-DD.log，保留天数参数化

import datetime
import logging
import sys
from pathlib import Path

from utils.file_utils import get_project_root
from config.static.static_config import get_static_config

# 日志格式（时间/级别/模块/消息）
_LOG_FORMAT = "%(asctime)s %(levelname)s [%(name)s] %(message)s"

# 默认日志级别
_DEFAULT_LEVEL = logging.INFO

# 根 logger 配置标记（避免重复添加 handler）
_setup_done = False


class _DailyFileHandler(logging.FileHandler):
    """每日独立文件 handler：写日志时检查日期，跨天自动切换到新日期文件"""

    def __init__(self, log_dir: Path):
        # 初始化当日文件路径（文件名含日期戳）
        self.log_dir = Path(log_dir)
        self._today: datetime.date | None = None
        super().__init__(self._today_path(), encoding="utf-8")

    def _today_path(self) -> str:
        # 生成当天日志文件路径 logs/app-YYYY-MM-DD.log
        self._today = datetime.date.today()
        return str(self.log_dir / f"app-{self._today.isoformat()}.log")

    def emit(self, record: logging.LogRecord) -> None:
        # 跨天检查：日期变化则关闭旧流并重建新日期文件
        today = datetime.date.today()
        if today != self._today:
            self.close()
            self.baseFilename = self._today_path()
            self._open()
        super().emit(record)


def _cleanup_old_logs(log_dir: Path, backup_days: int) -> None:
    # 删除超过保留天数的 app-*.log 文件（按文件名日期戳判断）
    today = datetime.date.today()
    for f in Path(log_dir).glob("app-*.log"):
        try:
            file_date = datetime.date.fromisoformat(f.stem[4:])  # 去掉 "app-" 前缀
        except ValueError:
            continue
        if (today - file_date).days > backup_days:
            try:
                f.unlink()
            except OSError:
                pass


def setup_logging(level: int = _DEFAULT_LEVEL) -> None:
    # 配置根 logger：控制台 + 每日文件双 handler，只执行一次
    global _setup_done
    if _setup_done:
        return

    root = logging.getLogger()
    root.setLevel(level)

    # 控制台 handler（stderr，中文输出需终端 UTF-8）
    console_handler = logging.StreamHandler(sys.stderr)
    console_handler.setFormatter(logging.Formatter(_LOG_FORMAT))
    root.addHandler(console_handler)

    # 文件 handler（项目内 logs/ 每日独立文件，参数来自静态配置）
    try:
        static = get_static_config()
        log_dir = get_project_root() / static.base["logs_dir"]
        log_dir.mkdir(parents=True, exist_ok=True)
        file_handler = _DailyFileHandler(log_dir)
        file_handler.setFormatter(logging.Formatter(_LOG_FORMAT))
        root.addHandler(file_handler)
        # 启动时清理过期日志
        _cleanup_old_logs(log_dir, static.base["log_backup_days"])
    except (OSError, RuntimeError):
        # 日志文件初始化失败（目录不可写/配置缺失）降级为仅控制台输出
        pass

    _setup_done = True


# ===== utils/logger.py 函数/常量说明 =====
# _DailyFileHandler(FileHandler): 每日独立文件 handler
#   _today_path(): 生成 logs/app-YYYY-MM-DD.log 路径
#   emit(): 每次写日志检查日期，跨天关闭旧流重建新文件（路径 2 定案）
# _cleanup_old_logs(log_dir, backup_days): 删除超过保留天数的 app-*.log
#   逻辑：按文件名日期戳解析 → (今天-文件日期).days > backup_days 则删除
# setup_logging(level): 初始化根 logger（控制台+每日文件双 handler）
#   设计理由：logging handler 属根 logger，各模块 getLogger 自动继承；
#   日志参数（logs_dir/log_backup_days）来自 config/static/base.json（零硬编码）
#   异常处理：文件 handler 创建失败仅降级控制台，不阻断程序
#   关联配置：logs_dir/log_backup_days 来自 config/static/base.json；由 main.py 启动时调用
