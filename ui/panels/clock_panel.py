# 时钟面板模块（S4 GUI 面板化拆分；PL002 Fluent 重写：qfw 组件 + 卡片布局；
# PL003 视觉迭代：英雄区重构——加速时间为绝对主角，等宽数字防走字抖动）
# 纯展示化（plan#UI2.0）：业务经 AppInterface，运算在 ui/tools，零后端 import

from PyQt6.QtCore import pyqtSignal, Qt, QPropertyAnimation
from PyQt6.QtGui import QColor, QFont
from PyQt6.QtWidgets import QHBoxLayout, QLabel, QVBoxLayout, QWidget

from qfluentwidgets import (
    CaptionLabel,
    DisplayLabel,
    ProgressRing,
    StrongBodyLabel,
)

from interface import AppInterface
from interface.types import TimeInfo
from ui.glass_card import CapsuleSlider, GlassCard, with_alpha
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
        # 启动回显持久化倍率（PL005.01）：滑杆初值跟引擎一致
        current_rate = interface.get_rate()
        self._progress_anim_ms = int(base["progress_anim_ms"])  # 动画时长（T004.3）
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

        # 今日膨胀进度环形表盘（PL007.01；热更新：caption/环/数值 改竖排；
        # 膨胀倍率百分比随减法轮移至倍率卡，hero 不再重复展示）
        ring_col = QVBoxLayout()
        ring_col.addWidget(CaptionLabel("今日膨胀进度"))
        self.progress_ring = ProgressRing()
        self.progress_ring.setFixedSize(
            int(layout_tokens["progress_ring_size"]), int(layout_tokens["progress_ring_size"])
        )
        self.progress_ring.setCustomBarColor(
            QColor(self._interface.get_ui_static()["colors"]["primary"]),
            QColor(self._interface.get_ui_static()["colors"]["primary"]),
        )
        self.progress_ring.setStrokeWidth(7)
        self.progress_ring.setTextVisible(True)
        ring_col.addWidget(self.progress_ring, 0, Qt.AlignmentFlag.AlignHCenter)
        self.hours_ring_caption = StrongBodyLabel("-- / --")
        self.hours_ring_caption.setAlignment(Qt.AlignmentFlag.AlignHCenter)
        ring_col.addWidget(self.hours_ring_caption)
        hero_layout.addLayout(ring_col)
        clock_layout.addLayout(hero_layout)

        # 环形进度平滑动画（T004.3 模式复用，惰性创建于 _animate_progress）
        self._progress_anim: QPropertyAnimation | None = None

        layout.addWidget(display_card)

        # ------------------- 倍率设置卡片（减法轮：滑杆整行 + 应用加速下方） -------------------
        settings_card = GlassCard(interface, radius_key="lg")
        rate_layout = QVBoxLayout(settings_card)
        rate_layout.setContentsMargins(*layout_tokens["card_padding"])
        rate_layout.setSpacing(10)

        # 胶囊粗轨滑杆（样式 B，用户选定）：整行铺满，强调色已过段，零进度保留橙圆槽
        self.slider = CapsuleSlider(
            with_alpha(QColor(255, 255, 255), 0.15),
            QColor(self._interface.get_ui_static()["colors"]["primary"]),
            track_h=float(layout_tokens["slider_track"]),
        )
        self.slider.setMinimum(int(rate_min * 10))  # 倍率 ×10
        self.slider.setMaximum(int(rate_max * 10))
        self.slider.setValue(int(current_rate * 10))
        # 控件高度 == 轨道高度：qfw 将旋钮固定在 y=0，等高即旋钮结构性垂直居中
        self.slider.setFixedHeight(
            int(layout_tokens["slider_track"]) + 2 * CapsuleSlider.TRACK_PAD
        )
        self.slider.valueChanged.connect(self.on_slider_change)
        rate_layout.addWidget(self.slider)

        # 倍率读数行：当前倍率大字（左）+ 膨胀倍率百分比（右）
        readout_row = QHBoxLayout()
        self.slider_value_label = QLabel(f"{current_rate:.1f}x")
        self.slider_value_label.setFont(
            _digit_font(self._font_family_digits, int(scale_tokens["slider_value"]))
        )
        readout_row.addWidget(self.slider_value_label)
        readout_row.addStretch()
        self.percent_label = StrongBodyLabel(f"膨胀倍率 {current_rate * 100:.0f}%")
        self.percent_label.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
        readout_row.addWidget(self.percent_label)
        rate_layout.addLayout(readout_row)

        layout.addWidget(settings_card)

    def update_time(self, info: TimeInfo) -> None:
        # 由主窗口 tick 传入 TimeInfo：英雄区刷加速时间，环刷膨胀进度；
        # 进度上界换算走 ui/tools（plan#UI2.0 铁律 1）
        self.accelerated_time_label.setText(info.custom_time)

        # 计算环形进度并更新（动画平滑过渡替代 setValue 跳变，T004.3 模式；
        # 语义与原横条等价：加速小时 / 膨胀日小时，PL007.01）
        total_hours = progress_bounds(info.expanded_hours_per_day)
        current_hour = info.custom_hour
        percent = round(current_hour / total_hours * 100) if total_hours > 0 else 0
        self.hours_ring_caption.setText(f"{current_hour} / {total_hours} 小时")
        self._animate_progress(max(0, min(percent, 100)))

    def _animate_progress(self, target: int) -> None:
        # QPropertyAnimation 从当前值平滑推进到目标百分比（时长来自静态配置）
        # 设计理由：每 tick 以当前动画值为起点重定目标，高频刷新下收敛自然、无跳变
        if self._progress_anim is None:
            self._progress_anim = QPropertyAnimation(self.progress_ring, b"value", self)
        anim = self._progress_anim
        anim.stop()
        anim.setDuration(self._progress_anim_ms)
        anim.setStartValue(self.progress_ring.value())
        anim.setEndValue(target)
        anim.start()

    def set_rate(self, rate: float) -> None:
        # 滑杆 setValue 触发 on_slider_change → 信号链自动生效（标签由信号链统一更新）
        self.slider.setValue(int(round(rate * 10)))

    def on_slider_change(self, value: int) -> None:
        # 值÷10 还原倍率，同步倍率大字与膨胀倍率百分比并外发信号
        slider_value = value / 10.0
        self.slider_value_label.setText(f"{slider_value:.1f}x")
        self.percent_label.setText(f"膨胀倍率 {slider_value * 100:.0f}%")
        self.rate_changed.emit(slider_value)


