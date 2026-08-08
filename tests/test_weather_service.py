# 天气服务模块测试（S9.7 测试引入）
# 覆盖：缓存命中/过期、重试机制、窄捕获降级、编程错误上抛、格式化容错、未知城市

import json
import time

import modules.weather_service as weather_service
from modules.weather_service import WeatherData


def _set_fetch(monkeypatch, func):
    # 覆盖 conftest 的默认打桩，模拟指定网络行为
    monkeypatch.setattr(weather_service, "_fetch_weather_data", func)
    weather_service.clear_weather_cache()


def test_format_empty():
    # 空值容错
    assert weather_service.format_weather_info(None) == "天气信息获取失败"


def test_format_full():
    # 完整展示文本（S10.11 C1：原 to_display 测试改测 format_weather_info）
    wd = WeatherData(20.0, 50, 5.0, 21.0, 0, "晴", "晴朗无云", "☀️")
    assert weather_service.format_weather_info(wd) == (
        "☀️ 晴 | 20.0°C | 体感 21.0°C | 湿度 50% | 风力 5.0km/h"
    )


def test_cache_hit(monkeypatch):
    # 缓存命中：两次调用只发一次请求
    calls = {"n": 0}

    def fake(url):
        # 模拟成功响应并计数调用次数（验证缓存只发一次请求）
        calls["n"] += 1
        return {
            "current": {
                "temperature_2m": 20.0,
                "relative_humidity_2m": 50,
                "weather_code": 0,
                "wind_speed_10m": 5.0,
                "apparent_temperature": 21.0,
            }
        }

    _set_fetch(monkeypatch, fake)
    w1 = weather_service.get_weather_by_city("北京")
    w2 = weather_service.get_weather_by_city("北京")
    assert w1 is not None and w2 is w1
    assert calls["n"] == 1


def test_cache_expiry(monkeypatch):
    # 缓存过期后重新请求
    calls = {"n": 0}

    def fake(url):
        # 模拟成功响应并计数调用次数（验证过期后重新请求）
        calls["n"] += 1
        return {
            "current": {
                "temperature_2m": 20.0,
                "relative_humidity_2m": 50,
                "weather_code": 0,
                "wind_speed_10m": 5.0,
                "apparent_temperature": 21.0,
            }
        }

    _set_fetch(monkeypatch, fake)
    weather_service.get_weather_by_city("北京")
    assert calls["n"] == 1
    # 模拟过期
    weather_service._weather_cache["北京"] = (
        time.time() - 7200,
        weather_service._weather_cache["北京"][1],
    )
    weather_service.get_weather_by_city("北京")
    assert calls["n"] == 2


def test_retry_success(monkeypatch):
    # 前 2 次失败第 3 次成功（总尝试 3 次，S5 回归）
    attempts = {"n": 0}

    def flaky(url):
        # 前 2 次抛超时、第 3 次返回成功（验证重试机制）
        attempts["n"] += 1
        if attempts["n"] < 3:
            raise TimeoutError("暂时失败")
        return {
            "current": {
                "temperature_2m": 30.0,
                "relative_humidity_2m": 50,
                "weather_code": 0,
                "wind_speed_10m": 5.0,
                "apparent_temperature": 31.0,
            }
        }

    _set_fetch(monkeypatch, flaky)
    result = weather_service.get_weather_by_city("广州")
    assert result is not None
    assert attempts["n"] == 3


def test_retry_exhausted(monkeypatch):
    # 重试耗尽返回 None
    def always_fail(url):
        # 恒定抛超时异常（验证重试耗尽返回 None）
        raise TimeoutError("一直失败")

    _set_fetch(monkeypatch, always_fail)
    assert weather_service.get_weather_by_city("广州") is None


def test_narrow_exception(monkeypatch):
    # 网络/解析类异常窄捕获降级（S9.1 回归）
    def timeout_fail(url):
        # 抛超时异常（验证窄捕获降级）
        raise TimeoutError("超时")

    _set_fetch(monkeypatch, timeout_fail)
    assert weather_service.get_weather_by_coords(39.9, 116.4) is None

    def json_fail(url):
        # 抛 JSON 解析异常（验证窄捕获降级）
        raise json.JSONDecodeError("bad", "doc", 0)

    _set_fetch(monkeypatch, json_fail)
    assert weather_service.get_weather_by_coords(39.9, 116.4) is None


def test_programming_error_raised(monkeypatch):
    # 编程错误（非网络类）上抛而非被吞（S9.1 回归）
    def bad_data(url):
        # 返回非 dict 数据（后续 .get 触发 AttributeError，验证编程错误上抛）
        return "not-a-dict"

    _set_fetch(monkeypatch, bad_data)
    try:
        weather_service.get_weather_by_coords(39.9, 116.4)
        raise AssertionError("编程错误被吞")
    except AttributeError:
        pass


def test_unknown_city(monkeypatch):
    # 未知城市返回 None（不发请求）
    calls = {"n": 0}

    def fake(url):
        # 模拟响应（未知城市实际不发请求，验证计数为 0）
        calls["n"] += 1
        return {"current": {}}

    _set_fetch(monkeypatch, fake)
    assert weather_service.get_weather_by_city("不存在的城市") is None
    assert calls["n"] == 0
