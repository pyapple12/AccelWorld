# 世界时钟面板模块（S4 GUI 面板化拆分，时区下拉 + 世界时间）
# PL002 Fluent 重写：qfw ComboBox 下拉；时区着色改 QPalette（QSS 管线退役 PL002.09，
# 不再使用样式表写法）；
# PL001（plan#UI2.0）：时区选项经 AppInterface 读取（铁律 2），pytz 换算属渲染逻辑保留面板

import datetime
import logging
from typing import Any

import pytz

from PyQt6.QtGui import QColor, QFont, QPalette
from PyQt6.QtWidgets import QHBoxLayout, QLabel, QWidget

from qfluentwidgets import BodyLabel, ComboBox

from interface import AppInterface

# 配置日志
logger = logging.getLogger(__name__)


class WorldClockPanel(QWidget):
    def __init__(self, interface: AppInterface, parent=None):
        # 用接口提供的时区选项表填充下拉框（索引即选项表下标，切换按名定位）；
        # 字体/强调色经接口读取
        super().__init__(parent)
        self._interface = interface
        self._options = interface.get_timezone_options()
        self._layout = interface.get_ui_static()["layout"]

        ui = interface.get_ui_static()

        world_clock_layout = QHBoxLayout(self)
        world_clock_layout.setContentsMargins(*self._layout["panel_margin"])

        # 世界时钟标题
        world_clock_title = BodyLabel("世界时钟:")
        world_clock_layout.addWidget(world_clock_title)

        # 时区选择（qfw ComboBox；选项表顺序与下拉索引一一对应）
        self.timezone_combo = ComboBox()
        self.timezone_combo.setFixedWidth(int(self._layout["world_combo_width"]))
        self.timezone_combo.addItems([name for name, _ in self._options])
        world_clock_layout.addWidget(self.timezone_combo)

        # 世界时钟显示（强调色经 QPalette，随深浅主题用同色值）
        self.world_clock_label = QLabel("00:00:00")
        self.world_clock_label.setFont(
            QFont(ui["font_family"], int(self._layout["world_time_font_size"]), QFont.Weight.Bold)
        )
        accent = QColor(ui["colors"]["accent"])
        palette = self.world_clock_label.palette()
        palette.setColor(QPalette.ColorRole.WindowText, accent)
        self.world_clock_label.setPalette(palette)
        world_clock_layout.addWidget(self.world_clock_label)

        world_clock_layout.addStretch()

        # 时区对象缓存（按时区名复用，避免 10Hz 重复构建）与秒级刷新标记
        self._tz_cache: dict[str, Any] = {}
        self._last_second = -1
        self._last_tz_name: str | None = None

    def update_world_clock(self) -> None:
        # 同秒跳过刷新（时区切换时强制），pytz 对象按名缓存；失败降级 00:00:00
        tz_name = self._current_iana()
        if not tz_name:
            return

        try:
            # 同秒且时区未变时跳过：主窗口 10Hz tick 只重算 1Hz
            now = datetime.datetime.now()
            if tz_name == self._last_tz_name and now.second == self._last_second:
                return
            self._last_second = now.second
            self._last_tz_name = tz_name

            # 缓存 pytz 时区对象，切换时区时按名重新构建
            tz = self._tz_cache.get(tz_name)
            if tz is None:
                tz = pytz.timezone(tz_name)
                self._tz_cache[tz_name] = tz

            world_time = now.astimezone(tz).strftime("%H:%M:%S")
            self.world_clock_label.setText(world_time)
        except Exception as e:
            # 统一 logger.exception 带堆栈（FIX001.23 P3#10：与其他面板回调风格一致）
            logger.exception(f"更新世界时钟时出错: {e}")
            self.world_clock_label.setText("00:00:00")

    def _current_iana(self) -> str:
        # 下拉当前索引 → IANA 标识（越界防御返回空串走降级）
        index = self.timezone_combo.currentIndex()
        if 0 <= index < len(self._options):
            return self._options[index][1]
        return ""

    def set_timezone(self, tz_name: str) -> None:
        # 按 IANA 标识线性查找选项表并定位下拉项（配置恢复用，S10.3 B1）
        for index, (_, iana) in enumerate(self._options):
            if iana == tz_name:
                self.timezone_combo.setCurrentIndex(index)
                return

    def current_timezone(self) -> str:
        # 当前 IANA 标识（越界回退静态配置默认时区，经接口读取）
        iana = self._current_iana()
        if iana:
            return iana
        return str(self._interface.get_app_static()["default_timezone"])


# ===== ui/panels/world_clock_panel.py 函数/类说明 =====
# WorldClockPanel(QWidget): 世界时钟面板
#   __init__(interface, parent): PL002 Fluent 重写——qfw ComboBox（索引即选项表下标，
#   不依赖 itemData）；时区强调色改 QPalette（样式表写法已随 QSS 管线退役 PL002.09）
#   update_world_clock(): 由主窗口时钟 tick 调用，pytz 换算当前时区时间（渲染逻辑保留面板）
#   _current_iana(): 当前下拉索引 → IANA 标识（越界返回空串）
#   set_timezone(tz_name): 按 IANA 标识定位下拉项（配置恢复用，S10.3 B1）
#   current_timezone(): 供主窗口 save_settings 持久化时区选择（回退默认时区经接口读取）
#   异常处理：pytz 转换失败降级显示 00:00:00 并记录日志
#   关联配置：时区表 data/timezones.py 经接口转出；字体/强调色/默认时区经接口读取
