# 音频播放 UI 层模块（S10.5 D2：从 modules/alarm_service 移出）
# UI 库依赖收敛到 ui 层：QMediaPlayer 需有 QApplication 的线程创建，故保持主线程调用
# E4 一并处理：模块级持有 player 引用，播放结束/失败后释放，防 GC 中断

import logging
import threading
from typing import Any, List

from PyQt6.QtMultimedia import QMediaPlayer, QAudioOutput
from PyQt6.QtCore import QUrl

from modules.alarm_service import Alarm, PresetSound, play_preset_sound

# 配置日志
logger = logging.getLogger(__name__)

# 持有播放中的 QMediaPlayer 引用（防 GC 回收中断播放，E4）
_active_players: List[Any] = []


def _release_player(player: Any, status: Any) -> None:
    # 播放结束/媒体无效后从持有集合移除引用（允许 GC 回收）
    if status in (
        QMediaPlayer.MediaStatus.EndOfMedia,
        QMediaPlayer.MediaStatus.InvalidMedia,
    ):
        if player in _active_players:
            _active_players.remove(player)


def play_custom_sound(file_path: str) -> bool:
    # QMediaPlayer 须在主线程创建（QObject 线程绑定），由 audio_player 统一承载
    try:
        player = QMediaPlayer()
        audio_output = QAudioOutput()
        player.setAudioOutput(audio_output)
        player.setSource(QUrl.fromLocalFile(file_path))
        audio_output.setVolume(1.0)
        player.mediaStatusChanged.connect(_release_player)
        _active_players.append(player)
        player.play()

        return True
    except Exception as e:
        logger.exception(f"播放自定义音频失败: {e}")
        return False


def play_alarm_sound(alarm: Alarm) -> None:
    # 同步版本供后台线程与测试复用；自定义铃声走本层 QMediaPlayer
    if alarm.sound_type == "preset":
        preset = PresetSound.from_value(alarm.sound_value)
        play_preset_sound(preset)
    else:
        play_custom_sound(alarm.sound_value)


def play_alarm_sound_async(alarm: Alarm) -> None:
    # 预设铃声走后台线程，自定义铃声保持主线程（QMediaPlayer 线程绑定）
    if alarm.sound_type == "preset":
        threading.Thread(target=play_alarm_sound, args=(alarm,), daemon=True).start()
    else:
        play_alarm_sound(alarm)


# ===== ui/audio_player.py 函数/常量说明 =====
# _active_players: List，持有播放中 QMediaPlayer 引用（防 GC 中断，E4）
# _release_player(player, status): 播放结束/媒体无效后移除引用
# play_custom_sound(file_path) -> bool: QMediaPlayer 异步播放（须主线程）
#   异常处理：播放异常记录日志返回 False（QtMultimedia 顶层 import，缺失时模块加载失败即暴露）
# play_alarm_sound(alarm): 按 sound_type 分发（preset→winsound 阻塞；custom→本层）
# play_alarm_sound_async(alarm): preset 走 daemon 后台线程，custom 保持主线程（S5 修复 D6）
#   设计理由：S10.5 D2 将 UI 库依赖从 modules 层迁出，业务层保持纯逻辑；
#   顶层 import 符合规范（禁止函数内延迟 import，原 try 内 import 已清理）；
#   关联配置：预设铃声枚举/播放来自 modules/alarm_service.py；由 ui/main_window.py 调用
