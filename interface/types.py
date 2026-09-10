# 接口层数据类型转出（DTO re-export + UI 偏好聚合）
# UI 层的类型引用统一走 interface.types，禁止直接 import modules（plan#UI2.0 铁律 2）；
# 本模块只含数据定义与转出，无任何行为逻辑，无 Qt 依赖

from dataclasses import dataclass

# 后端 DTO 转出（纯数据类：TimeInfo/WeatherData/Alarm/PresetSound 均无 Qt 依赖）
from modules.time_dilation import TimeInfo
from modules.weather_service import WeatherData
from modules.alarm_service import Alarm, PresetSound


@dataclass
class UiPreferences:
    theme: str  # 主题偏好（light/dark；PL002 扩展 auto/light/dark 三态）
    last_city: str  # 上次选择的城市（缺省回退 default_city）
    last_timezone: str  # 上次选择的时区（缺省回退 default_timezone）
    countdown_target: str  # 上次设置的倒计时目标文本（未设置为空串）


# ===== interface/types.py 函数/类说明 =====
# UiPreferences(dataclass): UI 会话偏好聚合（theme/last_city/last_timezone/countdown_target）
#   由 AppInterface.get_ui_preferences() 从 config.settings 组装（缺省值来自 base.json），
#   UI 层经该对象读取启动偏好，不再直接 import settings
#   设计理由：UI 需要的用户配置收敛为一个只读快照，save_* 方法单独承接写路径
# TimeInfo/WeatherData/Alarm/PresetSound: 后端 DTO 原样转出（re-export，不改行为）
#   设计理由：类型引用收口到 interface.types 后，ui/ 下可零 modules import；
#   UI 与后端共享同一 dataclass 定义（鸭子类型不引入复制层，避免双向转换样板）
#   关联配置：TimeInfo 来自 modules/time_dilation.py；WeatherData 来自
#   modules/weather_service.py；Alarm/PresetSound 来自 modules/alarm_service.py
