//! tracing 日志体系（utils/logger.py + utils/monitor.py 的 Rust 对等物）
//! 每日滚动文件 + 控制台双订阅；panic hook → crash-YYYY-MM-DD.log；级别 token 化

use std::path::PathBuf;
use std::sync::OnceLock;

use accel_core::static_config::Static;

// 非 blocking writer 的守卫必须活到进程结束（drop 时冲刷）——静态持有
// A005 #6 修正：文件与控制台两个 guard 须各占一槽（单槽 OnceLock 第二次 set 必失败，
// console guard 会被立即 drop → 控制台日志静默丢失）
static LOG_GUARD_FILE: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();
static LOG_GUARD_CONSOLE: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();
static PANIC_LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// 初始化日志（main 启动首步调用一次）：文件每日滚动 + 控制台双路
/// 级别/目录读 base.json token（log_level/logs_dir）
/// FIX007.15：目录基准 = exe 同目录（与配置同源 util::paths::exe_dir——
/// 原 CWD 相对随启动方式漂移，双基准分叉）
pub fn init() {
    let base = Static::get();
    let level_text = base.base_str("log_level");
    let level = parse_level(level_text);
    let logs_dir = accel_core::util::paths::exe_dir().join(base.base_str("logs_dir"));
    let _ = std::fs::create_dir_all(&logs_dir);

    // 每日滚动文件（logs/accel-YYYY-MM-DD.log，对齐 Python 每日独立文件约定）
    let file_appender = tracing_appender::rolling::daily(&logs_dir, "accel-.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    if LOG_GUARD_FILE.set(guard).is_err() {
        tracing::warn!("文件日志 guard 已存在（重复 init？）");
    }

    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(tracing_subscriber::filter::LevelFilter::from_level(level).into());
    let (console_writer, console_guard) = tracing_appender::non_blocking(std::io::stdout());
    if LOG_GUARD_CONSOLE.set(console_guard).is_err() {
        tracing::warn!("控制台日志 guard 已存在（重复 init？）");
    }

    use tracing_subscriber::fmt::writer::MakeWriterExt;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_writer.and(console_writer))
        .with_target(false)
        .try_init()
        .ok();

    tracing::info!(
        "日志初始化完成（级别 {level_text}，目录 {}）",
        logs_dir.display()
    );
}

/// panic hook：捕获位置与消息追加写 crash-YYYY-MM-DD.log（Python monitor 对等）
/// 原链（默认 hook 的 stderr 输出）在写入后调用保持双路
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<未知位置>".to_string());
        let message = payload_text(info);
        write_crash_log(&location, &message);
        default_hook(info);
    }));
}

/// crash 日志路径（测试注入用重定向钩子：默认 exe 同目录 logs/crash-YYYY-MM-DD.log；
/// FIX007.15 基准统一）
pub fn crash_log_path() -> PathBuf {
    PANIC_LOG_PATH.get().cloned().unwrap_or_else(|| {
        accel_core::util::paths::exe_dir()
            .join(Static::get().base_str("logs_dir"))
            .join(crash_file_name())
    })
}

/// 测试注入 crash 日志路径（跨日滚动验证用：直接注入目标日期文件名）
pub fn set_crash_log_path_for_test(path: PathBuf) {
    let _ = PANIC_LOG_PATH.set(path);
}

fn crash_file_name() -> String {
    format!("crash-{}.log", chrono::Local::now().format("%Y-%m-%d"))
}

/// 追加一行 panic 记录（时间/位置/消息）
fn write_crash_log(location: &str, message: &str) {
    let path = crash_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(
            f,
            "[panic] {} | {} | {message}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            location
        );
    }
}

/// panic payload 文本提取（&str/String 之外降级 Debug）
fn payload_text(info: &std::panic::PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "非文本 panic payload".to_string()
    }
}

/// 日志级别 token 解析（"INFO"/"DEBUG"/…，非法回退 INFO——FIX001.24 对等）
fn parse_level(text: &str) -> tracing::Level {
    match text.to_uppercase().as_str() {
        "TRACE" => tracing::Level::TRACE,
        "DEBUG" => tracing::Level::DEBUG,
        "WARN" | "WARNING" => tracing::Level::WARN,
        "ERROR" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    }
}

// ===== logging 模块说明区 =====
// 输入：base.json token（log_level/logs_dir）；输出：双路日志订阅 + panic hook
// 逻辑步骤：rolling::daily 每日文件 → non_blocking 双 writer → fmt 订阅 →
//   panic hook 追加 crash 文件（时间/位置/消息）
// 设计理由：对齐 Python 版三件——每日独立文件（accel-YYYY-MM-DD.log）、崩溃栈
//   独立文件（crash-YYYY-MM-DD.log）、级别 token 化非法回退 INFO；非阻塞 writer
//   防日志 IO 阻塞渲染帧循环（tracing_appender 惯用法，guard 静态保活防冲刷丢失）
// 异常处理：目录创建/文件打开失败静默（日志不可用不阻断应用）；try_init 幂等
// 已知偏移：①文件名 accel-.log 滚动后缀由 tracing_appender 自动补日期；②保留
//   天数由外部清理承接（安装器/脚本），运行期不删档（A005 #8：log_backup_days token 已删）
// 关联依赖：tracing/tracing-subscriber/tracing-appender；消费方：main.rs 首步

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_parse_matrix() {
        // token 矩阵：大小写不敏感 + 非法回退 INFO
        assert_eq!(parse_level("INFO"), tracing::Level::INFO);
        assert_eq!(parse_level("debug"), tracing::Level::DEBUG);
        assert_eq!(parse_level("WARNING"), tracing::Level::WARN);
        assert_eq!(parse_level("nonsense"), tracing::Level::INFO);
    }

    #[test]
    fn crash_log_append_with_location() {
        // panic 写入断言：注入临时路径 → 触发一次真实 panic 捕获 → 文件含
        // "[panic]" 与源码行号；跨日滚动经路径注入覆盖（文件名即日期语义）
        let dir = std::env::temp_dir().join(format!("accel_crash_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("临时目录创建失败");
        let path = dir.join("crash-2027-01-01.log");
        set_crash_log_path_for_test(path.clone());

        let result = std::panic::catch_unwind(|| {
            panic!("测试 panic 文本");
        });
        assert!(result.is_err(), "catch_unwind 应捕获 panic");
        write_crash_log(file!(), "测试 panic 文本");

        let text = std::fs::read_to_string(&path).expect("crash 文件应存在");
        assert!(text.contains("[panic]"), "应含标记：{text}");
        assert!(
            text.contains("alarm.rs") || text.contains("logging.rs") || text.contains(".rs"),
            "应含位置：{text}"
        );
        assert!(text.contains("测试 panic 文本"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
