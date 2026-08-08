# 农历/干支/节气/节日模块（迁移自 accelworld_date.py，S1 结构骨架阶段）

import datetime
from dataclasses import dataclass
from typing import Tuple

from lunar_python import Solar  # type: ignore
from chinese_calendar import get_holiday_detail  # type: ignore

# 时辰映射
SHI_CHEN = [
    (23, 1, "子时"),
    (1, 3, "丑时"),
    (3, 5, "寅时"),
    (5, 7, "卯时"),
    (7, 9, "辰时"),
    (9, 11, "巳时"),
    (11, 13, "午时"),
    (13, 15, "未时"),
    (15, 17, "申时"),
    (17, 19, "酉时"),
    (19, 21, "戌时"),
    (21, 23, "亥时"),
]

# 拜财神方向（按月份）
CAI_SHEN_DIRECTION = {
    1: ("正北", "坎位"),
    2: ("东北", "艮位"),
    3: ("正东", "震位"),
    4: ("东南", "巽位"),
    5: ("正南", "离位"),
    6: ("西南", "坤位"),
    7: ("正西", "兑位"),
    8: ("西北", "乾位"),
    9: ("正北", "坎位"),
    10: ("东北", "艮位"),
    11: ("正东", "震位"),
    12: ("正南", "离位"),
}

# 自定义节日列表（包括西方重要节日）
CUSTOM_HOLIDAYS = {
    (1, 1): "元旦",
    (2, 14): "情人节",
    (3, 8): "妇女节",
    (3, 12): "植树节",
    (4, 1): "愚人节",
    (5, 1): "劳动节",
    (5, 4): "青年节",
    (6, 1): "儿童节",
    (7, 1): "建党节",
    (8, 1): "建军节",
    (9, 10): "教师节",
    (10, 1): "国庆节",
    (12, 25): "圣诞节",
}

# 英文节日名称到中文的翻译映射
HOLIDAY_TRANSLATION = {"New Year's Day": "元旦", "National Day": "国庆节"}


@dataclass
class LunarInfo:
    """农历信息数据类（聚合农历计算的 10 个字段）"""

    lunar_year: str  # 天干地支年（如"丙午年"）
    shengxiao: str  # 生肖
    lunar_month: str  # 农历月（中文）
    lunar_day: str  # 农历日（中文）
    shichen: str  # 时辰
    yue_phase: str  # 月相
    jieqi: str  # 节气（无则为空字符串）
    public_holiday: str  # 公历节日（无则为空字符串）
    cai_shen_dir: str  # 拜财神方向
    position: str  # 八卦方位


# 农历计算核心函数
def get_chinese_lunar_calendar(year: int, month: int, day: int, hour: int) -> LunarInfo:
    """计算农历信息：干支/生肖/月日/时辰/月相/节气/节日/财神方位"""
    # lunar-python 提供干支生肖月日，chinese-calendar 兜底节假日，自定义表兜底
    # 使用lunar-python获取农历信息
    solar = Solar.fromYmdHms(year, month, day, hour, 0, 0)
    lunar = solar.getLunar()

    # 获取天干地支年份
    lunar_year = lunar.getYearInGanZhi() + "年"

    # 获取生肖
    shengxiao = lunar.getYearShengXiao()

    # 获取农历月份和日期
    lunar_month = lunar.getMonthInChinese() + "月"
    lunar_day = lunar.getDayInChinese()

    # 计算时辰
    for start, end, chen in SHI_CHEN:
        if start <= hour < end:
            current_chen = chen
            break
    else:
        current_chen = "子时"

    # 获取月相
    yue_phase = lunar.getYueXiang() + "月"

    # 获取节气
    jieqi = lunar.getJieQi()
    if not jieqi:
        # 如果当天不是节气日，获取当前节气（如果有的话）
        jieqi = lunar.getCurrentJieQi()
    # 确保节气值是字符串类型
    jieqi = str(jieqi) if jieqi else ""

    # 获取公历节日
    # 先检查lunar-python的节日
    festivals = lunar.getFestivals()
    public_holiday = festivals[0] if festivals else ""

    # 如果lunar-python没有找到节日，检查chinese-calendar
    if not public_holiday:
        holiday_detail = get_holiday_detail(datetime.datetime(year, month, day))
        # get_holiday_detail返回(Boolean, String)元组，第二个元素是节日名称
        public_holiday = holiday_detail[1] if holiday_detail[0] else ""

    # 如果chinese-calendar没有找到节日，检查自定义节日列表
    if not public_holiday:
        public_holiday = CUSTOM_HOLIDAYS.get((month, day), "")

    # 将英文节日名称转换为中文
    public_holiday = HOLIDAY_TRANSLATION.get(public_holiday, public_holiday)

    # 获取拜财神方向
    cai_shen_dir, position = CAI_SHEN_DIRECTION[month]

    return LunarInfo(
        lunar_year=lunar_year,
        shengxiao=shengxiao,
        lunar_month=lunar_month,
        lunar_day=lunar_day,
        shichen=current_chen,
        yue_phase=yue_phase,
        jieqi=jieqi,
        public_holiday=public_holiday,
        cai_shen_dir=cai_shen_dir,
        position=position,
    )


