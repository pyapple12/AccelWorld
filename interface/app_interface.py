# 应用接口层（AppInterface）：UI 访问后端的唯一契约入口（plan#UI2.0）
# 职责：持有 AcceleratedWorld 与 AlarmManager，封装时钟/倍率/天气/闹钟/时区/配置/几何七域契约；
# 接口保持同步拉取式、无 Qt 依赖——线程编排（天气后台查询/闹钟定时检查）与写盘去抖等
# UI 编排一律留在 ui 层（plan#UI2.0 铁律 3/PL001.02）

import logging
from datetime import datetime
from typing import Any

# 后端配置读写（用户配置经 settings 模块命名空间访问，便于测试打桩）
import config.settings as settings
from config.static.static_config import get_static_config

# UI 偏好聚合（同层 types 子模块，顶层 import 无循环：types.py 不依赖本模块）
from interface.types import UiPreferences

# 后端业务核心（接口层允许 import 后端，plan#UI2.0 铁律）
from data.cities import CITIES
from data.timezones import TIMEZONES
from modules.alarm_service import (
    Alarm,
    AlarmManager,
    PresetSound,
    play_preset_sound,
)
from modules.time_dilation import AcceleratedWorld, TimeInfo
from modules.weather_service import WeatherData, clear_weather_cache, get_weather_by_city

logger = logging.getLogger(__name__)

# 主题偏好合法取值（PL002.02 三态：跟随系统/浅色/深色）
# （系统深浅色注册表读取已随 get_system_theme_hint 删除，FIX003.13：qfw AUTO 内建跟随）
_THEME_CHOICES = ("auto", "light", "dark")


