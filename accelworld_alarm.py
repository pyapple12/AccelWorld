"""
闹钟管理模块

提供闹钟数据模型、闹钟匹配逻辑和音频播放功能
"""

import uuid
from dataclasses import dataclass, field, asdict
from datetime import datetime, time
from typing import List, Optional, Literal, Dict, Any
from enum import Enum


class PresetSound(Enum):
    """预设铃声枚举"""
    CLASSIC = "classic"
    GENTLE = "gentle"
    BEEP = "beep"
    CHIME = "chime"

    @classmethod
    def display_names(cls) -> List[str]:
        """获取显示名称列表"""
        return ["Classic", "Gentle", "Beep", "Chime"]

    @classmethod
    def from_value(cls, value: str) -> "PresetSound":
        """根据值获取枚举成员"""
        for member in cls:
            if member.value == value.lower():
                return member
        return cls.CLASSIC


# 支持的音频文件格式
SUPPORTED_AUDIO_FORMATS = (
    "Audio Files (*.wav *.mp3 *.ogg *.flac *.m4a *.wma *.aac);;"
    "All Files (*)"
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
        # 确保 time 格式正确
        if not self._validate_time(self.time):
            raise ValueError(f"Invalid time format: {self.time}, expected HH:MM")

    @staticmethod
    def _validate_time(t: str) -> bool:
        """验证时间格式"""
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
        if not self.enabled:
            return False

        # 检查时间是否匹配
        alarm_time = time.fromisoformat(self.time if ":" in self.time else self.time + ":00")
        current_time = check_time.time()

        if alarm_time.hour != current_time.hour or alarm_time.minute != current_time.minute:
            return False

        # 如果没有设置重复天数，则只在创建当天触发（简化处理：检查分钟对齐）
        if not self.repeat_days:
            return True

        # 检查当前星期是否在重复设置中
        return check_time.weekday() in self.repeat_days

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典（用于 JSON 序列化）"""
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Alarm":
        """从字典创建（用于 JSON 反序列化）"""
        return cls(**data)

    def is_one_time(self) -> bool:
        """是否为一次性闹钟（不重复）"""
        return len(self.repeat_days) == 0


# ------------------- 音频播放 -------------------

def play_preset_sound(preset: PresetSound) -> None:
    """
    播放预设铃声

    使用系统蜂鸣声作为预设铃声的基础实现
    """
    try:
        # 使用 winsound 在 Windows 上播放蜂鸣声
        import winsound

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
                import time
                time.sleep(interval / 1000.0)
    except Exception as e:
        print(f"播放预设铃声失败: {e}")


def play_custom_sound(file_path: str) -> bool:
    """
    播放自定义音频文件

    :param file_path: 音频文件路径
    :return: 是否播放成功
    """
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
        print("PyQt6.Multimedia 不可用，无法播放自定义音频")
        return False
    except Exception as e:
        print(f"播放自定义音频失败: {e}")
        return False


def play_alarm_sound(alarm: Alarm) -> None:
    """
    播放闹钟声音

    :param alarm: 闹钟对象
    """
    if alarm.sound_type == "preset":
        preset = PresetSound.from_value(alarm.sound_value)
        play_preset_sound(preset)
    else:
        play_custom_sound(alarm.sound_value)


# ------------------- 闹钟管理器 -------------------

class AlarmManager:
    """闹钟管理器"""

    def __init__(self) -> None:
        self.alarms: List[Alarm] = []
        self.max_alarms = 10
        self._last_triggered: Dict[str, str] = {}  # alarm_id -> "HH:MM"

    def add_alarm(self, alarm: Alarm) -> bool:
        """
        添加闹钟

        :param alarm: 闹钟对象
        :return: 是否添加成功
        """
        if len(self.alarms) >= self.max_alarms:
            print(f"已达到最大闹钟数量限制 ({self.max_alarms})")
            return False

        # 检查是否已存在相同时间的闹钟
        for existing in self.alarms:
            if existing.time == alarm.time and existing.label == alarm.label:
                print("已存在相同时间和标签的闹钟")
                return False

        self.alarms.append(alarm)
        return True

    def remove_alarm(self, alarm_id: str) -> bool:
        """
        移除闹钟

        :param alarm_id: 闹钟 ID
        :return: 是否移除成功
        """
        for alarm in self.alarms:
            if alarm.id == alarm_id:
                self.alarms.remove(alarm)
                return True
        return False

    def get_alarm(self, alarm_id: str) -> Optional[Alarm]:
        """获取闹钟"""
        for alarm in self.alarms:
            if alarm.id == alarm_id:
                return alarm
        return None

    def update_alarm(self, alarm_id: str, **kwargs) -> bool:
        """
        更新闹钟

        :param alarm_id: 闹钟 ID
        :param kwargs: 要更新的字段
        :return: 是否更新成功
        """
        alarm = self.get_alarm(alarm_id)
        if not alarm:
            return False

        for key, value in kwargs.items():
            if hasattr(alarm, key):
                setattr(alarm, key, value)

        return True

    def toggle_alarm(self, alarm_id: str) -> bool:
        """
        切换闹钟启用状态

        :param alarm_id: 闹钟 ID
        :return: 是否切换成功
        """
        alarm = self.get_alarm(alarm_id)
        if alarm:
            alarm.enabled = not alarm.enabled
            return True
        return False

    def get_enabled_alarms(self) -> List[Alarm]:
        """获取所有启用的闹钟"""
        return [a for a in self.alarms if a.enabled]

    def check_alarms(self, check_time: datetime) -> List[Alarm]:
        """
        检查指定时间应该触发的闹钟

        :param check_time: 要检查的时间
        :return: 应该触发的闹钟列表
        """
        time_str = f"{check_time.hour:02d}:{check_time.minute:02d}"
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

    def mark_triggered(self, alarm_id: str, check_time: datetime) -> None:
        """标记闹钟已触发"""
        time_str = f"{check_time.hour:02d}:{check_time.minute:02d}"
        self._last_triggered[alarm_id] = time_str

    def to_dict_list(self) -> List[Dict[str, Any]]:
        """转换为字典列表"""
        return [alarm.to_dict() for alarm in self.alarms]

    def from_dict_list(self, data: List[Dict[str, Any]]) -> None:
        """从字典列表加载"""
        self.alarms = [Alarm.from_dict(item) for item in data if item]


# ------------------- 测试 -------------------
if __name__ == "__main__":
    # 测试闹钟创建
    alarm = Alarm(
        label="起床闹钟",
        time="07:00",
        sound_type="preset",
        sound_value="classic",
        repeat_days=[0, 1, 2, 3, 4]  # 工作日
    )
    print(f"创建闹钟: {alarm}")
    print(f"一次性闹钟: {alarm.is_one_time()}")

    # 测试时间匹配
    test_time = datetime(2026, 1, 13, 7, 0, 0)  # 周二 7:00
    print(f"应该触发: {alarm.should_trigger_on(test_time)}")

    # 测试管理器
    manager = AlarmManager()
    manager.add_alarm(alarm)
    triggered = manager.check_alarms(test_time)
    print(f"触发的闹钟: {triggered}")
