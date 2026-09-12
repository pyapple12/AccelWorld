# AppInterface 契约测试（PL001 引入，纯 pytest 无 Qt 依赖）
# 覆盖：时钟/倍率（PL001.02）、配置/几何（PL001.03）、天气/时区（PL001.04）、闹钟（PL001.05）
# 结构性样例来源：config/static/base.json 真实文件内容（独立证据，禁止与实现同源 mock）
# 配置隔离：conftest.isolated_config 将 CONFIG_FILE 重定向到临时目录

import json
from datetime import datetime, timedelta
from pathlib import Path

from config import settings as settings_mod
from interface import AppInterface, types
from interface.types import TimeInfo, WeatherData, Alarm, UiPreferences
from utils.file_utils import clear_json_cache

# 独立证据：直接读静态配置文件（不经 get_static_config 单例，避免自证）
_BASE_JSON = json.loads(
    (Path(__file__).resolve().parent.parent / "config" / "static" / "base.json").read_text(
        encoding="utf-8"
    )
)


def test_interface_package_no_qt_dependency():
    # 无 Qt 依赖铁律：interface/ 全部源码不出现 Qt 绑定字样（PL001.01 验收）
    pkg_dir = Path(__file__).resolve().parent.parent / "interface"
    sources = "\n".join(p.read_text(encoding="utf-8") for p in pkg_dir.glob("*.py"))
    assert "PyQt6" not in sources, "interface 包出现 Qt 绑定引用"


def test_types_reexports_backend_dtos():
    # types.py 转出的就是后端 DTO 本体（同一类对象，非复制层）
    import modules.alarm_service as alarm_service
    import modules.time_dilation as time_dilation
    import modules.weather_service as weather_service

    assert types.TimeInfo is time_dilation.TimeInfo
    assert types.WeatherData is weather_service.WeatherData
    assert types.Alarm is alarm_service.Alarm
    assert types.PresetSound is alarm_service.PresetSound


# ------------------- PL001.02 时钟与倍率契约 -------------------


def test_get_time_info_returns_timeinfo():
    iface = AppInterface()
    info = iface.get_time_info()
    assert isinstance(info, TimeInfo)
    assert info.custom_time.count(":") == 2
    assert abs(info.dilation_percentage - iface.get_rate() * 100) < 1e-9


def test_get_tick_interval_ms_positive_and_follows_rate():
    iface = AppInterface()
    iface.set_rate(1.0)
    slow = iface.get_tick_interval_ms()
    iface.set_rate(20.0)
    fast = iface.get_tick_interval_ms()
    assert slow > 0 and fast > 0
    assert fast < slow, f"高倍率刷新周期应更短: {slow} -> {fast}"


def test_get_version_matches_base_json():
    iface = AppInterface()
    assert iface.get_version() == _BASE_JSON["version"]


def test_get_rate_bounds_match_base_json():
    iface = AppInterface()
    assert iface.get_rate_bounds() == (
        float(_BASE_JSON["rate_min"]),
        float(_BASE_JSON["rate_max"]),
    )
    # get_rate_presets 已随预设按钮移除（FIX003.13），base.json 键留待配置清理评估


def test_apply_rate_valid_applies_and_persists():
    iface = AppInterface()
    assert iface.apply_rate(5.0) is True
    assert abs(iface.get_rate() - 5.0) < 1e-9
    assert abs(settings_mod.get_setting("time_dilation_rate") - 5.0) < 1e-9


def test_apply_rate_out_of_bounds_rejected():
    iface = AppInterface()
    original = iface.get_rate()
    assert iface.apply_rate(float(_BASE_JSON["rate_min"]) - 0.1) is False
    assert iface.apply_rate(float(_BASE_JSON["rate_max"]) + 0.1) is False
    assert abs(iface.get_rate() - original) < 1e-9
    # 拒绝路径不得落盘
    assert abs(settings_mod.get_setting("time_dilation_rate") - original) < 1e-9


