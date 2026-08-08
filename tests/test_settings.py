# 用户配置模块测试（S9.7 测试引入）
# 覆盖：默认值（来自 static）、读写往返、损坏 JSON 容错、缓存、未知键、base64 几何、副本隔离

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
    # base64 编码往返 + 旧 latin1 格式兼容（S8.5 回归）
    assert settings.save_window_geometry(b"\x01\x02\x03\xff")
    assert settings.load_window_geometry() == b"\x01\x02\x03\xff"
    # 旧格式（latin1 字符串）兼容
    settings.save_config(UserConfig(window_geometry="legacy_str"))
    assert settings.load_window_geometry() == "legacy_str".encode("latin1")


def test_get_alarms_copy():
    # get_alarms 返回副本：外部修改不污染缓存（S8.5 回归）
    settings.save_alarms([{"id": "a1", "label": "测试"}])
    alarms = settings.get_alarms()
    alarms.append({"id": "污染"})
    assert len(settings.get_alarms()) == 1
