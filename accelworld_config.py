"""
配置文件管理模块

负责保存和加载用户设置
"""

import json
import os
from typing import Any, Dict, Optional, Union, List

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
    "alarms": [],  # 闹钟列表
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


def save_window_geometry(geometry: Union[bytes, bytearray]) -> bool:
    """
    保存窗口位置和大小

    :param geometry: 窗口几何数据
    :return: 是否保存成功
    """
    # 处理 bytes/bytearray 或其他类型（如 QByteArray）
    try:
        decoded = geometry.decode('latin1')
    except (AttributeError, TypeError):
        # 非 bytes 类型，直接存储
        decoded = geometry
    return set_setting("window_geometry", decoded)


def load_window_geometry() -> Optional[bytes]:
    """
    加载窗口位置和大小

    :return: 窗口几何数据
    """
    geometry_str = get_setting("window_geometry")
    if geometry_str:
        return geometry_str.encode('latin1')
    return None


# ------------------- 闹钟配置管理 -------------------

def get_alarms() -> List[Any]:
    """
    获取保存的闹钟列表

    :return: 闹钟字典列表
    """
    return get_setting("alarms", [])


def save_alarms(alarms: List[Any]) -> bool:
    """
    保存闹钟列表

    :param alarms: 闹钟字典列表
    :return: 是否保存成功
    """
    return set_setting("alarms", alarms)


def add_alarm(alarm_data: Dict[str, Any]) -> bool:
    """
    添加单个闹钟

    :param alarm_data: 闹钟数据字典
    :return: 是否添加成功
    """
    alarms = get_alarms()
    alarms.append(alarm_data)
    return save_alarms(alarms)


def remove_alarm(alarm_id: str) -> bool:
    """
    移除指定 ID 的闹钟

    :param alarm_id: 闹钟 ID
    :return: 是否移除成功
    """
    alarms = get_alarms()
    for i, alarm in enumerate(alarms):
        if alarm.get("id") == alarm_id:
            alarms.pop(i)
            return save_alarms(alarms)
    return False


def update_alarm(alarm_id: str, alarm_data: Dict[str, Any]) -> bool:
    """
    更新指定 ID 的闹钟

    :param alarm_id: 闹钟 ID
    :param alarm_data: 新的闹钟数据
    :return: 是否更新成功
    """
    alarms = get_alarms()
    for i, alarm in enumerate(alarms):
        if alarm.get("id") == alarm_id:
            alarms[i] = alarm_data
            return save_alarms(alarms)
    return False


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