def test_dirty_persisted_rate_falls_back_to_default(tmp_path):
    # 越界持久化倍率（0.5 < rate_min）构造时回退默认（FIX001.7 逻辑自 main_window 迁入）；
    # conftest.isolated_config 已将 CONFIG_FILE 指向同一 tmp_path，直接写入脏值后清缓存
    (tmp_path / "user_config.json").write_text(
        json.dumps({"time_dilation_rate": 0.5}), encoding="utf-8"
    )
    clear_json_cache()
    iface = AppInterface()
    assert abs(iface.get_rate() - float(_BASE_JSON["default_rate"])) < 1e-9


def test_set_rate_rebuilds_without_persist():
    iface = AppInterface()
    assert iface.set_rate(7.5) is True
    assert abs(iface.get_rate() - 7.5) < 1e-9
    # 轻量路径不落盘：配置中仍为默认倍率
    assert abs(
        settings_mod.get_setting("time_dilation_rate") - float(_BASE_JSON["default_rate"])
    ) < 1e-9


# ------------------- PL001.03 配置与几何契约 -------------------


def test_ui_preferences_roundtrip():
    iface = AppInterface()
    assert iface.save_theme("dark") is True
    assert iface.save_last_city("上海") is True
    assert iface.save_last_timezone("Asia/Tokyo") is True
    assert iface.save_countdown_target("2030-01-01 00:00:00") is True
    prefs = iface.get_ui_preferences()
    assert isinstance(prefs, UiPreferences)
    assert prefs.theme == "dark"
    assert prefs.last_city == "上海"
    assert prefs.last_timezone == "Asia/Tokyo"
    assert prefs.countdown_target == "2030-01-01 00:00:00"


def test_ui_preferences_defaults_match_base_json():
    iface = AppInterface()
    prefs = iface.get_ui_preferences()
    assert prefs.theme == _BASE_JSON["default_theme"]
    assert prefs.last_city == _BASE_JSON["default_city"]
    assert prefs.last_timezone == _BASE_JSON["default_timezone"]
    assert prefs.countdown_target == ""


def test_ui_preferences_theme_three_state(tmp_path):
    # PL002.02：theme 取值扩为 auto/light/dark，存量 light/dark 兼容
    (tmp_path / "user_config.json").write_text(
        json.dumps({"theme": "auto"}), encoding="utf-8"
    )
    clear_json_cache()
    iface = AppInterface()
    assert iface.get_ui_preferences().theme == "auto"
    assert iface.save_theme("dark") is True
    assert iface.get_ui_preferences().theme == "dark"
    assert iface.save_theme("light") is True
    assert iface.get_ui_preferences().theme == "light"


def test_ui_preferences_theme_invalid_falls_back(tmp_path):
    # 非法主题值回退 base.json 默认主题（default_theme 现为 auto）
    (tmp_path / "user_config.json").write_text(
        json.dumps({"theme": "bogus"}), encoding="utf-8"
    )
    clear_json_cache()
    iface = AppInterface()
    assert iface.get_ui_preferences().theme == _BASE_JSON["default_theme"]


# test_get_system_theme_hint 已删（FIX003.13：方法随三态主题 setTheme(AUTO) 内建跟随退役）


def test_window_geometry_base64_roundtrip():
    iface = AppInterface()
    assert iface.load_window_geometry() is None  # 未保存时无几何
    raw = bytes(range(256))
    assert iface.save_window_geometry(raw) is True
    assert iface.load_window_geometry() == raw


def test_invalid_geometry_returns_none():
    # 非法 base64 直接写入配置 → 严格解码返回 None（FIX001.25 validate=True 语义）
    iface = AppInterface()
    settings_mod.set_setting("window_geometry", "!!!not-base64!!!")
    assert iface.load_window_geometry() is None


# ------------------- PL001.04 天气与时区契约 -------------------


def test_city_names_sorted_from_cities_table():
    from data.cities import CITIES

    iface = AppInterface()
    assert iface.get_city_names() == sorted(CITIES.keys())


