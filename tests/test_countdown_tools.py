# 倒计时展示运算工具测试（PL001.06 引入，纯单测无 Qt）
# 覆盖：三种目标文本格式解析、非法输入、剩余时间拆解与结束态判定

import datetime

from ui.tools.countdown_tools import format_remaining, parse_target_text


# ------------------- parse_target_text：三种合法格式 -------------------


def test_parse_full_format_19_chars():
    parsed = parse_target_text("2027-01-01 08:30:15")
    assert parsed == datetime.datetime(2027, 1, 1, 8, 30, 15)


def test_parse_minute_format_16_chars():
    parsed = parse_target_text("2027-01-01 08:30")
    assert parsed == datetime.datetime(2027, 1, 1, 8, 30, 0)


def test_parse_date_format_10_chars_defaults_end_of_day():
    parsed = parse_target_text("2027-01-01")
    assert parsed == datetime.datetime(2027, 1, 1, 23, 59, 59)


# ------------------- parse_target_text：非法输入 -------------------


def test_parse_invalid_text_returns_none():
    assert parse_target_text("not-a-date") is None
    assert parse_target_text("2027-13-40 99:99:99") is None  # 合法长度但越界值
    assert parse_target_text("2027-01-01 08:30:99") is None
    assert parse_target_text("") is None
    assert parse_target_text("202701010830151234") is None  # 长度合法但格式不符


# ------------------- format_remaining：剩余拆解与结束态 -------------------


def test_format_remaining_future_multi_days():
    now = datetime.datetime(2026, 9, 11, 12, 0, 0)
    target = datetime.datetime(2026, 9, 14, 15, 30, 5)
    text, finished = format_remaining(target, now)
    assert finished is False
    assert text == "3天 03:30:05"


def test_format_remaining_same_day_hours():
    now = datetime.datetime(2026, 9, 11, 12, 0, 0)
    target = datetime.datetime(2026, 9, 11, 14, 5, 9)
    text, finished = format_remaining(target, now)
    assert finished is False
    assert text == "0天 02:05:09"


def test_format_remaining_expired():
    now = datetime.datetime(2026, 9, 11, 12, 0, 0)
    target = datetime.datetime(2026, 9, 11, 11, 59, 59)
    text, finished = format_remaining(target, now)
    assert finished is True
    assert text == "00天 00:00:00"


def test_format_remaining_exact_now_is_finished():
    # 到点即视为结束（total_seconds <= 0 边界）
    now = datetime.datetime(2026, 9, 11, 12, 0, 0)
    text, finished = format_remaining(now, now)
    assert finished is True
    assert text == "00天 00:00:00"
