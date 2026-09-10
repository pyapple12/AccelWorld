# 时钟面板模块（S4 GUI 面板化拆分，含时钟显示 + 参数标签 + 倍率输入区）
# PL001（plan#UI2.0）：纯展示化——后端参数/静态配置一律经 AppInterface 读取，
# 进度上界换算迁 ui/tools/clock_tools，本文件零后端 import

from PyQt6.QtCore import pyqtSignal, Qt, QPropertyAnimation
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

from interface import AppInterface
from interface.types import TimeInfo
from ui.tools.clock_tools import progress_bounds


class ClockPanel(QWidget):
    rate_changed = pyqtSignal(float)  # 倍率变化信号（滑杆/输入框共用）

    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 构建显示区（时间/进度/参数）与设置区（滑杆/输入/按钮）；
        # 范围/预设/默认值/样式参数经接口读取（零硬编码）
        super().__init__(parent)
        self._interface = interface

        ui = interface.get_ui_static()
        base = interface.get_app_static()
        rate_min, rate_max = interface.get_rate_bounds()
        default_rate = float(base["default_rate"])
        self._rate_min = rate_min  # 输入框应用加速校验用（apply_acceleration）
        self._rate_max = rate_max
        self._progress_anim_ms = int(base["progress_anim_ms"])  # 动画时长（T004.3）

        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        # ------------------- 时钟显示区域 -------------------
        clock_frame = QFrame()
        clock_frame.setFrameShape(QFrame.Shape.StyledPanel)
        clock_layout = QVBoxLayout(clock_frame)

        # 时间标签行
        time_label_layout = QHBoxLayout()
        self.standard_time_label = QLabel("标准时间: 00:00:00")
        self.standard_time_label.setFont(QFont(ui["font_family"], 16))
        time_label_layout.addWidget(self.standard_time_label)
        self.accelerated_time_label = QLabel("加速时间: 00:00:00")
        self.accelerated_time_label.setFont(QFont(ui["font_family"], 16))
        time_label_layout.addWidget(self.accelerated_time_label)
        clock_layout.addLayout(time_label_layout)

        # 加速时间进度条（样式由主窗口 apply_theme 统一下发，构造期不设样式）
        self.progress_bar = QProgressBar()
        self.progress_bar.setFont(QFont(ui["font_family"], 10))
        self.progress_bar.setFixedHeight(25)
        self.progress_bar.setFormat("%v / %m 小时")
        clock_layout.addWidget(self.progress_bar)

        # 进度条平滑动画（T004.3，惰性创建于 _animate_progress）
        self._progress_anim: QPropertyAnimation | None = None

        # 参数显示行（一天小时数/倍率/剩余小时数）
        params_layout = QGridLayout()
        params_layout.addWidget(QLabel("加速后一天小时数:"), 0, 0)
        self.hours_per_day_value_label = QLabel("48.00小时")
        self.hours_per_day_value_label.setFont(
            QFont(ui["font_family"], 12, QFont.Weight.Bold)
        )
        params_layout.addWidget(self.hours_per_day_value_label, 0, 1)
        params_layout.addWidget(QLabel("加速倍率:"), 0, 2)
        self.rate_value_label = QLabel("200%")
        self.rate_value_label.setFont(QFont(ui["font_family"], 12, QFont.Weight.Bold))
        params_layout.addWidget(self.rate_value_label, 0, 3)
        params_layout.addWidget(QLabel("加速后剩余小时数:"), 0, 4)
        self.remaining_hours_value_label = QLabel("45.00小时")
        self.remaining_hours_value_label.setFont(
            QFont(ui["font_family"], 12, QFont.Weight.Bold)
        )
        params_layout.addWidget(self.remaining_hours_value_label, 0, 5)
        clock_layout.addLayout(params_layout)

        layout.addWidget(clock_frame)

        # ------------------- 倍率设置区域 -------------------
        input_frame = QFrame()
        input_frame.setFrameShape(QFrame.Shape.StyledPanel)
        input_layout = QGridLayout(input_frame)

        rate_input_label = QLabel("加速倍率:")
        rate_input_label.setFont(QFont(ui["font_family"], 12))
        input_layout.addWidget(rate_input_label, 0, 0, Qt.AlignmentFlag.AlignRight)

        self.rate_entry = QLineEdit()
        self.rate_entry.setText(str(default_rate))
        self.rate_entry.setFont(QFont(ui["font_family"], 12))
        self.rate_entry.setFixedWidth(80)
        self.rate_entry.setValidator(QDoubleValidator(rate_min, rate_max, 2))
        input_layout.addWidget(self.rate_entry, 0, 1, Qt.AlignmentFlag.AlignLeft)

        rate_hint_label = QLabel(
            f"（必须不小于{rate_min}，步进 0.1，最大值{rate_max}，默认{default_rate}）"
        )
        rate_hint_label.setFont(QFont(ui["font_family"], 10))
        input_layout.addWidget(rate_hint_label, 0, 2, Qt.AlignmentFlag.AlignLeft)

        self.slider = QSlider(Qt.Orientation.Horizontal)
        self.slider.setMinimum(int(rate_min * 10))  # 倍率 ×10
        self.slider.setMaximum(int(rate_max * 10))
        self.slider.setValue(int(default_rate * 10))
        self.slider.setFixedHeight(30)
        self.slider.valueChanged.connect(self.on_slider_change)
        input_layout.addWidget(self.slider, 1, 0, 1, 3)

        self.slider_value_label = QLabel(f"{default_rate:.1f}x")
        self.slider_value_label.setFont(QFont(ui["font_family"], 12))
        input_layout.addWidget(self.slider_value_label, 2, 1, Qt.AlignmentFlag.AlignLeft)

        # 预设快捷按钮行（预设经接口读取，T004.2；点击经 set_rate 走滑杆信号链）
        self.preset_buttons: dict[str, QPushButton] = {}
        preset_row = QHBoxLayout()
        for preset_name, preset_rate in interface.get_rate_presets().items():
            preset_button = QPushButton(f"{preset_name} {preset_rate:.1f}x")
            preset_button.clicked.connect(
                lambda checked=False, rate=preset_rate: self._apply_preset(rate)
            )
            self.preset_buttons[preset_name] = preset_button
            preset_row.addWidget(preset_button)
        input_layout.addLayout(preset_row, 3, 0, 1, 4)

        self.confirm_button = QPushButton("应用加速")
        self.confirm_button.setFont(QFont(ui["font_family"], 12, QFont.Weight.Bold))
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
        # 由主窗口 tick 传入 TimeInfo，经计算属性取值更新标签与进度条；
        # 进度上界换算走 ui/tools（plan#UI2.0 铁律 1）
        self.standard_time_label.setText(f"标准时间: {info.standard_time}")
        self.accelerated_time_label.setText(f"加速时间: {info.custom_time}")
        self.hours_per_day_value_label.setText(f"{info.expanded_hours_per_day:.2f}小时")
        self.rate_value_label.setText(f"{info.dilation_percentage:.0f}%")
        self.remaining_hours_value_label.setText(f"{info.remaining_hours:.2f}小时")

        # 计算进度并更新进度条（动画平滑过渡替代 setValue 跳变，T004.3）
        total_hours = progress_bounds(info.expanded_hours_per_day)
        current_hour = info.custom_hour
        self.progress_bar.setMaximum(total_hours)
        self._animate_progress(current_hour)

    def _animate_progress(self, target: int) -> None:
        # QPropertyAnimation 从当前值平滑推进到目标小时数（时长来自静态配置）
        # 设计理由：每 tick 以当前动画值为起点重定目标，高频刷新下收敛自然、无跳变
        if self._progress_anim is None:
            self._progress_anim = QPropertyAnimation(self.progress_bar, b"value", self)
        anim = self._progress_anim
        anim.stop()
        anim.setDuration(self._progress_anim_ms)
        anim.setStartValue(self.progress_bar.value())
        anim.setEndValue(target)
        anim.start()

    def set_progress_style(self, qss: str) -> None:
        # 直接应用传入的 QSS 常量（主窗口按主题构建后下发）
        self.progress_bar.setStyleSheet(qss)

    def set_rate(self, rate: float) -> None:
        # 滑杆 setValue 触发 on_slider_change → 信号链自动生效（标签由信号链统一更新）
        self.slider.setValue(int(round(rate * 10)))
        self.rate_entry.setText("")

    def _apply_preset(self, rate: float) -> None:
        # 预设按钮回调：与滑杆/手输共用 set_rate 信号链（校验/持久化统一在主窗口）
        self.set_rate(rate)

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
            if not (self._rate_min <= rate <= self._rate_max):
                raise ValueError(
                    f"加速倍率必须在{self._rate_min}到{self._rate_max}之间"
                )

            # 统一经滑杆信号链发一次 rate_changed（FIX001.23：消除"显式 emit + setValue
            # 触发 emit"的双发冗余；值未变化时滑杆不发信号，此时显式补发一次）
            new_slider_value = int(rate * 10)
            if self.slider.value() != new_slider_value:
                self.slider.setValue(new_slider_value)
            else:
                self.rate_changed.emit(rate)
            self.rate_entry.setText("")
        except ValueError as e:
            QMessageBox.critical(self, "错误", str(e))


