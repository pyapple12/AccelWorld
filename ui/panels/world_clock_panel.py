# 世界时钟面板模块（S4 GUI 面板化拆分，时区下拉 + 世界时间）

import datetime
import logging

import pytz

from PyQt6.QtWidgets import QWidget, QHBoxLayout, QVBoxLayout, QFrame, QLabel, QComboBox
from PyQt6.QtGui import QFont

from data.timezones import TIMEZONES

# 配置日志
logger = logging.getLogger(__name__)


class WorldClockPanel(QWidget):
    """世界时钟面板：时区选择与对应时区时间显示"""

    def __init__(self, parent: QWidget | None = None):
        """初始化时区下拉框与世界时间标签"""
        # 用 data/timezones 常量填充下拉框，显示名+IANA 标识
        super().__init__(parent)

        world_clock_frame = QFrame()
        world_clock_frame.setFrameShape(QFrame.Shape.StyledPanel)
        world_clock_layout = QHBoxLayout(world_clock_frame)

        # 世界时钟标题
        world_clock_title = QLabel("世界时钟:")
        world_clock_title.setFont(QFont("Arial", 12))
        world_clock_layout.addWidget(world_clock_title)

        # 时区选择
        self.timezone_combo = QComboBox()
        self.timezone_combo.setFont(QFont("Arial", 11))
        self.timezone_combo.setFixedWidth(150)
        for name, tz in TIMEZONES:
            self.timezone_combo.addItem(name, tz)
        world_clock_layout.addWidget(self.timezone_combo)

        # 世界时钟显示
        self.world_clock_label = QLabel("00:00:00")
        self.world_clock_label.setFont(QFont("Arial", 14, QFont.Weight.Bold))
        self.world_clock_label.setStyleSheet("color: #2196F3;")
        world_clock_layout.addWidget(self.world_clock_label)

        world_clock_layout.addStretch()

        outer = QVBoxLayout(self)
        outer.addWidget(world_clock_frame)

    def update_world_clock(self) -> None:
        """刷新当前时区时间显示"""
        # pytz 按当前选择的时区换算本地时间，失败降级 00:00:00
        tz_name = self.timezone_combo.currentData()
        if not tz_name:
            return

        try:
            tz = pytz.timezone(tz_name)
            world_time = datetime.datetime.now(tz).strftime("%H:%M:%S")
            self.world_clock_label.setText(world_time)
        except Exception as e:
            logger.error(f"更新世界时钟时出错: {e}")
            self.world_clock_label.setText("00:00:00")

    def current_timezone(self) -> str:
        """获取当前选择的时区标识（供配置保存）"""
        # 下拉框 currentData 为空时回退上海时区
        return self.timezone_combo.currentData() or "Asia/Shanghai"


# ===== ui/panels/world_clock_panel.py 函数/类说明 =====
# WorldClockPanel(QWidget): 世界时钟面板
#   update_world_clock(): 由主窗口时钟 tick 调用，pytz 换算当前时区时间
#   current_timezone(): 供主窗口 save_settings 持久化时区选择
#   异常处理：pytz 转换失败降级显示 00:00:00 并记录日志
#   关联配置：时区表来自 data/timezones.py
