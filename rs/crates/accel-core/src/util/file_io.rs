//! 文件 IO 工具（utils/file_utils.py 的 Rust 对等物）
//! 原子写（tmp+rename）+ 写白名单 + 损坏转存辅助；read_json 容错读

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// 写白名单根目录：exe 同目录（安装态随应用走）+ 系统临时目录（测试隔离）+
/// 环境变量 ACCELWORLD_WRITE_ROOT 注入根（子进程测试隔离，对齐 Python env 手段）
/// A008 #11：exe 基准复用 util::paths::exe_dir 单源（原自行 current_exe 推导，
/// 失败语义与之分叉：exe_dir 回退 "." 而 dedup 前实现直接丢根）
pub fn allowed_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(root) = std::env::var("ACCELWORLD_WRITE_ROOT") {
        roots.push(PathBuf::from(root));
    }
    roots.push(super::paths::exe_dir());
    roots.push(std::env::temp_dir());
    roots
}

/// 路径是否位于写白名单内（Python is_write_allowed 对等：resolve 后逐根判定）
pub fn is_write_allowed(path: &Path) -> bool {
    let resolved = canonicalish(path);
    allowed_roots()
        .iter()
        .any(|root| resolved.starts_with(canonicalish(root)))
}

/// 规范化路径（存在则 canonicalize——Windows 返回 \\?\ 前缀须剥离与词法形态统一，
/// 不存在则保留词法形态；白名单判定两形态一致可用）
fn canonicalish(path: &Path) -> PathBuf {
    path.canonicalize()
        .map(strip_verbatim)
        .unwrap_or_else(|_| normalize_lexical(path))
}

/// 剥离 Windows verbatim 前缀（\\?\C:\… → C:\…；\\?\UNC\server\… → \\server\…）
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.as_os_str().to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        p
    }
}

/// 词法规范化：清理 ./ 与 ../（无 IO；大小写比较交由 canonicalish 两侧统一形态——
/// A005 #31 更正：`starts_with` 本身是大小写敏感的，不存在"Windows 语义容忍"）
fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out: Vec<std::ffi::OsString> = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.is_empty() {
                    out.pop();
                }
            }
            other => out.push(other.as_os_str().to_os_string()),
        }
    }
    out.iter().collect()
}

/// 容错读 JSON（Python read_json 对等）：不存在/IO 错/解析错/非 UTF-8 → None
pub fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// 写白名单拒绝错误（Python ValueError 对等：编程错误上抛）
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("拒绝写入允许目录之外的路径: {0}")]
    NotAllowed(PathBuf),
    #[error("写入失败: {0}")]
    Io(#[from] io::Error),
}

/// 原子写 JSON（Python write_json 对等）：白名单校验 → 同目录 .{pid}.tmp → rename 覆盖
/// 序列化形态：UTF-8、ensure_ascii=False；缩进实为 2（serde_json pretty 固定值，
/// 与 Python indent=4 的字节差异见文末「已知偏移」——值级语义一致）
pub fn write_json(path: &Path, data: &Value) -> Result<(), WriteError> {
    let resolved = canonicalish(path);
    if !is_write_allowed(&resolved) {
        return Err(WriteError::NotAllowed(resolved));
    }
    let text = serde_json::to_string_pretty(data).map_err(io::Error::other)?;
    write_text_atomic(&resolved, &text)
}

/// 原子写文本：同目录进程号中间文件 + rename 替换（防中途崩溃半截文件）
pub fn write_text_atomic(path: &Path, text: &str) -> Result<(), WriteError> {
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&tmp, text.as_bytes())?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // 中间文件清理失败不掩盖原错误（Windows AV 瞬时占用场景）
            let _ = fs::remove_file(&tmp);
            Err(WriteError::Io(e))
        }
    }
}

