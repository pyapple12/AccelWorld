# 倒计时面板模块（S4 GUI 面板化拆分，倒计时输入 + 日期/时间选择器）

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
from PyQt6.QtCore import Qt, QDate, QTime
from PyQt6.QtGui import QFont


class CountdownPanel(QWidget):
    """倒计时面板：目标时间输入、日期/时间选择器、倒计时显示"""

    def __init__(self, parent: QWidget | None = None):
        """初始化倒计时控件与内部状态"""
        # 目标时间内部态初始为 None；选择器只改写输入框文本
        super().__init__(parent)

        self.countdown_target_date = None  # 倒计时目标时间

        countdown_frame = QFrame()
        countdown_frame.setFrameShape(QFrame.Shape.StyledPanel)
        countdown_layout = QHBoxLayout(countdown_frame)

        countdown_title_label = QLabel("倒计时:")
        countdown_title_label.setFont(QFont("Arial", 12))
        countdown_layout.addWidget(countdown_title_label)

        # 目标时间输入框和选择器（水平排列）
        countdown_input_layout = QHBoxLayout()
        countdown_input_layout.setSpacing(5)

        self.countdown_target = QLineEdit()
        self.countdown_target.setPlaceholderText("YYYY-MM-DD HH:MM:SS")
        self.countdown_target.setFont(QFont("Arial", 9))
        self.countdown_target.setFixedWidth(200)
        countdown_input_layout.addWidget(self.countdown_target)

        # 日期选择器按钮
        self.date_picker_button = QPushButton("📅")
        self.date_picker_button.setFixedSize(32, 32)
        self.date_picker_button.setToolTip("选择日期")
        self.date_picker_button.setFont(QFont("Arial", 12))
        self.date_picker_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.date_picker_button.clicked.connect(self.show_date_picker)
        countdown_input_layout.addWidget(self.date_picker_button)

        # 时间选择器按钮
        self.time_picker_button = QPushButton("🕐")
        self.time_picker_button.setFixedSize(32, 32)
        self.time_picker_button.setToolTip("选择时间")
        self.time_picker_button.setFont(QFont("Arial", 12))
        self.time_picker_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.time_picker_button.clicked.connect(self.show_time_picker)
        countdown_input_layout.addWidget(self.time_picker_button)

        countdown_layout.addLayout(countdown_input_layout)

        # 倒计时显示
        self.countdown_label = QLabel("--天 --:--:--:--")
        self.countdown_label.setFont(QFont("Arial", 14, QFont.Weight.Bold))
        self.countdown_label.setStyleSheet("color: #4CAF50;")
        countdown_layout.addWidget(self.countdown_label)

        countdown_hint_label = QLabel("YYYY-MM-DD HH:MM:SS")
        countdown_hint_label.setFont(QFont("Arial", 9))
        countdown_hint_label.setStyleSheet("color: #888888")
        countdown_layout.addWidget(countdown_hint_label)

        countdown_layout.addStretch()

        # 设置/清除按钮
        self.set_countdown_button = QPushButton("设置")
        self.set_countdown_button.setFont(QFont("Arial", 10))
        self.set_countdown_button.clicked.connect(self.set_countdown)
        countdown_layout.addWidget(self.set_countdown_button)

        self.clear_countdown_button = QPushButton("清除")
        self.clear_countdown_button.setFont(QFont("Arial", 10))
        self.clear_countdown_button.clicked.connect(self.clear_countdown)
        countdown_layout.addWidget(self.clear_countdown_button)

        outer = QVBoxLayout(self)
        outer.addWidget(countdown_frame)

    def set_countdown(self) -> None:
        """解析输入框并设置倒计时目标时间"""
        # 三种长度格式解析；过期目标拒绝并置 None
        target_text = self.countdown_target.text().strip()
        if not target_text:
            QMessageBox.warning(self, "警告", "请输入目标时间")
            return

        try:
            # 尝试解析时间格式
            if len(target_text) == 19:  # YYYY-MM-DD HH:MM:SS
                self.countdown_target_date = datetime.datetime.strptime(
                    target_text, "%Y-%m-%d %H:%M:%S"
                )
            elif len(target_text) == 16:  # YYYY-MM-DD HH:MM
                self.countdown_target_date = datetime.datetime.strptime(
                    target_text, "%Y-%m-%d %H:%M"
                )
            elif len(target_text) == 10:  # YYYY-MM-DD
                self.countdown_target_date = datetime.datetime.strptime(
                    target_text, "%Y-%m-%d"
                )
                # 只有日期时默认当天 23:59:59
                self.countdown_target_date = self.countdown_target_date.replace(
                    hour=23, minute=59, second=59
                )
            else:
                raise ValueError("时间格式不正确")
        except ValueError as e:
            QMessageBox.critical(
                self, "错误", f"时间格式不正确，请使用 YYYY-MM-DD HH:MM:SS 格式\n{e}"
            )
            return

        # 检查时间是否已过期
        if self.countdown_target_date <= datetime.datetime.now():
            QMessageBox.warning(self, "警告", "目标时间已过期，请选择未来时间")
            self.countdown_target_date = None
            return

        self.update_countdown()

    def clear_countdown(self) -> None:
        """清除倒计时"""
        # 目标置 None、标签复位、输入框清空
        self.countdown_target_date = None
        self.countdown_label.setText("--天 --:--:--:--")
        self.countdown_target.clear()

    def update_countdown(self) -> None:
        """刷新倒计时显示（由主窗口时钟 tick 调用）"""
        # 剩余拆天/时/分/秒；结束红色、进行绿色
        if not self.countdown_target_date:
            return

        now = datetime.datetime.now()
        remaining = self.countdown_target_date - now

        if remaining.total_seconds() <= 0:
            self.countdown_label.setText("00天 00:00:00")
            self.countdown_label.setStyleSheet("color: #f44336;")  # 红色表示倒计时结束
            return

        days = remaining.days
        hours = remaining.seconds // 3600
        minutes = (remaining.seconds % 3600) // 60
        seconds = remaining.seconds % 60

        self.countdown_label.setText(
            f"{days}天 {hours:02d}:{minutes:02d}:{seconds:02d}"
        )
        self.countdown_label.setStyleSheet("color: #4CAF50;")

    def get_target_text(self) -> str:
        """获取当前倒计时目标文本（供配置保存，未设置时返回空串）"""
        # 仅在目标已设置时返回输入框文本
        if self.countdown_target_date:
            return self.countdown_target.text().strip()
        return ""

    def show_date_picker(self) -> None:
        """显示日期选择器对话框（仅更新输入框日期部分）"""
        # 弹窗选日期，保留输入框已有时间部分
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

        # 预设快捷按钮
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(5)

        today_btn = QPushButton("今天")
        today_btn.setFont(QFont("Arial", 10))
        today_btn.clicked.connect(lambda: calendar.setSelectedDate(QDate.currentDate()))
        btn_layout.addWidget(today_btn)

        tomorrow_btn = QPushButton("明天")
        tomorrow_btn.setFont(QFont("Arial", 10))
        tomorrow_btn.clicked.connect(
            lambda: calendar.setSelectedDate(QDate.currentDate().addDays(1))
        )
        btn_layout.addWidget(tomorrow_btn)

        week_btn = QPushButton("一周后")
        week_btn.setFont(QFont("Arial", 10))
        week_btn.clicked.connect(
            lambda: calendar.setSelectedDate(QDate.currentDate().addDays(7))
        )
        btn_layout.addWidget(week_btn)

        layout.addLayout(btn_layout)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.setFont(QFont("Arial", 10))
        buttons.accepted.connect(dialog.accept)
        buttons.rejected.connect(dialog.reject)
        layout.addWidget(buttons)

        if dialog.exec() == QDialog.DialogCode.Accepted:
            selected_date = calendar.selectedDate()
            current_text = self.countdown_target.text().strip()
            # 保留当前时间部分，只更新日期
            if current_text and len(current_text) >= 10:
                time_part = current_text[10:] if len(current_text) > 10 else " 00:00:00"
                self.countdown_target.setText(
                    f"{selected_date.toString('yyyy-MM-dd')}{time_part}"
                )
            else:
                self.countdown_target.setText(
                    f"{selected_date.toString('yyyy-MM-dd')} 00:00:00"
                )

    def show_time_picker(self) -> None:
        """显示时间选择器对话框（仅更新输入框时间部分）"""
        # 弹窗选时间，日期取输入框或今天
        # 使用输入框中的日期，为空则使用今天
        current_text = self.countdown_target.text().strip()
        if current_text and len(current_text) >= 10:
            date_str = current_text[:10]
        else:
            date_str = QDate.currentDate().toString("yyyy-MM-dd")

        # 解析当前时间（如果有）
        current_time = QTime.currentTime()
        if current_text and len(current_text) >= 16:
            try:
                time_str = current_text[11:16]
                current_time = QTime.fromString(time_str, "HH:mm")
            except (ValueError, TypeError):
                pass

        dialog = QDialog(self)
        dialog.setWindowTitle("选择时间")
        dialog.setFixedSize(220, 140)

        layout = QVBoxLayout(dialog)
        layout.setSpacing(10)

        time_edit = QTimeEdit()
        time_edit.setDisplayFormat("HH:mm:ss")
        time_edit.setTime(current_time)
        time_edit.setFont(QFont("Arial", 14))
        time_edit.setFixedSize(120, 40)
        layout.addWidget(time_edit)

        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(5)

        now_btn = QPushButton("现在")
        now_btn.setFont(QFont("Arial", 10))
        now_btn.clicked.connect(lambda: time_edit.setTime(QTime.currentTime()))
        btn_layout.addWidget(now_btn)

        layout.addLayout(btn_layout)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.setFont(QFont("Arial", 10))
        buttons.accepted.connect(dialog.accept)
        buttons.rejected.connect(dialog.reject)
        layout.addWidget(buttons)

        if dialog.exec() == QDialog.DialogCode.Accepted:
            selected_time = time_edit.time()
            time_str = selected_time.toString("HH:mm:ss")
            self.countdown_target.setText(f"{date_str} {time_str}")


# ===== ui/panels/countdown_panel.py 函数/类说明 =====
# CountdownPanel(QWidget): 倒计时面板
#   set_countdown(): 解析三种时间格式并校验过期，成功后刷新显示
#   clear_countdown(): 清除目标与显示
#   update_countdown(): 主窗口 tick 调用，计算剩余并着色（结束红/进行绿）
#   get_target_text(): 供主窗口保存配置；未设置返回空串
#   show_date_picker()/show_time_picker(): 弹窗选择，仅改写输入框对应部分
#   设计理由：倒计时状态（目标时间）内聚在面板，主窗口只做 tick 驱动
#   异常处理：格式解析 ValueError 弹窗提示；过期目标置 None
#   关联配置：countdown_target 配置项由主窗口经 get_target_text 持久化
