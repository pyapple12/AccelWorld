# 时钟面板模块（S4 GUI 面板化拆分；PL002 Fluent 重写：qfw 组件 + 卡片布局；
# PL003 视觉迭代：英雄区重构——加速时间为绝对主角，等宽数字防走字抖动）
# 纯展示化（plan#UI2.0）：业务经 AppInterface，运算在 ui/tools，零后端 import

from PyQt6.QtCore import pyqtSignal, Qt, QPropertyAnimation
from PyQt6.QtGui import QDoubleValidator, QFont
from PyQt6.QtWidgets import QHBoxLayout, QVBoxLayout, QGridLayout, QWidget

from qfluentwidgets import (
    CaptionLabel,
    DisplayLabel,
    InfoBar,
    InfoBarPosition,
    LineEdit,
    PrimaryPushButton,
    ProgressBar,
    PushButton,
    Slider,
    StrongBodyLabel,
)

from interface import AppInterface
from interface.types import TimeInfo
from ui.glass_card import GlassCard, apply_capsule
from ui.tools.clock_tools import progress_bounds


def _digit_font(family: str, size: int) -> QFont:
    # 时间/百分比数字字体：微软雅黑数字本身即等宽（实测 11:11:11 与 58:25:39 同宽），
    # 走字天然无抖动；不启用 QFont 特性（PyQt6 setFeature 实测毒化进程，
    # 后续 QFont 密集操作硬崩 0xC0000409，故弃用）
    return QFont(family, size)


class ClockPanel(QWidget):
    rate_changed = pyqtSignal(float)  # 倍率变化信号（滑杆/输入框共用）

    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 构建显示卡片（英雄区时间/百分比/进度/次要参数）与设置卡片（滑杆/输入/预设）；
        # 范围/预设/默认值经接口读取（零硬编码）
        super().__init__(parent)
        self._interface = interface

        ui = interface.get_ui_static()
        layout_tokens = ui["layout"]
        scale_tokens = ui["scale"]
        base = interface.get_app_static()
        rate_min, rate_max = interface.get_rate_bounds()
        # 启动回显持久化倍率（PL005.01）：控件初值跟引擎一致，静态 default_rate 不再进控件
        current_rate = interface.get_rate()
        self._rate_min = rate_min  # 输入框应用加速校验用（apply_acceleration）
        self._rate_max = rate_max
        self._progress_anim_ms = int(base["progress_anim_ms"])  # 动画时长（T004.3）
        self._notify_ms = int(base["notification_duration_ms"])  # 错误提示条时长
        self._font_family = ui["font_family"]
        self._font_family_digits = ui["font_family_digits"]

        layout = QVBoxLayout(self)
        layout.setSpacing(int(layout_tokens["page_spacing"]))

        # ------------------- 时钟显示卡片（英雄区，PL003.04；玻璃化 PL006.05） -------------------
        display_card = GlassCard(interface, radius_key="xl")
        clock_layout = QVBoxLayout(display_card)
        clock_layout.setContentsMargins(*layout_tokens["clock_card_padding"])

        # 英雄行：加速时间（绝对主角）+ 膨胀倍率大数字
        hero_layout = QHBoxLayout()
        hero_col = QVBoxLayout()
        hero_col.addWidget(CaptionLabel("加速时间"))
        self.accelerated_time_label = DisplayLabel("00:00:00")
        self.accelerated_time_label.setFont(
            _digit_font(self._font_family_digits, int(scale_tokens["hero_time"]))
        )
        hero_col.addWidget(self.accelerated_time_label)
        hero_layout.addLayout(hero_col)

        hero_layout.addStretch()

        percent_col = QVBoxLayout()
        percent_col.addWidget(CaptionLabel("膨胀倍率"))
        self.rate_value_label = DisplayLabel("200%")
        self.rate_value_label.setFont(
            _digit_font(self._font_family_digits, int(scale_tokens["hero_percent"]))
        )
        percent_col.addWidget(self.rate_value_label)
        hero_layout.addLayout(percent_col)
        clock_layout.addLayout(hero_layout)

        # 加速时间进度条（qfw ProgressBar，随主题自绘）
        self.progress_bar = ProgressBar()
        self.progress_bar.setFixedHeight(int(layout_tokens["progress_bar_height"]))
        clock_layout.addWidget(self.progress_bar)

        # 进度条平滑动画（T004.3，惰性创建于 _animate_progress）
        self._progress_anim: QPropertyAnimation | None = None

        # 次要参数行：标准时间对照 + 一天小时数/剩余小时数（弱化色随主题）
        params_layout = QHBoxLayout()
        self.standard_time_label = StrongBodyLabel("标准时间 00:00:00")
        params_layout.addWidget(self.standard_time_label)
        params_layout.addStretch()
        params_layout.addWidget(CaptionLabel("加速后一天"))
        self.hours_per_day_value_label = StrongBodyLabel("48.00小时")
        params_layout.addWidget(self.hours_per_day_value_label)
        params_layout.addWidget(CaptionLabel("加速后剩余"))
        self.remaining_hours_value_label = StrongBodyLabel("45.00小时")
        params_layout.addWidget(self.remaining_hours_value_label)
        clock_layout.addLayout(params_layout)

        layout.addWidget(display_card)

        # ------------------- 倍率设置卡片（玻璃化 PL006.05） -------------------
        settings_card = GlassCard(interface, radius_key="lg")
        input_layout = QGridLayout(settings_card)

        rate_input_label = StrongBodyLabel("加速倍率:")
        input_layout.addWidget(rate_input_label, 0, 0)

        self.rate_entry = LineEdit()
        self.rate_entry.setText(f"{current_rate:g}")
        self.rate_entry.setFixedWidth(int(layout_tokens["rate_entry_width"]))
        self.rate_entry.setValidator(QDoubleValidator(rate_min, rate_max, 2))
        # 输入框横跨两列占住原校验文案位（PL005.02 移除常驻 hint，规则提示归 InfoBar）
        input_layout.addWidget(self.rate_entry, 0, 1, 1, 2)

        self.slider = Slider(Qt.Orientation.Horizontal)
        self.slider.setMinimum(int(rate_min * 10))  # 倍率 ×10
        self.slider.setMaximum(int(rate_max * 10))
        self.slider.setValue(int(current_rate * 10))
        self.slider.setFixedHeight(int(layout_tokens["slider_height"]))
        self.slider.valueChanged.connect(self.on_slider_change)
        input_layout.addWidget(self.slider, 1, 0, 1, 3)

        self.slider_value_label = StrongBodyLabel(f"{current_rate:.1f}x")
        input_layout.addWidget(self.slider_value_label, 2, 1)

        # 预设快捷按钮行（紧凑收身 PL003.05；点击经 set_rate 走滑杆信号链，T004.2）
        self.preset_buttons: dict[str, PushButton] = {}
        preset_row = QHBoxLayout()
        preset_row.addStretch()
        for preset_name, preset_rate in interface.get_rate_presets().items():
            preset_button = PushButton(f"{preset_name} {preset_rate:.1f}x")
            preset_button.setFixedSize(*layout_tokens["preset_button_size"])
            apply_capsule(preset_button)  # 胶囊造型（PL006.04）
            preset_button.clicked.connect(
                lambda checked=False, rate=preset_rate: self._apply_preset(rate)
            )
            self.preset_buttons[preset_name] = preset_button
            preset_row.addWidget(preset_button)
        preset_row.addStretch()
        input_layout.addLayout(preset_row, 3, 0, 1, 4)

        self.confirm_button = PrimaryPushButton("应用加速")
        self.confirm_button.setFixedSize(*layout_tokens["confirm_button_size"])
        apply_capsule(self.confirm_button)  # 胶囊造型（PL006.04）
        self.confirm_button.clicked.connect(self.apply_acceleration)
        input_layout.addWidget(self.confirm_button, 0, 3, 2, 1)

        layout.addWidget(settings_card)

    def update_time(self, info: TimeInfo) -> None:
        # 由主窗口 tick 传入 TimeInfo：英雄区刷加速时间与百分比，次要行刷标准时间
        # 与剩余参数；进度上界换算走 ui/tools（plan#UI2.0 铁律 1）
        self.accelerated_time_label.setText(info.custom_time)
        self.rate_value_label.setText(f"{info.dilation_percentage:.0f}%")
        self.standard_time_label.setText(f"标准时间 {info.standard_time}")
        self.hours_per_day_value_label.setText(f"{info.expanded_hours_per_day:.2f}小时")
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
        # 输入框优先，为空时取滑杆值；非法输入经 InfoBar 提示（非模态，Fluent PL002.04）
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
            InfoBar.error(
                title="错误",
                content=str(e),
                orient=Qt.Orientation.Vertical,
                isClosable=True,
                position=InfoBarPosition.TOP,
                duration=self._notify_ms,
                parent=self,
            )


