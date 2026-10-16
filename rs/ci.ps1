# AccelWorld RS 本地 CI 四关卡（PL011.03 落位；GitHub Actions 于 PL011.14 接线复用同一命令集）
# 用法：在任意目录执行 `pwsh rs/ci.ps1`；任一关卡失败立即退出非零
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = "Stop"
Set-Location "$PSScriptRoot"

Write-Host "== 关卡 1/4 fmt（格式检查）=="
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: fmt"; exit 1 }

Write-Host "== 关卡 2/4 clippy（警告即红）=="
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: clippy"; exit 1 }

Write-Host "== 关卡 3/4 test（全量单测 + 集成）=="
cargo test --workspace
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: test"; exit 1 }

Write-Host "== 关卡 4/4 core 依赖树零 egui/eframe（单向分层机器强制）=="
$tree = cargo tree -p accel-core --edges normal --format "{p}"
if ($tree -match "egui|eframe") {
    Write-Host "FAIL: accel-core 依赖树混入 egui/eframe"
    exit 1
}

Write-Host "ALL_GREEN: 四关卡全过"

# ===== rs/ci.ps1 说明区 =====
# 关卡 4 原理：cargo tree 展平 core 正式依赖（--edges normal 排除 dev），
#   包名逐行匹配 egui/eframe——core 是纯业务 crate，任何 GUI 依赖混入即架构违规
# GitHub Actions（PL011.14）在 windows-latest 上按同序执行四命令，不重复实现逻辑
# fmt 用 --check：本地格式化须主动 cargo fmt 提交，CI 只做守门
