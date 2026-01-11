"""
天气服务模块

使用 Open-Meteo 免费天气 API（无需 API Key）
API 文档: https://open-meteo.com/
"""

import urllib.request
import json
import re
import logging
from typing import Optional, Tuple, Dict, Any

# 配置日志
logger = logging.getLogger(__name__)

# 默认城市（可配置）
DEFAULT_CITY = {
    "name": "北京",
    "latitude": 39.9042,
    "longitude": 116.4074
}

# 城市配置表（经纬度）
CITIES = {
    "北京": (39.9042, 116.4074),
    "上海": (31.2304, 121.4737),
    "广州": (23.1291, 113.2644),
    "深圳": (22.5431, 114.0579),
    "杭州": (30.2741, 120.1551),
    "成都": (30.5728, 104.0668),
    "武汉": (30.5928, 114.3055),
    "南京": (32.0603, 118.7969),
    "西安": (34.3416, 108.9398),
    "重庆": (29.5630, 106.5516),
    "天津": (39.1256, 117.1909),
    "苏州": (31.2989, 120.5853),
    "长沙": (28.2280, 112.9388),
    "青岛": (36.0671, 120.3826),
    "厦门": (24.4798, 118.0894),
    "香港": (22.3193, 114.1694),
    "台北": (25.0330, 121.5654),
}

# 天气代码映射（Open-Meteo WMO Weather interpretation codes）
WEATHER_CODES = {
    0: ("晴", "Clear sky", "☀️"),
    1: ("晴", "Mainly clear", "☀️"),
    2: ("多云", "Partly cloudy", "⛅"),
    3: ("阴", "Overcast", "☁️"),
    45: ("雾", "Fog", "🌫️"),
    48: ("雾", "Depositing rime fog", "🌫️"),
    51: ("小毛毛雨", "Light drizzle", "🌦️"),
    53: ("中毛毛雨", "Moderate drizzle", "🌦️"),
    55: ("稠密毛毛雨", "Dense drizzle", "🌧️"),
    56: ("冻毛毛雨", "Light freezing drizzle", "🌧️"),
    57: ("冻毛毛雨", "Dense freezing drizzle", "🌧️"),
    61: ("小雨", "Slight rain", "🌦️"),
    63: ("中雨", "Moderate rain", "🌧️"),
    65: ("大雨", "Heavy rain", "🌧️"),
    66: ("冻雨", "Light freezing rain", "🌧️"),
    67: ("冻雨", "Heavy freezing rain", "🌧️"),
    71: ("小雪", "Slight snow", "❄️"),
    73: ("中雪", "Moderate snow", "❄️"),
    75: ("大雪", "Heavy snow", "❄️"),
    77: ("雪粒", "Snow grains", "❄️"),
    80: ("小阵雨", "Slight rain showers", "🌦️"),
    81: ("中阵雨", "Moderate rain showers", "🌧️"),
    82: ("大阵雨", "Violent rain showers", "⛈️"),
    85: ("小阵雪", "Slight snow showers", "❄️"),
    86: ("大阵雪", "Heavy snow showers", "❄️"),
    95: ("雷暴", "Thunderstorm", "⛈️"),
    96: ("雷暴", "Thunderstorm with slight hail", "⛈️"),
    99: ("雷暴", "Thunderstorm with heavy hail", "⛈️"),
}

# 中文天气描述
WEATHER_DESCRIPTIONS = {
    0: "晴朗无云",
    1: "大致晴朗",
    2: "部分多云",
    3: "阴天",
    45: "有雾",
    48: "有雾凇",
    51: "轻度毛毛雨",
    53: "中度毛毛雨",
    55: "稠密毛毛雨",
    56: "轻度冻毛毛雨",
    57: "中度冻毛毛雨",
    61: "小雨",
    63: "中雨",
    65: "大雨",
    66: "轻度冻雨",
    67: "中度冻雨",
    71: "小雪",
    73: "中雪",
    75: "大雪",
    77: "雪粒",
    80: "小阵雨",
    81: "中阵雨",
    82: "大阵雨",
    85: "小阵雪",
    86: "大阵雪",
    95: "雷暴",
    96: "雷暴伴小冰雹",
    99: "雷暴伴大冰雹",
}


def get_weather_by_coords(lat: float, lon: float) -> Optional[Dict[str, Any]]:
    """
    根据经纬度获取天气信息

    :param lat: 纬度
    :param lon: 经度
    :return: 天气信息字典，失败返回 None
    """
    try:
        # 使用 Open-Meteo API
        url = (
            f"https://api.open-meteo.com/v1/forecast?"
            f"latitude={lat}&longitude={lon}"
            f"&current=temperature_2m,relative_humidity_2m,weather_code,"
            f"wind_speed_10m,apparent_temperature"
            f"&timezone=auto"
        )

        with urllib.request.urlopen(url, timeout=10) as response:
            data = json.loads(response.read().decode('utf-8'))

        current = data.get("current", {})
        weather_code = current.get("weather_code", 0)

        return {
            "temperature": current.get("temperature_2m", 0),
            "humidity": current.get("relative_humidity_2m", 0),
            "wind_speed": current.get("wind_speed_10m", 0),
            "apparent_temperature": current.get("apparent_temperature", 0),
            "weather_code": weather_code,
            "weather": WEATHER_CODES.get(weather_code, ("未知", "Unknown", "🌡️"))[0],
            "description": WEATHER_DESCRIPTIONS.get(weather_code, "未知天气"),
            "icon": WEATHER_CODES.get(weather_code, ("未知", "Unknown", "🌡️"))[2],
        }
    except Exception as e:
        logger.error(f"获取天气信息失败: {e}")
        return None


def get_weather_by_city(city_name: str) -> Optional[Dict[str, Any]]:
    """
    根据城市名获取天气信息

    :param city_name: 城市名
    :return: 天气信息字典，失败返回 None
    """
    city_info = CITIES.get(city_name)
    if city_info:
        lat, lon = city_info
        return get_weather_by_coords(lat, lon)
    return None


def format_weather_info(weather: Dict[str, Any], city_name: str = "") -> str:
    """
    格式化天气信息为字符串

    :param weather: 天气信息字典
    :param city_name: 城市名
    :return: 格式化的天气字符串
    """
    if not weather:
        return "天气信息获取失败"

    city = f"{city_name} " if city_name else ""
    return (
        f"{city}{weather['icon']} {weather['weather']} | "
        f"{weather['temperature']:.1f}°C | "
        f"体感 {weather['apparent_temperature']:.1f}°C | "
        f"湿度 {weather['humidity']}% | "
        f"风力 {weather['wind_speed']:.1f}km/h"
    )


def get_simple_weather(weather: Dict[str, Any]) -> str:
    """
    获取简洁天气信息

    :param weather: 天气信息字典
    :return: 简洁天气字符串
    """
    if not weather:
        return "天气未知"
    return f"{weather['icon']} {weather['temperature']:.1f}°C {weather['weather']}"


# ------------------- 测试 -------------------
if __name__ == "__main__":
    # 测试北京天气
    weather = get_weather_by_city("北京")
    if weather:
        print(f"北京天气: {format_weather_info(weather, '北京')}")
    else:
        print("获取天气失败")
