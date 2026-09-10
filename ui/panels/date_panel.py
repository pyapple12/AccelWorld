# 日期面板模块（S4 GUI 面板化拆分，中文日期 + 农历信息）
# PL001（plan#UI2.0）：纯展示化——样式经 AppInterface 读取，类型走 interface.types

from PyQt6.QtWidgets import QWidget, QVBoxLayout, QFrame, QLabel
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont

from interface import AppInterface
from interface.types import TimeInfo


class DatePanel(QWidget):
    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 构建日期/农历显示 frame，初始为占位文案（字体/颜色经接口读取）
        super().__init__(parent)

        ui = interface.get_ui_static()

        date_frame = QFrame()
        date_frame.setFrameShape(QFrame.Shape.StyledPanel)
        date_layout = QVBoxLayout(date_frame)

        # 中文日期标签（中性占位，首帧 tick 后刷新为真实日期，F4）
        self.date_label = QLabel("----年--月--日")
        self.date_label.setFont(QFont(ui["font_family"], 14))
        self.date_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.date_label)

        # 农历信息标签
        self.lunar_info_label = QLabel("农历信息...")
        self.lunar_info_label.setFont(QFont(ui["font_family"], 12))
        self.lunar_info_label.setStyleSheet("color: " + ui["colors"]["text_muted"])
        self.lunar_info_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.lunar_info_label)

        outer = QVBoxLayout(self)
        outer.addWidget(date_frame)

    def update_time(self, info: TimeInfo) -> None:
        # 由主窗口 tick 传入 TimeInfo，直接更新两个标签（纯 setText，零运算）
        self.date_label.setText(info.chinese_date)
        self.lunar_info_label.setText(info.lunar_info)


# ===== ui/panels/date_panel.py 函数/类说明 =====
# DatePanel(QWidget): 日期显示面板
#   __init__(interface, parent): 字体/颜色经 AppInterface.get_ui_static() 读取
#   update_time(info): 由主窗口时钟 tick 调用，刷新中文日期与农历标签
#   设计理由：显示职责独立成面板，主窗口只做装配与调度；类型引用走 interface.types
#   （plan#UI2.0 铁律 2：零后端 import）
#   关联配置：字体经 AppInterface（ui.json）
