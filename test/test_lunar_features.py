import datetime
from lunar_python import Solar, Lunar

# 创建测试日期
solar = Solar.fromYmdHms(2025, 12, 25, 12, 0, 0)
lunar = solar.getLunar()

# 查看Lunar类的所有方法
print("=== Lunar类的所有方法 ===")
methods = [method for method in dir(lunar) if not method.startswith('_')]
for method in methods:
    print(method)

# 测试获取时辰相关信息
print("\n=== 时辰相关测试 ===")
# 查看是否有时辰相关的方法
time_methods = [method for method in methods if 'hour' in method.lower() or 'time' in method.lower()]
print("时间相关方法:", time_methods)

# 测试获取节气信息
print("\n=== 节气相关测试 ===")
# 查看是否有节气相关的方法
solar_term_methods = [method for method in methods if 'term' in method.lower()]
print("节气相关方法:", solar_term_methods)

# 尝试获取节气
if hasattr(lunar, 'getJieQi'):
    print("节气:", lunar.getJieQi())
elif hasattr(solar, 'getJieQi'):
    print("节气:", solar.getJieQi())

# 测试其他可能的节气方法
if hasattr(lunar, 'getSolarTerm'):
    print("节气:", lunar.getSolarTerm())
elif hasattr(solar, 'getSolarTerm'):
    print("节气:", solar.getSolarTerm())

# 测试获取更多信息
print("\n=== 其他农历信息 ===")
print(f"年份: {lunar.getYear()}")
print(f"月份: {lunar.getMonth()}")
print(f"日期: {lunar.getDay()}")
print(f"天干地支年: {lunar.getYearInGanZhi()}")
print(f"生肖: {lunar.getYearShengXiao()}")
print(f"月相: {lunar.getYueXiang()}")

# 查看Solar类的方法，看是否有更多信息
print("\n=== Solar类的所有方法 ===")
solar_methods = [method for method in dir(solar) if not method.startswith('_')]
for method in solar_methods:
    print(method)

# 测试获取当天的节气
if hasattr(solar, 'getJieQi'):
    print("\n当天节气:", solar.getJieQi())
