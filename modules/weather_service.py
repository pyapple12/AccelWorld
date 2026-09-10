# 天气服务模块（S5 引入 30 分钟缓存 + 网络重试）
# 使用 Open-Meteo 免费天气 API（无需 API Key）
# API 文档: https://open-meteo.com/

import urllib.request
import urllib.error
import urllib.parse
import http.client
import ipaddress
import json
import logging
import socket
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

# 静态配置（缓存 TTL 参数）
from config.static.static_config import get_static_config

# 天气结果内存缓存：城市名 → (缓存时间戳, WeatherData)
_weather_cache: dict[str, tuple[float, "WeatherData"]] = {}

# 缓存有效期（秒，来自静态配置）
CACHE_TTL_SECONDS = int(get_static_config().base["weather_cache_ttl"])

# 经纬度合法定义域（地理常量，非业务调参；请求前校验防非法参数拼入 URL，V0.4.7.0 安全加固）
_LAT_RANGE = (-90.0, 90.0)
_LON_RANGE = (-180.0, 180.0)

# 请求主机白名单（SSRF 防护：仅允许向 Open-Meteo 官方接口发起 HTTPS 请求，V0.4.7.0）
_API_SCHEME = "https"
_API_HOST = "api.open-meteo.com"

# 必要响应字段（与请求 current 参数一致；缺失或值为 null 判失败，FIX001.14/FIX002.4）
_REQUIRED_FIELDS = (
    "temperature_2m",
    "relative_humidity_2m",
    "weather_code",
    "wind_speed_10m",
    "apparent_temperature",
)

# 数值型必要字段（weather_code 允许任意标量；其余须为数值，FIX002.4）
_NUMERIC_FIELDS = (
    "temperature_2m",
    "relative_humidity_2m",
    "wind_speed_10m",
    "apparent_temperature",
)

# 网络/解析类异常白名单（重试与降级共用；ConnectionResetError/HTTPException 覆盖
# 读体阶段中断，FIX002.5）
_NETWORK_ERRORS = (
    urllib.error.URLError,
    TimeoutError,
    socket.gaierror,
    ConnectionResetError,
    http.client.HTTPException,
)


@dataclass
class WeatherData:
    temperature: float  # 温度（℃）
    humidity: float  # 相对湿度（%）
    wind_speed: float  # 风速（km/h）
    apparent_temperature: float  # 体感温度（℃）
    weather_code: int  # WMO 天气代码
    weather: str  # 中文短名（如"晴"）
    description: str  # 中文完整描述
    icon: str  # emoji 图标


class _NoRedirectHandler(urllib.request.HTTPRedirectHandler):
    # 禁止跟随重定向（SSRF 防护：重定向可能绕过主机白名单，V0.4.7.0）
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        # 返回 None 表示不构造重定向请求，urlopen 将抛 HTTPError
        return None


# 禁止重定向的 handler 类（每次请求构建独立 opener，线程池并发下无共享可变状态，FIX001.22）
def _build_no_redirect_opener() -> urllib.request.OpenerDirector:
    # 构建禁止跟随重定向的 opener（重定向可能绕过主机白名单，V0.4.7.0 SSRF 防护）
    return urllib.request.build_opener(_NoRedirectHandler)


def _fetch_weather_data(url: str) -> dict:
    # 请求 Open-Meteo API 并解析 JSON（独立函数供 retry_call 重试）
    # SSRF 防护（V0.4.7.0）：协议/主机白名单 + 域名解析 IP 私网阻断 + 禁止重定向；
    # 剩余 TOCTOU 型 DNS rebinding 理论风险由域名固定为官方接口兜底（单机应用可接受）
    # DNS 解析失败抛 gaierror（FIX001.3：由上层重试白名单与降级 except 统一处理）
    parts = urllib.parse.urlsplit(url)
    if parts.scheme != _API_SCHEME or parts.hostname != _API_HOST:
        raise ValueError(f"拒绝请求非 Open-Meteo 接口地址: {url}")
    addr_infos = socket.getaddrinfo(parts.hostname, 443, proto=socket.IPPROTO_TCP)
    for info in addr_infos:
        ip = ipaddress.ip_address(info[4][0])
        if (
            ip.is_private
            or ip.is_loopback
            or ip.is_link_local
            or ip.is_multicast
            or ip.is_reserved
            or ip.is_unspecified
        ):
            raise ValueError(f"接口域名解析到受限地址: {ip}")
    opener = _build_no_redirect_opener()
    with opener.open(
        url, timeout=float(get_static_config().base["weather_timeout_s"])
    ) as response:
        return json.loads(response.read().decode("utf-8"))


