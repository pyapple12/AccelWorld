import datetime
from lunar_python import Solar
from chinese_calendar import get_holiday_detail, is_holiday

# 测试当前日期（2025年12月25日）
test_date = datetime.datetime(2025, 12, 25)
print(f"测试日期: {test_date}")
print()

# 测试lunar-python的节日获取
print("=== lunar-python 节日测试 ===")
solar = Solar.fromYmdHms(2025, 12, 25, 0, 0, 0)
lunar = solar.getLunar()

festivals = lunar.getFestivals()
print(f"getFestivals(): {festivals}")

other_festivals = lunar.getOtherFestivals()
print(f"getOtherFestivals(): {other_festivals}")

# 测试其他重要日期
print()
print("=== 其他重要日期测试 ===")

# 新年
solar_new_year = Solar.fromYmdHms(2026, 1, 1, 0, 0, 0)
lunar_new_year = solar_new_year.getLunar()
print(f"2026年1月1日 (新年): {lunar_new_year.getFestivals()}")

# 国庆节
solar_national = Solar.fromYmdHms(2025, 10, 1, 0, 0, 0)
lunar_national = solar_national.getLunar()
print(f"2025年10月1日 (国庆节): {lunar_national.getFestivals()}")

# 测试chinese-calendar库
print()
print("=== chinese-calendar 节日测试 ===")

# 检查是否为假日
print(f"2025年12月25日是否为假日: {is_holiday(test_date)}")

# 获取假日详情
holiday_detail = get_holiday_detail(test_date)
print(f"get_holiday_detail(): {holiday_detail}")

# 测试其他日期的假日
print(f"2026年1月1日是否为假日: {is_holiday(datetime.datetime(2026, 1, 1))}")
print(f"2026年1月1日假日详情: {get_holiday_detail(datetime.datetime(2026, 1, 1))}")

print(f"2025年10月1日是否为假日: {is_holiday(datetime.datetime(2025, 10, 1))}")
print(f"2025年10月1日假日详情: {get_holiday_detail(datetime.datetime(2025, 10, 1))}")
