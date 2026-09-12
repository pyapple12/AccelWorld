# GUI 功能测试（FIX001 引入，T004 探针断言沉淀；PL003 起改为分阶段子进程）
# 覆盖：快捷键/主题双路径/六导航页/等宽数字/首次天气查询/铃声切换/倒计时恢复/
#       选择器交互/保存失败提示/写盘去抖/进度条动画/托盘悬停/列表外城市/--version
# 执行模式：GUI 断言在子进程内完成（本机 GUI 进程退出期硬崩溃见 y.problems#6），
# 以 stdout 末尾标记断言结果；配置经 ACCELWORLD_CONFIG_FILE 环境变量重定向到临时目录。
# PL003 实测：单进程累积 ≥4 个 FluentWindow 会触发窗口资源型硬崩（#6 家族变体），
# 故按"每子进程 ≤3 窗、贴近生产单窗形态"拆为三个阶段，各自独立断言标记

import os
import subprocess
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent

# 公共前导：环境/隔离/助手（各阶段共享，argv[1]=项目根 argv[2]=临时配置文件）
_STAGE_PRELUDE = """
import os
import sys

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ["ACCELWORLD_CONFIG_FILE"] = sys.argv[2]
sys.path.insert(0, sys.argv[1])

from pathlib import Path

from PyQt6.QtCore import QEventLoop, QTimer
from PyQt6.QtWidgets import QApplication, QSystemTrayIcon

app = QApplication([])

import config.settings as cs
from config.settings import get_setting, set_setting
from config.static.static_config import get_static_config
from interface import AppInterface
from interface.types import Alarm, TimeInfo
from ui.alarm_dialog import AlarmEditDialog
from ui.main_window import AcceleratedWorldGUI
import interface.app_interface as app_interface

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


def clear_and_reload():
    from utils.file_utils import clear_json_cache

    clear_json_cache()

# 注意：不要对本脚本创建的 FluentWindow 调用 deleteLater/close——本机（offscreen）
# 销毁无边框窗口会立即硬崩 0xC0000409（y.problems#6 家族）；窗口随进程退出释放
"""

# ------------------- 阶段 1：启动语义（3 窗：window0/window/天气窗） -------------------
_STAGE1 = _STAGE_PRELUDE + """

# FIX002.1 越界持久化倍率启动不崩：窗口创建前写入越界倍率，主窗口必须回退默认存活
Path(sys.argv[2]).write_text(
    __import__("json").dumps({"time_dilation_rate": 0.5}), encoding="utf-8"
)


def c_dirty_rate_startup():
    window0 = AcceleratedWorldGUI(AppInterface())
    assert abs(window0._interface.get_rate() - 2.0) < 1e-9, (
        f"越界倍率未回退默认: {window0._interface.get_rate()}"
    )
    return "越界持久化倍率 0.5 启动回退默认 2.0"


check("FIX002.1 越界倍率启动回退", c_dirty_rate_startup)

# PL005.01 倍率回显：持久化非默认倍率启动时，滑杆/倍率小标签与引擎一致
# （热更新减法轮：输入框已移除，回显面收敛为滑杆+标签）
from utils.file_utils import clear_json_cache as _clear_cache_pl005  # noqa: E402

Path(sys.argv[2]).write_text(
    __import__("json").dumps({"time_dilation_rate": 6.4}), encoding="utf-8"
)
_clear_cache_pl005()


def c_rate_echo_startup():
    window = AcceleratedWorldGUI(AppInterface())
    rate = window._interface.get_rate()
    slider_value = window.clock_panel.slider.value() / 10.0
    label_text = window.clock_panel.slider_value_label.text()
    percent_text = window.clock_panel.percent_label.text()
    assert abs(rate - 6.4) < 1e-9, f"引擎倍率异常: {rate}"
    assert abs(slider_value - 6.4) < 1e-9, f"滑杆回显 {slider_value} != 持久化倍率 6.4"
    assert label_text == "6.4x", f"倍率小标签回显 {label_text!r} != '6.4x'"
    assert percent_text == "膨胀倍率 640%", f"百分比回显 {percent_text!r} != '膨胀倍率 640%'"
    # 拖动跟随：滑杆变更后倍率大字与百分比同步刷新（热更新修复回归）
    window.clock_panel.slider.setValue(75)
    assert window.clock_panel.slider_value_label.text() == "7.5x", "倍率大字未跟随滑杆"
    assert window.clock_panel.percent_label.text() == "膨胀倍率 750%", "百分比未跟随滑杆"
    return f"滑杆 {slider_value}/标签 {label_text} 与引擎一致，拖动读数同步"


check("PL005.01 倍率回显启动一致", c_rate_echo_startup)

# FIX001.5 首次天气查询（interface 层打桩，PL001.14）
weather_calls = []


def fake_fetch_weather(self, city_name, force=False):
    weather_calls.append((city_name, force))
    return None


app_interface.AppInterface.fetch_weather = fake_fetch_weather


def c_first_weather_query():
    window = AcceleratedWorldGUI(AppInterface())
    process_events_ms(1500)
    assert weather_calls, "启动后未发起首次天气查询"
    return f"启动即查询 {weather_calls[0][0]!r}"


check("FIX001.5 启动首次天气查询", c_first_weather_query)

print("GUI_STAGE1_OK" if not failures else "GUI_STAGE1_FAIL", flush=True)
"""

