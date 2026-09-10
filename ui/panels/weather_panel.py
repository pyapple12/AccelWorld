# 天气面板模块（S5 后台化：查询移入 QThreadPool，UI 不阻塞）
# PL001（plan#UI2.0）：天气查询/格式化/城市表经 AppInterface 读取（铁律 2），
# QThreadPool 线程编排保留在面板（接口保持同步拉取式，铁律 3）

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

from interface import AppInterface
from interface.types import WeatherData

# 配置日志
logger = logging.getLogger(__name__)


class _WeatherTaskSignals(QObject):
    finished = pyqtSignal(object, object)  # (city_name, WeatherData | None)


class _WeatherTask(QRunnable):
    def __init__(self, interface: AppInterface, city_name: str):
        # 记录接口引用与目标城市并创建信号载体（查询经接口同步拉取，线程由本任务承载）
        super().__init__()
        self._interface = interface
        self.city_name = city_name
        self.signals = _WeatherTaskSignals()

    def run(self) -> None:
        # 在线程池中执行查询（经接口），异常兜底记录并降级返回，保证 UI 不卡"获取天气中..."
        try:
            result = self._interface.fetch_weather(self.city_name)
        except Exception as e:
            logger.exception(f"后台天气查询异常: {e}")
            result = None
        self.signals.finished.emit(self.city_name, result)


class WeatherPanel(QWidget):
    theme_toggled = pyqtSignal()  # 主题切换请求信号

    def __init__(
        self,
        interface: AppInterface,
        parent: QWidget | None = None,
        initial_city: str | None = None,
    ):
        # 构建城市下拉/天气标签/主题与刷新按钮，并启动自动刷新定时器；
        # initial_city 为恢复的持久化城市（FIX002.18：首查即用恢复值，消除启动双请求）
        super().__init__(parent)
        self._interface = interface

        ui = interface.get_ui_static()
        default_city = interface.get_app_static()["default_city"]

        self.current_city = initial_city or default_city
        self._weather_pool = QThreadPool.globalInstance()

        weather_frame = QFrame()
        weather_frame.setFrameShape(QFrame.Shape.StyledPanel)
        weather_layout = QHBoxLayout(weather_frame)

        # 城市选择
        city_label = QLabel("城市:")
        city_label.setFont(QFont(ui["font_family"], 12))
        weather_layout.addWidget(city_label)

        city_names = interface.get_city_names()
        self.city_combo = QComboBox()
        self.city_combo.setFont(QFont(ui["font_family"], 12))
        self.city_combo.setFixedWidth(120)
        self.city_combo.addItems(city_names)
        # 初始城市写入下拉框：列表内直接选中；列表外补入并屏蔽信号
        # （FIX002.18：替换原 default_city 占位 setText，避免恢复路径二次查询）
        if self.current_city in city_names:
            self.city_combo.setCurrentText(self.current_city)
        else:
            # 列表外城市仅当次会话保留于下拉框（会话级展示项，FIX002.18 接受语义）
            self.city_combo.blockSignals(True)
            self.city_combo.addItem(self.current_city)
            self.city_combo.setCurrentText(self.current_city)
            self.city_combo.blockSignals(False)
        self.city_combo.currentTextChanged.connect(self.on_city_changed)
        weather_layout.addWidget(self.city_combo)

        # 天气图标与信息
        self.weather_icon_label = QLabel("☀️")
        self.weather_icon_label.setFont(QFont(ui["font_family"], 24))
        weather_layout.addWidget(self.weather_icon_label)

        self.weather_info_label = QLabel("获取天气中...")
        self.weather_info_label.setFont(QFont(ui["font_family"], 12))
        weather_layout.addWidget(self.weather_info_label)

        weather_layout.addStretch()

        # 主题切换按钮
        self.theme_button = QPushButton("🌙")
        self.theme_button.setFont(QFont(ui["font_family"], 12))
        self.theme_button.setFixedSize(36, 36)
        self.theme_button.setToolTip("切换主题")
        self.theme_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.theme_button.clicked.connect(self.theme_toggled.emit)
        weather_layout.addWidget(self.theme_button)

        # 刷新天气按钮
        self.refresh_weather_button = QPushButton("刷新")
        self.refresh_weather_button.setFont(QFont(ui["font_family"], 10))
        self.refresh_weather_button.clicked.connect(self.update_weather)
        weather_layout.addWidget(self.refresh_weather_button)

        outer = QVBoxLayout(self)
        outer.addWidget(weather_frame)

        # 自动刷新（周期 = 缓存 TTL 毫秒，经接口派生，单源配置避免双键漂移，E15）
        self.weather_timer: QTimer = QTimer(self)
        self.weather_timer.timeout.connect(self.update_weather)
        self.weather_timer.start(interface.get_weather_refresh_interval_ms())

        # 启动即发起首次查询（FIX001.5；FIX002.18 起查询城市即恢复的持久化城市，
        # main_window 不再二次 set_city，启动期仅此一次请求）
        self.update_weather()

    def update_weather(self) -> None:
        # 置过渡态后提交 QThreadPool 任务（查询经接口），UI 不阻塞
        self.weather_info_label.setText("获取天气中...")
        self.weather_icon_label.setText("⏳")
        task = _WeatherTask(self._interface, self.current_city)
        task.signals.finished.connect(self._on_weather_result)
        # globalInstance 运行时恒非 None（stub 标注 Optional，行级压制）
        self._weather_pool.start(task)  # pyright: ignore[reportOptionalMemberAccess]

    def _on_weather_result(
        self, city_name: str, weather: Optional[WeatherData]
    ) -> None:
        # 城市已切换时丢弃过期结果，避免旧数据覆盖新城市显示；格式化经接口
        if city_name != self.current_city:
            return
        try:
            if weather:
                self.weather_info_label.setText(
                    self._interface.format_weather_display(city_name, weather)
                )
                self.weather_icon_label.setText(weather.icon)
            else:
                self.weather_info_label.setText("天气获取失败")
                self.weather_icon_label.setText("❓")
        except Exception as e:
            logger.exception(f"更新天气显示时出错: {e}")
            self.weather_info_label.setText("天气获取失败")
            self.weather_icon_label.setText("❓")

    def set_city(self, city_name: str) -> None:
        # 列表内 setCurrentText 触发联动查询；列表外补入下拉框后直设并发起查询
        # （FIX001.23：此前列表外城市下拉框仍显示旧城市，与实际查询不一致）
        if city_name in self._interface.get_city_names():
            self.city_combo.setCurrentText(city_name)
            return
        if self.city_combo.findText(city_name) < 0:
            self.city_combo.blockSignals(True)
            self.city_combo.addItem(city_name)
            self.city_combo.blockSignals(False)
        self.city_combo.blockSignals(True)
        self.city_combo.setCurrentText(city_name)
        self.city_combo.blockSignals(False)
        self.current_city = city_name
        self.update_weather()

    def set_theme_button(self, is_dark: bool) -> None:
        # 深色显示☀️（切换至浅色），浅色显示🌙（切换至深色）
        if is_dark:
            self.theme_button.setText("☀️")
            self.theme_button.setToolTip("切换到浅色主题")
        else:
            self.theme_button.setText("🌙")
            self.theme_button.setToolTip("切换到深色主题")

    def on_city_changed(self, city_name: str) -> None:
        # 记录当前城市并发起查询
        self.current_city = city_name
        self.update_weather()

    def current_city_name(self) -> str:
        # 直接返回内部城市状态
        return self.current_city


