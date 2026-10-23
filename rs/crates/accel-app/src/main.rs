//! AccelWorld RS 应用入口（Python main.py 的 Rust 对等物）
//! PL013.01/.12：tracing 初始化（级别/路径读 token）→ AppInterface + tokio 后台
//! → eframe::run_native 装载 AppHandle；退出走 run_native 正常返回（exit 0）

fn main() {
    // 帧计数探针（PL013.13 零忙转验证）：ACCELWORLD_FPS_PROBE=<秒> 启动后静置
    // N 秒打印 repaint 次数并退出（预期 ≈ 1000ms/tick_ms × N，空闲 1s 兜底钳制）
    if let Ok(secs) = std::env::var("ACCELWORLD_FPS_PROBE") {
        return fps_probe(secs.parse().unwrap_or(5));
    }

    // 日志体系先行（PL013.12）：每日滚动文件 + 控制台；级别读 base.json token
    accel_app::logging::init();

    // panic hook → crash 日志（std hook，PL013.12；位置/回溯随事件落 crash 文件）
    accel_app::logging::install_panic_hook();

    let result = accel_app::window::run();

    // 退出路径：run_native 正常返回即零 hack 退出（对齐 y.problems#6 教训——
    // Python 版 exit 期 GL 驱动线程内核调用不可中断；wgpu/eframe 期望常规 drop）
    if let Err(e) = result {
        tracing::error!("应用异常退出: {e}");
        std::process::exit(1);
    }
    tracing::info!("应用正常退出");
}

/// 帧节奏探针：静置运行 N 秒后输出 repaint 次数（零忙转断言的数据源）
fn fps_probe(secs: u64) {
    accel_app::logging::init();
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let result = accel_app::window::run_with_counter(counter.clone(), Some(secs));
    let n = counter.load(std::sync::atomic::Ordering::SeqCst);
    let tick = accel_core::static_config::Static::get().base_i64("clock_tick_ms");
    let line = format!(
        "REPAINT_COUNT={n} in {secs}s（预期 ≈{}±{}）",
        expected_frames(secs, tick),
        secs.max(2)
    );
    println!("{line}");
    tracing::info!("{line}");
    if let Err(e) = result {
        tracing::error!("探针异常退出: {e}");
        std::process::exit(1);
    }
}

/// 探针预期帧数（FIX007.14 可测核心）：token 除零钳制——tick=0/负值按 1ms 计
/// 不 panic（当前 token=100 不可达；防御配置表损坏级输入）
fn expected_frames(secs: u64, tick_ms: i64) -> u64 {
    1000 * secs / tick_ms.max(1) as u64
}

#[cfg(test)]
mod tests {
    use super::expected_frames;

    #[test]
    fn expected_frames_clamps_zero_tick() {
        // FIX007.14 验收：tick token=0/负值钳制按 1ms 计不 panic；常规值口径不变
        assert_eq!(expected_frames(5, 100), 50, "常规 tick 口径不变");
        assert_eq!(expected_frames(5, 0), 5_000, "tick=0 钳制按 1ms 计");
        assert_eq!(expected_frames(5, -3), 5_000, "负值同钳制");
        assert_eq!(expected_frames(0, 100), 0, "零秒零帧");
    }
}

// ===== src/main.rs 说明区 =====
// 输入：无（命令行参数随 PL015 扩展）；输出：GUI 窗口进程，退出码 0/1
// 逻辑步骤：logging::init（token 级别）→ panic hook → window::run()（构造
//   AppInterface + runtime + eframe::run_native）→ 正常返回退出
// 设计理由：Python 版 exit 期 GL 驱动硬崩溃（y.problems#6/#7）教训平移——Rust 侧
//   坚持常规 drop 退出路径（eframe/wgpu 生命周期管理），绝不 os._exit 等价物；
//   runtime 在 window::run 内 Arc 持有（AppInterface::spawn 任务附着其上，drop 即收）
// 模块对位（树单源在 lib.rs）：window ← ui/main_window.py；bus ← 信号总线；
//   theme ← qss/token；nav ← glass_nav；glass ← ui/gl；pages ← panels；
//   widgets ← 自绘控件；tools ← ui/tools；platform ← backdrop/system_tray/notify/hotkey
// 关联配置：log_level/logs_dir（base.json）；消费方：操作系统