# ------------------- 阶段 2：主窗口交互（3 窗） -------------------
_STAGE2 = _STAGE_PRELUDE + """

# 天气查询打桩（interface 层，避免真实网络）
app_interface.AppInterface.fetch_weather = lambda self, city_name: None

window = AcceleratedWorldGUI(AppInterface())


# T004.1/PL003.02 快捷键 + 主题双路径（设置页选择器 + 快捷键循环）
def c_shortcuts_and_theme_persist():
    from PyQt6.QtGui import QShortcut

    scs = {s.key().toString(): s for s in window.findChildren(QShortcut)}
    for key in (_BASE["shortcuts"][k] for k in ("save", "quit", "theme")):
        assert key in scs, f"缺快捷键 {key}"
    assert window.theme_pref == "auto", f"初始主题应为 auto: {window.theme_pref}"
    scs[_BASE["shortcuts"]["theme"]].activated.emit()  # auto→light
    assert window.theme_pref == "light", "auto→light 未循环"
    assert window.is_dark_theme is False, "light 下生效深浅应为浅"
    assert get_setting("theme") == "light", "主题切换未持久化"
    assert window.settings_panel.current_theme() == "light", "设置页选中态未同步"
    clear_and_reload()
    window2 = AcceleratedWorldGUI(AppInterface())
    assert window2.theme_pref == "light", "重启后主题未从配置恢复"
    scs[_BASE["shortcuts"]["theme"]].activated.emit()  # light→dark
    assert window.theme_pref == "dark" and window.is_dark_theme is True
    window.settings_panel.theme_switch.setCurrentItem("auto")  # 选择器路径直达
    assert window.theme_pref == "auto", "设置页选择器未生效"
    assert get_setting("theme") == "auto", "选择器路径未持久化"
    clear_and_reload()
    window3 = AcceleratedWorldGUI(AppInterface())
    assert window3.theme_pref == "auto", "auto 持久化恢复失败"
    return "三快捷键在位，设置页选择器与快捷键循环双路径均生效并持久化"


check("T004.1/PL003.02 主题双路径持久化", c_shortcuts_and_theme_persist)


# PL003.01 六导航页存在与切换
def c_six_pages_navigation():
    stacked = window.stackedWidget
    assert stacked.count() == 6, f"导航页数应为 6: {stacked.count()}"
    names = [stacked.widget(i).objectName() for i in range(stacked.count())]
    expected = {
        "page-clock", "page-countdown", "page-world",
        "page-weather", "page-alarm", "page-settings",
    }
    assert set(names) == expected, f"页容器命名不符: {names}"
    for target in (window.countdown_panel, window.weather_panel, window.settings_panel):
        window.switchTo(target.parent())
        assert stacked.currentWidget() is target.parent(), f"切换到 {target} 失败"
    window.switchTo(window.clock_panel.parent())  # 回时钟页
    return "六导航页存在（时钟/倒计时/世界时钟/天气/闹钟/设置）且切换正常"


check("PL003.01 六导航页切换", c_six_pages_navigation)


# PL003.04 英雄区等宽数字（走字宽度稳定）
def c_hero_tabular_digits():
    # 字体级验证：经实现的 _digit_font 纯函数构建同参字体（不碰活动控件——
    # PyQt6 对活动控件字体做 featureTags/metrics 存在原生崩溃 heisenbug，实测规避）
    from PyQt6.QtGui import QFontMetrics

    from ui.panels.clock_panel import _digit_font

    assert ":" in window.clock_panel.accelerated_time_label.text(), "英雄区时间标签未刷新"
    font = _digit_font("Microsoft YaHei", 56)
    fm = QFontMetrics(font)
    w1 = fm.horizontalAdvance("11:11:11")
    w2 = fm.horizontalAdvance("58:25:39")
    assert w1 == w2, f"走字宽度抖动: {w1} vs {w2}"
    return f"英雄区数字宽度稳定（{w1}px，雅黑数字天然等宽）"


check("PL003.04 英雄区等宽数字", c_hero_tabular_digits)


# FIX001.6 铃声类型切换（PL002：对话框需父窗口）
def c_sound_switch_back_to_preset():
    from PyQt6.QtWidgets import QWidget

    host = QWidget()  # MessageBoxBase 遮罩依赖父窗口（PL002.07）
    custom = Alarm(label="自定义", time="07:00", sound_type="custom",
                   sound_value=r"C:/music/wake.wav")
    dialog = AlarmEditDialog(host, alarm=custom, interface=AppInterface())
    assert "wake.wav" in dialog.custom_sound_button.text(), "自定义铃声未回填按钮文案"
    dialog.sound_combo.setCurrentIndex(3)  # Chime
    out = dialog.get_alarm()
    assert out.sound_type == "preset", "选预设后 sound_type 未复位"
    assert out.sound_value == "chime", f"铃声值错误: {out.sound_value}"
    return "自定义→预设切换生效且按钮回填文件名"


check("FIX001.6 铃声类型切换", c_sound_switch_back_to_preset)


# FIX001.10 倒计时恢复不清空
def c_countdown_restore_kept():
    marker = "2027-01-01 00:00:01"
    window.countdown_panel.restore_target(marker)
    assert window.countdown_panel.get_target_text() == marker, "恢复后 get_target_text 为空"
    window.save_settings()
    clear_and_reload()
    assert get_setting("countdown_target") == marker, "保存后配置值丢失"
    return "恢复→保存往返保留倒计时目标"


check("FIX001.10 倒计时恢复不清空", c_countdown_restore_kept)


# PL003.06 选择器交互 check（qfw DatePicker/TimePicker 对话框）
def c_pickers_construct_and_interact():
    from PyQt6.QtCore import QDate, QTime

    from ui.panels.countdown_panel import _DatePickDialog, _TimePickDialog

    date_dialog = _DatePickDialog(window, on_quick_picked=lambda days: None)
    assert date_dialog.calendar.getDate().isValid(), "DatePicker 初始日期非法"
    date_dialog.calendar.setDate(QDate(2027, 1, 1))
    assert date_dialog.calendar.getDate() == QDate(2027, 1, 1), "DatePicker setDate 失效"
    quick_calls: list[int] = []
    dialog2 = _DatePickDialog(window, on_quick_picked=quick_calls.append)
    dialog2.yesButton.click()  # 确定接线（不 exec，仅验证信号通路）
    time_dialog = _TimePickDialog(window, QTime(8, 30))
    got = time_dialog.time_picker.getTime()
    assert got.hour() == 8 and got.minute() == 30, f"TimePicker 预填异常: {got.toString()}"
    time_dialog.time_picker.setTime(QTime(23, 59))
    got2 = time_dialog.time_picker.getTime()
    assert got2.hour() == 23 and got2.minute() == 59, "TimePicker setTime 失效"
    for d in (date_dialog, dialog2, time_dialog):
        d.deleteLater()
    assert quick_calls == []
    return "日期/时间选择器对话框构建、读写与确定接线 OK（不进模态 exec）"


check("PL003.06 选择器交互", c_pickers_construct_and_interact)


# FIX001.19 保存失败上浮提示（桩点迁 settings 层）
def c_save_failure_notified():
    original_save = cs.save_config
    original_notify = window.tray.show_notification
    cs.save_config = lambda config: False
    notified = []

    def spy_notify(title, message, kind="info"):
        notified.append(title)

    window.tray.show_notification = spy_notify
    try:
        window.save_settings()
        assert notified, "保存失败未上浮托盘提示"
        return f"保存失败触发提示: {notified[0]!r}"
    finally:
        cs.save_config = original_save
        window.tray.show_notification = original_notify


check("FIX001.19 保存失败上浮提示", c_save_failure_notified)


# FIX001.23 写盘去抖 + 双发消除（桩点迁 settings 层）
def c_slider_write_debounce():
    write_calls = []
    emit_calls = []

    def fake_set_setting(key, value):
        if key == "time_dilation_rate":
            write_calls.append(value)
        return True

    def sink(rate):
        emit_calls.append(rate)

    original_set_setting = cs.set_setting
    cs.set_setting = fake_set_setting
    window.clock_panel.rate_changed.connect(sink)
    try:
        for rate in (3.0, 4.0, 5.0):
            window.clock_panel.set_rate(rate)
        process_events_ms(50)
        immediate = len(write_calls)
        process_events_ms(int(_BASE["rate_save_debounce_ms"]) + 250)

        # 双发消除：滑杆驱动路径（应用加速按钮已随减法移除）
        window.clock_panel.slider.setValue(60)
        process_events_ms(int(_BASE["rate_save_debounce_ms"]) + 250)
    finally:
        cs.set_setting = original_set_setting
        window.clock_panel.rate_changed.disconnect(sink)

    assert immediate == 0, f"拖动未去抖，立即写盘 {immediate} 次"
    # 滑杆驱动单次变更单发，写盘由去抖归并单次
    assert emit_calls.count(6.0) == 1, f"应用加速发次数异常: {emit_calls}"
    assert write_calls and write_calls[-1] == 6.0, f"去抖后未落盘: {write_calls}"
    return "滑杆驱动单次变更、写盘去抖归并"


check("FIX001.23 写盘去抖与双发消除", c_slider_write_debounce)


# T004 沉淀：进度条动画（FIX002.8 全确定性，时间无关）
def c_progress_animation():
    from PyQt6.QtCore import QPropertyAnimation

    window.timer.stop()
    duration = int(_BASE["progress_anim_ms"])
    common = dict(
        standard_datetime="2026-09-10 12:00:00",
        chinese_date="2026年09月10日 星期四",
        lunar_info="农历七月廿九",
        dilation_percentage=200.0,
        expanded_hours_per_day=48.0,
        remaining_hours=34.25,
    )
    info_a = TimeInfo(custom_time="05:00:00", **common)
    info_b = TimeInfo(custom_time="13:45:10", **common)

    # 阶段一：收敛到已知值 10（custom_hour=5 / 膨胀日 48 → round(5/48*100)，PL007.01 环形）
    window.clock_panel.update_time(info_a)
    process_events_ms(duration + 250)
    assert window.clock_panel.progress_ring.value() == 10, (
        f"阶段一未收敛: {window.clock_panel.progress_ring.value()}"
    )

    # 阶段二：目标 27 与当前 10 不同 → 动画必须处于 Running（跳变实现无动画对象/状态）
    window.clock_panel.update_time(info_b)
    anim = window.clock_panel._progress_anim
    assert anim.state() == QPropertyAnimation.State.Running, "更新后动画未运行"
    assert anim.duration() == duration, f"动画时长 {anim.duration()} != 配置"
    process_events_ms(duration + 250)
    assert window.clock_panel.progress_ring.value() == 27, "动画终值未收敛"
    return f"Running 态 + 时长 {duration}ms + 环形终值收敛（平滑非跳变）"


check("T004.3/PL007.01 环形进度动画沉淀", c_progress_animation)


# T004 沉淀：托盘悬停
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


# PL007.02/03/04 仪表化落位：世界矩阵/常用目标/天气卡（复用本阶段窗口）
def c_pl007_instrumentation():
    # 世界矩阵：常驻 8 城卡片 + 点击切换基准时区
    assert len(window.world_clock_panel._cards) == 8, "世界矩阵卡片数异常"
    window.world_clock_panel.set_timezone("Asia/Tokyo")
    assert window.world_clock_panel.current_timezone() == "Asia/Tokyo", "点击切换基准时区失败"
    window.world_clock_panel.set_timezone("Asia/Shanghai")

    # 倒计时：常用目标一键设置（12-25 → 当年/次年零点）并填充两级仪表
    window.countdown_panel._apply_quick_target("12-25")
    assert window.countdown_panel.countdown_target_date is not None, "常用目标未设置"
    assert window.countdown_panel.get_target_text().endswith("00:00:00"), "常用目标时间非零点"
    window.countdown_panel.update_countdown()
    assert window.countdown_panel.days_label.text() != "-- 天", "天数仪表未填充"

    # 天气卡：结构化结果直填温度大数字（WeatherData 真实 DTO 行为验证）
    from interface.types import WeatherData as _WD

    weather = _WD(
        temperature=24.0, humidity=60.0, wind_speed=9.5, apparent_temperature=25.0,
        weather_code=0, weather="晴", description="晴朗无云", icon="☀️",
    )
    window.weather_panel.current_city = "上海"
    window.weather_panel._on_weather_result("上海", weather)
    assert window.weather_panel.weather_temp_label.text() == "24°", "温度大数字未填充"
    assert window.weather_panel.humidity_label.text() == "湿度 60%", "湿度未填充"
    return "世界 8 城矩阵/常用目标一键倒计时/天气卡结构化直填"


check("PL007 仪表化落位", c_pl007_instrumentation)


# FIX001.23 列表外城市显示一致
def c_out_of_list_city_display():
    city_names = window._interface.get_city_names()
    outside = next(n for n in ("葛底斯堡", "小城测试") if n not in city_names)
    window.weather_panel.set_city(outside)
    assert window.weather_panel.city_combo.currentText() == outside, (
        "列表外城市下拉框显示不一致"
    )
    assert window.weather_panel.current_city == outside
    return "列表外城市下拉框同步展示"


check("FIX001.23 列表外城市显示", c_out_of_list_city_display)

print("GUI_STAGE2_OK" if not failures else "GUI_STAGE2_FAIL", flush=True)
"""

