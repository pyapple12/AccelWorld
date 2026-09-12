# 闹钟管理模块（S5 引入异步播放，预设铃声移入后台线程）
# 提供闹钟数据模型、闹钟匹配逻辑和音频播放功能

import time as time_module
import uuid
import winsound
import logging
from dataclasses import dataclass, field, asdict
from datetime import datetime, time, timedelta
from typing import List, Optional, Literal, Dict, Any
from enum import Enum

# dataclass 反序列化通用工具（S9.4 抽象）
from utils.dataclass_utils import dataclass_from_dict

# 静态配置（闹钟上限参数）
from config.static.static_config import get_static_config

# 配置日志
logger = logging.getLogger(__name__)


class PresetSound(Enum):
    CLASSIC = "classic"
    GENTLE = "gentle"
    BEEP = "beep"
    CHIME = "chime"

    @classmethod
    def display_names(cls) -> List[str]:
        # 按枚举顺序自动生成（value.title()），避免硬编码列表与枚举顺序错位（E3）
        return [m.value.title() for m in cls]

    @classmethod
    def from_value(cls, value: str) -> "PresetSound":
        # 忽略大小写匹配枚举值，未命中返回默认铃声
        for member in cls:
            if member.value == value.lower():
                return member
        return cls.CLASSIC

    @property
    def display_name(self) -> str:
        # 当前成员的显示名（与 display_names 顺序对应，S9.6 封装互转）
        return self.display_names()[self.index()]

    def index(self) -> int:
        # 当前成员在枚举中的序号（下拉框索引互转用，S9.6 封装）
        return list(type(self)).index(self)

    @classmethod
    def from_index(cls, index: int) -> "PresetSound":
        # 按序号取枚举成员（S9.6 封装）
        return list(cls)[index]


# 预设铃声播放参数（频率, 重复次数, 间隔毫秒）——模块级常量，避免每次调用重建（E5）
# 注：SUPPORTED_AUDIO_FORMATS 已迁 ui/alarm_dialog.py（FIX002.17：Qt 文件对话框过滤器
# 字符串属 UI 展示配置，不应置于业务层）
_PRESET_SOUND_CONFIG = {
    PresetSound.CLASSIC: (800, 3, 500),
    PresetSound.GENTLE: (600, 2, 800),
    PresetSound.BEEP: (1200, 5, 200),
    PresetSound.CHIME: (1000, 4, 600),
}


# 支持的音频文件格式常量已迁 ui/alarm_dialog.py（FIX002.17 分层修正）


