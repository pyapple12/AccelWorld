# 用户配置模块测试（S9.7 测试引入）
# 覆盖：默认值（来自 static）、读写往返、损坏 JSON 容错、缓存、未知键、base64 几何、副本隔离；
#       FIX001 补充：损坏转存 .bak、反序列化类型校验、环境变量注入配置路径

import base64
import json

import config.settings as settings
from config.settings import UserConfig


def test_default_values():
    # UserConfig 默认值来自静态配置 base.json（零硬编码，S9.5 回归）
    u = UserConfig()
    assert u.time_dilation_rate == 2.0
    assert u.theme == "light"
    assert u.last_city == "北京"
    assert u.last_timezone == "Asia/Shanghai"
    assert u.countdown_target == ""
    assert u.window_geometry is None
    assert u.alarms == []


def test_load_save_roundtrip():
    # set_setting/get_setting 往返一致（fixture 已重定向到临时路径）
    assert settings.set_setting("time_dilation_rate", 3.5)
    assert settings.get_setting("time_dilation_rate") == 3.5
    assert settings.load_config().time_dilation_rate == 3.5
    # 未设置的键取默认
    assert settings.get_setting("theme") == "light"


def test_corrupted_json(tmp_path, monkeypatch):
    # 损坏 JSON 容错：加载返回默认值不崩溃（file_utils 兜底）
    bad_file = tmp_path / "user_config.json"
    bad_file.write_text("{ 这不是合法 JSON", encoding="utf-8")
    monkeypatch.setattr(settings, "CONFIG_FILE", bad_file)
    from utils.file_utils import clear_json_cache

    clear_json_cache()
    cfg = settings.load_config()
    assert cfg.time_dilation_rate == 2.0  # 默认值


def test_corrupted_config_backed_up(tmp_path, monkeypatch):
    # 损坏配置先转存 .bak 再回退默认（FIX001.2：防默认值覆盖导致不可恢复丢失）
    bad_file = tmp_path / "user_config.json"
    bad_file.write_text("{ 损坏内容", encoding="utf-8")
    monkeypatch.setattr(settings, "CONFIG_FILE", bad_file)
    from utils.file_utils import clear_json_cache

    clear_json_cache()
    cfg = settings.load_config()
    assert cfg.time_dilation_rate == 2.0
    backup = tmp_path / "user_config.json.bak"
    assert backup.exists(), "损坏文件未转存 .bak"
    assert backup.read_text(encoding="utf-8") == "{ 损坏内容"


def test_from_dict_invalid_types_fall_back_defaults():
    # 反序列化类型校验：类型不符字段剔除走默认值，合法字段保留（FIX001.9）
    cfg = settings.UserConfig.from_dict(
        {
            "theme": None,
            "time_dilation_rate": "abc",
            "last_city": 123,
            "countdown_target": "2027-01-01 00:00:00",
            "alarms": "notalist",
        }
    )
    assert cfg.theme == "light"
    assert cfg.time_dilation_rate == 2.0
    assert cfg.last_city == "北京"
    assert cfg.countdown_target == "2027-01-01 00:00:00"
    assert cfg.alarms == []


def test_env_config_path_override(tmp_path, monkeypatch):
    # 环境变量注入配置路径（子进程测试隔离用，FIX001.12）
    env_file = tmp_path / "env_config.json"
    monkeypatch.setenv("ACCELWORLD_CONFIG_FILE", str(env_file))
    assert settings._resolve_config_file() == env_file
    monkeypatch.delenv("ACCELWORLD_CONFIG_FILE")
    assert settings._resolve_config_file() == (
        settings.get_project_root() / "config" / "user_config.json"
    )


def test_cache_invalidation():
    # 写后读一致（write_json 清理缓存，S9.5 回归）
    settings.set_setting("theme", "dark")
    assert settings.get_setting("theme") == "dark"
    settings.set_setting("theme", "light")
    assert settings.get_setting("theme") == "light"


def test_set_unknown_key():
    # 未知配置键拒绝写入并返回 False
    assert settings.set_setting("不存在的键", 1) is False


def test_window_geometry_roundtrip():
    # base64 编码往返（S8.5 回归）；非法值返回 None（FIX001.25：latin1 兼容层按废弃方案移除）
    assert settings.save_window_geometry(b"\x01\x02\x03\xff")
    assert settings.load_window_geometry() == b"\x01\x02\x03\xff"
    settings.save_config(UserConfig(window_geometry="!!!not-base64!!!"))
    assert settings.load_window_geometry() is None


def test_get_alarms_copy():
    # get_alarms 返回副本：外部修改不污染缓存（S8.5 回归）
    settings.save_alarms([{"id": "a1", "label": "测试"}])
    alarms = settings.get_alarms()
    alarms.append({"id": "污染"})
    assert len(settings.get_alarms()) == 1
