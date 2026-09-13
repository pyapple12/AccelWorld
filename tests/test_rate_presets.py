# 倍率滑杆驱动测试（T004.2 引入；减法轮热更新：预设按钮移除，改滑杆驱动等价覆盖；
# FIX004.11：base.json rate_presets 死键删除，本文件改用滑杆代表点等价驱动）
# 覆盖：滑杆代表点范围合法性、滑杆驱动后配置与核心实例生效
# Qt 相关断言放子进程执行：本机 GUI 进程退出期存在已知硬崩溃（见 y.problems#6），
# 子进程隔离保证 pytest 主进程退出码不受污染（用 stdout 标记断言，不用退出码）；
# 配置经 ACCELWORLD_CONFIG_FILE 环境变量重定向到临时目录（FIX001.12），不污染真实配置

import os
import subprocess
import sys
from pathlib import Path

from config.static.static_config import get_static_config

# 项目根：tests/test_rate_presets.py → tests/ → 项目根
_PROJECT_ROOT = Path(__file__).resolve().parent.parent

_BASE = get_static_config().base

# 滑杆驱动代表点（原预设值等价覆盖：下界/默认常用/十倍档；FIX004.11 改内联代表点）
_SLIDER_POINTS = (1.0, 2.0, 10.0)

# 子进程脚本：无头创建主窗口，驱动滑杆至代表点，断言配置持久化与接口内核心生效；
# 另建独立 ClockPanel 验证滑杆变更经信号链发出对应倍率。
# 写盘断言前等待去抖定时器触发（FIX001.23 去抖；FIX002.19 以事件等待替代私有方法调用）；
# 天气查询打桩在 interface 层（PL001.14：AppInterface.fetch_weather 类级替换，
# 覆盖全部实例，避免真实网络请求引入尾延迟与外部依赖）
_SUBPROCESS_SCRIPT = """
import os
import sys

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ["ACCELWORLD_CONFIG_FILE"] = sys.argv[2]
os.environ.setdefault("ACCELWORLD_FORCE_NO_GL", "1")  # 回归路径确定性（FIX004.17）
sys.path.insert(0, sys.argv[1])

from PyQt6.QtCore import QEventLoop, QTimer
from PyQt6.QtWidgets import QApplication

from config.settings import get_setting
from config.static.static_config import get_static_config
from interface import AppInterface
from ui.main_window import AcceleratedWorldGUI
from ui.panels.clock_panel import ClockPanel

AppInterface.fetch_weather = lambda self, city_name: None  # 天气打桩（interface 层，PL001.14）

app = QApplication([])
window = AcceleratedWorldGUI(AppInterface())


def flush_rate_save():
    # 等待倍率写盘去抖定时器触发（FIX002.19：不调用私有 _flush_pending_rate）
    delay = int(get_static_config().base["rate_save_debounce_ms"]) + 150
    loop = QEventLoop()
    QTimer.singleShot(delay, loop.quit)
    loop.exec()


for rate in (1.0, 2.0, 10.0):
    window.clock_panel.slider.setValue(int(round(rate * 10)))
    flush_rate_save()
    assert abs(get_setting("time_dilation_rate") - rate) < 1e-9, (
        f"滑杆驱动 {rate} 后配置未生效: {get_setting('time_dilation_rate')}"
    )
    assert abs(window._interface.get_rate() - rate) < 1e-9, (
        f"滑杆驱动 {rate} 后核心实例未生效"
    )

panel = ClockPanel(AppInterface())
captured: list[float] = []
panel.rate_changed.connect(captured.append)
for rate in (1.0, 2.0, 10.0):
    captured.clear()
    panel.slider.setValue(int(round(rate * 10)))
    assert captured and abs(captured[0] - rate) < 1e-9, f"滑杆 {rate} 未发倍率"

print("PRESET_OK", flush=True)
"""


def test_slider_points_defined_and_in_range():
    # 滑杆代表点在配置范围内（零硬编码范围校验；FIX004.11 前身为预设定义校验）
    assert _SLIDER_POINTS, "代表点不能为空"
    for rate in _SLIDER_POINTS:
        assert _BASE["rate_min"] <= float(rate) <= _BASE["rate_max"], (
            f"代表点 {rate} 超出 [{_BASE['rate_min']}, {_BASE['rate_max']}]"
        )


def test_preset_switch_config_effect(tmp_path):
    # 完整主窗口路径：子进程内驱动滑杆 → 配置持久化 + 核心生效 + 信号链发倍率
    config_file = tmp_path / "user_config.json"
    script = tmp_path / "preset_switch_check.py"
    script.write_text(_SUBPROCESS_SCRIPT, encoding="utf-8")
    env = {
        **os.environ,
        "QT_QPA_PLATFORM": "offscreen",
        "PYTHONIOENCODING": "utf-8",
        "ACCELWORLD_CONFIG_FILE": str(config_file),
    }
    result = subprocess.run(
        [sys.executable, str(script), str(_PROJECT_ROOT), str(config_file)],
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=120,
        cwd=_PROJECT_ROOT,
        env=env,
    )
    assert "PRESET_OK" in result.stdout, (
        f"子进程预设校验未通过\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )
