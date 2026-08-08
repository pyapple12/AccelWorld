# 主窗口模块（S4 重构为面板装配器：QTimer 调度 + 信号连接 + 主题/托盘）

import logging
import traceback

# 配置日志
logger = logging.getLogger(__name__)

# 程序版本号（单一来源：main.py）
from main import VERSION

from PyQt6.QtWidgets import QApplication, QMainWindow, QWidget, QVBoxLayout
from PyQt6.QtCore import QTimer
from PyQt6.QtGui import QCloseEvent

from config.settings import (
    load_config,
    get_setting,
    load_window_geometry,
    save_window_geometry,
    set_setting,
    get_alarms,
    save_alarms,
)
from modules.time_dilation import AcceleratedWorld
from modules.alarm_service import play_alarm_sound_async, Alarm
from data.cities import CITIES
from ui.themes import LIGHT_THEME, DARK_THEME, LIGHT_THEME_PROGRESS, DARK_THEME_PROGRESS
from ui.system_tray import SystemTray
from ui.panels.clock_panel import ClockPanel
from ui.panels.date_panel import DatePanel
from ui.panels.countdown_panel import CountdownPanel
from ui.panels.world_clock_panel import WorldClockPanel
from ui.panels.weather_panel import WeatherPanel
from ui.panels.alarm_panel import AlarmPanel


class AcceleratedWorldGUI(QMainWindow):
    """加速世界图形界面主窗口 - 面板装配器"""

    def __init__(self):
        """初始化窗口：加载配置、装配面板、连接信号、启动定时器与托盘"""
        super().__init__()

        # 加载配置
        saved_rate = get_setting("time_dilation_rate", 2.0)

        self.setWindowTitle(f"加速世界 - 时间膨胀时钟 {VERSION}")

        # 恢复窗口位置和大小
        geometry = load_window_geometry()
        if geometry:
            self.restoreGeometry(geometry)
        else:
            self.setGeometry(100, 100, 900, 500)

        # 创建加速世界核心实例
        self.accel_world = AcceleratedWorld(time_dilation_rate=saved_rate)

        # 设置中心部件和主布局
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        self.main_layout = QVBoxLayout(central_widget)
        self.main_layout.setContentsMargins(20, 20, 20, 20)
        self.main_layout.setSpacing(10)

        # ------------------- 面板装配 -------------------
        self.clock_panel = ClockPanel()
        self.date_panel = DatePanel()
        self.countdown_panel = CountdownPanel()
        self.world_clock_panel = WorldClockPanel()
        self.weather_panel = WeatherPanel()
        self.alarm_panel = AlarmPanel()

        for panel in (
            self.clock_panel,
            self.date_panel,
            self.countdown_panel,
            self.world_clock_panel,
            self.weather_panel,
            self.alarm_panel,
        ):
            self.main_layout.addWidget(panel)

        # ------------------- 信号连接 -------------------
        self.clock_panel.rate_changed.connect(self._on_rate_changed)
        self.weather_panel.theme_toggled.connect(self.toggle_theme)
        self.alarm_panel.alarm_saved.connect(self._save_alarms)
        self.alarm_panel.alarm_triggered.connect(self._on_alarm_triggered)

        # 从配置加载闹钟
        self.alarm_panel.load_alarms(get_alarms())

        # ------------------- 时钟定时器（100ms 刷新） -------------------
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_clock)
        self.timer.start(100)

        # ------------------- 主题（默认浅色） -------------------
        self.is_dark_theme = False
        self.apply_theme()

        # ------------------- 系统托盘 -------------------
        self.tray = SystemTray(version=VERSION, parent=self)
        self.tray.show_requested.connect(self.show_normal)
        self.tray.hide_requested.connect(self.hide_to_tray)
        self.tray.quit_requested.connect(self.quit_app)

    # ------------------- 时钟调度 -------------------

    def update_clock(self) -> None:
        """时钟 tick：获取时间信息并分发到各面板刷新"""
        try:
            info = self.accel_world.get_custom_time()
            self.clock_panel.update_time(info)
            self.date_panel.update_time(info)
            self.countdown_panel.update_countdown()
            self.world_clock_panel.update_world_clock()
        except Exception as e:
            logger.error(f"更新时钟时出错: {e}")
            traceback.print_exc()

    # ------------------- 倍率处理 -------------------

    def _on_rate_changed(self, rate: float) -> None:
        """倍率变化信号处理：重建核心实例并更新托盘显示"""
        self._update_acceleration_rate(rate)
        self.tray.update_rate(self.accel_world.time_dilation_rate)

    def _update_acceleration_rate(self, rate: float) -> None:
        """更新加速倍率（内部方法，同步持久化到配置）"""
        # 验证倍率是否在有效范围内
        if not (1.0 <= rate <= 20.0):
            return
        # 更新加速世界实例
        self.accel_world = AcceleratedWorld(time_dilation_rate=rate)
        # 同步保存倍率（滑杆/输入框/启动参数共用此路径）
        set_setting("time_dilation_rate", rate)

    # ------------------- 闹钟处理 -------------------

    def _save_alarms(self) -> None:
        """闹钟列表变更持久化"""
        save_alarms(self.alarm_panel.to_dict_list())

    def _on_alarm_triggered(self, alarm: Alarm) -> None:
        """闹钟触发处理：异步播放声音、通知、一次性闹钟自动禁用"""
        # 异步播放（预设铃声在后台线程，UI 不冻结）
        play_alarm_sound_async(alarm)

        # 显示通知（emoji 装饰避免 Windows 通知栏兼容问题）
        self.tray.show_notification(
            "Alarm", f" ⏰ {alarm.label} @ {alarm.time} ", "warning"
        )

        # 一次性闹钟触发后自动禁用
        if alarm.is_one_time():
            alarm.enabled = False
            self.alarm_panel.save_and_refresh()

    # ------------------- 主题 -------------------

    def toggle_theme(self) -> None:
        """切换主题"""
        self.is_dark_theme = not self.is_dark_theme
        self.apply_theme()

    def apply_theme(self) -> None:
        """应用当前主题（主窗口 QSS + 进度条样式 + 主题按钮图标）"""
        if self.is_dark_theme:
            self.setStyleSheet(DARK_THEME)
            self.clock_panel.set_progress_style(DARK_THEME_PROGRESS)
        else:
            self.setStyleSheet(LIGHT_THEME)
            self.clock_panel.set_progress_style(LIGHT_THEME_PROGRESS)
        self.weather_panel.set_theme_button(self.is_dark_theme)

    # ------------------- 托盘/窗口 -------------------

    def hide_to_tray(self) -> None:
        """隐藏到系统托盘"""
        self.hide()
        self.tray.showMessage(
            "加速世界",
            "程序已隐藏到系统托盘，点击托盘图标可重新显示",
            self.tray.MessageIcon.Information,
            2000,
        )

    def show_normal(self) -> None:
        """显示窗口"""
        self.show()
        self.raise_()
        self.activateWindow()

    def quit_app(self) -> None:
        """退出程序（先保存设置）"""
        self.save_settings()
        QApplication.quit()

    def closeEvent(self, a0: QCloseEvent) -> None:
        """关闭窗口事件 - 最小化到托盘而非退出"""
        if self.tray.isVisible():
            self.hide_to_tray()
            a0.ignore()
        else:
            # 保存配置
            self.save_settings()
            a0.accept()

    def save_settings(self) -> None:
        """保存当前设置"""
        set_setting("time_dilation_rate", self.accel_world.time_dilation_rate)
        set_setting("last_city", self.weather_panel.current_city_name())
        set_setting("last_timezone", self.world_clock_panel.current_timezone())
        set_setting("countdown_target", self.countdown_panel.get_target_text())
        save_window_geometry(self.saveGeometry())

    def apply_startup_args(
        self,
        rate: float | None = None,
        theme: str | None = None,
        city: str | None = None,
    ) -> None:
        """
        应用启动参数（rate/theme/city）

        :param rate: 时间膨胀倍率
        :param theme: 主题 ("light" 或 "dark")
        :param city: 默认城市
        """
        # 应用倍率（面板 set_rate 触发 rate_changed → 重建+保存+托盘更新）
        if rate is not None:
            self.clock_panel.set_rate(rate)
            self.tray.update_rate(self.accel_world.time_dilation_rate)

        # 应用默认城市
        if city:
            self.weather_panel.set_city(city)

        # 应用深色主题（直接设置状态后刷新样式）
        if theme == "dark":
            self.is_dark_theme = True
            self.apply_theme()


