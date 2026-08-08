# 日期面板模块（S4 GUI 面板化拆分，中文日期 + 农历信息）

from PyQt6.QtWidgets import QWidget, QVBoxLayout, QFrame, QLabel
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont

from modules.time_dilation import TimeInfo
from config.static.static_config import get_static_config
_UI = get_static_config().ui


class DatePanel(QWidget):
    """日期面板：显示中文日期与农历信息"""

    def __init__(self, parent: QWidget | None = None):
        """初始化日期与农历标签"""
        # 构建日期/农历显示 frame，初始为占位文案
        super().__init__(parent)

        date_frame = QFrame()
        date_frame.setFrameShape(QFrame.Shape.StyledPanel)
        date_layout = QVBoxLayout(date_frame)

        # 中文日期标签
        self.date_label = QLabel("2025年12月25日 星期四")
        self.date_label.setFont(QFont(_UI["font_family"], 14))
        self.date_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.date_label)

        # 农历信息标签
        self.lunar_info_label = QLabel("农历信息...")
        self.lunar_info_label.setFont(QFont(_UI["font_family"], 12))
        self.lunar_info_label.setStyleSheet("color: " + _UI["colors"]["text_muted"])
        self.lunar_info_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.lunar_info_label)

        outer = QVBoxLayout(self)
        outer.addWidget(date_frame)

    def update_time(self, info: TimeInfo) -> None:
        """刷新日期与农历显示"""
        # 由主窗口 tick 传入 TimeInfo，直接更新两个标签
        self.date_label.setText(info.chinese_date)
        self.lunar_info_label.setText(info.lunar_info)


# ===== ui/panels/date_panel.py 函数/类说明 =====
# DatePanel(QWidget): 日期显示面板
#   update_time(info): 由主窗口时钟 tick 调用，刷新中文日期与农历标签
#   设计理由：显示职责独立成面板，主窗口只做装配与调度
#   关联配置：数据来自 modules/time_dilation.py 的 TimeInfo


