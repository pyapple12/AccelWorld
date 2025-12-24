import datetime
import time
import re
from accelworld_calc import AcceleratedWorld

def test_custom_time_based_on_current():
    """测试自定义时间是否基于当前标准时间转换"""
    print("=== 测试1: 自定义时间基于当前标准时间转换 ===")
    
    # 创建不同倍率的实例
    rates = [2.0, 3.0, 1.5]
    
    for rate in rates:
        accel = AcceleratedWorld(time_dilation_rate=rate)
        standard_datetime, custom_time, chinese_date, dilation_percentage, expanded_hours_per_day, remaining_hours = accel.get_custom_time()
        
        # 解析时间获取小时
        std_hour = int(standard_datetime.split()[1].split(':')[0])
        custom_hour = int(custom_time.split(':')[0])
        
        # 验证小时数是否按倍率转换
        expected_hour = int(std_hour * rate)
        
        print(f"倍率: {rate}x | 标准时间: {standard_datetime}")
        print(f"自定义时间: {custom_time}")
        print(f"中文日期: {chinese_date}")
        print(f"膨胀倍率: {dilation_percentage:.0f}% | 一天小时数: {expanded_hours_per_day:.2f}小时 | 当天剩余: {remaining_hours:.2f}小时")
        print(f"标准小时: {std_hour} → 期望自定义小时: ~{expected_hour} → 实际自定义小时: {custom_hour}")
        print(f"验证: {'通过' if abs(expected_hour - custom_hour) <= 1 else '失败'}")
        print()

if __name__ == "__main__":
    print("加速世界时间转换测试套件")
    print("=" * 50)
    
    test_custom_time_based_on_current()
    
    print("测试完成！")
