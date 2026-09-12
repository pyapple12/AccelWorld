# 文件读写工具模块
# pathlib 封装的 JSON 读写 + 缓存单例（参考 DeepTransHub utils/load_config.py 模式）
# S1 阶段创建工具，S2 由 config/settings.py 接入使用

import json
import os
import tempfile
from pathlib import Path
from typing import Any

# 缓存单例：路径 → 解析后的 JSON 数据
_json_cache: dict[str, Any] = {}

# 项目根：utils/file_utils.py → utils/ → 项目根
_PROJECT_ROOT = Path(__file__).resolve().parent.parent

# 写入允许根目录：项目根 + 系统临时目录（V0.4.7.0 安全加固，防路径穿越；
# 临时目录豁免为兼容 pytest tmp_path 测试隔离）
_WRITE_ALLOWED_ROOTS: tuple[Path, ...] = (
    _PROJECT_ROOT,
    Path(tempfile.gettempdir()).resolve(),
)


def get_project_root() -> Path:
    # 获取项目根目录并校验 main.py 存在（防止目录层级偏移）
    if not (_PROJECT_ROOT / "main.py").exists():
        raise RuntimeError(f"项目根目录检测失败：{_PROJECT_ROOT} 下缺少 main.py")
    return _PROJECT_ROOT


def is_write_allowed(path: Path) -> bool:
    # 路径是否位于写白名单内（项目根/系统临时目录）；供写路径与转存旁路防护复用（FIX003.10）
    resolved = path.resolve()
    return any(resolved.is_relative_to(root) for root in _WRITE_ALLOWED_ROOTS)


def read_json(path: Path | str, default: Any = None) -> Any:
    # 读取 JSON 文件，失败或文件不存在时返回 default
    # （FIX001.1：补捕 UnicodeDecodeError——非 UTF-8 字节文件此前会穿透崩溃）
    try:
        with open(Path(path), "r", encoding="utf-8") as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError, UnicodeDecodeError):
        return default


def read_json_cached(path: Path | str, default: Any = None) -> Any:
    # 带缓存的 JSON 读取：命中缓存直接返回，未命中读取后写入缓存
    # （FIX001.17：缓存键统一 resolve，与 write_json/clear_json_cache 的键一致）
    key = str(Path(path).resolve())
    if key in _json_cache:
        return _json_cache[key]
    data = read_json(path, default)
    if data is not None:
        _json_cache[key] = data
    return data


def write_json(path: Path | str, data: Any) -> bool:
    # 写入 JSON 文件（UTF-8、ensure_ascii=False、缩进 4），成功后刷新缓存
    # 安全约束：路径规范化后必须位于项目根/系统临时目录内，越界视为编程错误抛 ValueError
    resolved = Path(path).resolve()
    if not is_write_allowed(resolved):
        raise ValueError(f"拒绝写入允许目录之外的路径: {resolved}")
    # tmp 名带进程号：多进程同时保存不共享同一中间文件（FIX002.7）
    tmp_path = resolved.with_name(f"{resolved.name}.{os.getpid()}.tmp")
    try:
        resolved.parent.mkdir(parents=True, exist_ok=True)
        # 原子替换：先写同目录临时文件再 os.replace，防中途崩溃产生半截文件（FIX001.2）
        tmp_path.write_text(
            json.dumps(data, ensure_ascii=False, indent=4), encoding="utf-8"
        )
        os.replace(tmp_path, resolved)
        clear_json_cache(str(resolved))
        return True
    except OSError:
        try:
            tmp_path.unlink(missing_ok=True)
        except OSError:
            # Windows 下刚关闭的文件可能被 AV/索引服务瞬时占用，清理失败不外抛（FIX002.7）
            pass
        return False


def clear_json_cache(path: Path | str | None = None) -> None:
    # 清空缓存：指定路径则只清该路径（与 read_json_cached 同为 resolve 键，FIX001.17），None 清空全部
    if path is None:
        _json_cache.clear()
    else:
        _json_cache.pop(str(Path(path).resolve()), None)


# ===== utils/file_utils.py 函数/常量说明 =====
# read_json(path, default): 读取 JSON 文件
#   输入：文件路径、默认值；输出：解析后的数据或 default
#   设计理由：统一异常处理与编码，避免各模块重复 try/except
#   异常处理：捕获 OSError 与 json.JSONDecodeError、UnicodeDecodeError（FIX001.1），
#     损坏/编码异常文件返回默认值
# read_json_cached(path, default): 带缓存的 JSON 读取
#   输入：文件路径、默认值；输出：解析后的数据（命中缓存时直接返回）
#   设计理由：缓存单例避免高频重复 IO，配置文件被多次读取时显著降低开销（修复 D4）；
#   缓存键统一 resolve（FIX001.17），与写入清缓存键一致，杜绝相对路径写后读旧缓存
#   异常处理：同 read_json，缓存仅在成功解析后写入
# write_json(path, data): 写入 JSON 文件
#   输入：文件路径、数据；输出：bool 是否成功
#   设计理由：自动创建父目录，UTF-8 中文友好输出，写入后同步清理缓存保证一致性；
#   原子替换（同目录 .tmp 中间文件 + os.replace，FIX001.2）防中途崩溃产生半截配置
#   安全约束（V0.4.7.0）：路径规范化（resolve）后必须位于 _WRITE_ALLOWED_ROOTS
#   （项目根/系统临时目录）内，防路径穿越；IO 与缓存键统一使用解析后路径
#   异常处理：越界抛 ValueError（编程错误上抛）；IO 捕获 OSError 返回 False 并清理中间文件
# clear_json_cache(path): 清空缓存
#   输入：可选文件路径（resolve 后为键）；输出：None
#   设计理由：保存配置后调用，保证后续读取始终是最新数据
#   关联配置：由 config/settings.py 接入
