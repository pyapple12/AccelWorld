import datetime
from accelworld_date import get_lunar_info

# 测试不同日期的节气显示

test_dates = [
    # 接近节气的日期
    (2025, 12, 21, "冬至"),
    (2025, 12, 25, "圣诞节"),
    (2026, 1, 5, "小寒"),
    (2026, 1, 20, "大寒"),
    (2026, 2, 4, "立春"),
]

for year, month, day, desc in test_dates:
    print(f"\n=== {year}-{month}-{day} ({desc}) ===")
    test_date = datetime.datetime(year, month, day, 12, 0, 0)
    lunar_info = get_lunar_info(test_date)
    print(f"农历信息: {lunar_info}")

# 测试当前日期
print(f"\n=== 当前日期 ===")
now = datetime.datetime.now()
print(f"当前时间: {now}")
lunar_info = get_lunar_info(now)
print(f"农历信息: {lunar_info}")