"""
配置文件管理模块

负责保存和加载用户设置
"""

import json
import os
from typing import Any, Dict, Optional

# 配置文件路径
CONFIG_DIR = os.path.expanduser("~/.config/accelworld")
CONFIG_FILE = os.path.join(CONFIG_DIR, "config.json")

# 默认配置
DEFAULT_CONFIG = {
    "time_dilation_rate": 2.0,  # 时间膨胀倍率
    "theme": "light",  # 主题: light/dark
    "last_city": "北京",  # 上次选择的城市
    "last_timezone": "Asia/Shanghai",  # 上次选择的时区
    "countdown_target": "",  # 上次设置的倒计时目标
    "window_geometry": None,  # 窗口位置和大小
}


def get_config_dir() -> str:
    """获取配置目录路径"""
    return CONFIG_DIR


def get_config_file() -> str:
    """获取配置文件路径"""
    return CONFIG_FILE


def load_config() -> Dict[str, Any]:
    """
    加载配置文件

    :return: 配置字典
    """
    if not os.path.exists(CONFIG_FILE):
        return DEFAULT_CONFIG.copy()

    try:
        with open(CONFIG_FILE, "r", encoding="utf-8") as f:
            config = json.load(f)
            # 合并默认配置，确保所有键都存在
            merged = DEFAULT_CONFIG.copy()
            merged.update(config)
            return merged
    except (json.JSONDecodeError, IOError) as e:
        print(f"加载配置文件失败: {e}")
        return DEFAULT_CONFIG.copy()


def save_config(config: Dict[str, Any]) -> bool:
    """
    保存配置文件

    :param config: 配置字典
    :return: 是否保存成功
    """
    # 确保配置目录存在
    os.makedirs(CONFIG_DIR, exist_ok=True)

    try:
        with open(CONFIG_FILE, "w", encoding="utf-8") as f:
            json.dump(config, f, ensure_ascii=False, indent=4)
        return True
    except IOError as e:
        print(f"保存配置文件失败: {e}")
        return False


def get_setting(key: str, default: Any = None) -> Any:
    """
    获取单个设置

    :param key: 设置键
    :param default: 默认值
    :return: 设置值
    """
    config = load_config()
    return config.get(key, default)


def set_setting(key: str, value: Any) -> bool:
    """
    设置单个设置

    :param key: 设置键
    :param value: 设置值
    :return: 是否保存成功
    """
    config = load_config()
    config[key] = value
    return save_config(config)


def save_window_geometry(geometry: bytes) -> bool:
    """
    保存窗口位置和大小

    :param geometry: 窗口几何数据
    :return: 是否保存成功
    """
    return set_setting("window_geometry", geometry.decode('latin1') if isinstance(geometry, bytes) else geometry)


def load_window_geometry() -> Optional[bytes]:
    """
    加载窗口位置和大小

    :return: 窗口几何数据
    """
    geometry_str = get_setting("window_geometry")
    if geometry_str:
        return geometry_str.encode('latin1')
    return None


# ------------------- 测试 -------------------
if __name__ == "__main__":
    # 测试配置读写
    config = load_config()
    print(f"当前配置: {config}")

    # 修改配置
    config["time_dilation_rate"] = 3.0
    config["theme"] = "dark"
    save_config(config)

    # 重新加载
    config = load_config()
    print(f"修改后配置: {config}")
