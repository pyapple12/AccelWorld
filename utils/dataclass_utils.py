# dataclass 反序列化通用工具模块
# dataclass_from_dict：字段白名单 + 类型校验过滤 + 默认值兜底（tolerant 容错模式供列表加载）
# to_dict 直接用标准库 asdict（一行调用无需抽象，S9.6 清理冗余包装）

import typing
from typing import Any, Dict, Type, TypeVar

# dataclass 类型变量（保持返回类型）
T = TypeVar("T")


def _value_matches(annotation: Any, value: Any) -> bool:
    # 单层类型匹配：基础类型 / Union(Optional) / list / dict / Literal；
    # list 仅校验容器类型，元素级规范化由各 dataclass __post_init__ 负责（如 Alarm.repeat_days）
    origin = typing.get_origin(annotation)
    if origin is typing.Union:
        return any(_value_matches(arg, value) for arg in typing.get_args(annotation))
    if origin is list:
        return isinstance(value, list)
    if origin is dict:
        return isinstance(value, dict)
    if origin is typing.Literal:
        return value in typing.get_args(annotation)
    if annotation is Any:
        return True
    if isinstance(annotation, type):
        if annotation is float:
            # JSON 数字可整可浮；布尔是 int 子类需显式排除
            return isinstance(value, (int, float)) and not isinstance(value, bool)
        if annotation is int:
            return isinstance(value, int) and not isinstance(value, bool)
        if annotation is bool:
            return isinstance(value, bool)
        return isinstance(value, annotation)
    # 未知注解形态放行，交由构造期校验兜底
    return True


def dataclass_from_dict(
    cls: Type[T], data: Dict[str, Any], tolerant: bool = False
) -> T | None:
    # 从字典构造 dataclass：仅取类型匹配的有效字段并兜底默认值；tolerant=True 时构造失败返回 None
    # 类型不符的键剔除（FIX001.9：脏配置静默穿透的防御），剔除后由字段默认值兜底；
    # 必填字段被剔除时构造抛 TypeError → tolerant 模式返回 None 由调用方跳过该条目
    hints = typing.get_type_hints(cls)
    filtered = {
        k: v for k, v in data.items() if k in hints and _value_matches(hints[k], v)
    }
    try:
        return cls(**filtered)
    except (ValueError, TypeError):
        # 容错模式：非法数据（时间格式错误/必填字段缺失）由调用方跳过该条目
        # （捕获 TypeError：修复 S10.2 A2——time 为 null 时 ":" in None 抛 TypeError 而非 ValueError）
        if tolerant:
            return None
        raise


# ===== utils/dataclass_utils.py 函数/常量说明 =====
# _value_matches(annotation, value) -> bool: 单层类型匹配（私有辅助）
#   设计理由：JSON 反序列化的脏值防御（FIX001.9）——theme:null、rate:"abc" 等
#   此前会静默穿透到 UI/业务层；float 接受 int（JSON 无整浮之分）、排除布尔
# dataclass_from_dict(cls, data, tolerant): dict → dataclass 实例
#   输入：目标类、数据字典、是否容错；输出：实例或 None（tolerant 且构造失败）
#   逻辑步骤：get_type_hints 解析注解 → 类型匹配过滤 → cls(**filtered) 构造
#   设计理由：未知键过滤 + 类型不符剔除 + 缺省字段默认值兜底，消除各 dataclass 重复实现；
#   容器元素规范化下沉到 __post_init__（单一职责：此处管"类型"，模型管"取值域"）
#   异常处理：tolerant=False 时构造 (ValueError, TypeError) 原样上抛；
#     tolerant=True 时返回 None（S10.2 A2 补捕获 TypeError）
#   关联配置：供 config/settings.py（UserConfig）与 modules/alarm_service.py（Alarm）使用
# 注：dataclass_to_dict 已删除（S9.6）——纯转发 asdict 无额外逻辑，to_dict 由各 dataclass
#   直接调用标准库 asdict，避免冗余抽象层
