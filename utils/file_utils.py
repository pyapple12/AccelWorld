# 文件读写工具模块
# pathlib 封装的 JSON 读写 + 缓存单例（参考 DeepTransHub utils/load_config.py 模式）
# S1 阶段创建工具，S2 由 config/settings.py 接入使用

import json
from pathlib import Path
from typing import Any

# 缓存单例：路径 → 解析后的 JSON 数据
_json_cache: dict[str, Any] = {}


def read_json(path: Path | str, default: Any = None) -> Any:
    # 读取 JSON 文件，失败或文件不存在时返回 default
    try:
        with open(Path(path), "r", encoding="utf-8") as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError):
        return default


def read_json_cached(path: Path | str, default: Any = None) -> Any:
    # 带缓存的 JSON 读取：命中缓存直接返回，未命中读取后写入缓存
    key = str(Path(path))
    if key in _json_cache:
        return _json_cache[key]
    data = read_json(path, default)
    if data is not None:
        _json_cache[key] = data
    return data


def write_json(path: Path | str, data: Any) -> bool:
    # 写入 JSON 文件（UTF-8、ensure_ascii=False、缩进 4），成功后刷新缓存
    try:
        file_path = Path(path)
        file_path.parent.mkdir(parents=True, exist_ok=True)
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=4)
        clear_json_cache(str(file_path))
        return True
    except OSError:
        return False


def read_file(path: Path | str, default: str = "") -> str:
    # 读取文本文件，失败或文件不存在时返回 default
    try:
        return Path(path).read_text(encoding="utf-8")
    except OSError:
        return default


def clear_json_cache(path: Path | str | None = None) -> None:
    # 清空缓存：指定路径则只清该路径，None 清空全部
    if path is None:
        _json_cache.clear()
    else:
        _json_cache.pop(str(Path(path)), None)


# ===== utils/file_utils.py 函数/常量说明 =====
# read_json(path, default): 读取 JSON 文件
#   输入：文件路径、默认值；输出：解析后的数据或 default
#   设计理由：统一异常处理与编码，避免各模块重复 try/except
#   异常处理：捕获 OSError 与 json.JSONDecodeError，损坏文件返回默认值
# read_json_cached(path, default): 带缓存的 JSON 读取
#   输入：文件路径、默认值；输出：解析后的数据（命中缓存时直接返回）
#   设计理由：缓存单例避免高频重复 IO，配置文件被多次读取时显著降低开销（修复 D4）
#   异常处理：同 read_json，缓存仅在成功解析后写入
# write_json(path, data): 写入 JSON 文件
#   输入：文件路径、数据；输出：bool 是否成功
#   设计理由：自动创建父目录，UTF-8 中文友好输出，写入后同步清理缓存保证一致性
#   异常处理：捕获 OSError 返回 False
# read_file(path, default): 读取文本文件
#   输入：文件路径、默认值；输出：文本内容或 default
#   设计理由：统一 UTF-8 读取与 OSError 兜底
# clear_json_cache(path): 清空缓存
#   输入：可选文件路径；输出：None
#   设计理由：保存配置后调用，保证后续读取始终是最新数据
#   关联配置：S2 由 config/settings.py 接入
