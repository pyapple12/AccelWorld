# 倒计时面板模块（S4 GUI 面板化拆分，倒计时输入 + 日期/时间选择器）
# PL002 Fluent 重写：LineEdit/ToolButton/PushButton + MessageBoxBase 选择器对话框
# （内嵌 qfw DatePicker/TimePicker 与快捷按钮）+ InfoBar 非模态提示；
# PL001（plan#UI2.0）：解析与剩余运算迁 ui/tools/countdown_tools（铁律 1），
# 样式参数经 AppInterface 读取（铁律 2），本文件零后端 import

import datetime

from PyQt6.QtCore import QDate, QTime, Qt
from PyQt6.QtGui import QColor, QFont, QPalette
from PyQt6.QtWidgets import QHBoxLayout, QVBoxLayout, QLabel, QWidget

from qfluentwidgets import (
    BodyLabel,
    DatePicker,
    FluentIcon,
    InfoBar,
    InfoBarPosition,
    LineEdit,
    MessageBoxBase,
    PushButton,
    SubtitleLabel,
    TimePicker,
    ToolButton,
)

from interface import AppInterface
from ui.tools.countdown_tools import format_remaining, parse_target_text

# 快捷日期按钮组（文案, 相对天数）——选择器对话框与即时写回共用
_QUICK_DATE_CHOICES = (("今天", 0), ("明天", 1), ("一周后", 7))


class _DatePickDialog(MessageBoxBase):
    def __init__(self, parent: QWidget, on_quick_picked):
        # 日期选择对话框：qfw DatePicker + 快捷按钮（点击即写回输入框，不等确定，T001.2）
        super().__init__(parent)
        self.titleLabel = SubtitleLabel("选择日期", self)
        self.viewLayout.addWidget(self.titleLabel)

        self.calendar = DatePicker(parent=self)
        self.calendar.setDate(QDate.currentDate())
        self.viewLayout.addWidget(self.calendar)

        quick_row = QHBoxLayout()
        for label, days in _QUICK_DATE_CHOICES:
            quick_btn = PushButton(label)
            quick_btn.clicked.connect(
                lambda checked=False, d=days: on_quick_picked(d)
            )
            quick_row.addWidget(quick_btn)
        quick_row.addStretch()
        quick_host = QWidget(self)
        quick_host.setLayout(quick_row)
        self.viewLayout.addWidget(quick_host)

        self.yesButton.setText("确定")
        self.cancelButton.setText("取消")
        self.yesButton.clicked.connect(self.accept)
        self.cancelButton.clicked.connect(self.reject)


class _TimePickDialog(MessageBoxBase):
    def __init__(self, parent: QWidget, current_time: QTime):
        # 时间选择对话框：qfw TimePicker（HH:mm）+ "现在"快捷按钮
        super().__init__(parent)
        self.titleLabel = SubtitleLabel("选择时间", self)
        self.viewLayout.addWidget(self.titleLabel)

        self.time_picker = TimePicker(parent=self)
        self.time_picker.setTime(current_time)
        self.viewLayout.addWidget(self.time_picker)

        now_row = QHBoxLayout()
        now_btn = PushButton("现在")
        now_btn.clicked.connect(
            lambda checked=False: self.time_picker.setTime(QTime.currentTime())
        )
        now_row.addWidget(now_btn)
        now_row.addStretch()
        now_host = QWidget(self)
        now_host.setLayout(now_row)
        self.viewLayout.addWidget(now_host)

        self.yesButton.setText("确定")
        self.cancelButton.setText("取消")
        self.yesButton.clicked.connect(self.accept)
        self.cancelButton.clicked.connect(self.reject)


