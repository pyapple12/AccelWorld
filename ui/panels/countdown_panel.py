# 倒计时面板模块（S4 GUI 面板化拆分，倒计时输入 + 日期/时间选择器）
# PL001（plan#UI2.0）：解析与剩余运算迁 ui/tools/countdown_tools（铁律 1），
# 样式参数经 AppInterface 读取（铁律 2），本文件零后端 import

import datetime

from PyQt6.QtWidgets import (
    QWidget,
    QHBoxLayout,
    QVBoxLayout,
    QFrame,
    QLabel,
    QLineEdit,
    QPushButton,
    QMessageBox,
    QDialog,
    QDialogButtonBox,
    QCalendarWidget,
    QTimeEdit,
)
from PyQt6.QtCore import QDate, QTime
from PyQt6.QtGui import QFont

from interface import AppInterface
from ui.tools.countdown_tools import format_remaining, parse_target_text


class CountdownPanel(QWidget):
    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 目标时间内部态初始为 None；选择器只改写输入框文本；样式经接口读取
        super().__init__(parent)
        self._ui = interface.get_ui_static()

        self.countdown_target_date: datetime.datetime | None = None  # 倒计时目标时间

        countdown_frame = QFrame()
        countdown_frame.setFrameShape(QFrame.Shape.StyledPanel)
        countdown_layout = QHBoxLayout(countdown_frame)

        countdown_title_label = QLabel("倒计时:")
        countdown_title_label.setFont(QFont(self._ui["font_family"], 12))
        countdown_layout.addWidget(countdown_title_label)

        # 目标时间输入框和选择器（水平排列）
        countdown_input_layout = QHBoxLayout()
        countdown_input_layout.setSpacing(5)

        self.countdown_target = QLineEdit()
        self.countdown_target.setPlaceholderText("YYYY-MM-DD HH:MM:SS")
        self.countdown_target.setFont(QFont(self._ui["font_family"], 9))
        self.countdown_target.setFixedWidth(200)
        countdown_input_layout.addWidget(self.countdown_target)

        # 日期选择器按钮
        self.date_picker_button = QPushButton("📅")
        self.date_picker_button.setFixedSize(32, 32)
        self.date_picker_button.setToolTip("选择日期")
        self.date_picker_button.setFont(QFont(self._ui["font_family"], 12))
        self.date_picker_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.date_picker_button.clicked.connect(self.show_date_picker)
        countdown_input_layout.addWidget(self.date_picker_button)

        # 时间选择器按钮
        self.time_picker_button = QPushButton("🕐")
        self.time_picker_button.setFixedSize(32, 32)
        self.time_picker_button.setToolTip("选择时间")
        self.time_picker_button.setFont(QFont(self._ui["font_family"], 12))
        self.time_picker_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.time_picker_button.clicked.connect(self.show_time_picker)
        countdown_input_layout.addWidget(self.time_picker_button)

        countdown_layout.addLayout(countdown_input_layout)

        # 倒计时显示
        self.countdown_label = QLabel("--天 --:--:--:--")
        self.countdown_label.setFont(QFont(self._ui["font_family"], 14, QFont.Weight.Bold))
        self.countdown_label.setStyleSheet(
            "color: " + self._ui["colors"]["primary"] + ";"
        )
        countdown_layout.addWidget(self.countdown_label)

        countdown_layout.addStretch()

        # 设置/清除按钮
        self.set_countdown_button = QPushButton("设置")
        self.set_countdown_button.setFont(QFont(self._ui["font_family"], 10))
        self.set_countdown_button.clicked.connect(self.set_countdown)
        countdown_layout.addWidget(self.set_countdown_button)

        self.clear_countdown_button = QPushButton("清除")
        self.clear_countdown_button.setFont(QFont(self._ui["font_family"], 10))
        self.clear_countdown_button.clicked.connect(self.clear_countdown)
        countdown_layout.addWidget(self.clear_countdown_button)

        outer = QVBoxLayout(self)
        outer.addWidget(countdown_frame)

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
            QMessageBox.warning(self, "警告", "请输入目标时间")
            return

        parsed = parse_target_text(target_text)
        if parsed is None:
            QMessageBox.critical(
                self,
                "错误",
                "时间格式不正确，请使用 YYYY-MM-DD HH:MM:SS 格式",
            )
            return
        self.countdown_target_date = parsed

        # 检查时间是否已过期
        if self.countdown_target_date <= datetime.datetime.now():
            QMessageBox.warning(self, "警告", "目标时间已过期，请选择未来时间")
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
        # 结束红色、进行中主题色
        if not self.countdown_target_date:
            return

        text, finished = format_remaining(
            self.countdown_target_date, datetime.datetime.now()
        )
        color_key = "danger" if finished else "primary"
        self.countdown_label.setText(text)
        self.countdown_label.setStyleSheet(
            "color: " + self._ui["colors"][color_key] + ";"
        )

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

    def _apply_quick_date(self, calendar: QCalendarWidget, days: int) -> None:
        # 快捷日期按钮：日历勾选对应日期 + 输入框即时写回（实时反馈，修复 T001.2）
        target = QDate.currentDate().addDays(days)
        calendar.setSelectedDate(target)
        self._set_target_date_part(target)

    def _build_date_dialog(self) -> tuple[QDialog, QCalendarWidget]:
        # 构建日期选择弹窗（含日历点击/快捷按钮的实时反馈接线，修复 T001.2）
        dialog = QDialog(self)
        dialog.setWindowTitle("选择日期")
        dialog.setFixedSize(320, 340)

        layout = QVBoxLayout(dialog)
        layout.setSpacing(5)

        calendar = QCalendarWidget()
        calendar.setGridVisible(True)
        calendar.setSelectedDate(QDate.currentDate())
        calendar.setFixedSize(310, 250)
        layout.addWidget(calendar)

        # 日历点击实时反馈：点击日期立即写回输入框（不等 OK）
        calendar.clicked.connect(self._set_target_date_part)

        # 预设快捷按钮（勾选日历 + 即时写回）
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(5)

        for label, days in (("今天", 0), ("明天", 1), ("一周后", 7)):
            quick_btn = QPushButton(label)
            quick_btn.setFont(QFont(self._ui["font_family"], 10))
            quick_btn.clicked.connect(
                lambda _=False, c=calendar, d=days: self._apply_quick_date(c, d)
            )
            btn_layout.addWidget(quick_btn)

        layout.addLayout(btn_layout)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.setFont(QFont(self._ui["font_family"], 10))
        buttons.accepted.connect(dialog.accept)
        buttons.rejected.connect(dialog.reject)
        layout.addWidget(buttons)

        return dialog, calendar

    def show_date_picker(self) -> None:
        # 弹窗选日期：日历点击/快捷按钮即时写回输入框，OK 按当前选中日期定稿；
        # exec 后 deleteLater 释放子对话框（FIX001.23：重复打开不再累积存活对象）
        dialog, calendar = self._build_date_dialog()
        try:
            if dialog.exec() == QDialog.DialogCode.Accepted:
                self._set_target_date_part(calendar.selectedDate())
        finally:
            dialog.deleteLater()

    def _build_time_dialog(self, current_time: QTime) -> tuple[QDialog, QTimeEdit]:
        # 构建时间选择弹窗（尺寸交由 Qt sizeHint 自适应，修复 HH:mm:ss 在 120px 固定宽度内显示不全）
        dialog = QDialog(self)
        dialog.setWindowTitle("选择时间")

        layout = QVBoxLayout(dialog)
        layout.setSpacing(10)

        time_edit = QTimeEdit()
        time_edit.setDisplayFormat("HH:mm:ss")
        time_edit.setTime(current_time)
        time_edit.setFont(QFont(self._ui["font_family"], 14))
        layout.addWidget(time_edit)

        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(5)

        now_btn = QPushButton("现在")
        now_btn.setFont(QFont(self._ui["font_family"], 10))
        now_btn.clicked.connect(lambda: time_edit.setTime(QTime.currentTime()))
        btn_layout.addWidget(now_btn)

        layout.addLayout(btn_layout)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.setFont(QFont(self._ui["font_family"], 10))
        buttons.accepted.connect(dialog.accept)
        buttons.rejected.connect(dialog.reject)
        layout.addWidget(buttons)

        return dialog, time_edit

    def show_time_picker(self) -> None:
        # 弹窗选时间，日期取输入框或今天；exec 后 deleteLater 释放（FIX001.23）
        # 使用输入框中的日期，为空则使用今天
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

        dialog, time_edit = self._build_time_dialog(current_time)
        try:
            if dialog.exec() == QDialog.DialogCode.Accepted:
                selected_time = time_edit.time()
                time_str = selected_time.toString("HH:mm:ss")
                self.countdown_target.setText(f"{date_str} {time_str}")
        finally:
            dialog.deleteLater()