def get_chinese_date(now: datetime.datetime) -> str:
    """将 datetime 转为中文日期字符串（YYYY年MM月DD日 星期X）"""
    # 星期映射后经 strftime 格式化
    weekday_map = {
        0: "星期一",
        1: "星期二",
        2: "星期三",
        3: "星期四",
        4: "星期五",
        5: "星期六",
        6: "星期日",
    }
    return now.strftime(f"%Y年%m月%d日 {weekday_map[now.weekday()]}")


def get_lunar_info(now: datetime.datetime) -> str:
    """获取农历信息展示字符串（干支/生肖/月日/时辰/月相/节气/节日/财神）"""
    # 委托 get_chinese_lunar_calendar 后按固定格式拼接，空字段跳过
    year = now.year
    month = now.month
    day = now.day
    hour = now.hour

    info = get_chinese_lunar_calendar(year, month, day, hour)

    # 构建农历信息字符串
    lunar_info = (
        f"{info.lunar_year}（{info.shengxiao}年）"
        f"{info.lunar_month}{info.lunar_day}{info.shichen}"
    )
    lunar_info += f" 月相：{info.yue_phase}"

    if info.jieqi:
        lunar_info += f" 节气：{info.jieqi}"

    if info.public_holiday:
        lunar_info += f" 公历节日：{info.public_holiday}"

    lunar_info += f" 拜财神：{info.cai_shen_dir}方向（{info.position}）"

    return lunar_info


# ===== modules/chinese_calendar.py 函数/常量说明 =====
# 常量：SHI_CHEN 时辰表、CAI_SHEN_DIRECTION 财神方位表、CUSTOM_HOLIDAYS 自定义节日表、
#       HOLIDAY_TRANSLATION 英文节日翻译表（S2.2.2 提升为模块级）
# LunarInfo: dataclass，农历信息聚合（S2 引入，替代 10 元组返回）
# get_chinese_lunar_calendar(year, month, day, hour) -> LunarInfo:
#   输入：公历年月日时；输出：LunarInfo 各字段
#   逻辑步骤：lunar-python 取干支/生肖/农历月日/月相/节气 → 时辰表匹配 →
#            节日三级兜底（lunar-python → chinese-calendar → CUSTOM_HOLIDAYS）→ 翻译 → 财神方位
#   设计理由：三库兜底提高节日覆盖率；数据表模块级常量避免重复构建
# get_chinese_date(now) -> str: 中文日期字符串
# get_lunar_info(now) -> str: 拼装农历展示文本（空字段跳过）
#   异常处理：节气/节日可能为空，统一转空字符串避免拼接 None
#   关联配置：依赖 lunar-python 与 chinese-calendar 第三方库