# ===== ui/panels/clock_panel.py 函数/类说明 =====
# ClockPanel(QWidget): 时钟显示 + 参数标签 + 进度条 + 倍率设置（滑杆/输入框/预设按钮）
#   __init__(interface, parent): 静态样式/范围/预设/默认倍率经接口读取（PL001.09 纯展示化）
#   信号：rate_changed(float) 倍率变化，主窗口据此经接口重建实例并去抖持久化
#   update_time(info): 刷新时间/参数显示，进度上界经 clock_tools.progress_bounds 换算
#   _animate_progress(target): QPropertyAnimation 平滑过渡（时长 progress_anim_ms，T004.3）
#     输入：目标小时数；输出：无（副作用为进度条属性动画）
#     设计理由：动画对象惰性创建并持有（防 GC）；每次以当前值为起点重定目标，
#     tick 高频调用下动画连续收敛；stop() 防同对象重复启动告警
#     异常处理：无（属性动画不抛业务异常）
#   set_progress_style(qss): 主题切换时更新进度条样式（QSS 由主窗口经 themes.build_theme 构建）
#   set_rate(rate): 外部同步倍率（走滑杆触发信号，保证 UI 与核心一致）
#   _apply_preset(rate): 预设按钮回调（T004.2），走 set_rate 统一信号链
#     设计理由：预设只改倍率入口，校验/持久化仍归主窗口单一路径，避免双写
#   on_slider_change(value): 滑杆回调，同步标签后发信号
#   apply_acceleration(): 输入框解析/验证/发信号/同步滑杆（范围经接口 bounds 缓存）
#   设计理由：显示与设置同属"时钟域"；信号解耦面板与主窗口，无需反向引用；
#   面板零业务运算零后端 import（plan#UI2.0 铁律 1/2）
#   异常处理：输入解析 ValueError 弹窗提示
#   关联配置：范围/预设/默认倍率/动画时长经 AppInterface 读取（base.json）；字体经 ui.json
