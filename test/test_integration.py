import datetime
from accelworld_calc import AcceleratedWorld
from accelworld_date import get_chinese_date, get_lunar_info

# 测试1：日期模块测试
print("测试1：日期模块")
now = datetime.datetime.now()
chinese_date = get_chinese_date(now)
lunar_info = get_lunar_info(now)
print(f"中文日期: {chinese_date}")
print(f"农历信息: {lunar_info}")
print()

# 测试2：计算模块测试
print("测试2：计算模块")
accel_world = AcceleratedWorld(time_dilation_rate=2.0)
result = accel_world.get_custom_time()
print(f"返回值数量: {len(result)}")
print(f"标准日期时间: {result[0]}")
print(f"加速时间: {result[1]}")
print(f"中文日期: {result[2]}")
print(f"农历信息: {result[3]}")
print(f"时间膨胀百分比: {result[4]}")
print(f"每日额外小时: {result[5]}")
print(f"剩余小时数: {result[6]}")
print()

# 测试3：特定日期测试
print("测试3：特定日期测试")
specific_date = datetime.datetime(2025, 12, 25, 2, 0, 0)
chinese_date = get_chinese_date(specific_date)
lunar_info = get_lunar_info(specific_date)
print(f"2025年12月25日中文日期: {chinese_date}")
print(f"2025年12月25日农历信息: {lunar_info}")
print()

# 测试4：计算模块的特定日期测试
print("测试4：计算模块的特定日期测试")
# 保存原始的datetime.now函数
original_now = datetime.datetime.now
# 替换为特定日期
datetime.datetime.now = lambda: specific_date
try:
    result = accel_world.get_custom_time()
    print(f"特定日期下的农历信息: {result[3]}")
finally:
    # 恢复原始函数
    datetime.datetime.now = original_now

print("\n所有测试完成！")