# 倒计时展示运算工具（PL001.06 自 countdown_panel 迁入，plan#UI2.0 运算出面板）
# 纯函数：目标文本解析 + 剩余时间拆解，供 CountdownPanel 调用后仅做 setText/着色

import datetime


def parse_target_text(target_text: str) -> datetime.datetime | None:
    # 解析三种目标文本格式（19/16/10 位，日期格式默认当日 23:59:59）；非法返回 None
    # （设置倒计时与启动恢复共用，FIX001.10 抽取语义原样保留）
    try:
        if len(target_text) == 19:  # YYYY-MM-DD HH:MM:SS
            return datetime.datetime.strptime(target_text, "%Y-%m-%d %H:%M:%S")
        if len(target_text) == 16:  # YYYY-MM-DD HH:MM
            return datetime.datetime.strptime(target_text, "%Y-%m-%d %H:%M")
        if len(target_text) == 10:  # YYYY-MM-DD
            return datetime.datetime.strptime(
                target_text, "%Y-%m-%d"
            ).replace(hour=23, minute=59, second=59)
    except ValueError:
        return None
    return None


def format_remaining(
    target: datetime.datetime, now: datetime.datetime
) -> tuple[str, bool]:
    # 计算剩余时间文本（"X天 HH:MM:SS"）与结束态；已到点/过期返回 ("00天 00:00:00", True)
    # 着色决策（结束红/进行中主题色）归面板，本函数只给数据
    remaining = target - now
    if remaining.total_seconds() <= 0:
        return "00天 00:00:00", True
    days = remaining.days
    hours = remaining.seconds // 3600
    minutes = (remaining.seconds % 3600) // 60
    seconds = remaining.seconds % 60
    return f"{days}天 {hours:02d}:{minutes:02d}:{seconds:02d}", False


# ===== ui/tools/countdown_tools.py 函数说明 =====
# parse_target_text(target_text) -> datetime | None
#   输入：目标文本（19/16/10 位三种格式）；输出：解析结果或 None
#   逻辑步骤：按长度分发 strptime → 10 位日期格式补当日 23:59:59 → 越界值 ValueError 捕获
#   设计理由：解析规则单一来源（原面板私有方法迁入），设置与恢复路径共用
#   异常处理：非法格式/越界值捕获 ValueError 返回 None，不外抛
# format_remaining(target, now) -> tuple[str, bool]
#   输入：目标时刻与当前时刻；输出：(展示文本, 是否已结束)
#   逻辑步骤：timedelta 拆天/时/分/秒 → total_seconds <= 0 判结束
#   设计理由：剩余拆解是纯运算，迁出面板后面板只剩 setText/着色（plan#UI2.0 铁律 1）
#   异常处理：无（datetime 运算无业务异常路径）
#   关联配置：无（纯函数）
