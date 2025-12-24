import datetime
from lunar_python import Solar
from chinese_calendar import get_holiday_detail

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
    (21, 23, "亥时")
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
    12: ("正南", "离位")
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
    (12, 25): "圣诞节"
}

# 农历计算核心函数
def get_chinese_lunar_calendar(year: int, month: int, day: int, hour: int) -> tuple:
    """
    计算农历信息
    :param year: 公历年
    :param month: 公历月
    :param day: 公历日
    :param hour: 公历小时
    :return: (农历年, 生肖, 农历月, 农历日, 时辰, 月相, 节气, 公历节日, 拜财神方向)
    """
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
    holiday_translation = {
        "New Year's Day": "元旦",
        "National Day": "国庆节"
    }
    public_holiday = holiday_translation.get(public_holiday, public_holiday)
    
    # 获取拜财神方向
    cai_shen_dir, position = CAI_SHEN_DIRECTION[month]
    
    return lunar_year, shengxiao, lunar_month, lunar_day, current_chen, yue_phase, jieqi, public_holiday, cai_shen_dir, position

def get_chinese_date(now: datetime.datetime) -> str:
    """将datetime对象转换为中文日期格式字符串（含星期几）"""
    weekday_map = {0: "星期一", 1: "星期二", 2: "星期三", 3: "星期四", 4: "星期五", 5: "星期六", 6: "星期日"}
    return now.strftime(f"%Y年%m月%d日 {weekday_map[now.weekday()]}")

def get_lunar_info(now: datetime.datetime) -> str:
    """
    获取农历信息字符串
    :param now: datetime对象
    :return: 农历信息字符串
    """
    year = now.year
    month = now.month
    day = now.day
    hour = now.hour
    
    lunar_year, shengxiao, lunar_month, lunar_day, current_chen, yue_phase, jieqi, public_holiday, cai_shen_dir, position = get_chinese_lunar_calendar(year, month, day, hour)
    
    # 构建农历信息字符串
    lunar_info = f"{lunar_year}（{shengxiao}年）{lunar_month}{lunar_day}{current_chen}"
    lunar_info += f" 月相：{yue_phase}"
    
    if jieqi:
        lunar_info += f" 节气：{jieqi}"
    
    if public_holiday:
        lunar_info += f" 公历节日：{public_holiday}"
    
    lunar_info += f" 拜财神：{cai_shen_dir}方向（{position}）"
    
    return lunar_info
