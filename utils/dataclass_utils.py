# dataclass 反序列化通用工具模块
# 仅保留有真实逻辑的 dataclass_from_dict；to_dict 直接用标准库 asdict（一行调用无需抽象，S9.6 清理冗余包装）

from dataclasses import fields
from typing import Any, Dict, Type, TypeVar

# dataclass 类型变量（保持返回类型）
T = TypeVar("T")


def dataclass_from_dict(
    cls: Type[T], data: Dict[str, Any], tolerant: bool = False
) -> T | None:
    # 从字典构造 dataclass：仅取有效字段并兜底默认值；tolerant=True 时构造失败返回 None
    valid_fields = {f.name for f in fields(cls)}  # type: ignore[arg-type]
    filtered = {k: v for k, v in data.items() if k in valid_fields}
    try:
        return cls(**filtered)
    except (ValueError, TypeError):
        # 容错模式：非法数据（时间格式错误/字段类型不匹配如 None）由调用方跳过该条目
        # （捕获 TypeError：修复 S10.2 A2——time 为 null 时 ":" in None 抛 TypeError 而非 ValueError）
        if tolerant:
            return None
        raise


# ===== utils/dataclass_utils.py 函数/常量说明 =====
# dataclass_from_dict(cls, data, tolerant): dict → dataclass 实例
#   输入：目标类、数据字典、是否容错；输出：实例或 None（tolerant 且构造失败）
#   逻辑步骤：字段白名单过滤 → cls(**filtered) 构造
#   设计理由：未知键过滤 + 缺省字段默认值兜底，消除各 dataclass 重复的反序列化实现
#   异常处理：tolerant=False 时构造 (ValueError, TypeError) 原样上抛；
#     tolerant=True 时返回 None（S10.2 A2 补捕获 TypeError，防 null 字段崩溃）
#   关联配置：供 config/settings.py 与 modules/alarm_service.py 使用
# 注：dataclass_to_dict 已删除（S9.6）——纯转发 asdict 无额外逻辑，to_dict 由各 dataclass
#   直接调用标准库 asdict，避免冗余抽象层
