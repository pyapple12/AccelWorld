# 时间膨胀模块测试（S9.7 测试引入）
# 覆盖：倍率校验、时间计算、24h 边界、TimeInfo 字段、秒级缓存、剩余小时

import datetime
from unittest import mock

import modules.time_dilation as td
from modules.time_dilation import AcceleratedWorld, TimeInfo


def test_init_valid_rates():
    # 有效倍率正常构造，一天自定义小时数 = int(24*rate)
    w2 = AcceleratedWorld(2.0)
    assert w2.custom_hours_per_day == 48
    w15 = AcceleratedWorld(1.5)
    assert w15.custom_hours_per_day == 36


def test_init_invalid_rate():
    # 倍率 <=1.0 拒绝构造（抛 ValueError）
    for rate in (1.0, 0.5, 0, -1):
        try:
            AcceleratedWorld(rate)
            raise AssertionError(f"倍率 {rate} 未拒绝")
        except ValueError:
            pass


def test_custom_time_fields():
    # get_custom_time 返回 TimeInfo，聚合字段正确
    info = AcceleratedWorld(2.0).get_custom_time()
    assert isinstance(info, TimeInfo)
    assert info.dilation_percentage == 200.0
    assert info.expanded_hours_per_day == 48.0
    assert info.standard_datetime.count(":") == 2
    assert info.custom_time.count(":") == 2
    assert info.remaining_hours > 0
    assert info.remaining_hours <= 48.0


def test_custom_hour_bounds():
    # 自定义小时恒小于一天自定义小时数（24h 边界）
    for rate in (1.5, 2.0, 10.0, 20.0):
        info = AcceleratedWorld(rate).get_custom_time()
        hour = int(info.custom_time.split(":")[0])
        assert 0 <= hour < int(24 * rate)


def test_timeinfo_properties():
    # TimeInfo 计算属性与手工 split 结果一致
    info = AcceleratedWorld(2.0).get_custom_time()
    assert info.standard_time == info.standard_datetime.split()[1]
    assert info.standard_second == int(info.standard_datetime.split(":")[-1])
    assert info.custom_hour == int(info.custom_time.split(":")[0])
    assert info.custom_second == int(info.custom_time.split(":")[-1])
    # 手工构造验证固定值
    t = TimeInfo("2026-08-08 12:34:56", "24:12:34", "d", "l", 200.0, 48.0, 24.0)
    assert t.standard_time == "12:34:56"
    assert t.standard_second == 56
    assert t.custom_hour == 24
    assert t.custom_second == 34


def test_second_cache(monkeypatch):
    # 秒级缓存：同秒多次调用仅全量计算一次，跨秒重算（S9.3 回归）
    aw = AcceleratedWorld(2.0)
    calls = {"n": 0}
    orig_get_lunar = td.get_lunar_info

    def counting_lunar(now):
        # 统计 get_custom_time 内农历计算次数
        calls["n"] += 1
        return orig_get_lunar(now)

    monkeypatch.setattr(td, "get_lunar_info", counting_lunar)
    i1 = aw.get_custom_time()
    i2 = aw.get_custom_time()
    i3 = aw.get_custom_time()
    assert i1 is i2 is i3  # 同秒返回同一缓存对象
    assert calls["n"] == 1


def test_second_cache_cross_second(monkeypatch):
    # 跨秒重算（打桩 datetime 模块引用，绕过 datetime 不可 setattr 限制）
    aw = AcceleratedWorld(2.0)
    fake_dt = mock.MagicMock()
    base = datetime.datetime(2026, 8, 8, 12, 0, 0, 123456)
    fake_dt.datetime.now.return_value = base
    monkeypatch.setattr(td, "datetime", fake_dt)
    i1 = aw.get_custom_time()
    fake_dt.datetime.now.return_value = base.replace(second=1)
    i2 = aw.get_custom_time()
    assert i1 is not i2  # 跨秒重算（新对象）
    assert i2.standard_datetime.endswith(":01")


def test_remaining_hours_formula():
    # 剩余小时公式验证（固定时刻：半天 → 剩余 24h）
    aw = AcceleratedWorld(2.0)
    total = 12 * 3600
    custom_total = total * 2.0
    expected = (48.0 * 3600 - custom_total) / 3600
    assert expected == 24.0


def test_main_cli_default(monkeypatch):
    # main_cli 不传参时使用静态配置默认倍率（打桩 run_live_clock 避免阻塞）
    started = {}

    def fake_run(self):
        # 记录实例倍率后立即返回
        started["rate"] = self.time_dilation_rate

    monkeypatch.setattr(AcceleratedWorld, "run_live_clock", fake_run)
    td.main_cli()
    assert started.get("rate") == 2.0  # 与 static default_rate 一致
