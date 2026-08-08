# 时钟面板模块（S4 GUI 面板化拆分，含时钟显示 + 参数标签 + 倍率输入区）

from PyQt6.QtCore import pyqtSignal, Qt
from PyQt6.QtWidgets import (
    QWidget,
    QVBoxLayout,
    QHBoxLayout,
    QGridLayout,
    QFrame,
    QLabel,
    QSlider,
    QLineEdit,
    QPushButton,
    QProgressBar,
    QMessageBox,
)
from PyQt6.QtGui import QFont, QDoubleValidator

from modules.time_dilation import TimeInfo
from ui.themes import LIGHT_THEME_PROGRESS
from config.static.static_config import get_static_config

# 静态配置（倍率范围/默认倍率/字体）
_BASE = get_static_config().base
_UI = get_static_config().ui


class ClockPanel(QWidget):
    rate_changed = pyqtSignal(float)  # 倍率变化信号（滑杆/输入框共用）

    def __init__(self, parent: QWidget | None = None):
        # 构建显示区（时间/进度/参数）与设置区（滑杆/输入/按钮）
        super().__init__(parent)

        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        # ------------------- 时钟显示区域 -------------------
        clock_frame = QFrame()
        clock_frame.setFrameShape(QFrame.Shape.StyledPanel)
        clock_layout = QVBoxLayout(clock_frame)

        # 时间标签行
        time_label_layout = QHBoxLayout()
        self.standard_time_label = QLabel("标准时间: 00:00:00")
        self.standard_time_label.setFont(QFont(_UI["font_family"], 16))
        time_label_layout.addWidget(self.standard_time_label)
        self.accelerated_time_label = QLabel("加速时间: 00:00:00")
        self.accelerated_time_label.setFont(QFont(_UI["font_family"], 16))
        time_label_layout.addWidget(self.accelerated_time_label)
        clock_layout.addLayout(time_label_layout)

        # 加速时间进度条
        self.progress_bar = QProgressBar()
        self.progress_bar.setFont(QFont(_UI["font_family"], 10))
        self.progress_bar.setFixedHeight(25)
        self.progress_bar.setFormat("%v / %m 小时")
        self.progress_bar.setStyleSheet(LIGHT_THEME_PROGRESS)
        clock_layout.addWidget(self.progress_bar)

        # 参数显示行（一天小时数/倍率/剩余小时数）
        params_layout = QGridLayout()
        params_layout.addWidget(QLabel("加速后一天小时数:"), 0, 0)
        self.hours_per_day_value_label = QLabel("48.00小时")
        self.hours_per_day_value_label.setFont(
            QFont(_UI["font_family"], 12, QFont.Weight.Bold)
        )
        params_layout.addWidget(self.hours_per_day_value_label, 0, 1)
        params_layout.addWidget(QLabel("加速倍率:"), 0, 2)
        self.rate_value_label = QLabel("200%")
        self.rate_value_label.setFont(QFont(_UI["font_family"], 12, QFont.Weight.Bold))
        params_layout.addWidget(self.rate_value_label, 0, 3)
        params_layout.addWidget(QLabel("加速后剩余小时数:"), 0, 4)
        self.remaining_hours_value_label = QLabel("45.00小时")
        self.remaining_hours_value_label.setFont(
            QFont(_UI["font_family"], 12, QFont.Weight.Bold)
        )
        params_layout.addWidget(self.remaining_hours_value_label, 0, 5)
        clock_layout.addLayout(params_layout)

        layout.addWidget(clock_frame)

        # ------------------- 倍率设置区域 -------------------
        input_frame = QFrame()
        input_frame.setFrameShape(QFrame.Shape.StyledPanel)
        input_layout = QGridLayout(input_frame)

        rate_input_label = QLabel("加速倍率:")
        rate_input_label.setFont(QFont(_UI["font_family"], 12))
        input_layout.addWidget(rate_input_label, 0, 0, Qt.AlignmentFlag.AlignRight)

        self.rate_entry = QLineEdit()
        self.rate_entry.setText(str(_BASE["default_rate"]))
        self.rate_entry.setFont(QFont(_UI["font_family"], 12))
        self.rate_entry.setFixedWidth(80)
        self.rate_entry.setValidator(
            QDoubleValidator(_BASE["rate_min"], _BASE["rate_max"], 2)
        )
        input_layout.addWidget(self.rate_entry, 0, 1, Qt.AlignmentFlag.AlignLeft)

        rate_hint_label = QLabel(
            f"（必须大于{_BASE['rate_min']}，默认值{_BASE['default_rate']}，最大值{_BASE['rate_max']}）"
        )
        rate_hint_label.setFont(QFont(_UI["font_family"], 10))
        input_layout.addWidget(rate_hint_label, 0, 2, Qt.AlignmentFlag.AlignLeft)

        self.slider = QSlider(Qt.Orientation.Horizontal)
        self.slider.setMinimum(int(_BASE["rate_min"] * 10))  # 倍率 ×10
        self.slider.setMaximum(int(_BASE["rate_max"] * 10))
        self.slider.setValue(int(_BASE["default_rate"] * 10))
        self.slider.setFixedHeight(30)
        self.slider.valueChanged.connect(self.on_slider_change)
        input_layout.addWidget(self.slider, 1, 0, 1, 3)

        self.slider_value_label = QLabel(f"{_BASE['default_rate']:.1f}x")
        self.slider_value_label.setFont(QFont(_UI["font_family"], 12))
        input_layout.addWidget(
            self.slider_value_label, 2, 1, Qt.AlignmentFlag.AlignLeft
        )

        self.confirm_button = QPushButton("应用加速")
        self.confirm_button.setFont(QFont(_UI["font_family"], 12, QFont.Weight.Bold))
        self.confirm_button.setFixedSize(120, 50)
        self.confirm_button.clicked.connect(self.apply_acceleration)
        input_layout.addWidget(
            self.confirm_button,
            0,
            3,
            2,
            1,
            Qt.AlignmentFlag.AlignVCenter | Qt.AlignmentFlag.AlignRight,
        )

        layout.addWidget(input_frame)

    def update_time(self, info: TimeInfo) -> None:
        # 由主窗口 tick 传入 TimeInfo，经计算属性取值更新标签与进度条
        self.standard_time_label.setText(f"标准时间: {info.standard_time}")
        self.accelerated_time_label.setText(f"加速时间: {info.custom_time}")
        self.hours_per_day_value_label.setText(f"{info.expanded_hours_per_day:.2f}小时")
        self.rate_value_label.setText(f"{info.dilation_percentage:.0f}%")
        self.remaining_hours_value_label.setText(f"{info.remaining_hours:.2f}小时")

        # 计算进度并更新进度条
        total_hours = int(info.expanded_hours_per_day)
        current_hour = info.custom_hour
        self.progress_bar.setMaximum(total_hours)
        self.progress_bar.setValue(current_hour)

    def set_progress_style(self, qss: str) -> None:
        # 直接应用传入的 QSS 常量
        self.progress_bar.setStyleSheet(qss)

    def set_rate(self, rate: float) -> None:
        # 滑杆 setValue 触发 on_slider_change → 信号链自动生效（标签由信号链统一更新）
        self.slider.setValue(int(round(rate * 10)))
        self.rate_entry.setText("")

    def on_slider_change(self, value: int) -> None:
        # 值÷10 还原倍率，输入框同步显示两位小数
        slider_value = value / 10.0
        self.slider_value_label.setText(f"{slider_value:.1f}x")
        self.rate_entry.setText(f"{slider_value:.2f}")
        self.rate_changed.emit(slider_value)

    def apply_acceleration(self) -> None:
        # 输入框优先，为空时取滑杆值；非法输入弹窗提示
        rate_text = self.rate_entry.text().strip()
        if not rate_text:
            # 输入框为空时使用滑杆当前值
            rate_text = str(self.slider.value() / 10.0)

        try:
            rate = float(rate_text)
            # 对齐滑杆 0.1 粒度（round 后除回，避免浮点 2.05*10=20.4999 截断歧义，F2）
            rate = round(rate * 10) / 10
            if not (_BASE["rate_min"] <= rate <= _BASE["rate_max"]):
                raise ValueError(
                    f"加速倍率必须在{_BASE['rate_min']}到{_BASE['rate_max']}之间"
                )

            self.rate_changed.emit(rate)
            # 同步滑杆（setValue 触发 on_slider_change 统一更新标签，界面与核心一致）
            self.slider.setValue(int(rate * 10))
            self.rate_entry.setText("")
        except ValueError as e:
            QMessageBox.critical(self, "错误", str(e))


# ===== ui/panels/clock_panel.py 函数/类说明 =====
# ClockPanel(QWidget): 时钟显示 + 参数标签 + 进度条 + 倍率设置（滑杆/输入框/按钮）
#   信号：rate_changed(float) 倍率变化，主窗口据此重建实例并持久化
#   update_time(info): 刷新时间/参数/进度条显示
#   set_progress_style(qss): 主题切换时更新进度条样式
#   set_rate(rate): 外部同步倍率（走滑杆触发信号，保证 UI 与核心一致）
#   on_slider_change(value): 滑杆回调，同步标签后发信号
#   apply_acceleration(): 输入框解析/验证/发信号/同步滑杆
#   设计理由：显示与设置同属"时钟域"；信号解耦面板与主窗口，无需反向引用
#   异常处理：输入解析 ValueError 弹窗提示
#   关联配置：进度条样式来自 ui/themes.py
