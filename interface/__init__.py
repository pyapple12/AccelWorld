# 接口层包（UI 2.0 架构：后端 / 接口 / UI 三大块，plan#UI2.0）
# interface 是 UI 访问后端的唯一契约层：允许 import 后端（modules/config/data/utils），
# 严禁引入任何 Qt 绑定（无 Qt 依赖铁律）——保证"后端 + 接口"的 pytest 可脱离 Qt 独立运行

from interface.app_interface import AppInterface
from interface.types import Alarm, PresetSound, TimeInfo, UiPreferences, WeatherData

__all__ = [
    "AppInterface",
    "TimeInfo",
    "WeatherData",
    "Alarm",
    "PresetSound",
    "UiPreferences",
]


# ===== interface/__init__.py 说明 =====
# 包职责：UI 层（ui/）只允许 import interface（及其 types 子模块），不允许直接
# import modules/config/data（plan#UI2.0 铁律 2）；本包对后端单向依赖、对 UI 单向暴露
# 导出成员：AppInterface（应用接口）+ TimeInfo/WeatherData/Alarm/PresetSound（后端 DTO
# 转出）+ UiPreferences（UI 偏好聚合 dataclass，定义于 types.py）
# 关联约束：Qt 绑定字样在本包必须恒为零结果（rg 验收命令，PL001.01）
