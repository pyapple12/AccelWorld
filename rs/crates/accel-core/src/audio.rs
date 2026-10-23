//! 音频播放（Python modules/alarm_service.py play_preset_sound +
//! ui/audio_player.py play_custom_sound 的 Rust 合并对等物，PL012.08）
//! rodio 0.23：静态单流（MixerDeviceSink 保活）+ Player 顺序队列拼段播放

use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;
use std::time::Duration;

use rodio::mixer::Mixer;
use rodio::source::{SineWave, Zero};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source};

use crate::alarm::{PRESET_BEEP_DURATION_MS, PRESET_SOUND_CONFIG, PresetSound};

// 全局唯一输出流（MixerDeviceSink 被 drop 即断声——静态持有贯穿进程生命周期防泄漏）
static STREAM: Mutex<Option<MixerDeviceSink>> = Mutex::new(None);

/// 取全局 Mixer 句柄（懒初始化；失败记日志返 None——无音频设备环境不 panic）
fn mixer_handle() -> Option<Mixer> {
    let mut guard = STREAM.lock().expect("音频流锁中毒");
    if guard.is_none() {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => *guard = Some(sink),
            Err(e) => {
                tracing::error!("音频输出流初始化失败: {e}");
                return None;
            }
        }
    }
    guard.as_ref().map(|sink| sink.mixer().clone())
}

/// 播放预设铃声（Python play_preset_sound 对等：频率/次数/间隔三元组 + 200ms 蜂鸣）
/// Player 队列逐段追加（蜂鸣段-静默段交替顺序播放），detach 后台异步
/// 返回 true = 已提交播放；失败（无设备）记日志返 false 不 panic
pub fn play_preset(preset: PresetSound) -> bool {
    let Some(mixer) = mixer_handle() else {
        return false;
    };
    let Some(&(_, frequency, repeat_count, interval_ms)) =
        PRESET_SOUND_CONFIG.iter().find(|(p, ..)| *p == preset)
    else {
        tracing::error!("播放预设铃声失败: 未知预设 {preset:?}");
        return false;
    };
    let player = Player::connect_new(&mixer);
    let beep = Duration::from_millis(PRESET_BEEP_DURATION_MS);
    let gap = Duration::from_millis(interval_ms as u64);
    for i in 0..repeat_count {
        player.append(
            SineWave::new(frequency as f32)
                .amplify(0.2)
                .take_duration(beep),
        );
        if i < repeat_count - 1 {
            player.append(
                Zero::new(
                    std::num::NonZeroU16::new(1).expect("声道数非零"),
                    std::num::NonZeroU32::new(44_100).expect("采样率非零"),
                )
                .take_duration(gap),
            );
        }
    }
    player.detach();
    true
}

/// 播放音频文件（Python play_custom_sound 对等：rodio Decoder，mp3/wav 等）
/// 失败记日志返 false 不 panic
pub fn play_file(path: &std::path::Path) -> bool {
    let Some(mixer) = mixer_handle() else {
        return false;
    };
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("播放自定义音频失败（打开文件）: {path:?} {e}");
            return false;
        }
    };
    match rodio::Decoder::new(BufReader::new(file)) {
        Ok(decoder) => {
            let player = Player::connect_new(&mixer);
            player.append(decoder);
            player.detach();
            true
        }
        Err(e) => {
            tracing::error!("播放自定义音频失败（解码）: {path:?} {e}");
            false
        }
    }
}

// ===== src/audio.rs 说明区 =====
// 输入：PresetSound 枚举 / 音频文件路径；输出：bool（是否已提交播放）
// 逻辑步骤：懒初始化全局输出流（DeviceSinkBuilder::open_default_sink）→ 取 Mixer
//   克隆 → Player::connect_new 挂队列 → 预设（蜂鸣 0.2 增益 + 静默段交替 append）/
//   文件（Decoder append）→ detach 后台播放
// 设计理由：Python 侧预设铃经 winsound（业务层阻塞式）+ 自定义铃经 QMediaPlayer
//   （UI 层）——Rust 合并至 core::audio（rodio 统一；Player detach 即后台播放，
//   对齐 Python 后台线程语义）；静态单流贯穿进程（设备会话逐次创建会闪断）；
//   Player.append 顺序语义对齐 winsound.Beep+sleep 串行听觉形态；正弦 0.2 增益
//   对齐 basic 示例安全音量（满幅正弦爆音）
// 异常处理：无音频设备/文件缺失/解码失败全路径记日志返 false（Python
//   logger.exception 对等），绝不 panic（调度任务内调用）
// 已知偏移：①蜂鸣静默段用 44.1kHz Zero 源填充（winsound 为硬静默）——听觉等价；
//   ②rodio 0.23 API（Player/Mixer）与 plan 预想 OutputStream/Sink 已变——按实测
//   版本适配，登记 z.plan
// 关联依赖：rodio 0.23；消费方：app bus（AlarmTriggered 副作用 PlaySound → 平台播放）
#[cfg(test)]
mod tests {
    use super::*;
    use crate::alarm::PresetSound;

    #[test]
    fn preset_play_smoke_no_panic() {
        // 冒烟：有设备则提交播放（true），无设备则 false——两态都不 panic 即过
        let result = play_preset(PresetSound::Beep);
        tracing::info!("预设铃冒烟结果: {result}（无音频设备环境允许 false）");
    }

    #[test]
    fn file_play_missing_file_no_panic() {
        // 不存在文件：false 且不 panic
        assert!(!play_file(std::path::Path::new("Z:/不存在/无此文件.mp3")));
    }

    #[test]
    fn mixer_handle_reuse() {
        // 静态单流复用：连续两次取 Mixer 成败一致（同一次初始化结果，不重建流）
        let h1 = mixer_handle();
        let h2 = mixer_handle();
        assert_eq!(h1.is_some(), h2.is_some(), "两次取流成败应一致");
    }
}
