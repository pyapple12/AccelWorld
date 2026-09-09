# 文件读写工具测试（V0.4.7.0 安全加固引入）
# 覆盖：write_json 路径约束（项目根内/系统临时目录内放行，越界拒绝且不落盘）

import pytest

from utils.file_utils import get_project_root, write_json


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