def test_fetch_weather_unknown_city_returns_none():
    iface = AppInterface()
    assert iface.fetch_weather("不存在的城市XYZ") is None


def test_fetch_weather_cached_queries_once(monkeypatch):
    import modules.weather_service as weather_service

    calls = []

    def counting_fetch(url):
        calls.append(url)
        return {
            "current": {
                "temperature_2m": 20.0,
                "relative_humidity_2m": 50,
                "weather_code": 0,
                "wind_speed_10m": 5.0,
                "apparent_temperature": 21.0,
            }
        }

    monkeypatch.setattr(weather_service, "_fetch_weather_data", counting_fetch)
    weather_service.clear_weather_cache()
    iface = AppInterface()
    first = iface.fetch_weather("北京")
    second = iface.fetch_weather("北京")
    assert isinstance(first, WeatherData)
    assert second is first  # 缓存命中返回同一对象
    assert len(calls) == 1  # 只发起一次网络查询


# test_format_weather_display_with_and_without_data 已删（FIX003.13：方法随天气卡
# 结构化直填退役，format_weather_info 保留为 modules 独立展示工具）


def test_get_timezone_options_non_empty_with_known_pair():
    from data.timezones import TIMEZONES

    iface = AppInterface()
    options = iface.get_timezone_options()
    assert options and ("北京 (UTC+8)", "Asia/Shanghai") in options


# ------------------- PL001.05 闹钟契约（所有权迁移） -------------------


def test_alarm_load_save_roundtrip():
    iface = AppInterface()
    alarm = Alarm(label="晨间", time="07:30", repeat_days=[0])
    assert iface.add_alarm(alarm) is True
    assert iface.save_alarm_dicts() is True
    # 新接口实例（模拟重启）经配置重载后闹钟一致
    iface2 = AppInterface()
    iface2.load_alarm_dicts()
    loaded = iface2.get_alarms()
    assert len(loaded) == 1
    assert loaded[0].label == "晨间" and loaded[0].time == "07:30"
    assert loaded[0].id == alarm.id


def test_check_alarms_triggers_matching_repeat_alarm():
    iface = AppInterface()
    check_time = datetime.now() + timedelta(days=1)
    alarm = Alarm(label="工作日", time=f"{check_time:%H:%M}", repeat_days=[check_time.weekday()])
    iface.add_alarm(alarm)
    triggered = iface.check_alarms(check_time.replace(second=0, microsecond=0))
    assert [a.id for a in triggered] == [alarm.id]
    # 同分钟去重：第二次检查不再触发
    assert iface.check_alarms(check_time.replace(second=30)) == []


def test_alarm_crud_via_interface():
    iface = AppInterface()
    alarm = Alarm(label="临时", time="08:00")
    assert iface.add_alarm(alarm) is True
    assert iface.get_alarm(alarm.id) is alarm
    assert iface.toggle_alarm(alarm.id) is True
    assert iface.get_alarm(alarm.id).enabled is False
    edited = Alarm(label="临时改", time="09:00", repeat_days=[1])
    edited.id = alarm.id
    assert iface.replace_alarm(edited) is True
    assert iface.get_alarm(alarm.id).label == "临时改"
    assert iface.remove_alarm(alarm.id) is True
    assert iface.get_alarm(alarm.id) is None


def test_add_alarm_dedup_and_max_limit():
    iface = AppInterface()
    first = Alarm(label="重复", time="06:00")
    assert iface.add_alarm(first) is True
    assert iface.add_alarm(Alarm(label="重复", time="06:00")) is False  # 同时间同标签去重
    limit = int(_BASE_JSON["max_alarms"])
    assert iface.get_max_alarms() == limit
    for i in range(limit - 1):
        assert iface.add_alarm(Alarm(label=f"补充{i}", time=f"{i % 24:02d}:{i % 60:02d}")) is True
    assert iface.add_alarm(Alarm(label="超限", time="23:59")) is False  # 达上限拒绝