class CountdownPanel(QWidget):
    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 目标时间内部态初始为 None；选择器只改写输入框文本；提示时长经接口读取
        super().__init__(parent)
        self._interface = interface
        self._layout = interface.get_ui_static()["layout"]
        self._notify_ms = int(interface.get_app_static()["notification_duration_ms"])
        self._date_dialog: _DatePickDialog | None = None  # 当前打开的日期选择器

        self.countdown_target_date: datetime.datetime | None = None  # 倒计时目标时间

        countdown_layout = QHBoxLayout(self)
        countdown_layout.setContentsMargins(*self._layout["panel_margin"])

        countdown_title_label = BodyLabel("倒计时:")
        countdown_layout.addWidget(countdown_title_label)

        # 目标时间输入框和选择器（水平排列）
        countdown_input_layout = QHBoxLayout()
        countdown_input_layout.setSpacing(int(self._layout["input_spacing"]))

        self.countdown_target = LineEdit()
        self.countdown_target.setPlaceholderText("YYYY-MM-DD HH:MM:SS")
        self.countdown_target.setFixedWidth(int(self._layout["countdown_input_width"]))
        countdown_input_layout.addWidget(self.countdown_target)

        # 日期/时间选择器按钮（qfw ToolButton + Fluent 图标，PL002.06）
        self.date_picker_button = ToolButton(FluentIcon.CALENDAR)
        self.date_picker_button.setFixedSize(*self._layout["picker_button_size"])
        self.date_picker_button.setToolTip("选择日期")
        self.date_picker_button.clicked.connect(self.show_date_picker)
        countdown_input_layout.addWidget(self.date_picker_button)

        self.time_picker_button = ToolButton(FluentIcon.DATE_TIME)
        self.time_picker_button.setFixedSize(*self._layout["picker_button_size"])
        self.time_picker_button.setToolTip("选择时间")
        self.time_picker_button.clicked.connect(self.show_time_picker)
        countdown_input_layout.addWidget(self.time_picker_button)

        countdown_layout.addLayout(countdown_input_layout)

        # 倒计时显示（着色经 QPalette：结束红/进行中主题色，QSS 管线已退役 PL002.09）
        self.countdown_label = QLabel("--天 --:--:--:--")
        self.countdown_label.setFont(
            QFont(
                interface.get_ui_static()["font_family_digits"],
                int(self._layout["countdown_font_size"]),
                QFont.Weight.Bold,
            )
        )
        self._set_countdown_color("primary")
        countdown_layout.addWidget(self.countdown_label)

        countdown_layout.addStretch()

        # 设置/清除按钮
        self.set_countdown_button = PushButton("设置")
        self.set_countdown_button.clicked.connect(self.set_countdown)
        countdown_layout.addWidget(self.set_countdown_button)

        self.clear_countdown_button = PushButton("清除")
        self.clear_countdown_button.clicked.connect(self.clear_countdown)
        countdown_layout.addWidget(self.clear_countdown_button)

    def _set_countdown_color(self, color_key: str) -> None:
        # 经 QPalette 上色（ui.json colors 键经接口读取；不用 QSS，PL002.09）
        color = QColor(self._interface.get_ui_static()["colors"][color_key])
        palette = self.countdown_label.palette()
        palette.setColor(QPalette.ColorRole.WindowText, color)
        self.countdown_label.setPalette(palette)

    def _show_warning(self, title: str, content: str) -> None:
        # 非模态警告提示条（原模态警告弹窗，PL002.06 Fluent 化）
        InfoBar.warning(
            title=title,
            content=content,
            orient=Qt.Orientation.Vertical,
            isClosable=True,
            position=InfoBarPosition.TOP,
            duration=self._notify_ms,
            parent=self,
        )

    def restore_target(self, text: str) -> None:
        # 启动恢复：回填输入框并解析内部目标态（FIX001.10）。
        # 此前仅回填文本、countdown_target_date 保持 None，get_target_text 返回空串，
        # 下一次正常退出即把持久化值清空（跨会话数据丢失）；解析失败时内部态为 None，
        # 恢复"输入即存、点设置才倒计时"语义不变
        self.countdown_target.setText(text)
        self.countdown_target_date = parse_target_text(text.strip())

    def set_countdown(self) -> None:
        # 解析目标文本并校验过期；过期目标拒绝并置 None
        target_text = self.countdown_target.text().strip()
        if not target_text:
            self._show_warning("警告", "请输入目标时间")
            return

        parsed = parse_target_text(target_text)
        if parsed is None:
            self._show_warning("错误", "时间格式不正确，请使用 YYYY-MM-DD HH:MM:SS 格式")
            return
        self.countdown_target_date = parsed

        # 检查时间是否已过期
        if self.countdown_target_date <= datetime.datetime.now():
            self._show_warning("警告", "目标时间已过期，请选择未来时间")
            self.countdown_target_date = None
            return

        self.update_countdown()

    def clear_countdown(self) -> None:
        # 目标置 None、标签复位、输入框清空
        self.countdown_target_date = None
        self.countdown_label.setText("--天 --:--:--:--")
        self.countdown_target.clear()

    def update_countdown(self) -> None:
        # 剩余拆解/结束判定在 ui/tools（PL001.06 迁入），此处只做 setText 与着色：
        # 结束红（danger）、进行中主题色（primary）
        if not self.countdown_target_date:
            return

        text, finished = format_remaining(
            self.countdown_target_date, datetime.datetime.now()
        )
        color_key = "danger" if finished else "primary"
        self.countdown_label.setText(text)
        self._set_countdown_color(color_key)

    def get_target_text(self) -> str:
        # 仅在目标已设置时返回输入框文本
        if self.countdown_target_date:
            return self.countdown_target.text().strip()
        return ""

    def _set_target_date_part(self, selected_date: QDate) -> None:
        # 将选中日期写回输入框日期部分，保留原时间部分（实时反馈不等 OK，修复 T001.2）
        current_text = self.countdown_target.text().strip()
        if current_text and len(current_text) >= 10:
            time_part = current_text[10:] if len(current_text) > 10 else " 00:00:00"
            self.countdown_target.setText(
                f"{selected_date.toString('yyyy-MM-dd')}{time_part}"
            )
        else:
            self.countdown_target.setText(
                f"{selected_date.toString('yyyy-MM-dd')} 00:00:00"
            )

    def show_date_picker(self) -> None:
        # 弹窗选日期：快捷按钮勾选日历并即时写回输入框，确定按当前选中日期定稿
        # （快捷与日历状态保持一致，避免确定键覆盖快捷写入）；exec 后 deleteLater（FIX001.23）
        self._date_dialog = _DatePickDialog(self, on_quick_picked=self._apply_quick_date)
        dialog = self._date_dialog
        try:
            if dialog.exec():
                self._set_target_date_part(dialog.calendar.getDate())
        finally:
            self._date_dialog = None
            dialog.deleteLater()

    def _apply_quick_date(self, days: int) -> None:
        # 快捷日期按钮：弹窗内日历勾选对应日期 + 输入框即时写回（实时反馈，T001.2）
        target = QDate.currentDate().addDays(days)
        if self._date_dialog is not None:
            self._date_dialog.calendar.setDate(target)
        self._set_target_date_part(target)

    def show_time_picker(self) -> None:
        # 弹窗选时间，日期取输入框或今天；exec 后 deleteLater 释放（FIX001.23）
        current_text = self.countdown_target.text().strip()
        if current_text and len(current_text) >= 10:
            date_str = current_text[:10]
        else:
            date_str = QDate.currentDate().toString("yyyy-MM-dd")

        # 解析当前时间（如果有）：fromString 不抛异常，无效值回退当前时间（E10 去无意义 try）
        current_time = QTime.currentTime()
        if current_text and len(current_text) >= 16:
            parsed = QTime.fromString(current_text[11:16], "HH:mm")
            if parsed.isValid():
                current_time = parsed

        dialog = _TimePickDialog(self, current_time)
        try:
            if dialog.exec():
                selected_time = dialog.time_picker.getTime()
                self.countdown_target.setText(
                    f"{date_str} {selected_time.toString('HH:mm:ss')}"
                )
        finally:
            dialog.deleteLater()