@dataclass
class Alarm:
    label: str
    time: str  # HH:MM 格式
    sound_type: Literal["preset", "custom"] = "preset"
    sound_value: str = "classic"
    repeat_days: List[int] = field(default_factory=list)  # 0-6, 空=不重复
    enabled: bool = True
    id: str = field(default_factory=lambda: str(uuid.uuid4())[:8])
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())

    def __post_init__(self) -> None:
        # repeat_days 规范化：数字字符串强转 int、布尔剔除（int(True)=1 穿透防御）、
        # 非整数值剔除（[1.7] 与 ["1.5"] 行为一致，FIX002.17）、越界/重复剔除
        normalized: List[int] = []
        for day in self.repeat_days or []:
            if isinstance(day, bool):
                continue
            if isinstance(day, float) and not day.is_integer():
                continue
            try:
                day_int = int(day)
            except (TypeError, ValueError):
                continue
            if 0 <= day_int <= 6 and day_int not in normalized:
                normalized.append(day_int)
        self.repeat_days = normalized
        # 时间格式非法直接拒绝构造，保证后续匹配逻辑安全
        if not self._validate_time(self.time):
            raise ValueError(f"Invalid time format: {self.time}, expected HH:MM")

    @staticmethod
    def _validate_time(t: str) -> bool:
        # 无冒号时补 :00 再走 fromisoformat 校验
        try:
            time.fromisoformat(t if ":" in t else t + ":00")
            return True
        except ValueError:
            return False

    def should_trigger_on(self, check_time: datetime) -> bool:
        # 启用检查 → 时分匹配 → 重复规则：一次性仅创建当天触发，重复闹钟按星期
        if not self.enabled:
            return False

        # 检查时间是否匹配
        alarm_time = time.fromisoformat(
            self.time if ":" in self.time else self.time + ":00"
        )
        current_time = check_time.time()

        if (
            alarm_time.hour != current_time.hour
            or alarm_time.minute != current_time.minute
        ):
            return False

        # 一次性闹钟（无重复天数）：按"计划触发日"匹配——创建时设定时间未过当日为创建当天，
        # 已过当日自动顺延次日触发（FIX001.20 用户定案，替代 S8.4 的"仅创建当天"语义，
        # 修复创建时已过时间永不触发的死角）
        if not self.repeat_days:
            try:
                created_dt = datetime.fromisoformat(self.created_at)
            except (ValueError, TypeError):
                # created_at 数据异常（缺失/非字符串，FIX001.8 补捕 TypeError）时保守不触发，
                # 避免意外每天响
                return False
            scheduled_date = created_dt.date()
            if (alarm_time.hour, alarm_time.minute) <= (
                created_dt.hour,
                created_dt.minute,
            ):
                # 创建时刻已到/过设定时分（同分钟视为已过，防创建后当分钟即响）
                scheduled_date += timedelta(days=1)
            return check_time.date() == scheduled_date

        # 重复闹钟：检查当前星期是否在重复设置中
        return check_time.weekday() in self.repeat_days

    def to_dict(self) -> Dict[str, Any]:
        # asdict 递归转 dict（标准库一行调用，无需包装层）
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Optional["Alarm"]:
        # created_at 语义必填：键存在但类型非法时整条拒绝（FIX003.8：防一次性闹钟
        # 按载入时刻重建"复活"）；键缺失走默认重建（旧版本配置兼容）
        if "created_at" in data and not isinstance(data["created_at"], str):
            logger.warning(f"闹钟 created_at 类型非法，整条跳过: {data.get('created_at')!r}")
            return None
        # 委托通用工具（容错模式）：非法 time 返回 None 由调用方跳过
        return dataclass_from_dict(cls, data, tolerant=True)

    def is_one_time(self) -> bool:
        # 无重复天数即为一次性
        return len(self.repeat_days) == 0


# ------------------- 预设铃声播放（纯 winsound，无 UI 依赖） -------------------
# S10.5 D2：自定义音频/异步分发已迁至 ui/audio_player.py（UI 库依赖收敛到 ui 层）


def play_preset_sound(preset: PresetSound) -> None:
    # 按预设频率/次数/间隔循环 Beep，调用方应经 audio_player 的 async 入口后台化
    try:
        frequency, repeat_count, interval = _PRESET_SOUND_CONFIG[preset]
        duration = 200  # 每次蜂鸣持续时间（毫秒）

        for i in range(repeat_count):
            winsound.Beep(frequency, duration)
            if i < repeat_count - 1:
                time_module.sleep(interval / 1000.0)
    except Exception as e:
        # 播放失败记录堆栈（GUI 应用 print 不可见，日志系统已配置）
        logger.exception(f"播放预设铃声失败: {e}")


def _trigger_key(check_time: datetime) -> str:
    # 生成闹钟触发去重键（"YYYY-MM-DD HH:MM"，含日期维度跨天不误判，S9.4 抽取）
    return check_time.strftime("%Y-%m-%d %H:%M")


# ------------------- 闹钟管理器 -------------------