def get_weather_by_coords(lat: float, lon: float) -> Optional[WeatherData]:
    # 经纬度定义域校验：越界属编程错误直接上抛（城市坐标来自 data/cities.py 静态表）
    lat_ok = _LAT_RANGE[0] <= lat <= _LAT_RANGE[1]
    lon_ok = _LON_RANGE[0] <= lon <= _LON_RANGE[1]
    if not (lat_ok and lon_ok):
        raise ValueError(f"经纬度越界: lat={lat}, lon={lon}")

    # 拼接 API URL（主机部分复用白名单常量单源，FIX001.22），重试耗尽后统一返回 None；
    # 仅捕获网络/解析类异常，编程错误上抛
    try:
        url = (
            f"{_API_SCHEME}://{_API_HOST}/v1/forecast?"
            f"latitude={lat}&longitude={lon}"
            f"&current={','.join(_REQUIRED_FIELDS)}"
            f"&timezone=auto"
        )

        # 网络错误自动重试（次数/间隔来自静态配置 FIX001.16；白名单含 gaierror FIX001.3、
        # 读体阶段 ConnectionResetError/HTTPException FIX002.5）
        base = get_static_config().base
        data = retry_call(
            _fetch_weather_data,
            url,
            retries=int(base["weather_retries"]),
            exceptions=_NETWORK_ERRORS,
            delay=float(base["weather_retry_delay"]),
        )

        # 响应结构/必要字段校验（FIX001.14/FIX002.4：缺失或 null/错型值均判失败，不以假数据兜底）
        if not isinstance(data, dict):
            logger.error(f"天气响应结构异常: {data!r}")
            return None
        current = data.get("current")
        if not isinstance(current, dict) or any(
            field_name not in current or current[field_name] is None
            for field_name in _REQUIRED_FIELDS
        ):
            logger.error(f"天气响应缺少必要字段: {data!r}")
            return None
        if any(
            not isinstance(current[field_name], (int, float))
            or isinstance(current[field_name], bool)
            for field_name in _NUMERIC_FIELDS
        ):
            logger.error(f"天气响应数值字段类型非法: {data!r}")
            return None

        weather_code = current["weather_code"]
        code_info = WEATHER_CODE_INFO.get(weather_code, UNKNOWN_WEATHER)

        return WeatherData(
            temperature=current["temperature_2m"],
            humidity=current["relative_humidity_2m"],
            wind_speed=current["wind_speed_10m"],
            apparent_temperature=current["apparent_temperature"],
            weather_code=weather_code,
            weather=code_info.name,
            description=code_info.description,
            icon=code_info.icon,
        )
    except _NETWORK_ERRORS + (json.JSONDecodeError,) as e:
        # 网络/超时/DNS/读体中断/JSON 解析失败：记录堆栈并降级返回 None（FIX002.5 复用白名单）
        logger.exception(f"获取天气信息失败: {e}")
        return None


def get_weather_by_city(city_name: str) -> Optional[WeatherData]:
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
    # 直接清空模块级缓存字典
    _weather_cache.clear()


def format_weather_info(weather: Optional[WeatherData], city_name: str = "") -> str:
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


# ===== modules/weather_service.py 函数/常量说明 =====
# WeatherData: dataclass，天气信息聚合类（S10.11 C1：to_display 已删，展示统一走 format_weather_info）
# _fetch_weather_data(url): 请求 API 并解析 JSON（供 retry_call 重试的可调用对象）；
#   请求前校验 scheme/hostname 白名单并解析域名阻断私网/环回等受限 IP，
#   非官方接口抛 ValueError（编程错误上抛，不被网络异常降级吞掉）
# _NoRedirectHandler/_build_no_redirect_opener: 禁止重定向（重定向可能绕过主机白名单）；
#   opener 每次请求独立构建，线程池并发下无共享可变状态（FIX001.22）
# _REQUIRED_FIELDS: 必要响应字段（缺失即失败，FIX001.14）
# _LAT_RANGE/_LON_RANGE: 经纬度合法定义域（地理常量），请求前校验防非法参数拼入 URL
# get_weather_by_coords(lat, lon): 经纬度查询，越界抛 ValueError；
#   URLError/TimeoutError/gaierror 自动重试（次数与间隔来自静态配置，FIX001.3/16）；
#   响应结构/必要字段缺失降级 None（FIX001.14）
# get_weather_by_city(city_name): 城市查询，30 分钟缓存（仅缓存成功，失败可立即重试）
# clear_weather_cache(): 清空缓存
# format_weather_info(weather, city_name): 完整展示文本
#   设计理由：缓存减少 API 调用（对应 M09a）；失败不缓存保证网络恢复后及时更新
#   异常处理：网络/解析异常统一返回 None 并记录堆栈；其余异常上抛暴露编程错误
#   关联配置：城市表 data/cities.py；天气代码表 data/weather_codes.py；重试工具 utils/retry.py
