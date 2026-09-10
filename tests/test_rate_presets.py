# 倍率预设测试（T004.2 引入）
# 覆盖：预设定义合法性（来自 static 的范围校验）、预设按钮点击后配置与核心实例生效
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

# 子进程脚本：无头创建主窗口，逐个点击预设按钮，断言配置持久化与核心实例生效；
# 另建独立 ClockPanel 验证按钮点击经信号链发出对应倍率。
# 写盘断言前等待去抖定时器触发（FIX001.23 去抖；FIX002.19 以事件等待替代私有方法调用）；
# 天气查询打桩（FIX002.11：避免真实网络请求引入尾延迟与外部依赖）
_SUBPROCESS_SCRIPT = """
import os
import sys

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ["ACCELWORLD_CONFIG_FILE"] = sys.argv[2]
sys.path.insert(0, sys.argv[1])

from PyQt6.QtCore import QEventLoop, QTimer
from PyQt6.QtWidgets import QApplication

from config.settings import get_setting
from config.static.static_config import get_static_config
from ui.main_window import AcceleratedWorldGUI
from ui.panels.clock_panel import ClockPanel
import ui.panels.weather_panel as weather_panel

weather_panel.get_weather_by_city = lambda city_name: None  # 天气打桩（FIX002.11）

app = QApplication([])
window = AcceleratedWorldGUI()
presets = get_static_config().base["rate_presets"]

assert set(window.clock_panel.preset_buttons) == set(presets), "预设按钮集合不符"


def flush_rate_save():
    # 等待倍率写盘去抖定时器触发（FIX002.19：不调用私有 _flush_pending_rate）
    delay = int(get_static_config().base["rate_save_debounce_ms"]) + 150
    loop = QEventLoop()
    QTimer.singleShot(delay, loop.quit)
    loop.exec()


for name, rate in presets.items():
    window.clock_panel.preset_buttons[name].click()
    flush_rate_save()
    assert abs(get_setting("time_dilation_rate") - float(rate)) < 1e-9, (
        f"预设 {name} 后配置未生效: {get_setting('time_dilation_rate')}"
    )
    assert abs(window.accel_world.time_dilation_rate - float(rate)) < 1e-9, (
        f"预设 {name} 后核心实例未生效"
    )

panel = ClockPanel()
captured: list[float] = []
panel.rate_changed.connect(captured.append)
for name, rate in presets.items():
    captured.clear()
    panel.preset_buttons[name].click()
    assert captured and abs(captured[0] - float(rate)) < 1e-9, f"预设 {name} 未发倍率"

print("PRESET_OK", flush=True)
"""


def test_presets_defined_and_in_range():
    # 预设定义来自静态配置：非空、名称非空、倍率在配置范围内（零硬编码数据合法性）
    presets = _BASE["rate_presets"]
    assert presets, "rate_presets 不能为空"
    for name, rate in presets.items():
        assert name, "预设名不能为空"
        assert _BASE["rate_min"] <= float(rate) <= _BASE["rate_max"], (
            f"预设 {name}={rate} 超出 [{_BASE['rate_min']}, {_BASE['rate_max']}]"
        )


def test_preset_switch_config_effect(tmp_path):
    # 完整主窗口路径：子进程内点击预设 → 配置持久化 + 核心生效 + 信号链发倍率
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
