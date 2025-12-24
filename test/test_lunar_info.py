import datetime
from accelworld_date import get_chinese_date, get_lunar_info

# 测试当前日期（应该是圣诞节）
now = datetime.datetime.now()
print(f"当前时间: {now}")
print(f"中文日期: {get_chinese_date(now)}")
print(f"农历信息: {get_lunar_info(now)}")

# 测试圣诞节
test_date = datetime.datetime(2025, 12, 25, 12, 0, 0)
print(f"\n测试圣诞节(2025-12-25):")
print(f"中文日期: {get_chinese_date(test_date)}")
print(f"农历信息: {get_lunar_info(test_date)}")

# 测试新年
test_date = datetime.datetime(2026, 1, 1, 12, 0, 0)
print(f"\n测试新年(2026-01-01):")
print(f"中文日期: {get_chinese_date(test_date)}")
print(f"农历信息: {get_lunar_info(test_date)}")