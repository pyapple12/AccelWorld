# 日期面板模块（S4 GUI 面板化拆分，中文日期 + 农历信息）
# PL002 Fluent 重写：SubtitleLabel/CaptionLabel 自带主题字体与深浅色适配；
# PL001（plan#UI2.0）：纯展示化——样式经 AppInterface 读取，类型走 interface.types

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import QWidget, QVBoxLayout

from qfluentwidgets import CaptionLabel, SubtitleLabel

from interface import AppInterface
from interface.types import TimeInfo


class DatePanel(QWidget):
    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 构建日期/农历显示行，初始为占位文案（qfw 标签组件自带深浅色适配）
        super().__init__(parent)

        date_layout = QVBoxLayout(self)
        date_layout.setContentsMargins(4, 2, 4, 2)

        # 中文日期标签（中性占位，首帧 tick 后刷新为真实日期，F4）
        self.date_label = SubtitleLabel("----年--月--日")
        self.date_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.date_label)

        # 农历信息标签（CaptionLabel 自带弱化色，随主题深浅切换）
        self.lunar_info_label = CaptionLabel("农历信息...")
        self.lunar_info_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.lunar_info_label)

    def update_time(self, info: TimeInfo) -> None:
        # 由主窗口 tick 传入 TimeInfo，直接更新两个标签（纯 setText，零运算）
        self.date_label.setText(info.chinese_date)
        self.lunar_info_label.setText(info.lunar_info)


# ===== ui/panels/date_panel.py 函数/类说明 =====
# DatePanel(QWidget): 日期显示面板
#   __init__(interface, parent): PL002 Fluent 重写——SubtitleLabel（日期）+
#   CaptionLabel（农历弱化色随主题），不再手工上色/设字体
#   update_time(info): 由主窗口时钟 tick 调用，刷新中文日期与农历标签
#   设计理由：显示职责独立成面板，主窗口只做装配与调度；类型引用走 interface.types
#   （plan#UI2.0 铁律 2：零后端 import）
#   关联配置：无直接配置依赖（标签样式由 qfw 主题体系承担）