class AppInterface:
    def __init__(self) -> None:
        # 持有加速世界与闹钟管理器（所有权自 ui 层迁入，plan#UI2.0 契约清单）
        # 构造时以持久化倍率初始化世界；脏值/越界值回退默认并记日志（FIX001.7 逻辑自 main_window 迁入）
        base = get_static_config().base
        saved_rate = settings.get_setting("time_dilation_rate", base["default_rate"])
        try:
            self._world = AcceleratedWorld(time_dilation_rate=float(saved_rate))
        except (TypeError, ValueError):
            logger.warning(f"持久化倍率非法，已回退默认值: {saved_rate!r}")
            self._world = AcceleratedWorld()
        self._alarm_manager = AlarmManager()

    # ------------------- 静态配置（PL001.02/09：UI 零硬编码来源收口接口） -------------------

    @staticmethod
    def get_app_static() -> dict[str, Any]:
        # base.json 静态参数（版本/倍率范围/周期/快捷键/窗口默认几何等 UI 需要的键）
        return get_static_config().base

    @staticmethod
    def get_ui_static() -> dict[str, Any]:
        # ui.json 静态样式参数（字体/颜色表），面板样式零硬编码来源
        return get_static_config().ui

    def get_version(self) -> str:
        # 版本号单一来源 base.json version（main.py/窗口标题/托盘 tooltip 统一经此读取）
        return str(get_static_config().base["version"])

    # ------------------- 时钟（PL001.02） -------------------

    def get_time_info(self) -> TimeInfo:
        # 标准/加速时间信息快照（农历按标准秒缓存，tick 高频调用廉价）
        return self._world.get_custom_time()

    def get_tick_interval_ms(self) -> int:
        # 刷新周期随倍率联动（T001.1），GUI 定时器每次倍率变化后经此重启
        return self._world.tick_interval_ms

    # ------------------- 倍率（PL001.02） -------------------

    def get_rate(self) -> float:
        # 当前生效倍率
        return self._world.time_dilation_rate

    def get_rate_bounds(self) -> tuple[float, float]:
        # 倍率合法区间 (rate_min, rate_max)，滑杆/输入框校验共用
        base = get_static_config().base
        return (float(base["rate_min"]), float(base["rate_max"]))

    # get_rate_presets 已删（FIX003.13：预设按钮随时钟页减法移除，方法生产零调用；
    # base.json rate_presets 键随下轮配置清理一并评估删除）

    def set_rate(self, rate: float) -> bool:
        # 轻量路径：范围校验 + 世界重建，不落盘——UI 滑杆拖动的实时生效路径，
        # 持久化由 UI 层去抖后经 apply_rate 完成（FIX001.23 防拖动高频写盘）
        try:
            self._world = AcceleratedWorld(time_dilation_rate=float(rate))
        except (TypeError, ValueError):
            return False
        return True

    def apply_rate(self, rate: float) -> bool:
        # 内聚动作：范围校验 + 重建 + 持久化；越界/非法返回 False（不重建不落盘）
        if not self.set_rate(rate):
            return False
        return settings.set_setting("time_dilation_rate", self._world.time_dilation_rate)

    # ------------------- 天气（PL001.04） -------------------

    def get_city_names(self) -> list[str]:
        # 城市名有序列表（下拉框填充与列表内外判定共用）
        return sorted(CITIES.keys())

    def fetch_weather(self, city: str, force: bool = False) -> WeatherData | None:
        # 城市天气查询（含缓存/重试/降级）；force=True 穿透缓存强制请求（手动刷新，FIX003.6）；
        # 失败返回 None，网络异常不外抛
        if force:
            clear_weather_cache()
        return get_weather_by_city(city)

    def get_weather_refresh_interval_ms(self) -> int:
        # 自动刷新周期 = 缓存 TTL 秒 × 1000（单源派生，E15）
        return int(get_static_config().base["weather_cache_ttl"]) * 1000

    # format_weather_display 已删（FIX003.13：天气卡 PL007.04 起结构化字段直填，
    # 方法生产零调用；modules.format_weather_info 保留为独立展示工具）

    # ------------------- 时区（PL001.04） -------------------

    def get_timezone_options(self) -> list[tuple[str, str]]:
        # 时区选项表（显示名, IANA 标识），副本出栈防调用方污染常量
        return list(TIMEZONES)

    def get_world_pins(self) -> list[str]:
        # 世界时钟常驻城市集（IANA 标识列表，用户配置，PL007.02）
        pins = settings.get_setting("world_pins")
        return list(pins) if isinstance(pins, list) else []

    # ------------------- 闹钟（PL001.05，AlarmManager 所有权在接口） -------------------

    def load_alarm_dicts(self) -> None:
        # 从配置拉取闹钟列表载入管理器（损坏条目容错跳过并记日志）
        self._alarm_manager.from_dict_list(settings.get_alarms())

    def save_alarm_dicts(self) -> bool:
        # 管理器当前列表序列化落盘
        return settings.save_alarms(self._alarm_manager.to_dict_list())

    def check_alarms(self, now: datetime) -> list[Alarm]:
        # 触发检查（同分钟去重），命中列表由 UI 层编排播放/通知
        return self._alarm_manager.check_alarms(now)

    def get_max_alarms(self) -> int:
        # 闹钟数量上限（添加失败弹窗文案使用）
        return self._alarm_manager.max_alarms

    def get_alarms(self) -> list[Alarm]:
        # 当前闹钟列表（列表行刷新只读遍历用；浅拷贝防外部增删）
        return list(self._alarm_manager.alarms)

    def get_alarm(self, alarm_id: str) -> Alarm | None:
        # 按 ID 查找，未命中返回 None
        return self._alarm_manager.get_alarm(alarm_id)

    def add_alarm(self, alarm: Alarm) -> bool:
        # 添加（上限/同时间同标签去重校验）
        return self._alarm_manager.add_alarm(alarm)

    def remove_alarm(self, alarm_id: str) -> bool:
        # 删除（同时清理触发去重记录）
        return self._alarm_manager.remove_alarm(alarm_id)

    def replace_alarm(self, alarm: Alarm) -> bool:
        # 编辑替换（保留原 ID 原位替换）
        return self._alarm_manager.replace_alarm(alarm)

    def toggle_alarm(self, alarm_id: str) -> bool:
        # 启用/禁用翻转（触发后一次性禁用也走此路径）
        return self._alarm_manager.toggle_alarm(alarm_id)

    def play_preset_sound(self, preset: PresetSound) -> None:
        # 预设铃声播放（winsound 阻塞式），由 ui/audio_player 在后台线程调用
        play_preset_sound(preset)

    # ------------------- 配置偏好（PL001.03） -------------------

    def get_ui_preferences(self) -> UiPreferences:
        # 组装 UI 会话偏好快照；主题归一化为三态（auto/light/dark），非法值回退默认；
        # 缺省值回退 base.json 默认
        base = get_static_config().base
        theme = str(settings.get_setting("theme", base["default_theme"]))
        if theme not in _THEME_CHOICES:
            theme = str(base["default_theme"])
        return UiPreferences(
            theme=theme,
            last_city=str(settings.get_setting("last_city", base["default_city"])),
            last_timezone=str(settings.get_setting("last_timezone", base["default_timezone"])),
            countdown_target=str(settings.get_setting("countdown_target", "")),
        )

    # get_system_theme_hint 已删（FIX003.13：三态主题改由 qfw setTheme(AUTO) 内建跟随，
    # 方法生产零调用；连带注销 winreg 依赖与注册表常量）

    def save_theme(self, theme: str) -> bool:
        # 主题偏好落盘（light/dark；PL002 扩展 auto）
        return settings.set_setting("theme", theme)

    def save_last_city(self, city: str) -> bool:
        # 上次城市落盘（退出时由主窗口汇总）
        return settings.set_setting("last_city", city)

    def save_last_timezone(self, timezone: str) -> bool:
        # 上次时区落盘
        return settings.set_setting("last_timezone", timezone)

    def save_countdown_target(self, target_text: str) -> bool:
        # 倒计时目标文本落盘（未设置为空串）
        return settings.set_setting("countdown_target", target_text)

    # ------------------- 窗口几何（PL001.03） -------------------

    def load_window_geometry(self) -> bytes | None:
        # base64 解码后的几何字节（QByteArray restoreGeometry 输入）；非法/未存返回 None
        return settings.load_window_geometry()

    def save_window_geometry(self, geometry: bytes) -> bool:
        # 几何字节经 base64 编码落盘
        return settings.save_window_geometry(geometry)


