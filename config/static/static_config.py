# 应用静态配置加载模块（S9.5 定案：config/static/ 命名，只读）

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict

from utils.file_utils import read_json

# 引导路径：static_config.py 所在目录（唯一结构约定，__file__ 自定位）
STATIC_DIR = Path(__file__).resolve().parent


@dataclass
class StaticConfig:
    """应用静态配置聚合（base/ui 各为 dict，只读，供全项目引用）"""

    base: Dict[str, Any]
    ui: Dict[str, Any]


def _load_static_config() -> StaticConfig:
    # 私有加载：读引导映射表 → 遍历读取各分类 json → 聚合返回；文件缺失/损坏抛错暴露
    mapping = read_json(STATIC_DIR / "config.json", default={})
    result: Dict[str, Dict[str, Any]] = {}
    for key, rel_path in mapping.items():
        data = read_json(STATIC_DIR / rel_path, default=None)
        if data is None:
            raise RuntimeError(f"静态配置文件缺失或损坏: {rel_path}")
        result[key] = data
    return StaticConfig(base=result["base"], ui=result["ui"])


_static_config_cache: StaticConfig | None = None


def get_static_config() -> StaticConfig:
    # 公开单例访问：缓存懒加载，首次调用后不再读文件
    global _static_config_cache
    if _static_config_cache is None:
        _static_config_cache = _load_static_config()
    return _static_config_cache


# ===== config/static/static_config.py 函数/常量说明 =====
# STATIC_DIR: 引导路径（唯一硬编码），static_config.py 所在目录，__file__ 自定位
# StaticConfig(dataclass): 静态配置聚合（base/ui 两个 dict 字段，只读）
# _load_static_config() -> StaticConfig: 私有加载
#   逻辑：读 config.json 映射表 → 遍历读取各分类 json → 聚合
#   异常：映射/文件缺失或损坏抛 RuntimeError（开发期快速暴露，不静默兜底）
# get_static_config() -> StaticConfig: 公开单例访问（缓存懒加载）
#   设计理由：static 前缀命名与用户配置（settings.py load_config/save_config）明确区分；
#   缓存单例避免重复 IO
#   关联配置：config/static/config.json 映射表、base.json 应用参数、ui.json UI 参数
