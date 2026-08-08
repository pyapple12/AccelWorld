# 闹钟管理模块（S5 引入异步播放，预设铃声移入后台线程）
# 提供闹钟数据模型、闹钟匹配逻辑和音频播放功能

import threading
import time as time_module
import uuid
import winsound
import logging
from dataclasses import dataclass, field, asdict
from datetime import datetime, time
from typing import List, Optional, Literal, Dict, Any
from enum import Enum

# dataclass 反序列化通用工具（S9.4 抽象）
from utils.dataclass_utils import dataclass_from_dict

# 静态配置（闹钟上限参数）
from config.static.static_config import get_static_config

# 配置日志
logger = logging.getLogger(__name__)


class PresetSound(Enum):
    """预设铃声枚举"""

    CLASSIC = "classic"
    GENTLE = "gentle"
    BEEP = "beep"
    CHIME = "chime"

    @classmethod
    def display_names(cls) -> List[str]:
        """获取显示名称列表"""
        # 供 GUI 下拉框固定顺序展示
        return ["Classic", "Gentle", "Beep", "Chime"]

    @classmethod
    def from_value(cls, value: str) -> "PresetSound":
        """根据值获取枚举成员（未知值兜底 CLASSIC）"""
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


# 支持的音频文件格式
SUPPORTED_AUDIO_FORMATS = (
    "Audio Files (*.wav *.mp3 *.ogg *.flac *.m4a *.wma *.aac);;All Files (*)"
)