# ===== ui/panels/weather_panel.py 函数/类说明 =====
# _WeatherTask(QRunnable): 后台查询任务，持接口引用与城市名，完成后发 finished(city, result)
#   设计理由：接口保持同步拉取式（plan#UI2.0 铁律 3），线程编排留在 UI 层
# _WeatherTaskSignals(QObject): 任务信号载体（跨线程排队回 GUI 线程）
# WeatherPanel(QWidget): 天气面板
#   信号：theme_toggled 主题切换请求（主窗口负责应用 QSS）
#   __init__(interface, parent, initial_city): 城市表/默认城市/刷新周期/样式经接口读取
#   update_weather(): 提交后台任务立即返回，UI 不因网络阻塞（修复 D5）
#   _on_weather_result(city, weather): 回调更新标签；城市已切换则丢弃过期结果；
#     展示文本经 AppInterface.format_weather_display 格式化
#   set_city()/set_theme_button()/on_city_changed()/current_city_name(): 见 S4
#   设计理由：QThreadPool 全局实例复用线程；信号跨线程自动排队，避免手动锁；
#   面板零后端 import（plan#UI2.0 铁律 2）
#   异常处理：查询失败在 service 层返回 None，回调显示失败文案
#   关联配置：last_city 配置项由主窗口经接口持久化；城市表/默认城市/缓存 TTL 经接口读取