# ===== ui/panels/countdown_panel.py 函数/类说明 =====
# CountdownPanel(QWidget): 倒计时面板
#   __init__(interface, parent): 样式参数经 AppInterface.get_ui_static() 读取（PL001.10）
#   set_countdown(): 解析（经 tools.parse_target_text）并校验过期，成功后刷新显示
#   clear_countdown(): 清除目标与显示
#   update_countdown(): 主窗口 tick 调用；剩余拆解/结束态经 tools.format_remaining，
#     本方法只做 setText 与着色（结束红 danger/进行绿 primary，plan#UI2.0 铁律 1）
#   get_target_text(): 供主窗口保存配置；未设置返回空串
#   restore_target(text): 启动恢复（回填输入框并经 tools 解析内部态，FIX001.10）
#   _set_target_date_part(selected_date): 选中日期写回输入框日期部分并保留时间部分
#     （实时反馈不等 OK，修复 T001.2；日历点击/快捷按钮/OK 共用此路径）
#   _apply_quick_date(calendar, days): 快捷日期按钮（今天/明天/一周后），勾选日历 + 即时写回
#   _build_date_dialog()/_build_time_dialog(...): 弹窗构建器（接线实时反馈；时间弹窗尺寸
#     交由 Qt sizeHint 自适应，修复 HH:mm:ss 显示不全，T001.3）
#   show_date_picker()/show_time_picker(): 弹窗选择，仅改写输入框对应部分；
#     exec 后 deleteLater 释放子对话框（FIX001.23）
#   设计理由：倒计时状态（目标时间）内聚在面板，运算在 ui/tools，主窗口只做 tick 驱动
#   异常处理：格式解析（tools 层返回 None）弹窗提示；过期目标置 None
#   关联配置：countdown_target 配置项由主窗口经 get_target_text 持久化；样式经 ui.json
