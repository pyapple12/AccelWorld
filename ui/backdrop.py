# Acrylic 窗口背板模块（PL003：Win11 系统级亚克力材质，plan#UI2.0 视觉迭代）
# 策略（用户定案：只做 Acrylic，不做 Mica）：复用 qfw 底座透明机制
# （setMicaEffectEnabled 内部的 DwmExtendFrameIntoClientArea + HOSTBACKDROP + 背板类型 2），
# 再把 DWM 背板类型覆写为 3（TRANSIENTWINDOW = Acrylic）；失败静默降级实底背景

import ctypes
import os
import sys

# DWM 常量（Windows SDK dwmapi 头文件）
_DWMWA_SYSTEMBACKDROP_TYPE = 38
_DWMSBT_TRANSIENTWINDOW = 3  # Acrylic


def enable_acrylic(window) -> bool:
    # 开启 Win11 Acrylic 背板；不支持/失败静默降级实底背景并返回 False（调用方无须提示）
    # offscreen 平台（仅测试用）无原生窗口，DWM 试探会毒化进程内后续窗口（实测硬崩
    # 0xC0000409），必须最先短路——真实桌面不含此环境变量，行为不受影响
    if sys.platform != "win32" or os.environ.get("QT_QPA_PLATFORM") == "offscreen":
        return False
    try:
        # 底座：扩展框进客户区 + HOSTBACKDROP 貌 + 背板类型 2；qfw 内部按构建号守护（<Win11 空操作）
        window.setMicaEffectEnabled(True)
        value = ctypes.c_int(_DWMSBT_TRANSIENTWINDOW)
        hr = ctypes.windll.dwmapi.DwmSetWindowAttribute(
            int(window.winId()),
            _DWMWA_SYSTEMBACKDROP_TYPE,
            ctypes.byref(value),
            ctypes.sizeof(value),
        )
        if hr == 0:
            return True
    except OSError:
        pass
    # 失败降级：恢复实底背景，避免透明底座悬空成黑底
    window.setMicaEffectEnabled(False)
    return False


# ===== ui/backdrop.py 函数/常量说明 =====
# _DWMWA_SYSTEMBACKDROP_TYPE = 38: DWM 系统背板类型属性（Win11 22H2+）
# _DWMSBT_TRANSIENTWINDOW = 3: Acrylic 材质（2=Mica、4=Mica Alt，均不采用）
# enable_acrylic(window) -> bool
#   输入：FluentWindow 实例；输出：是否成功开启 Acrylic
#   逻辑步骤：qfw 透明底座（setMicaEffectEnabled=True，内部按构建号守护）
#   → DwmSetWindowAttribute 覆写背板类型为 3 → hr=0 即成功；
#   失败路径 setMicaEffectEnabled(False) 恢复实底，防透明底座悬空成黑底
#   设计理由：复用库内既有透明机制降低脆弱性；主题切换时主窗口重调本函数
#   （setMicaEffect 内部按 isDarkTheme 重取深浅色 tint）；无头/不支持环境返回 False
#   异常处理：仅捕获 OSError（ctypes 调用层），DWM 错误码走 hr 判断，不上抛
#   已知特性：DWM 材质不进截图/录屏；失焦时系统可能短暂收材质（可读性保护）
