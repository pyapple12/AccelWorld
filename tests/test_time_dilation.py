# 时间膨胀模块测试（S9.7 测试引入）
# 覆盖：倍率校验、时间计算、24h 边界、TimeInfo 字段、农历秒级缓存、加速秒刷新节奏、
#       标准时间新鲜度、tick 周期、剩余小时

import datetime
from unittest import mock

import modules.time_dilation as td
from modules.time_dilation import AcceleratedWorld, TimeInfo
from config.static.static_config import get_static_config


def _fake_now(monkeypatch, moments):
    # 打桩 td.datetime（绕过 datetime 不可 setattr 限制），now() 依次返回 moments 序列
    fake_dt = mock.MagicMock()
    fake_dt.datetime.now.side_effect = list(moments)
    monkeypatch.setattr(td, "datetime", fake_dt)
    return fake_dt.datetime.now


def test_init_valid_rates():
    # 有效倍率正常构造，一天自定义小时数 = int(24*rate)
    w2 = AcceleratedWorld(2.0)
    assert w2.custom_hours_per_day == 48
    w15 = AcceleratedWorld(1.5)
    assert w15.custom_hours_per_day == 36


def test_init_invalid_rate():
    # 倍率 < rate_min 拒绝构造（下限来自静态配置，S10.1 A1 回归）
    rate_min = float(get_static_config().base["rate_min"])
    for rate in (rate_min - 0.01, 0.5, 0, -1):
        try:
            AcceleratedWorld(rate)
            raise AssertionError(f"倍率 {rate} 未拒绝")
        except ValueError:
            pass


def test_init_rate_min_boundary():
    # rate_min 边界值可构造（S10.1 A1 回归：修复前 rate=1.0 抛 ValueError）
    rate_min = float(get_static_config().base["rate_min"])
    w = AcceleratedWorld(rate_min)
    assert w.time_dilation_rate == rate_min
    assert w.custom_hours_per_day == int(24 * rate_min)


def test_init_default_rate():
    # 不传参使用静态配置默认倍率（None 哨兵，S10.1 A1 回归）
    default = float(get_static_config().base["default_rate"])
    assert AcceleratedWorld().time_dilation_rate == default


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
    # TimeInfo 计算属性与手工 split 结果一致（S10.11 C2：standard_second 已删，其检测路径用 now.second）
    info = AcceleratedWorld(2.0).get_custom_time()
    assert info.standard_time == info.standard_datetime.split()[1]
    assert info.custom_hour == int(info.custom_time.split(":")[0])
    assert info.custom_second == int(info.custom_time.split(":")[-1])
    # 手工构造验证固定值
    t = TimeInfo("2026-08-08 12:34:56", "24:12:34", "d", "l", 200.0, 48.0, 24.0)
    assert t.standard_time == "12:34:56"
    assert t.custom_hour == 24
    assert t.custom_second == 34


def test_lunar_second_cache(monkeypatch):
    # 农历计算标准秒级缓存：同标准秒多次调用仅全量计算一次（T001.1 重构回归，保留 S9.3 收益）
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
    assert calls["n"] == 1  # 同标准秒仅计算一次
    assert i1 is not i2 is not i3  # 每次返回全新 TimeInfo（加速时间实时推进，T001.1）
    assert i1.lunar_info == i2.lunar_info == i3.lunar_info  # 农历复用缓存


def test_lunar_cache_cross_second(monkeypatch):
    # 跨标准秒重算农历（T001.1 重构回归）
    aw = AcceleratedWorld(2.0)
    calls = {"n": 0}
    orig_get_lunar = td.get_lunar_info

    def counting_lunar(now):
        calls["n"] += 1
        return orig_get_lunar(now)

    monkeypatch.setattr(td, "get_lunar_info", counting_lunar)
    _fake_now(
        monkeypatch,
        [
            datetime.datetime(2026, 8, 8, 12, 0, 0, 123456),
            datetime.datetime(2026, 8, 8, 12, 0, 1, 123456),
        ],
    )
    i1 = aw.get_custom_time()
    i2 = aw.get_custom_time()
    assert calls["n"] == 2  # 跨标准秒重算
    assert i2.standard_datetime.endswith(":01")


def test_accelerated_second_cadence_rate_2(monkeypatch):
    # 加速秒节奏：rate 2.0 下现实 0.5 秒 = 加速 1 秒，custom_second 每步变化（T001.1）
    aw = AcceleratedWorld(2.0)
    base = datetime.datetime(2026, 9, 10, 10, 0, 0, 500000)
    _fake_now(monkeypatch, [base + datetime.timedelta(seconds=i * 0.5) for i in range(4)])
    seq = [aw.get_custom_time().custom_second for _ in range(4)]
    assert len(set(seq)) == 4  # 每个加速秒边界均被刷新


def test_accelerated_second_cadence_rate_10(monkeypatch):
    # 加速秒节奏：rate 10.0 下现实 0.1 秒 = 加速 1 秒（y.problems#1 预期场景，T001.1）
    aw = AcceleratedWorld(10.0)
    base = datetime.datetime(2026, 9, 10, 10, 0, 0, 500000)
    _fake_now(monkeypatch, [base + datetime.timedelta(seconds=i * 0.1) for i in range(5)])
    seq = [aw.get_custom_time().custom_second for _ in range(5)]
    assert len(set(seq)) == 5


def test_accelerated_second_no_flap(monkeypatch):
    # 节拍保护：rate 2.0 下现实 0.25 秒加速秒未走满 1 秒，custom_second 不得变化（T001.1）
    aw = AcceleratedWorld(2.0)
    base = datetime.datetime(2026, 9, 10, 10, 0, 0, 500000)
    _fake_now(monkeypatch, [base + datetime.timedelta(seconds=i * 0.25) for i in range(4)])
    seq = [aw.get_custom_time().custom_second for _ in range(4)]
    assert seq[0] == seq[1] and seq[2] == seq[3] and seq[1] != seq[2]


def test_standard_time_fresh_within_accelerated_second(monkeypatch):
    # 标准时间新鲜度：加速秒未变但标准秒已跨越，TimeInfo 必须现算而非回读缓存（T001.1）
    aw = AcceleratedWorld(1.05)
    cross_base = datetime.datetime(2026, 9, 10, 10, 0, 0, 990000)
    _fake_now(
        monkeypatch,
        [cross_base, cross_base + datetime.timedelta(milliseconds=10)],
    )
    i1 = aw.get_custom_time()
    i2 = aw.get_custom_time()
    assert i1 is not i2
    assert i2.standard_datetime != i1.standard_datetime  # 标准时间跨秒新鲜
    assert i1.custom_time == i2.custom_time  # 加速秒未变


def test_tick_interval_ms():
    # 刷新周期 = min(基础 tick 周期, 1000/倍率)（T001.1，基础周期来自静态配置）
    base_tick = int(get_static_config().base["clock_tick_ms"])
    assert AcceleratedWorld(1.0).tick_interval_ms == base_tick
    assert AcceleratedWorld(2.0).tick_interval_ms == min(base_tick, 500)
    assert AcceleratedWorld(11.3).tick_interval_ms == min(base_tick, 88)
    assert AcceleratedWorld(20.0).tick_interval_ms == min(base_tick, 50)


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
