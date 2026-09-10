# GUI 功能测试（FIX001 引入，T004 探针断言沉淀）
# 覆盖：快捷键/主题持久化/首次天气查询/铃声切换/倒计时恢复/保存失败提示/写盘去抖/
#       进度条动画/托盘悬停/列表外城市显示/--version 无日志副作用
# 执行模式：GUI 断言在子进程内完成（本机 GUI 进程退出期硬崩溃见 y.problems#6），
# 以 stdout 末尾标记断言结果；配置经 ACCELWORLD_CONFIG_FILE 环境变量重定向到临时目录

import os
import subprocess
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent

# 子进程脚本：argv[1]=项目根，argv[2]=临时配置文件路径（环境变量注入，FIX001.12）
_SUBPROCESS_SCRIPT = """
import os
import sys

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ["ACCELWORLD_CONFIG_FILE"] = sys.argv[2]
sys.path.insert(0, sys.argv[1])

import datetime

from PyQt6.QtCore import QEventLoop, QTimer
from PyQt6.QtWidgets import QApplication, QSystemTrayIcon

app = QApplication([])

from config.settings import get_setting
from config.static.static_config import get_static_config
from modules.alarm_service import Alarm
from modules.time_dilation import TimeInfo
from ui.alarm_dialog import AlarmEditDialog
from ui.main_window import AcceleratedWorldGUI
from ui.panels.countdown_panel import CountdownPanel
from ui.panels.weather_panel import WeatherPanel
import ui.main_window as mw
import ui.panels.weather_panel as wp

_BASE = get_static_config().base
failures = []


def check(name, fn):
    try:
        detail = fn()
        print(f"[PASS] {name}: {detail}", flush=True)
    except Exception as e:
        failures.append(name)
        print(f"[FAIL] {name}: {type(e).__name__}: {e}", flush=True)


def process_events_ms(ms):
    loop = QEventLoop()
    QTimer.singleShot(ms, loop.quit)
    loop.exec()


# ------------------- FIX001.5 首次天气查询 -------------------
weather_calls = []


def fake_city_weather(city_name):
    weather_calls.append(city_name)
    return None


wp.get_weather_by_city = fake_city_weather


def c_first_weather_query():
    window = AcceleratedWorldGUI()
    process_events_ms(1500)
    assert weather_calls, "启动后未发起首次天气查询"
    return f"启动即查询 {weather_calls[0]!r}"


check("FIX001.5 启动首次天气查询", c_first_weather_query)

window = AcceleratedWorldGUI()


# ------------------- T004 沉淀：快捷键 + 主题持久化（FIX001.11） -------------------
def c_shortcuts_and_theme_persist():
    from PyQt6.QtGui import QShortcut

    scs = {s.key().toString(): s for s in window.findChildren(QShortcut)}
    for key in (_BASE["shortcuts"][k] for k in ("save", "quit", "theme")):
        assert key in scs, f"缺快捷键 {key}"
    scs[_BASE["shortcuts"]["theme"]].activated.emit()
    assert window.is_dark_theme is True, "Ctrl+T 未翻转主题"
    assert get_setting("theme") == "dark", "主题切换未持久化"
    clear_and_reload()
    window2 = AcceleratedWorldGUI()
    assert window2.is_dark_theme is True, "重启后主题未从配置恢复"
    scs[_BASE["shortcuts"]["theme"]].activated.emit()  # 还原浅色
    return "三快捷键在位，主题持久化往返生效"


def clear_and_reload():
    from utils.file_utils import clear_json_cache

    clear_json_cache()


check("T004.1/FIX001.11 快捷键与主题持久化", c_shortcuts_and_theme_persist)


# ------------------- FIX001.6 铃声类型切换 -------------------
def c_sound_switch_back_to_preset():
    custom = Alarm(label="自定义", time="07:00", sound_type="custom",
                   sound_value=r"C:/music/wake.wav")
    dialog = AlarmEditDialog(alarm=custom)
    assert "wake.wav" in dialog.custom_sound_button.text(), "自定义铃声未回填按钮文案"
    dialog.sound_combo.setCurrentIndex(3)  # Chime
    out = dialog.get_alarm()
    assert out.sound_type == "preset", "选预设后 sound_type 未复位"
    assert out.sound_value == "chime", f"铃声值错误: {out.sound_value}"
    return "自定义→预设切换生效且按钮回填文件名"


check("FIX001.6 铃声类型切换", c_sound_switch_back_to_preset)


# ------------------- FIX001.10 倒计时恢复不清空 -------------------
def c_countdown_restore_kept():
    marker = "2027-01-01 00:00:01"
    window.countdown_panel.restore_target(marker)
    assert window.countdown_panel.get_target_text() == marker, "恢复后 get_target_text 为空"
    window.save_settings()
    clear_and_reload()
    assert get_setting("countdown_target") == marker, "保存后配置值丢失"
    return "恢复→保存往返保留倒计时目标"


check("FIX001.10 倒计时恢复不清空", c_countdown_restore_kept)


# ------------------- FIX001.19 保存失败上浮提示 -------------------
def c_save_failure_notified():
    original = mw.save_config
    mw.save_config = lambda config: False
    try:
        notified = []
        window.tray.show_notification = lambda t, m, kind="info": notified.append(t)
        window.save_settings()
        assert notified, "保存失败未上浮托盘提示"
        return f"保存失败触发提示: {notified[0]!r}"
    finally:
        mw.save_config = original


check("FIX001.19 保存失败上浮提示", c_save_failure_notified)


# ------------------- FIX001.23 写盘去抖 + 双发消除 -------------------
def c_slider_write_debounce():
    write_calls = []
    emit_calls = []

    def fake_set_setting(key, value):
        if key == "time_dilation_rate":
            write_calls.append(value)
        return True

    mw.set_setting = fake_set_setting
    window.clock_panel.rate_changed.connect(lambda r: emit_calls.append(r))

    for rate in (3.0, 4.0, 5.0):
        window.clock_panel.set_rate(rate)
    process_events_ms(50)
    immediate = len(write_calls)
    process_events_ms(int(_BASE["rate_save_debounce_ms"]) + 250)

    # 双发消除：应用加速按钮路径
    window.clock_panel.rate_entry.setText("6.0")
    window.clock_panel.confirm_button.click()
    process_events_ms(int(_BASE["rate_save_debounce_ms"]) + 250)

    assert immediate == 0, f"拖动未去抖，立即写盘 {immediate} 次"
    assert emit_calls.count(6.0) == 1, f"应用加速双发: {emit_calls}"
    assert write_calls and write_calls[-1] == 6.0, f"去抖后未落盘: {write_calls}"
    return f"拖动 0 次立即写盘、去抖后单次落盘、应用加速单发"


check("FIX001.23 写盘去抖与双发消除", c_slider_write_debounce)


# ------------------- T004 沉淀：进度条动画 -------------------
def c_progress_animation():
    window.timer.stop()
    duration = int(_BASE["progress_anim_ms"])
    info = TimeInfo(
        standard_datetime="2026-09-10 12:00:00",
        custom_time="13:45:10",
        chinese_date="2026年09月10日 星期四",
        lunar_info="农历七月廿九",
        dilation_percentage=200.0,
        expanded_hours_per_day=48.0,
        remaining_hours=34.25,
    )
    start_value = window.clock_panel.progress_bar.value()
    window.clock_panel.update_time(info)
    process_events_ms(duration // 2)
    mid_value = window.clock_panel.progress_bar.value()
    assert start_value < mid_value < info.custom_hour, f"半程 {mid_value} 疑似跳变"
    process_events_ms(duration)
    assert window.clock_panel.progress_bar.value() == info.custom_hour
    return f"半程 {mid_value} ∈ ({start_value}, {info.custom_hour})，平滑收敛"


check("T004.3 进度条动画沉淀", c_progress_animation)


# ------------------- T004 沉淀：托盘悬停 -------------------
def c_tray_tooltip():
    count = {"n": 0}
    real = QSystemTrayIcon.setToolTip

    def spy(self, text):
        count["n"] += 1
        real(self, text)

    QSystemTrayIcon.setToolTip = spy
    try:
        window.tray.update_tooltip("01:02:03", 5.0)
        assert "01:02:03" in window.tray.toolTip() and "5.0x" in window.tray.toolTip()
        n1 = count["n"]
        window.tray.update_tooltip("01:02:03", 5.0)
        assert count["n"] == n1, "相同文本重复 setToolTip"
        window.tray.update_tooltip("01:02:04", 5.0)
        assert count["n"] == n1 + 1
    finally:
        QSystemTrayIcon.setToolTip = real
    return "悬停随时间/倍率更新且去重"


check("T004.4 托盘悬停沉淀", c_tray_tooltip)


# ------------------- FIX001.23 列表外城市显示一致 -------------------
def c_out_of_list_city_display():
    outside = next(n for n in ("葛底斯堡", "小城测试") if n not in wp.CITIES)
    window.weather_panel.set_city(outside)
    assert window.weather_panel.city_combo.currentText() == outside, (
        "列表外城市下拉框显示不一致"
    )
    assert window.weather_panel.current_city == outside
    return "列表外城市下拉框同步展示"


check("FIX001.23 列表外城市显示", c_out_of_list_city_display)


print("GUI_FIX001_OK" if not failures else "GUI_FIX001_FAIL", flush=True)
"""


