import datetime
from accelworld_date import get_lunar_info, get_chinese_date

# 获取当前时间
now = datetime.datetime.now()

# 测试中文日期
chinese_date = get_chinese_date(now)
print("中文日期:", chinese_date)

# 测试农历信息
lunar_info = get_lunar_info(now)
print("农历信息:", lunar_info)

# 测试特定日期（2025年12月25日）
specific_date = datetime.datetime(2025, 12, 25, 2, 0, 0)
lunar_info_specific = get_lunar_info(specific_date)
print("2025年12月25日农历信息:", lunar_info_specific)