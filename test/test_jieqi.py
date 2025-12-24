import datetime
from lunar_python import Solar, Lunar

# 测试不同日期的节气信息

test_dates = [
    # 接近节气的日期
    (2025, 12, 21, "冬至"),
    (2025, 12, 25, "圣诞节"),
    (2026, 1, 5, "小寒"),
    (2026, 1, 20, "大寒"),
    (2026, 2, 4, "立春"),
    (2026, 2, 19, "雨水"),
    (2026, 3, 5, "惊蛰"),
]

for year, month, day, desc in test_dates:
    print(f"\n=== {year}-{month}-{day} ({desc}) ===")
    solar = Solar.fromYmdHms(year, month, day, 12, 0, 0)
    lunar = solar.getLunar()
    
    # 测试各种节气相关方法
    print(f"getJieQi(): {lunar.getJieQi()}")
    print(f"getCurrentJieQi(): {lunar.getCurrentJieQi()}")
    print(f"getCurrentJie(): {lunar.getCurrentJie()}")
    print(f"getCurrentQi(): {lunar.getCurrentQi()}")
    print(f"getPrevJie(): {lunar.getPrevJie()}")
    print(f"getNextJie(): {lunar.getNextJie()}")
    print(f"getPrevQi(): {lunar.getPrevQi()}")
    print(f"getNextQi(): {lunar.getNextQi()}")
    
    # 测试时辰相关方法
    print(f"\n时辰相关:")
    print(f"getTimeInGanZhi(): {lunar.getTimeInGanZhi()}")
    print(f"getTimeGan(): {lunar.getTimeGan()}")
    print(f"getTimeZhi(): {lunar.getTimeZhi()}")
    print(f"getTimeShengXiao(): {lunar.getTimeShengXiao()}")