def test_gui_features_subprocess(tmp_path):
    # GUI 断言子进程：stdout 末尾标记为通过依据（退出码受已知退出期崩溃污染，不作依据）
    config_file = tmp_path / "user_config.json"
    env = {
        **os.environ,
        "QT_QPA_PLATFORM": "offscreen",
        "PYTHONIOENCODING": "utf-8",
        "ACCELWORLD_CONFIG_FILE": str(config_file),
    }
    result = subprocess.run(
        [sys.executable, "-c", _SUBPROCESS_SCRIPT, str(_PROJECT_ROOT), str(config_file)],
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=180,
        cwd=_PROJECT_ROOT,
        env=env,
    )
    assert "GUI_FIX001_OK" in result.stdout, (
        f"GUI 功能子进程校验未通过\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
    )


def test_version_flag_creates_no_log_file(tmp_path):
    # --version 即刻返回路径不产生当日日志文件（FIX001.24：setup_logging 移至参数解析后）
    logs_dir = _PROJECT_ROOT / "logs"
    before = set(logs_dir.glob("app-*.log")) if logs_dir.exists() else set()
    result = subprocess.run(
        [sys.executable, "main.py", "--version"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=60,
        cwd=_PROJECT_ROOT,
    )
    after = set(logs_dir.glob("app-*.log")) if logs_dir.exists() else set()
    assert result.returncode == 0, f"--version 异常退出: {result.stderr}"
    assert before == after, f"--version 产生日志文件副作用: {after - before}"