@dataclass
class Alarm:
    """
    闹钟数据类

    属性:
        id: 唯一标识符
        label: 闹钟标签
        time: 触发时间 (HH:MM 格式)
        enabled: 是否启用
        sound_type: 声音类型 ("preset" 或 "custom")
        sound_value: 声音值 (预设音效名或自定义文件路径)
        repeat_days: 重复天数列表 (0=周一, 6=周日, 空列表表示不重复)
        created_at: 创建时间
    """

    label: str
    time: str  # HH:MM 格式
    sound_type: Literal["preset", "custom"] = "preset"
    sound_value: str = "classic"
    repeat_days: List[int] = field(default_factory=list)  # 0-6, 空=不重复
    enabled: bool = True
    id: str = field(default_factory=lambda: str(uuid.uuid4())[:8])
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())

    def __post_init__(self) -> None:
        """验证和规范化数据"""
        # 时间格式非法直接拒绝构造，保证后续匹配逻辑安全
        if not self._validate_time(self.time):
            raise ValueError(f"Invalid time format: {self.time}, expected HH:MM")

    @staticmethod
    def _validate_time(t: str) -> bool:
        """验证时间格式"""
        # 无冒号时补 :00 再走 fromisoformat 校验
        try:
            time.fromisoformat(t if ":" in t else t + ":00")
            return True
        except ValueError:
            return False

    def should_trigger_on(self, check_time: datetime) -> bool:
        """
        检查是否应该在指定时间触发

        :param check_time: 要检查的时间
        :return: 是否应该触发
        """
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

        # 一次性闹钟（无重复天数）：仅在创建当天触发（S8.4 语义澄清，与注释一致）
        if not self.repeat_days:
            try:
                created_date = datetime.fromisoformat(self.created_at).date()
            except ValueError:
                # created_at 数据异常时保守不触发，避免意外每天响
                return False
            return check_time.date() == created_date

        # 重复闹钟：检查当前星期是否在重复设置中
        return check_time.weekday() in self.repeat_days

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典（用于 JSON 序列化）"""
        # asdict 递归转 dict（标准库一行调用，无需包装层）
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Optional["Alarm"]:
        """从字典创建（用于 JSON 反序列化），未知键过滤，非法数据返回 None"""
        # 委托通用工具（容错模式）：非法 time 返回 None 由调用方跳过
        return dataclass_from_dict(cls, data, tolerant=True)

    def is_one_time(self) -> bool:
        """是否为一次性闹钟（不重复）"""
        # 无重复天数即为一次性
        return len(self.repeat_days) == 0


# ------------------- 音频播放 -------------------


def play_preset_sound(preset: PresetSound) -> None:
    """播放预设铃声（winsound 蜂鸣组合，阻塞式）"""
    # 按预设频率/次数/间隔循环 Beep，调用方应经 async 入口后台化
    try:
        # 预设音效通过频率、重复播放次数和间隔模拟
        preset_config = {
            PresetSound.CLASSIC: (800, 3, 500),
            PresetSound.GENTLE: (600, 2, 800),
            PresetSound.BEEP: (1200, 5, 200),
            PresetSound.CHIME: (1000, 4, 600),
        }

        frequency, repeat_count, interval = preset_config.get(preset, (800, 3, 500))
        duration = 200  # 每次蜂鸣持续时间（毫秒）

        for i in range(repeat_count):
            winsound.Beep(frequency, duration)
            if i < repeat_count - 1:
                time_module.sleep(interval / 1000.0)
    except Exception as e:
        # 播放失败记录堆栈（GUI 应用 print 不可见，日志系统已配置）
        logger.exception(f"播放预设铃声失败: {e}")


def play_custom_sound(file_path: str) -> bool:
    """
    播放自定义音频文件（QMediaPlayer 异步播放，不阻塞）

    :param file_path: 音频文件路径
    :return: 是否播放成功
    """
    # QMediaPlayer 需在有 QApplication 的线程创建，故保持主线程调用
    try:
        from PyQt6.QtMultimedia import QMediaPlayer, QAudioOutput
        from PyQt6.QtCore import QUrl

        player = QMediaPlayer()
        audio_output = QAudioOutput()
        player.setAudioOutput(audio_output)
        player.setSource(QUrl.fromLocalFile(file_path))
        audio_output.setVolume(1.0)
        player.play()

        return True
    except ImportError:
        # PyQt6.Multimedia 缺失属环境问题，记录日志返回 False
        logger.exception("PyQt6.Multimedia 不可用，无法播放自定义音频")
        return False
    except Exception as e:
        logger.exception(f"播放自定义音频失败: {e}")
        return False


def play_alarm_sound(alarm: Alarm) -> None:
    """播放闹钟声音（按 sound_type 分发到预设/自定义播放）"""
    # 同步版本供后台线程与测试复用
    if alarm.sound_type == "preset":
        preset = PresetSound.from_value(alarm.sound_value)
        play_preset_sound(preset)
    else:
        play_custom_sound(alarm.sound_value)


def play_alarm_sound_async(alarm: Alarm) -> None:
    """
    异步播放闹钟声音（UI 不冻结）

    预设铃声（winsound 阻塞+sleep）移入后台 daemon 线程；
    自定义铃声（QMediaPlayer 异步播放）保持主线程执行。

    :param alarm: 闹钟对象
    """
    # 预设铃声走后台线程，自定义铃声保持主线程（QMediaPlayer 线程绑定）
    if alarm.sound_type == "preset":
        threading.Thread(target=play_alarm_sound, args=(alarm,), daemon=True).start()
    else:
        play_alarm_sound(alarm)


def _trigger_key(check_time: datetime) -> str:
    # 生成闹钟触发去重键（"YYYY-MM-DD HH:MM"，含日期维度跨天不误判，S9.4 抽取）
    return check_time.strftime("%Y-%m-%d %H:%M")


# ------------------- 闹钟管理器 -------------------


class AlarmManager:
    """闹钟管理器"""

    def __init__(self) -> None:
        # 空列表启动；上限来自静态配置；_last_triggered 存"日期+分钟"触发去重记录
        self.alarms: List[Alarm] = []
        self.max_alarms = int(get_static_config().base["max_alarms"])
        self._last_triggered: Dict[str, str] = {}  # alarm_id -> "HH:MM"

    def add_alarm(self, alarm: Alarm) -> bool:
        """
        添加闹钟

        :param alarm: 闹钟对象
        :return: 是否添加成功
        """
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
        """按 ID 移除闹钟（同步清理触发去重记录）"""
        # 线性查找并 remove，同时清理 _last_triggered 防止残留
        for alarm in self.alarms:
            if alarm.id == alarm_id:
                self.alarms.remove(alarm)
                self._last_triggered.pop(alarm_id, None)
                return True
        return False

    def get_alarm(self, alarm_id: str) -> Optional[Alarm]:
        """按 ID 获取闹钟"""
        # 线性查找，未命中返回 None
        for alarm in self.alarms:
            if alarm.id == alarm_id:
                return alarm
        return None

    def replace_alarm(self, alarm: Alarm) -> bool:
        """整体替换闹钟（编辑场景，按 ID 定位）"""
        # 编辑对话框保留原 ID 构造新对象，此处原位替换
        for i, existing in enumerate(self.alarms):
            if existing.id == alarm.id:
                self.alarms[i] = alarm
                return True
        return False

    def toggle_alarm(self, alarm_id: str) -> bool:
        """切换闹钟启用状态"""
        # 取到对象后翻转 enabled
        alarm = self.get_alarm(alarm_id)
        if alarm:
            alarm.enabled = not alarm.enabled
            return True
        return False

    def check_alarms(self, check_time: datetime) -> List[Alarm]:
        """
        检查指定时间应该触发的闹钟

        :param check_time: 要检查的时间
        :return: 应该触发的闹钟列表
        """
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
        """转换为字典列表（供 JSON 序列化）"""
        # 逐闹钟 to_dict 收集
        return [alarm.to_dict() for alarm in self.alarms]

    def from_dict_list(self, data: List[Dict[str, Any]]) -> None:
        """从字典列表加载（供 JSON 反序列化），非法条目跳过不阻断整体加载"""
        # 空条目与构造失败（from_dict 返回 None）的闹钟过滤后加载
        self.alarms = [
            alarm
            for item in data
            if item
            if (alarm := Alarm.from_dict(item)) is not None
        ]


# ===== modules/alarm_service.py 函数/类说明 =====
# PresetSound(Enum): 预设铃声枚举；display_names 供下拉框，from_value 大小写不敏感匹配兜底 CLASSIC
# Alarm(dataclass): 闹钟数据模型
#   __post_init__: 时间格式校验（非法抛 ValueError 拒绝构造）
#   should_trigger_on(check_time): 启用 → 时分匹配 → 重复规则
#     （一次性仅创建当天触发，依据 created_at 日期；重复闹钟按星期）
#   to_dict/from_dict: JSON 序列化往返；from_dict 容错（未知键过滤，非法数据返回 None）
#   is_one_time: 无重复天数即一次性
# play_preset_sound(preset): winsound.Beep 组合（阻塞，由 async 入口后台化）
# play_custom_sound(path): QMediaPlayer 异步播放（须主线程，因 QObject 绑定线程）
# play_alarm_sound(alarm): 按 sound_type 分发；play_alarm_sound_async(alarm):
#   预设→daemon 后台线程，自定义→主线程（S5 修复 D6，UI 不冻结）
# AlarmManager: 闹钟管理（上限 10、同时间同标签去重、同分钟触发去重 _last_triggered）
#   add/remove/get/replace/toggle/check/to_dict_list/from_dict_list
#   设计理由：数据模型与匹配逻辑集中在 service 层，UI 只做展示与持久化
#   异常处理：构造校验抛 ValueError；播放失败打印提示（后续可换日志）
#   关联配置：无（纯业务层）；UI 依赖 ui/panels/alarm_panel.py 与 ui/alarm_dialog.py
