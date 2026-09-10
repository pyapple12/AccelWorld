# 文件读写工具测试（V0.4.7.0 安全加固引入）
# 覆盖：write_json 路径约束（项目根内/系统临时目录内放行，越界拒绝且不落盘）；
#       FIX001 补充：非 UTF-8 容错、原子替换无残留、缓存键 resolve 一致性

import pytest

from utils.file_utils import (
    clear_json_cache,
    get_project_root,
    read_json,
    read_json_cached,
    write_json,
)


def test_write_inside_project_root():
    # 项目根内写入放行（.temp/ 为项目约定的临时目录，已 gitignore）
    target = get_project_root() / ".temp" / "test_file_utils_write.json"
    try:
        assert write_json(target, {"ok": True})
        assert target.exists()
    finally:
        target.unlink(missing_ok=True)


def test_write_inside_temp_dir(tmp_path):
    # 系统临时目录内写入放行（兼容 pytest tmp_path 测试隔离）
    assert write_json(tmp_path / "user_config.json", {"ok": True})


def test_write_outside_roots_rejected():
    # 项目根与系统临时目录之外的路径拒绝写入并抛 ValueError（不落盘）
    outside = get_project_root().parent / "file_utils_越界测试.json"
    with pytest.raises(ValueError):
        write_json(outside, {"evil": True})
    assert not outside.exists()


def test_read_json_unicode_error_returns_default(tmp_path):
    # 非 UTF-8 字节（如 GBK 编码内容）读取返回 default 而非抛 UnicodeDecodeError（FIX001.1）
    bad = tmp_path / "bad.json"
    bad.write_bytes(b'{"k": "\xc4\xe3\xba\xc3"}')  # GBK 编码的 "你好"
    assert read_json(bad, default=[]) == []


def test_write_json_replaces_without_tmp_residue(tmp_path):
    # 原子替换：覆盖写后内容为新值且无任何 .tmp 中间文件残留（FIX001.2/FIX002.7 唯一化后缀）
    target = tmp_path / "cfg.json"
    assert write_json(target, {"a": 1}) is True
    assert write_json(target, {"a": 2}) is True
    assert read_json(target) == {"a": 2}
    assert not list(tmp_path.glob("*.tmp")), "残留 .tmp 中间文件"


def test_cache_key_resolved_relative_and_absolute(tmp_path, monkeypatch):
    # 相对路径与绝对路径读写缓存一致（FIX001.17：缓存键统一 resolve）
    monkeypatch.chdir(tmp_path)
    clear_json_cache()
    try:
        assert write_json("rel.json", {"v": 1}) is True
        assert read_json("rel.json") == {"v": 1}  # 绕过缓存的直读
        assert read_json_cached("rel.json") == {"v": 1}  # 相对路径键命中
        absolute = str((tmp_path / "rel.json").resolve())
        assert read_json_cached(absolute) == {"v": 1}  # 绝对路径键命中同一缓存
    finally:
        clear_json_cache()
