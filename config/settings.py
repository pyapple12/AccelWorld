# 配置文件管理模块（用户配置：UserConfig dataclass，可读写）
# S9.5 定案：配置保存于项目目录（修正原 ~/.config/accelworld 决策），默认值从静态配置读取

import base64
import logging
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

# 通用文件读写工具（缓存单例 + 项目根定位）
from utils.file_utils import read_json_cached, write_json, get_project_root

# dataclass 反序列化通用工具（S9.4 抽象）
from utils.dataclass_utils import dataclass_from_dict

# 应用静态配置（默认值/路径参数来源）
from config.static.static_config import get_static_config

# 配置日志
logger = logging.getLogger(__name__)

# 用户配置目录/文件（项目内，随项目走）
CONFIG_DIR = get_project_root() / "config"
CONFIG_FILE = get_project_root() / get_static_config().base["user_config"]


@dataclass
class UserConfig:
    time_dilation_rate: float = field(
        default_factory=lambda: get_static_config().base["default_rate"]
    )  # 时间膨胀倍率
    theme: str = field(
        default_factory=lambda: get_static_config().base["default_theme"]
    )  # 主题: light/dark
    last_city: str = field(
        default_factory=lambda: get_static_config().base["default_city"]
    )  # 上次选择的城市
    last_timezone: str = field(
        default_factory=lambda: get_static_config().base["default_timezone"]
    )  # 上次选择的时区
    countdown_target: str = ""  # 上次设置的倒计时目标（结构默认：未设置）
    window_geometry: Optional[str] = None  # 窗口位置和大小（base64 编码）
    alarms: List[Any] = field(default_factory=list)  # 闹钟列表（结构默认：空）

    def to_dict(self) -> Dict[str, Any]:
        # asdict 递归转 dict（标准库一行调用，无需包装层）
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "UserConfig":
        # 委托通用工具：字段白名单过滤，缺省字段由 dataclass 默认值兜底（非容错模式，必不为 None）
        result = dataclass_from_dict(cls, data)
        assert result is not None
        return result


def load_config() -> UserConfig:
    # 经 file_utils 缓存单例读取，仅成功解析才入缓存
    data = read_json_cached(CONFIG_FILE, None)
    if data is None:
        return UserConfig()
    return UserConfig.from_dict(data)


def save_config(config: UserConfig) -> bool:
    # 写入后自动清理缓存，保证下次读取一致
    result = write_json(CONFIG_FILE, config.to_dict())
    if not result:
        logger.error("保存配置文件失败")
    return result


def get_setting(key: str, default: Any = None) -> Any:
    # 经 AppConfig 字段反射取值，未知键返回 default
    config = load_config()
    return getattr(config, key, default)


def set_setting(key: str, value: Any) -> bool:
    # 未知键拒绝写入并记日志；修改后立即落盘
    config = load_config()
    if not hasattr(config, key):
        logger.error(f"未知配置键: {key}")
        return False
    setattr(config, key, value)
    return save_config(config)


def save_window_geometry(geometry: bytes) -> bool:
    # QByteArray/bytes 统一转 base64 ASCII 串，JSON 友好且可移植
    try:
        encoded = base64.b64encode(bytes(geometry)).decode("ascii")
    except (TypeError, ValueError) as e:
        logger.error(f"窗口几何数据编码失败: {e}")
        return False
    return set_setting("window_geometry", encoded)


def load_window_geometry() -> Optional[bytes]:
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
    # list() 浅拷贝隔离缓存共享引用
    return list(load_config().alarms)


def save_alarms(alarms: List[Any]) -> bool:
    # 委托 set_setting 统一写盘
    return set_setting("alarms", alarms)


# ===== config/settings.py 函数/常量说明 =====
# UserConfig(dataclass): 用户配置聚合（可读写）
#   to_dict(): JSON 序列化；from_dict(): 反序列化（仅取有效字段+默认值兜底）
#   默认值：rate/theme/city/timezone 经 default_factory 从 static base.json 现取（零硬编码）；
#   countdown_target/window_geometry/alarms 为结构默认（"用户未设置"兜底）
# load_config() -> UserConfig: 缓存单例读取（经 utils/file_utils.py read_json_cached）
# save_config(config) -> bool: 写盘并清理缓存
# get_setting(key, default): 字段反射取值；set_setting(key, value): 未知键拒绝+落盘
# save_window_geometry(geometry): base64 编码存储（修复 D3）
# load_window_geometry(): base64 解码，兼容旧 latin1 格式
# get_alarms/save_alarms: 闹钟配置读写（增删改由 AlarmManager 负责，S9.2 去重清理）
#   设计理由：配置聚合为 dataclass 避免魔法键；缓存单例避免重复 IO（修复 D4）
#   异常处理：JSON 损坏/IO 错误在 file_utils 层兜底返回默认值
#   关联配置：配置文件 config/user_config.json（项目内，S9.5 修正）；默认值来自 config/static/base.json