def main_gui(**kwargs) -> None:
    """
    图形界面主函数

    :param kwargs: 可选参数
        - rate: 时间膨胀倍率
        - theme: 主题 ("light" 或 "dark")
        - city: 默认城市
        - hidden: 是否隐藏到托盘
    """
    app = QApplication([])
    window = AcceleratedWorldGUI()

    # 应用启动参数（rate/theme/city）
    window.apply_startup_args(
        rate=kwargs.get("rate"),
        theme=kwargs.get("theme"),
        city=kwargs.get("city"),
    )

    if kwargs.get("hidden"):
        window.hide_to_tray()
    else:
        window.show()

    app.exec()


# ===== ui/main_window.py 函数/类说明 =====
# AcceleratedWorldGUI(QMainWindow): 主窗口装配器
#   __init__: 加载配置 → 装配 6 个面板 → 连接信号 → 闹钟加载 → 100ms 定时器 → 主题 → 托盘
#   update_clock(): tick 分发 TimeInfo 到时钟/日期/倒计时/世界时钟面板
#   _on_rate_changed(rate): 倍率信号 → 重建核心实例 + 持久化 + 托盘更新
#   _update_acceleration_rate(rate): 倍率验证/重建/保存共用路径
#   _save_alarms(): 闹钟变更持久化（alarm_saved 信号）
#   _on_alarm_triggered(alarm): 播放/通知/一次性禁用（alarm_triggered 信号）
#   toggle_theme()/apply_theme(): 主题切换（窗口 QSS + 进度条样式 + 按钮图标）
#   hide_to_tray()/show_normal()/quit_app(): 托盘交互（SystemTray 信号回调）
#   closeEvent(): 托盘可见时隐藏而非退出
#   save_settings(): 汇总各面板当前状态持久化
#   apply_startup_args(rate/theme/city): 启动参数应用
# main_gui(**kwargs): 创建应用/窗口/启动参数/显示/事件循环
#   设计理由：主窗口只做装配与调度，业务 UI 全部内聚在面板（signal/slot 解耦）
#   关联配置：config/settings.py 配置读写；ui/system_tray.py 托盘；ui/themes.py 样式
