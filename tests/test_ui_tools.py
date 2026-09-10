# 时钟与闹钟展示运算工具测试（PL001.07 引入，纯单测无 Qt）
# 覆盖：进度条上界换算、重复闹钟文案、铃声文件名截断文案

from ui.tools.alarm_text import format_repeat_display, format_sound_button_name
from ui.tools.clock_tools import progress_bounds


# ------------------- clock_tools.progress_bounds -------------------


def test_progress_bounds_integer_hours():
    assert progress_bounds(48.0) == 48


def test_progress_bounds_truncates_fraction():
    # int() 截断语义与原 update_time 行为一致（13.7 → 13）
    assert progress_bounds(13.7) == 13
    assert progress_bounds(24.0 * 2.0) == 48


# ------------------- alarm_text.format_repeat_display -------------------


def test_format_repeat_display_one_time():
    assert format_repeat_display([]) == "一次"


def test_format_repeat_display_single_day():
    # weekday() 语义：0=周一 … 6=周日
    assert format_repeat_display([1]) == "周二"
    assert format_repeat_display([6]) == "周日"


def test_format_repeat_display_multiple_days():
    assert format_repeat_display([0, 1, 2]) == "周一二三"
    assert format_repeat_display([0, 6]) == "周一日"


# ------------------- alarm_text.format_sound_button_name -------------------


def test_format_sound_button_name_short_path():
    assert format_sound_button_name(r"C:/music/wake.wav") == "📁 wake.wav"


def test_format_sound_button_name_truncates_long_name():
    # 超长文件名截断 15 字符（与原 alarm_panel/alarm_dialog 行为一致）
    long_name = "a" * 20 + ".wav"
    assert format_sound_button_name(f"/tmp/{long_name}") == f"📁 {'a' * 15}"
