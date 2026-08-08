# 配置文件管理模块（S2 引入 AppConfig dataclass + file_utils 缓存单例）
# 负责保存和加载用户设置；路径处理使用 pathlib

import base64
import logging
from dataclasses import dataclass, field, fields, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

# 通用文件读写工具（缓存单例）
from utils.file_utils import read_json_cached, write_json

# 配置日志
logger = logging.getLogger(__name__)

# 配置文件路径（pathlib）
CONFIG_DIR = Path.home() / ".config" / "accelworld"
CONFIG_FILE = CONFIG_DIR / "config.json"


@dataclass
class AppConfig:
    """应用配置数据类（聚合全部用户设置，替代裸 dict）"""

    time_dilation_rate: float = 2.0  # 时间膨胀倍率
    theme: str = "light"  # 主题: light/dark
    last_city: str = "北京"  # 上次选择的城市
    last_timezone: str = "Asia/Shanghai"  # 上次选择的时区
    countdown_target: str = ""  # 上次设置的倒计时目标
    window_geometry: Optional[str] = None  # 窗口位置和大小（base64 编码）
    alarms: List[Any] = field(default_factory=list)  # 闹钟列表

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典（用于 JSON 序列化）"""
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "AppConfig":
        """从字典创建（用于 JSON 反序列化），仅取有效字段并兜底默认值"""
        valid_fields = {f.name for f in fields(cls)}
        filtered = {k: v for k, v in data.items() if k in valid_fields}
        return cls(**filtered)


def load_config() -> AppConfig:
    """
    加载配置文件（缓存单例，仅首次读取磁盘）

    :return: AppConfig 配置数据类
    """
    # 经 file_utils 缓存单例读取，仅成功解析才入缓存
    data = read_json_cached(CONFIG_FILE, None)
    if data is None:
        return AppConfig()
    return AppConfig.from_dict(data)


def save_config(config: AppConfig) -> bool:
    """
    保存配置文件

    :param config: AppConfig 配置数据类
    :return: 是否保存成功
    """
    # 写入后自动清理缓存，保证下次读取一致
    result = write_json(CONFIG_FILE, config.to_dict())
    if not result:
        logger.error("保存配置文件失败")
    return result


def get_setting(key: str, default: Any = None) -> Any:
    """
    获取单个设置

    :param key: 设置键
    :param default: 默认值
    :return: 设置值
    """
    # 经 AppConfig 字段反射取值，未知键返回 default
    config = load_config()
    return getattr(config, key, default)


def set_setting(key: str, value: Any) -> bool:
    """
    设置单个设置

    :param key: 设置键
    :param value: 设置值
    :return: 是否保存成功
    """
    # 未知键拒绝写入并记日志；修改后立即落盘
    config = load_config()
    if not hasattr(config, key):
        logger.error(f"未知配置键: {key}")
        return False
    setattr(config, key, value)
    return save_config(config)


def save_window_geometry(geometry: bytes) -> bool:
    """
    保存窗口位置和大小（base64 编码存储）

    :param geometry: 窗口几何数据（bytes/QByteArray）
    :return: 是否保存成功
    """
    # QByteArray/bytes 统一转 base64 ASCII 串，JSON 友好且可移植
    try:
        encoded = base64.b64encode(bytes(geometry)).decode("ascii")
    except (TypeError, ValueError) as e:
        logger.error(f"窗口几何数据编码失败: {e}")
        return False
    return set_setting("window_geometry", encoded)


def load_window_geometry() -> Optional[bytes]:
    """
    加载窗口位置和大小（兼容旧版 latin1 存储格式）

    :return: 窗口几何数据 bytes，无则返回 None
    """
    # base64 解码失败时回退旧格式 latin1，双重兼容
    encoded = get_setting("window_geometry")
    if not encoded:
        return None
    try:
        # 新版 base64 存储
        return base64.b64decode(encoded)
    except (ValueError, TypeError):
        # 兼容旧版 latin1 字符串存储
        try:
            return encoded.encode("latin1")
        except (UnicodeEncodeError, AttributeError):
            return None


# ------------------- 闹钟配置管理 -------------------


def get_alarms() -> List[Any]:
    """
    获取保存的闹钟列表

    :return: 闹钟字典列表
    """
    return load_config().alarms


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
    # 按 ID 定位后整体替换条目并落盘
    alarms = get_alarms()
    for i, alarm in enumerate(alarms):
        if alarm.get("id") == alarm_id:
            alarms[i] = alarm_data
            return save_alarms(alarms)
    return False


# ===== config/settings.py 函数/常量说明 =====
# AppConfig(dataclass): 应用配置聚合（S2 替代裸 dict）
#   to_dict(): JSON 序列化；from_dict(): 反序列化（仅取有效字段+默认值兜底）
# load_config() -> AppConfig: 缓存单例读取（经 utils/file_utils.py read_json_cached）
# save_config(config) -> bool: 写盘并清理缓存
# get_setting(key, default): 字段反射取值；set_setting(key, value): 未知键拒绝+落盘
# save_window_geometry(geometry): base64 编码存储（修复 D3）
# load_window_geometry(): base64 解码，兼容旧 latin1 格式
# get_alarms/save_alarms/add_alarm/remove_alarm/update_alarm: 闹钟配置 CRUD
#   设计理由：配置聚合为 dataclass 避免魔法键；缓存单例避免重复 IO（修复 D4）
#   异常处理：JSON 损坏/IO 错误在 file_utils 层兜底返回默认值
#   关联配置：配置文件 ~/.config/accelworld/config.json