# ===== interface/app_interface.py 函数/类说明 =====
# AppInterface: 应用接口类（UI 唯一的后端访问入口，plan#UI2.0 三大块架构的中间层）
#   __init__: 构建 AcceleratedWorld（持久化倍率初始化，脏值回退默认 FIX001.7）与 AlarmManager
#     输入：无（自配置读取）；输出：无（副作用为实例状态）；异常：倍率非法回退默认记 warning
#   静态配置域：get_app_static()/get_ui_static()（staticmethod，无实例副作用，main.py 装配期
#     即可用）；get_version()
#   时钟域：get_time_info()/get_tick_interval_ms()——UI 定时器拉取式驱动
#   倍率域：get_rate()/get_rate_bounds()/get_rate_presets()；
#     set_rate(rate) 轻量路径（校验+重建不落盘，滑杆拖动实时生效）；
#     apply_rate(rate) 内聚动作（校验+重建+持久化，去抖 flush/退出保存/单发场景）
#     设计理由：拆两档是为了 FIX001.23 去抖语义留在 UI 层时重建仍可实时——拖动期零写盘
#   天气域：get_city_names()/fetch_weather(city)/get_weather_refresh_interval_ms()/
#     format_weather_display(city, weather)——缓存与重试由后端 weather_service 承担
#   时区域：get_timezone_options()（TIMEZONES 副本）
#   闹钟域：AlarmManager 所有权迁入接口（plan#UI2.0 迁移要点）——load_alarm_dicts()/
#     save_alarm_dicts()/check_alarms(now)/get_max_alarms() 契约方法 + CRUD 转发
#     （get_alarms/get_alarm/add_alarm/remove_alarm/replace_alarm/toggle_alarm）；
#     play_preset_sound(preset) 供 ui/audio_player 后台线程调用
#   配置偏好域：get_ui_preferences() 组装 UiPreferences 快照；save_theme/save_last_city/
#     save_last_timezone/save_countdown_target 逐项落盘（set_setting 即写即存）
#   几何域：load_window_geometry()/save_window_geometry(bytes) 委托 settings 的 base64 封装
#   设计理由：UI 与后端单向经本类交互——UI 不 import 后端、后端不感知 UI；同步拉取式
#   （UI 定时器主动查询），接口内无线程/定时器，编排归 UI 层（plan#UI2.0 铁律 3）
#   异常处理：倍率非法值 try/except (TypeError, ValueError) 回退/拒绝；保存类方法返回
#   bool 由 UI 决定上浮方式；天气网络异常在 service 层降级 None
#   关联配置：用户配置 config/user_config.json（经 config.settings，测试可打桩模块方法）；
#   应用静态配置 config/static/base.json 与 ui.json（经 get_static_config()）；
#   独立测试证据见 tests/test_interface.py（直接读 base.json 文件比对）
