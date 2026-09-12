# GL 能力探测与开关（capability）：gl_enabled（ui.json 开关）与 gl_available（环境探测）
# 与运算决定 GL 路径是否生效；探测结果进程内缓存（首查后不再重复创建上下文）

import ctypes
import os

from PyQt6.QtGui import QOffscreenSurface, QOpenGLContext, QSurfaceFormat
from PyQt6.QtWidgets import QApplication

from interface import AppInterface

_SM_REMOTESESSION = 0x1000  # GetSystemMetrics：非零表示远程桌面会话

_cached_available: bool | None = None


def gl_enabled() -> bool:
    # ui.json 开关（顶层 gl_enabled，默认 false；PL009 才开放 true）
    return bool(AppInterface.get_ui_static().get("gl_enabled", False))


def _detect_gl_available() -> bool:
    # 环境探测：远程桌面会话直接不可用；离屏创建 2.1+ 兼容上下文成功才算可用
    if os.environ.get("ACCELWORLD_FORCE_NO_GL"):
        return False
    windll = getattr(ctypes, "windll", None)
    if windll is None:
        return False
    try:
        if windll.user32.GetSystemMetrics(_SM_REMOTESESSION):
            return False
    except OSError:
        pass
    ctx = QOpenGLContext()
    surface = QOffscreenSurface()
    surface.create()
    if not ctx.create() or not surface.isValid():
        return False
    ok = ctx.makeCurrent(surface) is not None
    ctx.doneCurrent()
    return ok


def gl_available() -> bool:
    # 环境探测结果进程内缓存（首查后不再重复创建上下文）
    global _cached_available
    if _cached_available is None:
        _cached_available = _detect_gl_available()
    return _cached_available


def gl_active() -> bool:
    # GL 路径总开关：ui.json 开启 且 环境可用；需存在 QApplication 实例
    if QApplication.instance() is None:
        return False
    return gl_enabled() and gl_available()
