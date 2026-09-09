# 时间膨胀核心模块
# 提供 AcceleratedWorld 类与 CLI 实时钟入口

import datetime
import logging
import time
import sys
from dataclasses import dataclass

# 配置日志
logger = logging.getLogger(__name__)

# 导入日期处理模块
from modules.chinese_calendar import get_chinese_date, get_lunar_info

# 静态配置（默认倍率）
from config.static.static_config import get_static_config


@dataclass
class TimeInfo:
    standard_datetime: str  # 标准日期时间字符串
    custom_time: str  # 自定义时间字符串
    chinese_date: str  # 中文日期字符串（YYYY年MM月DD日 星期X）
    lunar_info: str  # 农历信息字符串
    dilation_percentage: float  # 时间膨胀倍率百分比
    expanded_hours_per_day: float  # 膨胀后一天的小时数
    remaining_hours: float  # 加速后当天剩余的小时数

    @property
    def standard_time(self) -> str:
        # 标准时间的 HH:MM:SS 部分（显示用，避免调用点重复 split）
        return self.standard_datetime.split()[1]

    @property
    def custom_hour(self) -> int:
        # 自定义时间小时数（进度条用）
        return int(self.custom_time.split(":")[0])

    @property
    def custom_second(self) -> int:
        # 自定义时间秒数（秒变化检测用）
        return int(self.custom_time.split(":")[-1])


class AcceleratedWorld:
    time_dilation_rate: float
    """时间膨胀倍率（下限来自静态配置 rate_min，默认 default_rate）"""

    custom_hours_per_day: int
    """基于膨胀率计算的一天总小时数"""

    def __init__(self, time_dilation_rate: float | None = None):
        # None 哨兵避免默认参数在定义时求值硬编码；下限读 base.rate_min（消除与配置的 1.0 边界矛盾）
        base = get_static_config().base
        if time_dilation_rate is None:
            time_dilation_rate = float(base["default_rate"])
        rate_min = float(base["rate_min"])
        if time_dilation_rate < rate_min:
            raise ValueError(f"时间膨胀倍率必须大于或等于{rate_min}！")
        self.time_dilation_rate = time_dilation_rate
        self.custom_hours_per_day = int(
            24 * time_dilation_rate
        )  # 计算一天的自定义小时数
        self._lunar_cache: tuple[tuple[int, ...], str, str] | None = (
            None  # (标准秒键, 中文日期, 农历信息) 昂贵农历计算的标准秒级缓存
        )

    @property
    def tick_interval_ms(self) -> int:
        # 刷新周期（毫秒）：取基础 tick 周期与加速秒周期（1000/倍率）的较小值，
        # 保证加速秒边界不被 tick 周期错过（修复 T001.1：刷新频率随倍率变化）
        base_tick_ms = int(get_static_config().base["clock_tick_ms"])
        accelerated_second_ms = 1000.0 / self.time_dilation_rate
        return max(1, min(base_tick_ms, int(accelerated_second_ms)))

    def get_custom_time(self) -> TimeInfo:
        # 标准时间与加速时间每次调用现算（廉价算术，加速时间随倍率节奏实时推进，修复 T001.1）；
        # 昂贵的农历/中文日期计算按标准秒缓存，同标准秒内直接复用
        # 获取当前系统时间（带毫秒精度）
        now = datetime.datetime.now()
        second_key = (now.year, now.month, now.day, now.hour, now.minute, now.second)
        if self._lunar_cache is not None and self._lunar_cache[0] == second_key:
            chinese_date = self._lunar_cache[1]
            lunar_info = self._lunar_cache[2]
        else:
            # 使用日期模块获取中文日期与农历信息
            chinese_date = get_chinese_date(now)
            lunar_info = get_lunar_info(now)
            self._lunar_cache = (second_key, chinese_date, lunar_info)

        # 格式化标准日期时间（只显示到秒，每次现算保证标准时间新鲜不回读缓存）
        standard_datetime = now.strftime("%Y-%m-%d %H:%M:%S")

        # 计算当前时刻在标准一天中的总秒数（毫秒级精度）
        current_hour = now.hour
        current_minute = now.minute
        current_second = now.second
        current_microsecond = now.microsecond

        # 计算总秒数，包含毫秒精度
        total_seconds = (
            current_hour * 3600
            + current_minute * 60
            + current_second
            + current_microsecond / 1e6
        )

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
        # 总自定义时间秒数 - 当前自定义时间秒数 = 剩余秒数（rate≤20 时当前值恒小于一天总量，无需取模）
        total_custom_seconds_per_day = expanded_hours_per_day * 3600
        remaining_seconds = total_custom_seconds_per_day - custom_total_seconds
        remaining_hours = remaining_seconds / 3600

        return TimeInfo(
            standard_datetime=standard_datetime,
            custom_time=custom_time,
            chinese_date=chinese_date,
            lunar_info=lunar_info,
            dilation_percentage=dilation_percentage,
            expanded_hours_per_day=expanded_hours_per_day,
            remaining_hours=remaining_hours,
        )

    def run_live_clock(self) -> None:
        # 标准时间或加速时间的显示内容变化时整行覆写输出（加速时间随倍率节奏刷新，修复 T001.1）
        print(
            f"=== 加速世界 | 时间膨胀倍率{self.time_dilation_rate}倍 | "
            f"一天{self.custom_hours_per_day}小时制实时时钟 ==="
        )
        print("按 Ctrl+C 退出\n")

        last_output_key: tuple[str, str] | None = None

        try:
            while True:
                try:
                    # 每轮现算 TimeInfo（农历已被标准秒缓存兜住开销），
                    # 显示内容（标准时间/加速时间字符串）变化才重绘
                    info = self.get_custom_time()
                    output_key = (info.standard_datetime, info.custom_time)
                    if output_key != last_output_key:
                        # 同时显示所有信息
                        output = f"\r标准时间：{info.standard_datetime} | 自定义时间：{info.custom_time}"
                        output += (
                            f" | 膨胀倍率：{info.dilation_percentage:.0f}% | "
                            f"一天小时数：{info.expanded_hours_per_day:.2f}小时 | "
                            f"当天剩余：{info.remaining_hours:.2f}小时"
                        )
                        sys.stdout.write(output)
                        sys.stdout.flush()
                        last_output_key = output_key
                except Exception as e:
                    # 单轮异常（如农历库异常）记录后继续，避免 CLI 崩溃退出
                    logger.exception(f"实时时钟单轮刷新异常: {e}")
                    time.sleep(1.0)

                # 使用短暂的休眠，平衡精度和CPU使用率
                time.sleep(0.01)  # 10毫秒休眠
        except KeyboardInterrupt:
            print("\n\n时钟已停止运行～")


