# 天气代码映射表（Open-Meteo WMO Weather interpretation codes）
# WEATHER_CODES/WEATHER_DESCRIPTIONS 已合并为单表

from dataclasses import dataclass


@dataclass(frozen=True)
class WeatherCodeInfo:
    name: str  # 中文短名（如"晴"）
    icon: str  # emoji 图标
    description: str  # 中文完整描述（如"晴朗无云"）


# 天气代码映射（WMO 标准代码 → WeatherCodeInfo）
WEATHER_CODE_INFO = {
    0: WeatherCodeInfo("晴", "☀️", "晴朗无云"),
    1: WeatherCodeInfo("晴", "☀️", "大致晴朗"),
    2: WeatherCodeInfo("多云", "⛅", "部分多云"),
    3: WeatherCodeInfo("阴", "☁️", "阴天"),
    45: WeatherCodeInfo("雾", "🌫️", "有雾"),
    48: WeatherCodeInfo("雾", "🌫️", "有雾凇"),
    51: WeatherCodeInfo("小毛毛雨", "🌦️", "轻度毛毛雨"),
    53: WeatherCodeInfo("中毛毛雨", "🌦️", "中度毛毛雨"),
    55: WeatherCodeInfo("稠密毛毛雨", "🌧️", "稠密毛毛雨"),
    56: WeatherCodeInfo("冻毛毛雨", "🌧️", "轻度冻毛毛雨"),
    57: WeatherCodeInfo("冻毛毛雨", "🌧️", "中度冻毛毛雨"),
    61: WeatherCodeInfo("小雨", "🌦️", "小雨"),
    63: WeatherCodeInfo("中雨", "🌧️", "中雨"),
    65: WeatherCodeInfo("大雨", "🌧️", "大雨"),
    66: WeatherCodeInfo("冻雨", "🌧️", "轻度冻雨"),
    67: WeatherCodeInfo("冻雨", "🌧️", "中度冻雨"),
    71: WeatherCodeInfo("小雪", "❄️", "小雪"),
    73: WeatherCodeInfo("中雪", "❄️", "中雪"),
    75: WeatherCodeInfo("大雪", "❄️", "大雪"),
    77: WeatherCodeInfo("雪粒", "❄️", "雪粒"),
    80: WeatherCodeInfo("小阵雨", "🌦️", "小阵雨"),
    81: WeatherCodeInfo("中阵雨", "🌧️", "中阵雨"),
    82: WeatherCodeInfo("大阵雨", "⛈️", "大阵雨"),
    85: WeatherCodeInfo("小阵雪", "❄️", "小阵雪"),
    86: WeatherCodeInfo("大阵雪", "❄️", "大阵雪"),
    95: WeatherCodeInfo("雷暴", "⛈️", "雷暴"),
    96: WeatherCodeInfo("雷暴", "⛈️", "雷暴伴小冰雹"),
    99: WeatherCodeInfo("雷暴", "⛈️", "雷暴伴大冰雹"),
}

# 未知天气代码兜底
UNKNOWN_WEATHER = WeatherCodeInfo("未知", "🌡️", "未知天气")

# ===== data/weather_codes.py 函数/常量说明 =====
# WeatherCodeInfo: dataclass，天气代码信息聚合类（frozen 不可变）
#   字段：name 中文短名、icon emoji 图标、description 中文完整描述
# WEATHER_CODE_INFO: dict[int, WeatherCodeInfo]，WMO 天气代码 → 完整信息
#   设计理由：S2 合并原 WEATHER_CODES/WEATHER_DESCRIPTIONS 两张键重复的表，
#   单一来源避免重复维护（修复 M06 遗留问题）
# UNKNOWN_WEATHER: WeatherCodeInfo，未知代码兜底值
#   设计理由：API 返回未知代码时提供统一的降级展示
#   关联配置：无外部依赖，供 modules/weather_service.py 使用
