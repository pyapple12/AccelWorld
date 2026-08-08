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
        self._time_cache: tuple[tuple[int, ...], TimeInfo] | None = (
            None  # (标准秒键, TimeInfo) 秒级缓存
        )

    def get_custom_time(self) -> TimeInfo:
        # 同秒内直接返回缓存，避免 GUI 10Hz tick 重复农历全量计算（S9.3）
        # 基于当前时刻秒数 × 倍率得到自定义秒数，再拆分时分秒
        # 获取当前系统时间（带毫秒精度）
        now = datetime.datetime.now()
        cache_key = (now.year, now.month, now.day, now.hour, now.minute, now.second)
        if self._time_cache is not None and self._time_cache[0] == cache_key:
            return self._time_cache[1]

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

        info = TimeInfo(
            standard_datetime=standard_datetime,
            custom_time=custom_time,
            chinese_date=chinese_date,
            lunar_info=lunar_info,
            dilation_percentage=dilation_percentage,
            expanded_hours_per_day=expanded_hours_per_day,
            remaining_hours=remaining_hours,
        )
        self._time_cache = (cache_key, info)
        return info

    def run_live_clock(self) -> None:
        # 秒数变化时整行覆写输出，10ms 轮询平衡精度与 CPU
        print(
            f"=== 加速世界 | 时间膨胀倍率{self.time_dilation_rate}倍 | "
            f"一天{self.custom_hours_per_day}小时制实时时钟 ==="
        )
        print("按 Ctrl+C 退出\n")

        last_standard_second = None
        last_custom_second = None

        try:
            while True:
                try:
                    # 秒变化检测前置：标准秒未变不调 get_custom_time（配合秒级缓存，每秒仅 1 次全量计算）
                    now = datetime.datetime.now()
                    current_standard_second = now.second
                    if current_standard_second == last_standard_second:
                        time.sleep(0.01)
                        continue

                    # 获取当前标准日期时间和自定义时间
                    info = self.get_custom_time()

                    # 提取自定义时间的秒数（经 TimeInfo 计算属性）
                    current_custom_second = info.custom_second

                    # 当标准时间或自定义时间的秒数变化时，更新显示
                    if (
                        last_standard_second != current_standard_second
                        or last_custom_second != current_custom_second
                    ):
                        # 同时显示所有信息
                        output = f"\r标准时间：{info.standard_datetime} | 自定义时间：{info.custom_time}"
                        output += (
                            f" | 膨胀倍率：{info.dilation_percentage:.0f}% | "
                            f"一天小时数：{info.expanded_hours_per_day:.2f}小时 | "
                            f"当天剩余：{info.remaining_hours:.2f}小时"
                        )
                        sys.stdout.write(output)
                        sys.stdout.flush()
                        last_standard_second = current_standard_second
                        last_custom_second = current_custom_second
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
#   get_custom_time() -> TimeInfo: 当前秒数×倍率 → 时分秒；含农历/中文日期/剩余小时
#   run_live_clock(): CLI 实时钟，秒变化时覆写输出，KeyboardInterrupt 优雅退出
# main_cli(rate): CLI 入口（倍率直接传参，修复 D1），校验后启动实时钟
#   设计理由：纯计算无 GUI 依赖，CLI/GUI 共用；整数运算避免浮点进位误差
#   异常处理：rate < rate_min 抛 ValueError；运行期 KeyboardInterrupt 捕获退出
#   关联配置：农历数据来自 modules/chinese_calendar.py；倍率参数来自 config/static/base.json
