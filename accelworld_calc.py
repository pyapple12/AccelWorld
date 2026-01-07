import datetime
import time
import sys
import argparse

# 导入日期处理模块
from accelworld_date import get_chinese_date, get_lunar_info

# 程序版本号
VERSION = "ver 0.11"

class AcceleratedWorld:
    """加速世界 - 基于时间膨胀倍率的自定义小时制时间显示核心类"""
    
    time_dilation_rate: float
    """时间膨胀倍率（必须>1.0，默认2.0）"""
    
    seconds_per_day: int
    """标准一天的总秒数（固定为86400）"""
    
    custom_hours_per_day: int
    """基于膨胀率计算的一天总小时数"""
    
    def __init__(self, time_dilation_rate: float = 2.0):
        """
        初始化时间膨胀倍率
        :param time_dilation_rate: 时间膨胀倍率（必须>1.0，以24小时为基准）
        """
        if time_dilation_rate <= 1.0:
            raise ValueError("时间膨胀倍率必须大于1.0！")
        self.time_dilation_rate = time_dilation_rate
        self.seconds_per_day = 86400  # 标准一天的总秒数
        self.custom_hours_per_day = int(24 * time_dilation_rate)  # 计算一天的自定义小时数
        self.start_time = None  # 记录时钟启动时间（毫秒级精度）
    
    def get_custom_time(self) -> tuple[str, str, str, str, float, float, float]:
        """
        计算并返回当前的自定义时间和标准日期时间
        :return: (
            标准日期时间字符串,
            自定义时间字符串,
            中文日期字符串（格式：YYYY年MM月DD日 星期X）,
            农历信息字符串,
            时间膨胀倍率百分比,
            膨胀后一天的小时数（精确到两位小数）,
            加速后当天剩余的小时数（精确到两位小数）
        )
        """
        # 获取当前系统时间（带毫秒精度）
        now = datetime.datetime.now()
        
        # 格式化标准日期时间（只显示到秒）
        standard_datetime = now.strftime("%Y-%m-%d %H:%M:%S")
        
        # 使用日期模块获取中文日期
        chinese_date = get_chinese_date(now)
        
        # 获取农历信息
        lunar_info = get_lunar_info(now)
        
        # 计算当前时刻在标准一天中的总秒数（毫秒级精度）
        current_hour = now.hour
        current_minute = now.minute
        current_second = now.second
        current_microsecond = now.microsecond
        
        # 计算总秒数，包含毫秒精度
        total_seconds = current_hour * 3600 + current_minute * 60 + current_second + current_microsecond / 1e6
        
        # 使用时间膨胀倍率计算自定义时间的总秒数（毫秒级精度）
        custom_total_seconds = total_seconds * self.time_dilation_rate
        
        # 使用整数运算直接计算小时、分钟和秒，避免手动进位
        custom_hour = int(custom_total_seconds // 3600) % self.custom_hours_per_day
        custom_minute = int((custom_total_seconds % 3600) // 60)
        custom_second = int(custom_total_seconds % 60)
        
        # 格式化自定义时间（只显示到秒）
        custom_time = f"{custom_hour:02d}:{custom_minute:02d}:{custom_second:02d}"
        
        # 计算时间膨胀倍率百分比
        dilation_percentage = self.time_dilation_rate * 100
        
        # 计算膨胀后一天的小时数（精确到两位小数）
        expanded_hours_per_day = 24.0 * self.time_dilation_rate
        
        # 计算加速后当天剩余的小时数（精确到两位小数）
        # 总自定义时间秒数 - 当前自定义时间秒数 = 剩余秒数
        total_custom_seconds_per_day = expanded_hours_per_day * 3600
        remaining_seconds = total_custom_seconds_per_day - custom_total_seconds % total_custom_seconds_per_day
        remaining_hours = remaining_seconds / 3600
        
        return standard_datetime, custom_time, chinese_date, lunar_info, dilation_percentage, expanded_hours_per_day, remaining_hours
    
    def run_live_clock(self) -> None:
        """
        运行实时更新的自定义时钟
        标准时间每秒刷新一次，自定义时间也以1秒为间隔递增
        同时显示标准日期时间和自定义时间，方便对比
        """
        print(f"=== 加速世界 | 时间膨胀倍率{self.time_dilation_rate}倍 | 一天{self.custom_hours_per_day}小时制实时时钟 ===")
        print("按 Ctrl+C 退出\n")
        
        last_standard_second = None
        last_custom_second = None
        
        try:
            while True:
                # 获取当前标准日期时间和自定义时间
                standard_datetime, custom_time, chinese_date, lunar_info, dilation_percentage, expanded_hours_per_day, remaining_hours = self.get_custom_time()
                
                # 提取标准时间的秒数
                current_standard_second = int(standard_datetime.split(':')[-1])
                # 提取自定义时间的秒数
                current_custom_second = int(custom_time.split(':')[-1])
                
                # 当标准时间或自定义时间的秒数变化时，更新显示
                if (last_standard_second != current_standard_second or 
                    last_custom_second != current_custom_second):
                    # 同时显示所有信息
                    output = f"\r标准时间：{standard_datetime} | 自定义时间：{custom_time}"
                    output += f" | 膨胀倍率：{dilation_percentage:.0f}% | 一天小时数：{expanded_hours_per_day:.2f}小时 | 当天剩余：{remaining_hours:.2f}小时"
                    sys.stdout.write(output)
                    sys.stdout.flush()
                    last_standard_second = current_standard_second
                    last_custom_second = current_custom_second
                
                # 使用短暂的休眠，平衡精度和CPU使用率
                time.sleep(0.01)  # 10毫秒休眠
        except KeyboardInterrupt:
            print("\n\n时钟已停止运行～")





# ------------------- 命令行界面 -------------------
def main_cli() -> None:
    """命令行主函数"""
    parser = argparse.ArgumentParser(description=f"加速世界 - 时间膨胀时钟工具 {VERSION}")
    parser.add_argument(
        "--rate", "-R", 
        type=float, 
        default=2.0, 
        help="时间膨胀倍率（必须大于1.0，默认2.0）"
    )
    
    args = parser.parse_args()
    
    try:
        # 初始化时间膨胀倍率
        accel_world = AcceleratedWorld(time_dilation_rate=args.rate)
        # 运行实时时钟
        accel_world.run_live_clock()
    except ValueError as e:
        print(f"错误：{e}")
        sys.exit(1)


# ------------------- 主程序入口 -------------------
if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        # 如果有命令行参数，运行命令行界面
        main_cli()
    else:
        # 运行图形界面
        from accelworld_gui import main_gui
        main_gui()