/// 损坏配置转存辅助（Python _backup_corrupted_config 对等）
/// 白名单外跳过转存返 None；成功返 .bak 路径；IO 失败仅日志不外抛
pub fn backup_corrupted(path: &Path) -> Option<PathBuf> {
    if !is_write_allowed(path) {
        tracing::warn!("配置路径在写白名单之外，跳过损坏转存: {}", path.display());
        return None;
    }
    let bak = path.with_file_name(format!(
        "{}.bak",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    match fs::copy(path, &bak) {
        Ok(_) => {
            tracing::error!("配置文件损坏，已转存: {}", bak.display());
            Some(bak)
        }
        Err(e) => {
            tracing::error!("损坏配置转存失败: {e}");
            None
        }
    }
}

// ===== src/util/file_io.rs 说明区 =====
// 输入：目标路径 + JSON 值/文本；输出：read Option / write Result / 转存 Option
// 逻辑步骤：白名单 roots（env 注入根 + exe 目录 + temp）→ resolve 判定 → 原子写
// 设计理由：对齐 Python file_utils 三契约——越界 ValueError 上抛（编程错误不吞）、
//   IO 失败清理中间文件返错、转存白名单外跳过；中间文件带进程号防多进程互踩；
//   Python _PROJECT_ROOT（__file__ 相对）在编译型语言无对等物，改为 exe 目录 +
//   env 注入根（安装态/测试态两场景各取所需）
// 异常处理：read 全容错（None）；write 区分 NotAllowed（上抛）与 Io（清理后上抛）；
//   backup 仅日志不外抛（对齐 Python logger.error 语义）
// 已知偏移：serde_json::to_string_pretty 缩进为 2，Python json.dumps indent=4——
//   落盘字节与 Python 版不一致（解析语义一致）；如需字节级一致在 settings 层后处理
// 关联依赖：serde_json / tracing；消费方：settings（load/save/转存）
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("accel_fileio_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("临时目录创建失败");
        dir
    }

    #[test]
    fn roundtrip_json() {
        // 原子写 → 容错读往返：值等价（中间 .tmp 不残留）
        let path = temp_root().join("round.json");
        let data = json!({"rate": 2.0, "theme": "auto", "城市": "北京"});
        write_json(&path, &data).expect("白名单内写入应成功");
        assert_eq!(read_json(&path), Some(data));
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        assert!(!tmp.exists(), "中间文件应已替换清除");
    }

    #[test]
    fn read_missing_or_corrupt_returns_none() {
        // 不存在 / 非 JSON / 非 UTF-8 三态均 None（Python read_json 对等）
        let root = temp_root();
        assert_eq!(read_json(&root.join("nope.json")), None);
        let corrupt = root.join("corrupt.json");
        fs::write(&corrupt, b"{ not json !!!").expect("写入失败");
        assert_eq!(read_json(&corrupt), None);
        let non_utf8 = root.join("binary.json");
        fs::write(&non_utf8, [0xFF, 0xFE, 0x00]).expect("写入失败");
        assert_eq!(read_json(&non_utf8), None);
    }

    #[test]
    fn write_outside_whitelist_rejected() {
        // 白名单外路径：NotAllowed 上抛且不落盘（Python ValueError 对等）
        let evil = Path::new(r"C:\Windows\accel_evil.json");
        match write_json(evil, &json!({"x": 1})) {
            Err(WriteError::NotAllowed(p)) => assert_eq!(p, evil),
            other => panic!("应拒绝白名单外写入，实际: {other:?}"),
        }
        assert!(!evil.exists());
    }

    #[test]
    fn backup_corrupted_copies_and_throttles_none() {
        // 损坏转存：.bak 生成且内容一致；不存在源文件时返 None
        let root = temp_root();
        let cfg = root.join("cfg.json");
        fs::write(&cfg, "原始损坏内容").expect("写入失败");
        let bak = backup_corrupted(&cfg).expect("白名单内应转存");
        assert_eq!(
            fs::read_to_string(&bak).expect("转存文件应可读"),
            "原始损坏内容"
        );
        assert!(backup_corrupted(&root.join("ghost.json")).is_none());
    }
}
