# 天气服务模块（S5 引入 30 分钟缓存 + 网络重试）
# 使用 Open-Meteo 免费天气 API（无需 API Key）
# API 文档: https://open-meteo.com/

import urllib.request
import urllib.error
import json
import logging
import time
from dataclasses import dataclass
from typing import Optional

# 配置日志
logger = logging.getLogger(__name__)

# 通用重试工具
from utils.retry import retry_call

# 城市配置表（经纬度）
from data.cities import CITIES

# 天气代码映射表
from data.weather_codes import WEATHER_CODE_INFO, UNKNOWN_WEATHER

# 天气结果内存缓存：城市名 → (缓存时间戳, WeatherData)
_weather_cache: dict[str, tuple[float, "WeatherData"]] = {}

# 缓存有效期（秒）
CACHE_TTL_SECONDS = 30 * 60


@dataclass
class WeatherData:
    """天气信息数据类（聚合 Open-Meteo 返回的天气字段）"""

    temperature: float  # 温度（℃）
    humidity: float  # 相对湿度（%）
    wind_speed: float  # 风速（km/h）
    apparent_temperature: float  # 体感温度（℃）
    weather_code: int  # WMO 天气代码
    weather: str  # 中文短名（如"晴"）
    description: str  # 中文完整描述
    icon: str  # emoji 图标

    def to_display(self) -> str:
        # 生成简洁展示文本（图标 + 温度 + 天气名）
        return f"{self.icon} {self.temperature:.1f}°C {self.weather}"


def _fetch_weather_data(url: str) -> dict:
    # 请求 Open-Meteo API 并解析 JSON（独立函数供 retry_call 重试）
    with urllib.request.urlopen(url, timeout=10) as response:
        return json.loads(response.read().decode("utf-8"))


def get_weather_by_coords(lat: float, lon: float) -> Optional[WeatherData]:
    """
    根据经纬度获取天气信息（网络请求带重试）

    :param lat: 纬度
    :param lon: 经度
    :return: WeatherData 天气信息数据类，失败返回 None
    """
    # 拼接 API URL，重试耗尽后统一返回 None
    try:
        # 使用 Open-Meteo API
        url = (
            f"https://api.open-meteo.com/v1/forecast?"
            f"latitude={lat}&longitude={lon}"
            f"&current=temperature_2m,relative_humidity_2m,weather_code,"
            f"wind_speed_10m,apparent_temperature"
            f"&timezone=auto"
        )

        # 网络错误自动重试（总尝试 3 次 = 首次 + 2 次重试）
        data = retry_call(
            _fetch_weather_data,
            url,
            retries=3,
            exceptions=(urllib.error.URLError, TimeoutError),
            delay=1.0,
        )

        current = data.get("current", {})
        weather_code = current.get("weather_code", 0)
        code_info = WEATHER_CODE_INFO.get(weather_code, UNKNOWN_WEATHER)

        return WeatherData(
            temperature=current.get("temperature_2m", 0),
            humidity=current.get("relative_humidity_2m", 0),
            wind_speed=current.get("wind_speed_10m", 0),
            apparent_temperature=current.get("apparent_temperature", 0),
            weather_code=weather_code,
            weather=code_info.name,
            description=code_info.description,
            icon=code_info.icon,
        )
    except Exception as e:
        logger.error(f"获取天气信息失败: {e}")
        return None


def get_weather_by_city(city_name: str) -> Optional[WeatherData]:
    """
    根据城市名获取天气信息（带 30 分钟内存缓存，仅缓存成功结果）

    :param city_name: 城市名
    :return: WeatherData 天气信息数据类，失败返回 None
    """
    # 命中缓存直接返回
    cached = _weather_cache.get(city_name)
    if cached and time.time() - cached[0] < CACHE_TTL_SECONDS:
        return cached[1]

    # 实际查询（失败不缓存，下次立即重试）
    city_info = CITIES.get(city_name)
    if not city_info:
        return None
    lat, lon = city_info
    result = get_weather_by_coords(lat, lon)
    if result is not None:
        _weather_cache[city_name] = (time.time(), result)
    return result


def clear_weather_cache() -> None:
    """清空天气缓存（供测试与手动刷新使用）"""
    # 直接清空模块级缓存字典
    _weather_cache.clear()


def format_weather_info(weather: Optional[WeatherData], city_name: str = "") -> str:
    """
    格式化天气信息为字符串

    :param weather: WeatherData 天气信息数据类
    :param city_name: 城市名
    :return: 格式化的天气字符串
    """
    # 空数据返回失败文案；否则拼装完整展示文本
    if not weather:
        return "天气信息获取失败"

    city = f"{city_name} " if city_name else ""
    return (
        f"{city}{weather.icon} {weather.weather} | "
        f"{weather.temperature:.1f}°C | "
        f"体感 {weather.apparent_temperature:.1f}°C | "
        f"湿度 {weather.humidity}% | "
        f"风力 {weather.wind_speed:.1f}km/h"
    )


def get_simple_weather(weather: Optional[WeatherData]) -> str:
    """
    获取简洁天气信息

    :param weather: WeatherData 天气信息数据类
    :return: 简洁天气字符串
    """
    # 委托 WeatherData.to_display，空值返回"天气未知"
    if not weather:
        return "天气未知"
    return weather.to_display()


# ===== modules/weather_service.py 函数/常量说明 =====
# WeatherData: dataclass，天气信息聚合类，to_display() 生成简洁文本
# _fetch_weather_data(url): 请求 API 并解析 JSON（供 retry_call 重试的可调用对象）
# get_weather_by_coords(lat, lon): 经纬度查询，URLError/TimeoutError 自动重试 2 次
# get_weather_by_city(city_name): 城市查询，30 分钟缓存（仅缓存成功，失败可立即重试）
# clear_weather_cache(): 清空缓存
# format_weather_info(weather, city_name): 完整展示文本
# get_simple_weather(weather): 简洁展示文本
#   设计理由：缓存减少 API 调用（对应 M09a）；失败不缓存保证网络恢复后及时更新
#   异常处理：网络/解析异常统一返回 None 并记录日志；重试达上限抛异常被外层捕获
#   关联配置：城市表 data/cities.py；天气代码表 data/weather_codes.py；重试工具 utils/retry.py
