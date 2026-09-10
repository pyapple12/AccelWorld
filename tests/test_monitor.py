# 运行监控模块测试（FIX001.24 引入）
# 覆盖：excepthook 链式保留原钩子（不静默覆盖既有装配）

import sys

import utils.monitor as monitor


def test_excepthook_chains_previous_hook():
    # install_excepthook 后原 sys.excepthook 仍被链式调用（FIX001.24）
    calls = []

    def previous_hook(exc_type, exc_value, exc_tb):
        calls.append(exc_type)

    old_hook = sys.excepthook
    sys.excepthook = previous_hook
    try:
        monitor.install_excepthook()
        try:
            raise RuntimeError("链式验证")
        except RuntimeError:
            sys.excepthook(*sys.exc_info())
        assert calls and calls[0] is RuntimeError
    finally:
        sys.excepthook = old_hook
