//! 运行期文件路径基准（FIX007.15 新增）：日志与用户配置统一 exe 同目录相对
//! 原「日志 CWD 相对 / 配置 exe 相对」双基准分叉——CWD 随启动方式漂移
//! （资源管理器双击/快捷方式/计划任务各不同），exe 同目录是打包单文件产物的
//! 唯一稳定锚点；cargo run 下两者均落 rs/target/debug/（target 已 gitignore）

use std::path::PathBuf;

/// 运行目录基准：当前 exe 所在目录（取不到时回退 CWD——探针/嵌入式测试环境）
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

// ===== src/util/paths.rs 说明区 =====
// 输入：无（进程环境）；输出：exe 同目录基准 PathBuf
// 逻辑步骤：current_exe → parent → 回退 "."
// 设计理由：FIX007.15 基准统一——配置（interface default_config_path）与日志
//   （app logging logs_dir/crash 路径）同源此基准；放 core 因配置路径属 core
//   职责而 app 侧复用（依赖方向 app→core 单向不破坏）
// 异常处理：current_exe 失败（极罕见：进程已被删除等）回退 CWD，不 panic
// 关联配置：base.json user_config / logs_dir token（相对本基准解析）；
//   消费方：interface.rs / app logging.rs
#[cfg(test)]
mod tests {
    use super::exe_dir;

    #[test]
    fn exe_dir_is_absolute_or_cwd_fallback() {
        // 基准可取：绝对路径（常规）或 CWD 回退（"."）——两者皆为可用基准
        let dir = exe_dir();
        assert!(
            dir.is_absolute() || dir == std::path::Path::new("."),
            "基准应为绝对路径或 CWD 回退: {dir:?}"
        );
    }
}
