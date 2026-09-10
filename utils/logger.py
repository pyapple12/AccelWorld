# 统一日志配置模块
# 提供根日志初始化，所有模块的 logging.getLogger(__name__) 自动继承统一 handler
# S9.5 定案：日志集中项目内 logs/，每天独立文件 app-YYYY-MM-DD.log，保留天数参数化
# S10.4 D1：日志目录/保留天数由 main.py 从静态配置读取后传入，utils 层不再反向依赖 config

import datetime
import logging
import sys
from pathlib import Path

# 日志格式（时间/级别/模块/消息）
_LOG_FORMAT = "%(asctime)s %(levelname)s [%(name)s] %(message)s"

# 崩溃栈日志文件名前缀（单源常量，utils/monitor.py 引用同一份，FIX001.24）
CRASH_LOG_PREFIX = "crash-"

# 根 logger 配置标记（避免重复添加 handler）
# 默认级别/保留天数不在本模块维护：唯一来源是 config/static/base.json
# （log_level/log_backup_days），由 main.py 显式传入（FIX002.14 消除双源漂移）
_setup_done = False


class _DailyFileHandler(logging.FileHandler):
    def __init__(self, log_dir: Path):
        # 初始化当日文件路径（文件名含日期戳）
        self.log_dir = Path(log_dir)
        self._today: datetime.date | None = None
        self._today = datetime.date.today()
        super().__init__(self._path_for(self._today), encoding="utf-8")

    def _path_for(self, day: datetime.date) -> str:
        # 无副作用路径拼装（__init__/emit 共用，FIX002.16 消除双处维护）
        return str(self.log_dir / f"app-{day.isoformat()}.log")

    def emit(self, record: logging.LogRecord) -> None:
        # 跨天检查：日期变化则关闭旧流、切换新日期文件并显式重开（stream 必须自行赋值，
        # FileHandler._open 只返回流不落成员）；重开失败回退旧路径且 _today 保持旧值，
        # 本次跳过文件写入（仅控制台通道），后续每次 emit 自动重试直至恢复
        # （FIX001.18；Python 3.14 的 FileHandler.emit 不再代管 _open 异常与惰性重开）
        today = datetime.date.today()
        if today != self._today:
            old_path = self.baseFilename
            try:
                self.close()
                self.baseFilename = self._path_for(today)
                self.stream = self._open()
                self._today = today
            except OSError:
                self.baseFilename = old_path
        if self.stream is None:
            # 惰性重开（含上轮重开失败后的恢复路径）：仍失败则本次仅控制台通道
            try:
                self.stream = self._open()
            except OSError:
                return
        super().emit(record)


def _cleanup_old_logs(log_dir: Path, backup_days: int) -> None:
    # 删除超过保留天数的 app-*.log 与 crash-*.log 文件（按文件名日期戳判断，T002 新增 crash 前缀）
    today = datetime.date.today()
    for pattern in ("app-*.log", f"{CRASH_LOG_PREFIX}*.log"):
        for f in Path(log_dir).glob(pattern):
            try:
                file_date = datetime.date.fromisoformat(
                    f.stem.split("-", 1)[1]
                )  # 去掉 "app-"/"crash-" 前缀
            except (ValueError, IndexError):
                continue
            if (today - file_date).days > backup_days:
                try:
                    f.unlink()
                except OSError:
                    pass


def setup_logging(
    level: int,
    log_dir: Path | None = None,
    backup_days: int | None = None,
) -> None:
    # 配置根 logger：控制台 + 每日文件双 handler，只执行一次
    # level/backup_days 由调用方（main.py）从 base.json 显式传入（FIX002.14：取消模块内
    # 字面默认值，杜绝双源漂移）；log_dir 未提供时仅控制台输出
    global _setup_done
    if _setup_done:
        return

    root = logging.getLogger()
    root.setLevel(level)

    # 控制台 handler（stderr，中文输出需终端 UTF-8）
    console_handler = logging.StreamHandler(sys.stderr)
    console_handler.setFormatter(logging.Formatter(_LOG_FORMAT))
    root.addHandler(console_handler)

    # 文件 handler（每日独立文件；log_dir 未提供时仅控制台输出）
    if log_dir is not None:
        try:
            log_dir.mkdir(parents=True, exist_ok=True)
            file_handler = _DailyFileHandler(log_dir)
            file_handler.setFormatter(logging.Formatter(_LOG_FORMAT))
            root.addHandler(file_handler)
            # 启动时清理过期日志（backup_days 未传时跳过清理，调用方应显式传入）
            if backup_days is not None:
                _cleanup_old_logs(log_dir, backup_days)
        except OSError:
            # 日志文件初始化失败（目录不可写）降级为仅控制台输出
            pass

    _setup_done = True


# ===== utils/logger.py 函数/常量说明 =====
# CRASH_LOG_PREFIX: 崩溃栈日志前缀单源常量（monitor.py 共用，FIX001.24）
# _DailyFileHandler(FileHandler): 每日独立文件 handler
#   _path_for(day): 无副作用路径拼装（__init__/emit 共用，FIX002.16）
#   emit(): 每次写日志检查日期，跨天关闭旧流、切换新日期文件并显式重开（stream 自行赋值）；
#     重开失败回退旧路径并跳过本次文件写入，后续 emit 自动重试（FIX001.18）
# _cleanup_old_logs(log_dir, backup_days): 删除超过保留天数的 app-*.log 与 crash-*.log
#   逻辑：按文件名日期戳解析 → (今天-文件日期).days > backup_days 则删除
# setup_logging(level, log_dir, backup_days): 初始化根 logger（控制台+每日文件双 handler）
#   设计理由：logging handler 属根 logger，各模块 getLogger 自动继承；
#   日志级别/目录/保留天数由 main.py 从 base.json 显式传入，模块内不维护字面默认值
#   （FIX002.14 单源化）；log_dir 为 None 时仅控制台输出（文件日志降级），不阻断程序
#   异常处理：文件 handler 创建失败（OSError）仅降级控制台
#   关联配置：参数来源 config/static/base.json（log_level/logs_dir/log_backup_days）；
#   由 main.py 启动时传入
