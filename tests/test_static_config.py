# 静态配置加载模块测试（FIX002.13 引入）
# 覆盖：映射表与分类内容的三形态损坏均抛 RuntimeError（与注释承诺一致）

import pytest

import config.static.static_config as static_config


def _reset_cache(monkeypatch):
    # 重置模块级单例缓存，保证每个用例独立加载
    monkeypatch.setattr(static_config, "_static_config_cache", None)


def test_missing_required_key_raises_runtime_error(monkeypatch):
    # 映射表缺必需分类（如缺 ui）→ RuntimeError 而非 KeyError（FIX002.13）
    _reset_cache(monkeypatch)

    def fake_read_json(path, default=None):
        # 模拟 config.json 只注册了 base 分类
        if path.name == "config.json":
            return {"base": "base.json"}
        return {"version": "0.0.0"}

    monkeypatch.setattr(static_config, "read_json", fake_read_json)
    with pytest.raises(RuntimeError):
        static_config._load_static_config()


def test_non_string_path_raises_runtime_error(monkeypatch):
    # 映射表键值为非字符串 → RuntimeError 而非 TypeError（FIX002.13）
    _reset_cache(monkeypatch)

    def fake_read_json(path, default=None):
        if path.name == "config.json":
            return {"base": "base.json", "ui": 123}
        return {}

    monkeypatch.setattr(static_config, "read_json", fake_read_json)
    with pytest.raises(RuntimeError):
        static_config._load_static_config()


def test_non_dict_category_content_raises_runtime_error(monkeypatch):
    # 分类文件内容为合法 JSON 但非 dict → RuntimeError 而非下游 TypeError（FIX002.13）
    _reset_cache(monkeypatch)

    def fake_read_json(path, default=None):
        if path.name == "config.json":
            return {"base": "base.json", "ui": "ui.json"}
        if path.name == "base.json":
            return {"version": "0.0.0"}
        return 123  # ui.json 内容为非 dict

    monkeypatch.setattr(static_config, "read_json", fake_read_json)
    with pytest.raises(RuntimeError):
        static_config._load_static_config()