# ===== ui/panels/clock_panel.py 函数/类说明 =====
# _digit_font(family, size) -> QFont: 数字字体工厂（雅黑数字天然等宽，走字不抖动；
#   PyQt6 setFeature 实测毒化进程故弃用，详见 PL004.01 定案）
# ClockPanel(QWidget): 时钟显示（英雄区）+ 倍率设置（滑杆/输入框/预设按钮）
#   __init__(interface, parent): 范围/预设/当前倍率/动画时长/提示时长/类型尺度经接口读取
#   PL003.04 英雄区：加速时间 DisplayLabel（scale.hero_time）为绝对主角，
#   膨胀倍率大数字（scale.hero_percent）并列，标准时间退次要参数行
#   PL005.01/02：控件启动回显持久化倍率（interface.get_rate），常驻校验文案移除
#   （规则提示归 InfoBar 按需弹出），滑杆/输入框/倍率小标签三处与引擎一致
#   信号：rate_changed(float) 倍率变化，主窗口据此经接口重建实例并去抖持久化
#   update_time(info): 英雄区/次要行 setText + 进度上界经 clock_tools.progress_bounds
#   _animate_progress(target): QPropertyAnimation 平滑过渡（T004.3）
#   set_rate(rate)/_apply_preset(rate)/on_slider_change(value)/apply_acceleration():
#     倍率信号链（校验/持久化统一在主窗口；非法输入 InfoBar 非模态提示）
#   设计理由：显示与设置同属"时钟域"；面板零业务运算零后端 import（plan#UI2.0 铁律 1/2）
#   异常处理：输入解析 ValueError 弹 InfoBar 提示
#   关联配置：范围/预设/当前倍率/动画时长/提示时长经 AppInterface（base.json）；
#   字体族/类型尺度（scale 节）经 AppInterface（ui.json）