# ===== ui/panels/clock_panel.py 函数/类说明 =====
# _digit_font(family, size) -> QFont: 数字字体工厂（雅黑数字天然等宽，走字不抖动；
#   PyQt6 setFeature 实测毒化进程故弃用，详见 PL004.01 定案）
# ClockPanel(QWidget): 时钟显示（英雄区）+ 倍率设置（胶囊滑杆直动）
#   __init__(interface, parent): 范围/当前倍率/动画时长/类型尺度经接口读取
#   PL003.04 英雄区：加速时间 DisplayLabel（scale.hero_time）为绝对主角，
#   今日膨胀进度环组竖排（PL007.01；标准时间已独立成卡，date_panel.std_card）
#   热更新减法轮：加速后一天/剩余统计、倍率标签、输入框、预设按钮、应用加速
#   按钮全部移除——滑杆整行直动（拖动即生效），读数行左倍率右百分比；
#   rim/环境光见 ui/glass_card.py
#   信号：rate_changed(float) 倍率变化，主窗口据此经接口重建实例并去抖持久化
#   update_time(info): 英雄区 setText + 进度上界经 clock_tools.progress_bounds
#   _animate_progress(target): QPropertyAnimation 平滑过渡（T004.3）
#   set_rate(rate)/on_slider_change(value):
#     倍率信号链（校验/持久化统一在主窗口；滑杆量程天然限界，无非法输入路径）
#   设计理由：显示与设置同属"时钟域"；面板零业务运算零后端 import（plan#UI2.0 铁律 1/2）
#   异常处理：滑杆量程天然限界，无非法输入路径
#   关联配置：范围/当前倍率/动画时长经 AppInterface（base.json）；
#   字体族/类型尺度（scale 节）经 AppInterface（ui.json）