# ===== ui/panels/countdown_panel.py 函数/类说明 =====
# CountdownPanel(QWidget): 倒计时面板
#   __init__(interface, parent): qfw LineEdit/ToolButton/PushButton；提示时长经接口
#   _DatePickDialog(MessageBoxBase): qfw DatePicker + 今天/明天/一周后快捷按钮
#   _TimePickDialog(MessageBoxBase): qfw TimePicker + "现在"按钮
#   set_countdown(): 解析（经 tools.parse_target_text）并校验过期，成功后刷新显示；
#     空输入/过期经 InfoBar 非模态提示（原模态警告弹窗）
#   clear_countdown(): 清除目标与显示
#   update_countdown(): 主窗口 tick 调用；剩余拆解/结束态经 tools.format_remaining，
#     本方法只做 setText 与 QPalette 着色（结束 danger/进行 primary，PL002.09 去 QSS）
#   get_target_text(): 供主窗口保存配置；未设置返回空串
#   restore_target(text): 启动恢复（回填输入框并经 tools 解析内部态，FIX001.10）
#   _set_target_date_part(selected_date): 选中日期写回输入框日期部分并保留时间部分
#     （日历选择/快捷按钮/确定共用此路径）
#   _apply_quick_date(days): 快捷日期按钮即时写回（不再依赖弹窗内日历勾选，
#     PL002.06 起 DatePicker 与写回解耦，语义保持"点击即写入"）
#   show_date_picker()/show_time_picker(): MessageBoxBase 对话框选择，仅改写输入框
#     对应部分；exec 后 deleteLater 释放（FIX001.23）
#   _set_countdown_color(color_key)/_show_warning(title, content): 着色与提示辅助
#   设计理由：倒计时状态（目标时间）内聚在面板，运算在 ui/tools，主窗口只做 tick 驱动
#   异常处理：格式解析（tools 层返回 None）InfoBar 提示；过期目标置 None
#   关联配置：countdown_target 配置项由主窗口经 get_target_text 持久化；提示时长经接口
