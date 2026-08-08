# pytest 共享 fixture（S9.7 测试引入）
# 隔离策略：用户配置重定向到临时目录（不污染真实 user_config.json）+ 天气网络打桩（不发真实请求）

import pytest

import config.settings as settings
from utils.file_utils import clear_json_cache


@pytest.fixture(autouse=True)
def isolated_config(tmp_path, monkeypatch):
    # 用户配置隔离：CONFIG_DIR/CONFIG_FILE 指向 pytest 临时目录，并清理 file_utils 缓存
    # 设计理由：测试读写配置不污染项目内真实 user_config.json；缓存清理保证读到的始终是最新值
    monkeypatch.setattr(settings, "CONFIG_DIR", tmp_path)
    monkeypatch.setattr(settings, "CONFIG_FILE", tmp_path / "user_config.json")
    clear_json_cache()
    yield
    clear_json_cache()


@pytest.fixture(autouse=True)
def mock_weather(monkeypatch):
    # 天气网络打桩：patch weather_service 模块内 _fetch_weather_data，测试不依赖真实网络
    # 用例可再次 monkeypatch 覆盖该函数以模拟不同场景（重试/超时/编程错误）
    import modules.weather_service as weather_service

    def fake_fetch(url):
        # 模拟 Open-Meteo 成功响应
        return {
            "current": {
                "temperature_2m": 20.0,
                "relative_humidity_2m": 50,
                "weather_code": 0,
                "wind_speed_10m": 5.0,
                "apparent_temperature": 21.0,
            }
        }

    monkeypatch.setattr(weather_service, "_fetch_weather_data", fake_fetch)
    weather_service.clear_weather_cache()
    yield
    weather_service.clear_weather_cache()