# ------------------- 命令行界面 -------------------
def main_cli(rate: float | None = None) -> None:
    # 倍率默认值来自静态配置，下限校验非法则退出
    if rate is None:
        rate = float(get_static_config().base["default_rate"])
    rate_min = float(get_static_config().base["rate_min"])
    if rate < rate_min:
        print(f"错误: --rate 参数必须大于或等于 {rate_min}")
        print("例如: python main.py --cli --rate 2.0")
        sys.exit(1)

    try:
        # 初始化时间膨胀倍率
        accel_world = AcceleratedWorld(time_dilation_rate=rate)
        # 运行实时时钟
        accel_world.run_live_clock()
    except ValueError as e:
        print(f"错误：{e}")
        sys.exit(1)


# ------------------- 主程序入口 -------------------
if __name__ == "__main__":
    main_cli()


# ===== modules/time_dilation.py 函数/类说明 =====
# TimeInfo: dataclass，时间信息聚合（S2 引入，替代 7 元组返回）
#   standard_datetime/custom_time/chinese_date/lunar_info/dilation_percentage/
#   expanded_hours_per_day/remaining_hours
# AcceleratedWorld: 时间膨胀核心类
#   __init__(rate=None): 默认值与下限校验来自静态配置（default_rate/rate_min，None 哨兵零硬编码），
#     下限为 rate_min（含边界，修复 S10.1 A1 的 1.0 矛盾），计算一天自定义小时数（int(24*rate)）
#   tick_interval_ms 属性: 刷新周期 = min(基础 tick 周期, 1000/倍率)，保证加速秒边界不被错过
#     （T001.1 新增，GUI 定时器与 CLI 轮询节奏随倍率联动）
#   get_custom_time() -> TimeInfo: 标准时间/加速时间每次现算（加速时间随倍率节奏实时推进，
#     修复 T001.1 标准秒缓存导致的固定 1 秒刷新）；农历/中文日期按标准秒缓存复用，
#     同标准秒仅全量计算一次（保留 S9.3 的性能收益）
#   run_live_clock(): CLI 实时钟，标准/加速显示内容变化时覆写输出，KeyboardInterrupt 优雅退出
# main_cli(rate): CLI 入口（倍率直接传参，修复 D1），校验后启动实时钟
#   设计理由：纯计算无 GUI 依赖，CLI/GUI 共用；整数运算避免浮点进位误差
#   异常处理：rate < rate_min 抛 ValueError；运行期 KeyboardInterrupt 捕获退出
#   关联配置：农历数据来自 modules/chinese_calendar.py；倍率参数来自 config/static/base.json