class AlarmManager:
    def __init__(self) -> None:
        # 空列表启动；上限来自静态配置；_last_triggered 存"日期+分钟"触发去重记录
        self.alarms: List[Alarm] = []
        self.max_alarms = int(get_static_config().base["max_alarms"])
        self._last_triggered: Dict[
            str, str
        ] = {}  # alarm_id -> "YYYY-MM-DD HH:MM"（含日期维度，S8.1）；
        # 仅内存不持久化：同分钟内重启理论上可重复响一次，窗口极窄接受（FIX001.21 P3#15）

    def add_alarm(self, alarm: Alarm) -> bool:
        # 上限校验 + 同时间同标签去重（失败经日志记录，GUI 弹窗提示由面板层负责）
        if len(self.alarms) >= self.max_alarms:
            logger.warning(f"已达到最大闹钟数量限制 ({self.max_alarms})")
            return False

        # 检查是否已存在相同时间的闹钟
        for existing in self.alarms:
            if existing.time == alarm.time and existing.label == alarm.label:
                logger.warning("已存在相同时间和标签的闹钟")
                return False

        self.alarms.append(alarm)
        return True

    def remove_alarm(self, alarm_id: str) -> bool:
        # 线性查找并 remove，同时清理 _last_triggered 防止残留
        for alarm in self.alarms:
            if alarm.id == alarm_id:
                self.alarms.remove(alarm)
                self._last_triggered.pop(alarm_id, None)
                return True
        return False

    def get_alarm(self, alarm_id: str) -> Optional[Alarm]:
        # 线性查找，未命中返回 None
        for alarm in self.alarms:
            if alarm.id == alarm_id:
                return alarm
        return None

    def replace_alarm(self, alarm: Alarm) -> bool:
        # 编辑对话框保留原 ID 构造新对象，此处原位替换；
        # 复用 add_alarm 的去重检查（排除自身 id，FIX003.7：防编辑成重复条目绕过 add 路径）
        for existing in self.alarms:
            if (
                existing.id != alarm.id
                and existing.time == alarm.time
                and existing.label == alarm.label
            ):
                logger.warning("已存在相同时间和标签的闹钟")
                return False
        for i, existing in enumerate(self.alarms):
            if existing.id == alarm.id:
                self.alarms[i] = alarm
                return True
        return False

    def toggle_alarm(self, alarm_id: str) -> bool:
        # 取到对象后翻转 enabled
        alarm = self.get_alarm(alarm_id)
        if alarm:
            alarm.enabled = not alarm.enabled
            return True
        return False

    def check_alarms(self, check_time: datetime) -> List[Alarm]:
        # 去重键含日期维度（经 _trigger_key），跨天不误判；命中即标记
        time_str = _trigger_key(check_time)
        triggered = []

        for alarm in self.alarms:
            if not alarm.enabled:
                continue

            # 检查是否已在同一分钟触发过
            last_triggered = self._last_triggered.get(alarm.id)
            if last_triggered == time_str:
                continue

            if alarm.should_trigger_on(check_time):
                triggered.append(alarm)
                # 记录触发时间
                self._last_triggered[alarm.id] = time_str

        return triggered

    def to_dict_list(self) -> List[Dict[str, Any]]:
        # 逐闹钟 to_dict 收集
        return [alarm.to_dict() for alarm in self.alarms]

    def from_dict_list(self, data: List[Dict[str, Any] | None]) -> None:
        # 空条目与构造失败（from_dict 返回 None）的闹钟过滤后加载；
        # 跳过条目记 warning 便于诊断（FIX001.21：静默丢弃不可诊断）
        loaded: List[Alarm] = []
        for item in data:
            if not item:
                continue
            alarm = Alarm.from_dict(item)
            if alarm is not None:
                loaded.append(alarm)
            else:
                logger.warning(f"跳过无法解析的闹钟条目: {item!r}")
        self.alarms = loaded


# ===== modules/alarm_service.py 函数/类说明 =====
# PresetSound(Enum): 预设铃声枚举；display_names 供下拉框，from_value 大小写不敏感匹配兜底 CLASSIC
# Alarm(dataclass): 闹钟数据模型
#   __post_init__: repeat_days 规范化（字符串强转/越界剔除，FIX001.9/21）+ 时间格式校验
#     （非法抛 ValueError 拒绝构造）
#   should_trigger_on(check_time): 启用 → 时分匹配 → 重复规则
#     （一次性按计划触发日：创建时未过时间为当天、已过自动顺延次日 FIX001.20；重复闹钟按星期）
#   to_dict/from_dict: JSON 序列化往返；from_dict 容错（类型校验过滤，非法数据返回 None）
#   is_one_time: 无重复天数即一次性
# play_preset_sound(preset): winsound.Beep 组合（阻塞，由 ui/audio_player.py async 入口后台化）
# AlarmManager: 闹钟管理（上限 10、同时间同标签去重、同分钟触发去重 _last_triggered）
#   add/remove/get/replace/toggle/check/to_dict_list/from_dict_list
#   设计理由：数据模型与匹配逻辑集中在 service 层，UI 只做展示与持久化；
#   播放职责已迁至 ui/audio_player.py（S10.5 D2：UI 库依赖不进入业务层）
#   异常处理：构造校验抛 ValueError；播放失败记录日志
#   关联配置：无（纯业务层）；UI 依赖 ui/panels/alarm_panel.py 与 ui/alarm_dialog.py
