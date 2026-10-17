//! 构建期脚本（PL015.03）：版本单源注入
//! 读 rs/config/static/base.json 的 version（单一来源）→ ACCELWORLD_VERSION 编译期 env

use std::path::{Path, PathBuf};

fn main() {
    // 版本单源：base.json → cargo:rustc-env（crate 根经 ancestors 上溯至 rs 工作区根）
    // manifest 指向 rs/crates/accel-app，ancestor 第 2 级即 rs 工作区根
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let rs_root = manifest
        .ancestors()
        .nth(2)
        .expect("crate 根上溯两级（rs 工作区根）")
        .to_path_buf();
    let raw = read_workspace_file(&rs_root, Path::new("config/static/base.json"));
    let base: serde_json::Value = serde_json::from_str(&raw).expect("base.json 合法 JSON");
    let version = base
        .get("version")
        .and_then(serde_json::Value::as_str)
        .expect("base.json version 字段存在");
    println!("cargo:rustc-env=ACCELWORLD_VERSION={version}");
}

/// 工作区内文件安全读取：规范化后校验仍位于工作区根内（防穿越纪律），越界即 panic
/// 输入相对路径禁止点分量（谓词前置），路径规范化失败/越界属构建环境损坏级错误
fn read_workspace_file(rs_root: &Path, rel: &Path) -> String {
    assert!(
        rel.components()
            .all(|c| matches!(c, std::path::Component::Normal(_))),
        "工作区相对路径禁止点分量: {rel:?}"
    );
    let resolved = rs_root
        .join(rel)
        .canonicalize()
        .expect("文件规范化（应存在）");
    let root_canonical = rs_root.canonicalize().expect("工作区根规范化");
    assert!(
        resolved.starts_with(&root_canonical),
        "解析路径越出工作区根: {resolved:?}"
    );
    println!("cargo:rerun-if-changed={}", resolved.display());
    std::fs::read_to_string(&resolved).expect("工作区文件可读")
}

// ===== crates/accel-app/build.rs 说明区 =====
// 输入：rs/config/static/base.json（version 字段）
// 输出：ACCELWORLD_VERSION 编译期 env
// 逻辑步骤：读 base.json → 注入 env（cargo:rustc-env）
// 设计理由：版本单一来源纪律的编译期承接——env! 与运行期 Static::get().version()
//   同出 base.json 一文件（lib.rs compiled_version 单测锚定两路 parity）
// 异常处理：base.json 缺失/损坏/version 缺字段 panic 快速失败（构建期暴露）；
//   工作区文件读取前置谓词（相对路径禁点分量）+ 规范化后容器校验，越界即中止
// 关联配置：base.json version；消费方：lib.rs compiled_version（编译期 parity 锚定）
