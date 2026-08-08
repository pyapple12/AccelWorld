# 主窗口模块（S4 重构为面板装配器：QTimer 调度 + 信号连接 + 主题/托盘）

import logging
from typing import Any

# 配置日志
logger = logging.getLogger(__name__)

from PyQt6.QtWidgets import QApplication, QMainWindow, QWidget, QVBoxLayout
from PyQt6.QtCore import QTimer
from PyQt6.QtGui import QCloseEvent

from config.settings import (
    load_config,
    save_config,
    get_setting,
    set_setting,
    load_window_geometry,
    save_window_geometry,
    get_alarms,
    save_alarms,
)
from config.static.static_config import get_static_config
from modules.time_dilation import AcceleratedWorld
from modules.alarm_service import Alarm
from ui.audio_player import play_alarm_sound_async
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
    def __init__(self):
        # 配置→面板装配→信号→闹钟/天气/倒计时恢复→定时器→主题→托盘
        super().__init__()

        # 静态配置（倍率范围/窗口几何/时钟周期等参数）
        base = get_static_config().base

        # 加载配置
        saved_rate = get_setting("time_dilation_rate", base["default_rate"])

        self.setWindowTitle(f"加速世界 - 时间膨胀时钟 {base['version']}")

        # 恢复窗口位置和大小（默认几何来自静态配置）
        geometry = load_window_geometry()
        if geometry:
            self.restoreGeometry(geometry)
        else:
            self.setGeometry(
                int(base["window_x"]),
                int(base["window_y"]),
                int(base["window_width"]),
                int(base["window_height"]),
            )

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

        # 恢复上次城市/时区（S10.3 B1：修复只存不读；set_city 自带联动查询）
        self.weather_panel.set_city(get_setting("last_city", base["default_city"]))
        self.world_clock_panel.set_timezone(
            get_setting("last_timezone", base["default_timezone"])
        )

        # 恢复倒计时目标（S8.5：仅填充输入框显示，不自动启动计时）
        saved_countdown = get_setting("countdown_target", "")
        if saved_countdown:
            self.countdown_panel.countdown_target.setText(saved_countdown)

        # 从配置加载闹钟
        self.alarm_panel.load_alarms(get_alarms())

        # ------------------- 时钟定时器（周期来自静态配置） -------------------
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_clock)
        self.timer.start(int(get_static_config().base["clock_tick_ms"]))

        # ------------------- 主题（默认浅色） -------------------
        self.is_dark_theme = False
        self.apply_theme()

        # ------------------- 系统托盘 -------------------
        self.tray = SystemTray(parent=self)
        self.tray.show_requested.connect(self.show_normal)
        self.tray.hide_requested.connect(self.hide_to_tray)
        self.tray.quit_requested.connect(self.quit_app)

    # ------------------- 时钟调度 -------------------

    def update_clock(self) -> None:
        # 100ms 定时器驱动，异常不外抛仅记录日志
        try:
            info = self.accel_world.get_custom_time()
            self.clock_panel.update_time(info)
            self.date_panel.update_time(info)
            self.countdown_panel.update_countdown()
            self.world_clock_panel.update_world_clock()
        except Exception as e:
            # logger.exception 自带堆栈，单通道记录
            logger.exception(f"更新时钟时出错: {e}")

    # ------------------- 倍率处理 -------------------

    def _on_rate_changed(self, rate: float) -> None:
        # 面板信号触发，统一走 _update_acceleration_rate 校验保存
        self._update_acceleration_rate(rate)
        self.tray.update_rate(self.accel_world.time_dilation_rate)

    def _update_acceleration_rate(self, rate: float) -> None:
        # 验证倍率是否在有效范围内（范围来自静态配置）
        base = get_static_config().base
        if not (base["rate_min"] <= rate <= base["rate_max"]):
            return
        # 更新加速世界实例
        self.accel_world = AcceleratedWorld(time_dilation_rate=rate)
        # 同步保存倍率（滑杆/输入框/启动参数共用此路径）
        set_setting("time_dilation_rate", rate)

    # ------------------- 闹钟处理 -------------------

    def _save_alarms(self) -> None:
        # alarm_saved 信号回调，导出管理器列表写入配置
        save_alarms(self.alarm_panel.to_dict_list())

    def _on_alarm_triggered(self, alarm: Alarm) -> None:
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
        # 翻转状态后应用样式
        self.is_dark_theme = not self.is_dark_theme
        self.apply_theme()

    def apply_theme(self) -> None:
        # 深浅主题三处联动：窗口样式、进度条、按钮图标
        if self.is_dark_theme:
            self.setStyleSheet(DARK_THEME)
            self.clock_panel.set_progress_style(DARK_THEME_PROGRESS)
        else:
            self.setStyleSheet(LIGHT_THEME)
            self.clock_panel.set_progress_style(LIGHT_THEME_PROGRESS)
        self.weather_panel.set_theme_button(self.is_dark_theme)

    # ------------------- 托盘/窗口 -------------------

    def hide_to_tray(self) -> None:
        # 隐藏窗口并弹托盘通知提示
        self.hide()
        self.tray.show_notification(
            "加速世界",
            "程序已隐藏到系统托盘，点击托盘图标可重新显示",
            "info",
        )

    def show_normal(self) -> None:
        # 显示并置顶激活
        self.show()
        self.raise_()
        self.activateWindow()

    def quit_app(self) -> None:
        # 保存后退出事件循环
        self.save_settings()
        QApplication.quit()

    def closeEvent(self, a0: QCloseEvent) -> None:  # pyright: ignore[reportIncompatibleMethodOverride]
        # 行级忽略说明：PyQt6 stub 将参数标为 QCloseEvent|None，Qt 运行时恒传有效对象，
        # 改签名会引发函数体内 Optional 成员访问连锁报错，故局部压制（收紧检查策略）
        # 托盘可见时拦截为隐藏；否则保存设置放行退出
        if self.tray.isVisible():
            self.hide_to_tray()
            a0.ignore()
        else:
            # 保存配置
            self.save_settings()
            a0.accept()

    def save_settings(self) -> None:
        # 合并字段单次写盘（E11：原 4 次 set_setting 各写一次，现 load 后改字段一次 save_config）
        config = load_config()
        config.time_dilation_rate = self.accel_world.time_dilation_rate
        config.last_city = self.weather_panel.current_city_name()
        config.last_timezone = self.world_clock_panel.current_timezone()
        config.countdown_target = self.countdown_panel.get_target_text()
        save_config(config)
        # 窗口几何经既有 base64 封装单独落盘（QByteArray 运行时支持 bytes()，stub 未标注 Buffer 协议）
        save_window_geometry(bytes(self.saveGeometry()))  # pyright: ignore[reportArgumentType]

    def apply_startup_args(
        self,
        rate: float | None = None,
        theme: str | None = None,
        city: str | None = None,
    ) -> None:
        # 应用倍率（面板 set_rate 触发 rate_changed → 重建+保存+托盘更新，无需重复 update_rate，F1）
        if rate is not None:
            self.clock_panel.set_rate(rate)

        # 应用默认城市
        if city:
            self.weather_panel.set_city(city)

        # 应用深色主题（直接设置状态后刷新样式）
        if theme == "dark":
            self.is_dark_theme = True
            self.apply_theme()


def main_gui(**kwargs: Any) -> None:
    # 创建应用与窗口，应用启动参数后进入事件循环
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
#   关联配置：config/settings.py 配置读写；ui/audio_player.py 闹钟播放；
#     ui/system_tray.py 托盘；ui/themes.py 样式
