# 农历/日期模块测试（S9.7 测试引入）
# 覆盖：干支/生肖/月日/时辰/节日/年份边界/格式化

import datetime

from modules.chinese_calendar import (
    get_chinese_lunar_calendar,
    get_chinese_date,
    get_lunar_info,
)


def test_known_date():
    # 已知日期断言：2026-08-08 丙午年/马/六月廿六/午时
    info = get_chinese_lunar_calendar(2026, 8, 8, 12)
    assert info.lunar_year == "丙午年"
    assert info.shengxiao == "马"
    assert info.lunar_month == "六月"
    assert info.lunar_day == "廿六"
    assert info.shichen == "午时"
    assert info.yue_phase  # 月相非空
    assert info.cai_shen_dir and info.position  # 财神方位非空


def test_chinese_date_format():
    # get_chinese_date 精确格式（YYYY年MM月DD日 星期X）
    assert get_chinese_date(datetime.datetime(2026, 8, 8)) == "2026年08月08日 星期六"
    assert get_chinese_date(datetime.datetime(2026, 8, 10)) == "2026年08月10日 星期一"


def test_shichen_bounds():
    # 时辰边界：0/23 子时、5 卯时、12 午时、17 酉时
    cases = {
        0: "子时",
        12: "午时",
        5: "卯时",
        17: "酉时",
        23: "子时",
    }
    for hour, expected in cases.items():
        info = get_chinese_lunar_calendar(2026, 8, 8, hour)
        assert info.shichen == expected, f"{hour} 时应为 {expected}"


def test_custom_holiday():
    # 自定义节日兜底：10-01 国庆节
    info = get_chinese_lunar_calendar(2026, 10, 1, 12)
    assert info.public_holiday == "国庆节"


def test_holiday_translation():
    # 英文节日翻译：元旦（1-01 由 lunar-python 提供 New Year's Day）
    info = get_chinese_lunar_calendar(2026, 1, 1, 12)
    assert info.public_holiday == "元旦"


def test_year_out_of_range():
    # 年份超出 chinese-calendar 支持范围 [2004, 2026] 时降级不崩溃（S8.5 回归）
    info_old = get_chinese_lunar_calendar(2003, 8, 8, 12)
    info_future = get_chinese_lunar_calendar(2030, 8, 8, 12)
    assert info_old.lunar_year and info_future.lunar_year


def test_lunar_info_string():
    # get_lunar_info 展示字符串包含关键信息
    text = get_lunar_info(datetime.datetime(2026, 8, 8, 12))
    assert "丙午年" in text
    assert "马" in text
    assert "拜财神" in text