# ------------------- 阶段 3：持久化重启语义（2 窗） -------------------
_STAGE3 = _STAGE_PRELUDE + """

# FIX002.10 --theme light 生效（深色持久化下复位并持久化）
set_setting("theme", "dark")
clear_and_reload()
window3 = AcceleratedWorldGUI(AppInterface())
assert window3.is_dark_theme is True, "前置深色未恢复"
window3.apply_startup_args(theme="light")
assert window3.is_dark_theme is False, "--theme light 未生效"
clear_and_reload()
assert get_setting("theme") == "light", "--theme light 未持久化"
print("[PASS] FIX002.10 --theme light 生效", flush=True)

# FIX002.9 托盘初始倍率同步持久化值
set_setting("time_dilation_rate", 10.0)
clear_and_reload()
window4 = AcceleratedWorldGUI(AppInterface())
text = window4.tray.rate_action.text()
assert "10.0x" in text, f"托盘初始倍率未同步持久化值: {text!r}"
print(f"[PASS] FIX002.9 托盘初始倍率同步: {text!r}", flush=True)

print("GUI_STAGE3_OK", flush=True)
"""

_STAGE_SCRIPTS = {
    1: (_STAGE1, "GUI_STAGE1_OK"),
    2: (_STAGE2, "GUI_STAGE2_OK"),
    3: (_STAGE3, "GUI_STAGE3_OK"),
}


def test_gui_features_subprocess(tmp_path):
    # 分阶段子进程：每阶段独立进程/独立配置，单进程 ≤3 窗（规避窗口资源累积硬崩）；
    # stdout 末尾标记为通过依据（退出码受已知退出期崩溃污染，不作依据）
    env = {
        **os.environ,
        "QT_QPA_PLATFORM": "offscreen",
        "PYTHONIOENCODING": "utf-8",
    }
    for stage, (script, marker) in _STAGE_SCRIPTS.items():
        config_file = tmp_path / f"user_config_s{stage}.json"
        stage_env = {**env, "ACCELWORLD_CONFIG_FILE": str(config_file)}
        result = subprocess.run(
            [sys.executable, "-c", script, str(_PROJECT_ROOT), str(config_file)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=180,
            cwd=_PROJECT_ROOT,
            env=stage_env,
        )
        assert marker in result.stdout, (
            f"阶段 {stage} GUI 校验未通过\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
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
