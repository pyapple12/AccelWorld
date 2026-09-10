# 时钟展示运算工具（PL001.07 自 clock_panel.update_time 迁入，plan#UI2.0 运算出面板）


def progress_bounds(expanded_hours_per_day: float) -> int:
    # 进度条上界（膨胀后一天总小时数取整），int() 截断语义与原 update_time 一致
    return int(expanded_hours_per_day)


# ===== ui/tools/clock_tools.py 函数说明 =====
# progress_bounds(expanded_hours_per_day) -> int
#   输入：膨胀后一天小时数（TimeInfo.expanded_hours_per_day）；输出：进度条 maximum 值
#   设计理由：进度换算迁出面板（plan#UI2.0 铁律 1），面板 update_time 只剩 setText/动画驱动
#   异常处理：无（float → int 截断无异常路径）
#   关联配置：无（纯函数）
