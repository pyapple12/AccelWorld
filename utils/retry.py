# 通用重试工具模块
# 泛型重试函数，异常元组参数化（参考 DeepTransHub utils/error_handler.py 的 retry_call）
# S1 阶段创建工具，S5 由天气网络请求接入使用

import time
from typing import Any, Callable, Tuple, Type


def retry_call(
    func: Callable[..., Any],
    *args: Any,
    retries: int = 3,
    exceptions: Tuple[Type[Exception], ...] = (Exception,),
    delay: float = 1.0,
    **kwargs: Any,
) -> Any:
    # 通用重试：调用 func，失败时按异常元组判断是否重试，达上限抛出最后一次异常
    last_exc: Exception | None = None
    for attempt in range(retries):
        try:
            return func(*args, **kwargs)
        except exceptions as e:
            last_exc = e
            if attempt < retries - 1:
                time.sleep(delay)
    raise last_exc  # type: ignore[misc]


# ===== utils/retry.py 函数/常量说明 =====
# retry_call(func, *args, retries, exceptions, delay, **kwargs): 通用重试函数
#   输入：目标函数及其参数、重试次数、可重试异常元组、重试间隔
#   输出：目标函数的返回值；全部失败时抛出最后一次异常
#   逻辑步骤：循环尝试 → 命中 exceptions 且未达上限则 sleep 后重试 → 达上限抛异常
#   设计理由：与业务解耦的泛型实现（*args/**kwargs 适配任意签名），供网络请求等
#   不稳定调用复用，避免各模块重复编写重试循环
#   异常处理：只捕获 exceptions 元组内异常；最后抛出原异常保留错误信息
#   关联配置：S5 由 modules/weather_service.py 接入
