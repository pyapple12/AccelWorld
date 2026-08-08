# 天气面板模块（S5 后台化：查询移入 QThreadPool，UI 不阻塞）

import logging
from typing import Optional

from PyQt6.QtCore import pyqtSignal, QTimer, QThreadPool, QRunnable, QObject
from PyQt6.QtWidgets import (
    QWidget,
    QHBoxLayout,
    QVBoxLayout,
    QFrame,
    QLabel,
    QComboBox,
    QPushButton,
)
from PyQt6.QtGui import QFont

from modules.weather_service import (
    get_weather_by_city,
    format_weather_info,
    WeatherData,
)
from data.cities import CITIES

# 配置日志
logger = logging.getLogger(__name__)


class _WeatherTaskSignals(QObject):
    """后台天气任务信号（跨线程回调）"""

    finished = pyqtSignal(object, object)  # (city_name, WeatherData | None)


class _WeatherTask(QRunnable):
    """后台天气查询任务（QThreadPool 执行，避免阻塞 GUI 线程）"""

    def __init__(self, city_name: str):
        super().__init__()
        self.city_name = city_name
        self.signals = _WeatherTaskSignals()

    def run(self) -> None:
        # 在线程池中执行网络查询，完成后带城市名发信号
        result = get_weather_by_city(self.city_name)
        self.signals.finished.emit(self.city_name, result)


class WeatherPanel(QWidget):
    """天气面板：城市选择、天气信息显示、主题切换与天气刷新（后台查询）"""

    theme_toggled = pyqtSignal()  # 主题切换请求信号

    def __init__(self, parent: QWidget | None = None):
        """初始化城市下拉、天气显示与 30 分钟自动刷新定时器"""
        super().__init__(parent)

        self.current_city = "北京"
        self._weather_pool = QThreadPool.globalInstance()

        weather_frame = QFrame()
        weather_frame.setFrameShape(QFrame.Shape.StyledPanel)
        weather_layout = QHBoxLayout(weather_frame)

        # 城市选择
        city_label = QLabel("城市:")
        city_label.setFont(QFont("Arial", 12))
        weather_layout.addWidget(city_label)

        self.city_combo = QComboBox()
        self.city_combo.setFont(QFont("Arial", 12))
        self.city_combo.setFixedWidth(120)
        self.city_combo.addItems(sorted(CITIES.keys()))
        self.city_combo.setCurrentText("北京")
        self.city_combo.currentTextChanged.connect(self.on_city_changed)
        weather_layout.addWidget(self.city_combo)

        # 天气图标与信息
        self.weather_icon_label = QLabel("☀️")
        self.weather_icon_label.setFont(QFont("Arial", 24))
        weather_layout.addWidget(self.weather_icon_label)

        self.weather_info_label = QLabel("获取天气中...")
        self.weather_info_label.setFont(QFont("Arial", 12))
        weather_layout.addWidget(self.weather_info_label)

        weather_layout.addStretch()

        # 主题切换按钮
        self.theme_button = QPushButton("🌙")
        self.theme_button.setFont(QFont("Arial", 12))
        self.theme_button.setFixedSize(36, 36)
        self.theme_button.setToolTip("切换主题")
        self.theme_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.theme_button.clicked.connect(self.theme_toggled.emit)
        weather_layout.addWidget(self.theme_button)

        # 刷新天气按钮
        self.refresh_weather_button = QPushButton("刷新")
        self.refresh_weather_button.setFont(QFont("Arial", 10))
        self.refresh_weather_button.clicked.connect(self.update_weather)
        weather_layout.addWidget(self.refresh_weather_button)

        outer = QVBoxLayout(self)
        outer.addWidget(weather_frame)

        # 每 30 分钟自动刷新天气
        self.weather_timer = QTimer(self)
        self.weather_timer.timeout.connect(self.update_weather)
        self.weather_timer.start(30 * 60 * 1000)

    def update_weather(self) -> None:
        """发起后台天气查询（立即返回，结果经信号回调更新）"""
        self.weather_info_label.setText("获取天气中...")
        self.weather_icon_label.setText("⏳")
        task = _WeatherTask(self.current_city)
        task.signals.finished.connect(self._on_weather_result)
        self._weather_pool.start(task)

    def _on_weather_result(
        self, city_name: str, weather: Optional[WeatherData]
    ) -> None:
        """后台查询结果回调（乱序结果丢弃）"""
        # 城市已切换时丢弃过期结果，避免旧数据覆盖新城市显示
        if city_name != self.current_city:
            return
        if weather:
            self.weather_info_label.setText(format_weather_info(weather, city_name))
            self.weather_icon_label.setText(weather.icon)
        else:
            self.weather_info_label.setText("天气获取失败")
            self.weather_icon_label.setText("❓")

    def set_city(self, city_name: str) -> None:
        """设置当前城市（启动参数）：列表内走下拉联动，列表外直接设置"""
        if city_name in CITIES:
            self.city_combo.setCurrentText(city_name)
        else:
            self.current_city = city_name
            self.update_weather()

    def set_theme_button(self, is_dark: bool) -> None:
        """同步主题按钮图标与提示（主题切换时由主窗口调用）"""
        if is_dark:
            self.theme_button.setText("☀️")
            self.theme_button.setToolTip("切换到浅色主题")
        else:
            self.theme_button.setText("🌙")
            self.theme_button.setToolTip("切换到深色主题")

    def on_city_changed(self, city_name: str) -> None:
        """城市选择变更回调"""
        self.current_city = city_name
        self.update_weather()

    def current_city_name(self) -> str:
        """获取当前城市名（供配置保存）"""
        return self.current_city


# ===== ui/panels/weather_panel.py 函数/类说明 =====
# _WeatherTask(QRunnable): 后台查询任务，携带城市名，完成后发 finished(city, result)
# _WeatherTaskSignals(QObject): 任务信号载体（跨线程排队回 GUI 线程）
# WeatherPanel(QWidget): 天气面板
#   信号：theme_toggled 主题切换请求（主窗口负责应用 QSS）
#   update_weather(): 提交后台任务立即返回，UI 不因网络阻塞（修复 D5）
#   _on_weather_result(city, weather): 回调更新标签；城市已切换则丢弃过期结果
#   set_city()/set_theme_button()/on_city_changed()/current_city_name(): 见 S4
#   设计理由：QThreadPool 全局实例复用线程；信号跨线程自动排队，避免手动锁
#   异常处理：查询失败在 service 层返回 None，回调显示失败文案
#   关联配置：last_city 配置项由主窗口持久化；城市表来自 data/cities.py
