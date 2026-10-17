//! 系统通知（tauri-winrt-notification 集成，PL015.02）
//! 两调用点：闹钟触发 / 配置保存失败；失败仅日志不阻塞（行为无损纪律）

// AUMID（应用用户模型 ID：通知中心归组标识；NSIS 快捷方式安装后归组显示）
#[cfg(windows)]
const APP_ID: &str = "AccelWorld";

/// 闹钟触发通知（title=闹钟；body=HH:MM · 标签，空标签回退默认文案）
pub fn notify_alarm(label: &str, time: &str) -> bool {
    let body = if label.trim().is_empty() {
        "闹钟时间到".to_string()
    } else {
        label.to_string()
    };
    show("闹钟", &format!("{time} · {body}"))
}

/// 配置保存失败通知
pub fn notify_save_failed() -> bool {
    show("配置保存失败", "设置未能写入磁盘，请查阅 logs/ 日志")
}

/// WinRT toast 发送（静音——铃声由 rodio 独立负责，避免双声）
#[cfg(windows)]
fn show(title: &str, body: &str) -> bool {
    let result = tauri_winrt_notification::Toast::new(APP_ID)
        .title(title)
        .text1(body)
        .sound(None)
        .show();
    match result {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!("系统通知发送失败（降级跳过）: {e}");
            false
        }
    }
}

/// 非 Windows 编译期桩（FIX006.28）：无系统通知能力——仅记日志返 false
/// （调用方不分支，行为无损：通知属增强体验）
#[cfg(not(windows))]
fn show(title: &str, body: &str) -> bool {
    tracing::info!("非 Windows 平台无系统通知（降级跳过）: {title} / {body}");
    false
}

// ===== src/platform/notify.rs 说明区 =====
// 输入：闹钟（label/time）/保存失败两调用点文案
// 输出：bool（toast 是否已提交；仅日志用途——调用方不分支）
// 逻辑步骤：Toast::new(AUMID) → title/text1 → sound(None) 静音 → show 提交
// 设计理由：Python 版通知走 QSystemTrayIcon.showMessage（托盘气泡），Rust 换
//   WinRT toast（现代通知中心）；托盘常驻时气泡与 toast 等价可达，托盘降级路径
//   toast 仍可用（更稳）；静音避免与 rodio 铃声叠加双声
// 异常处理：show 失败（无通知能力/权限受限）记 warn 返 false——通知属增强体验，
//   绝不影响闹钟已触发/保存流程本体
// 已知偏移：通知时长由 WinRT toast 自管（notification_duration_ms token 已于 A005 #8 删除）
// FIX006.28：tauri-winrt-notification 仅 Windows 依赖——`show` 走 cfg 分档，
//   非 Windows 为编译期桩（仅日志返 false）；notify_alarm/notify_save_failed 不分档
// 关联依赖：tauri-winrt-notification（Windows 通知基础设施）；消费方：bus.rs drain
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_smoke_no_panic() {
        // 冒烟：通知发送成功/失败两态皆不 panic（受限环境 toast 可能被策略拒绝）
        let r1 = notify_alarm("响铃", "07:30");
        let r2 = notify_alarm("  ", "08:00"); // 空标签回退文案路径
        let r3 = notify_save_failed();
        tracing::info!("通知冒烟结果: alarm={r1} empty={r2} save_fail={r3}");
    }
}
