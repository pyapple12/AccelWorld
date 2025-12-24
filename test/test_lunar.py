import datetime
from lunar_python import Solar
from chinese_calendar import get_holiday_detail

# 获取当前时间
now = datetime.datetime.now()

# 使用lunar-python获取农历信息
solar = Solar.fromYmdHms(now.year, now.month, now.day, now.hour, now.minute, now.second)
lunar = solar.getLunar()

# 输出基本信息
print("当前时间:", now)
print("农历年份:", lunar.getYearInChinese())
print("生肖:", lunar.getYearShengXiao())
print("农历月份:", lunar.getMonthInChinese())
print("农历日期:", lunar.getDayInChinese())
print("月相:", lunar.getYueXiang())

# 使用chinese-calendar获取节日信息
holiday_detail = get_holiday_detail(now)
holiday = holiday_detail[0] if holiday_detail else None
print("公历节日:", holiday if holiday else "无")

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

# 计算时辰
hour = now.hour
for start, end, chen in SHI_CHEN:
    if start <= hour < end:
        current_chen = chen
        break
else:
    current_chen = "子时"

print("当前时辰:", current_chen)

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

cai_shen_dir, position = CAI_SHEN_DIRECTION[now.month]
print("拜财神方向:", cai_shen_dir, position)

# 构建最终格式的农历信息字符串
# 获取天干地支年份
year_in_gan_zhi = lunar.getYearInGanZhi()
# 月份添加"月"字
lunar_month = lunar.getMonthInChinese() + "月"
# 月相添加"月"字
yue_phase = lunar.getYueXiang() + "月"

lunar_info = f"{year_in_gan_zhi}年（{lunar.getYearShengXiao()}年）{lunar_month}{lunar.getDayInChinese()}{current_chen}"
lunar_info += f" 月相：{yue_phase}"

# 检查lunar-python的节日
festivals = lunar.getFestivals()
if festivals:
    public_holiday = festivals[0]
    lunar_info += f" 公历节日：{public_holiday}"
elif holiday:
    lunar_info += f" 公历节日：{holiday}"

lunar_info += f" 拜财神：{cai_shen_dir}方向（{position}）"

print("\n最终格式:")
print(lunar_info)