# 日志模块测试（FIX001 引入）
# 覆盖：跨天重开失败防护与自恢复、crash-*.log 同保留期清理（前缀常量单源）

import datetime
import logging

from utils.logger import CRASH_LOG_PREFIX, _DailyFileHandler, _cleanup_old_logs


def _make_record() -> logging.LogRecord:
    # 构造最小可用日志记录（emit 直调用）
    return logging.LogRecord("test", logging.INFO, __file__, 1, "msg", None, None)


def test_rollover_failure_degrades_and_recovers(tmp_path):
    # 跨天重开失败（目录不可写）不外抛，恢复后继续写文件（FIX001.18）
    handler = _DailyFileHandler(tmp_path)
    try:
        record = _make_record()
        handler.emit(record)  # 正常写入当日文件

        original_open = _DailyFileHandler._open

        def broken_open(self):
            # 模拟重开失败（目录被删/占用）
            raise OSError("denied")

        _DailyFileHandler._open = broken_open
        try:
            handler._today = datetime.date.today() - datetime.timedelta(days=1)
            handler.emit(record)  # 跨天分支触发重开失败，不应外抛
        finally:
            _DailyFileHandler._open = original_open

        handler.emit(record)  # 恢复后重开成功继续写
    finally:
        handler.close()

    content = "".join(
        p.read_text(encoding="utf-8") for p in sorted(tmp_path.glob("app-*.log"))
    )
    assert "msg" in content


def test_cleanup_removes_expired_app_and_crash_logs(tmp_path):
    # app-*.log 与 crash-*.log 同保留期清理，当日文件保留（FIX001.24 前缀单源回归）
    old_date = datetime.date.today() - datetime.timedelta(days=30)
    app_prefix = "app"
    crash_prefix = CRASH_LOG_PREFIX.rstrip("-")
    for prefix in (app_prefix, crash_prefix):
        stale = tmp_path / f"{prefix}-{old_date.isoformat()}.log"
        stale.write_text("x", encoding="utf-8")
    fresh = tmp_path / f"{crash_prefix}-{datetime.date.today().isoformat()}.log"
    fresh.write_text("y", encoding="utf-8")

    _cleanup_old_logs(tmp_path, 7)

    assert [p.name for p in tmp_path.iterdir()] == [fresh.name]
