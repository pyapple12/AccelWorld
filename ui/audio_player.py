# 音频播放 UI 层模块（S10.5 D2：从 modules/alarm_service 移出）
# UI 库依赖收敛到 ui 层：QMediaPlayer 需有 QApplication 的线程创建，故保持主线程调用
# E4 一并处理：模块级持有 player 引用，播放结束/失败后释放，防 GC 中断
# PL001（plan#UI2.0）：Alarm/PresetSound 类型经 interface.types 转出，预设铃声播放
# 经 AppInterface（后端 winsound 执行），本文件零后端 import

import logging
import threading
from typing import Any, List

from PyQt6.QtMultimedia import QMediaPlayer, QAudioOutput
from PyQt6.QtCore import QUrl

from interface import AppInterface
from interface.types import Alarm, PresetSound

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


def play_alarm_sound(alarm: Alarm, interface: AppInterface) -> None:
    # 同步版本供后台线程与测试复用；自定义铃声走本层 QMediaPlayer，预设经接口播放
    if alarm.sound_type == "preset":
        preset = PresetSound.from_value(alarm.sound_value)
        interface.play_preset_sound(preset)
    else:
        play_custom_sound(alarm.sound_value)


def play_alarm_sound_async(alarm: Alarm, interface: AppInterface) -> None:
    # 预设铃声走后台线程，自定义铃声保持主线程（QMediaPlayer 线程绑定）
    if alarm.sound_type == "preset":
        threading.Thread(
            target=play_alarm_sound, args=(alarm, interface), daemon=True
        ).start()
    else:
        play_alarm_sound(alarm, interface)


# ===== ui/audio_player.py 函数/常量说明 =====
# _active_players: List，持有播放中 QMediaPlayer 引用（防 GC 中断，E4）
# _release_player(player, status): 播放结束/媒体无效后移除引用
# play_custom_sound(file_path) -> bool: QMediaPlayer 异步播放（须主线程）
#   异常处理：播放异常记录日志返回 False（QtMultimedia 顶层 import，缺失时模块加载失败即暴露）
# play_alarm_sound(alarm, interface): 按 sound_type 分发（preset→经接口 winsound 阻塞；
#   custom→本层 QMediaPlayer）
# play_alarm_sound_async(alarm, interface): preset 走 daemon 后台线程，custom 保持主线程
#   （S5 修复 D6）
#   设计理由：S10.5 D2 将 UI 库依赖从 modules 层迁出；PL001 起类型经 interface.types、
#   后端铃声执行经 AppInterface（本文件零后端 import，plan#UI2.0 铁律 2）；
#   播放编排（同步/异步/线程）保留 UI 层（铁律 3）；
#   关联配置：由 ui/main_window.py 调用（闹钟触发编排）
