# 主窗口模块（S4 重构为面板装配器：QTimer 调度 + 信号连接 + 主题/托盘）
# PL001（plan#UI2.0）：后端访问一律经 AppInterface 注入，本文件零后端 import；
# 保留 UI 编排：去抖定时器、闹钟播放/通知编排、主题翻转、快捷键、托盘接线

import logging
from typing import Any

# 配置日志
logger = logging.getLogger(__name__)

from PyQt6.QtWidgets import QApplication, QMainWindow, QWidget, QVBoxLayout
from PyQt6.QtCore import QTimer
from PyQt6.QtGui import QCloseEvent, QKeySequence, QShortcut

from interface import AppInterface
from interface.types import Alarm
from ui.audio_player import play_alarm_sound_async
from ui.themes import build_theme
from ui.system_tray import SystemTray
from ui.panels.clock_panel import ClockPanel
from ui.panels.date_panel import DatePanel
from ui.panels.countdown_panel import CountdownPanel
from ui.panels.world_clock_panel import WorldClockPanel
from ui.panels.weather_panel import WeatherPanel
from ui.panels.alarm_panel import AlarmPanel


class AcceleratedWorldGUI(QMainWindow):
    def __init__(self, interface: AppInterface):
        # 接口注入→偏好恢复→面板装配→信号→恢复→定时器→主题→快捷键→托盘
        super().__init__()
        self._interface = interface

        # 静态配置（窗口默认几何/快捷键/去抖周期等 UI 编排参数，经接口只读引用）
        self._base = interface.get_app_static()

        # 启动偏好快照（主题/城市/时区/倒计时，缺省值已在接口内回退 base.json 默认）
        prefs = interface.get_ui_preferences()

        # 倍率写盘去抖状态（FIX001.23：拖动滑杆高频变化仅停止后落盘一次；
        # 实时生效经接口 set_rate，落盘经接口 apply_rate，去抖定时器属 UI 编排保留在此）
        self._rate_save_timer: QTimer | None = None
        self._pending_rate: float | None = None

        self.setWindowTitle(f"加速世界 - 时间膨胀时钟 {interface.get_version()}")

        # 恢复窗口位置和大小（默认几何来自静态配置）
        geometry = interface.load_window_geometry()
        if geometry:
            self.restoreGeometry(geometry)
        else:
            self.setGeometry(
                int(self._base["window_x"]),
                int(self._base["window_y"]),
                int(self._base["window_width"]),
                int(self._base["window_height"]),
            )

        # 主题 QSS 构建一次按需取用（颜色经接口读取，工厂在 ui/themes，PL002 退役）
        colors = interface.get_ui_static()["colors"]
        self._qss_light, self._progress_light = build_theme(colors, dark=False)
        self._qss_dark, self._progress_dark = build_theme(colors, dark=True)

        # 设置中心部件和主布局
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        self.main_layout = QVBoxLayout(central_widget)
        self.main_layout.setContentsMargins(20, 20, 20, 20)
        self.main_layout.setSpacing(10)

        # ------------------- 面板装配（接口注入） -------------------
        self.clock_panel = ClockPanel(interface)
        self.date_panel = DatePanel(interface)
        self.countdown_panel = CountdownPanel(interface)
        self.world_clock_panel = WorldClockPanel(interface)
        # 天气面板以恢复城市作为初始城市（首查即用恢复值，FIX002.18 消除启动双请求）
        self.weather_panel = WeatherPanel(interface, initial_city=prefs.last_city)
        self.alarm_panel = AlarmPanel(interface)

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

        # 恢复上次时区（S10.3 B1：修复只存不读；天气城市已并入面板初始城市，FIX002.18）
        self.world_clock_panel.set_timezone(prefs.last_timezone)

        # 恢复倒计时目标（解析内部态避免下次保存被清空的跨会话丢失，S8.5/FIX001.10）
        if prefs.countdown_target:
            self.countdown_panel.restore_target(prefs.countdown_target)

        # 从配置加载闹钟（经接口载入管理器，所有权在接口，PL001.05）
        self.alarm_panel.load_alarms()

        # ------------------- 时钟定时器（周期随倍率联动，T001.1） -------------------
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_clock)
        self.timer.start(interface.get_tick_interval_ms())

        # ------------------- 主题（持久化恢复，FIX001.11） -------------------
        self.is_dark_theme = prefs.theme == "dark"
        self.apply_theme()

        # ------------------- 快捷键（键位来自静态配置，T004.1） -------------------
        self._install_shortcuts()

        # ------------------- 系统托盘 -------------------
        self.tray = SystemTray(interface, parent=self)
        self.tray.show_requested.connect(self.show_normal)
        self.tray.hide_requested.connect(self.hide_to_tray)
        self.tray.quit_requested.connect(self.quit_app)
        # 初始倍率同步托盘菜单（持久化值 ≠ 默认值时菜单不再显示错值，FIX002.9）
        self.tray.update_rate(interface.get_rate())

    # ------------------- 时钟调度 -------------------

    def update_clock(self) -> None:
        # tick 定时器驱动（周期随倍率联动，T001.1），异常不外抛仅记录日志；
        # 时间信息经接口拉取（plan#UI2.0 铁律 3）
        try:
            info = self._interface.get_time_info()
            self.clock_panel.update_time(info)
            self.date_panel.update_time(info)
            self.countdown_panel.update_countdown()
            self.world_clock_panel.update_world_clock()
            # 托盘悬停随 tick 实时显示加速时间与倍率（文本未变时托盘内部跳过，T004.4）
            self.tray.update_tooltip(info.custom_time, self._interface.get_rate())
        except Exception as e:
            # logger.exception 自带堆栈，单通道记录
            logger.exception(f"更新时钟时出错: {e}")

    # ------------------- 倍率处理 -------------------

    def _on_rate_changed(self, rate: float) -> None:
        # 面板信号触发，统一走 _update_acceleration_rate 校验生效；托盘同步当前倍率
        self._update_acceleration_rate(rate)
        self.tray.update_rate(self._interface.get_rate())

    def _update_acceleration_rate(self, rate: float) -> None:
        # 实时生效走接口轻量路径（范围校验 + 世界重建，越界被拒）；
        # 持久化去抖（FIX001.23：拖动 2.0→10.0 此前会写盘约 80 次，现仅停止后一次）
        if not self._interface.set_rate(rate):
            return
        # 刷新周期随倍率联动重启（加速秒周期 1000/倍率，修复 T001.1）
        self.timer.start(self._interface.get_tick_interval_ms())
        self._pending_rate = rate
        if self._rate_save_timer is None:
            self._rate_save_timer = QTimer(self)
            self._rate_save_timer.setSingleShot(True)
            self._rate_save_timer.timeout.connect(self._flush_pending_rate)
        self._rate_save_timer.start(int(self._base["rate_save_debounce_ms"]))

    def _flush_pending_rate(self) -> None:
        # 去抖定时器回调：经接口内聚动作落盘最近一次倍率（校验+重建+持久化；
        # 退出路径 save_settings 仍会兜底保存）
        if self._pending_rate is not None:
            self._interface.apply_rate(self._pending_rate)
            self._pending_rate = None

    # ------------------- 闹钟处理 -------------------

    def _save_alarms(self) -> None:
        # alarm_saved 信号回调，经接口将管理器列表落盘；失败上浮托盘提示（FIX001.19）
        if not self._interface.save_alarm_dicts():
            self.tray.show_notification(
                "保存失败", "闹钟写入失败，请检查磁盘空间或文件权限", "warning"
            )

    def _on_alarm_triggered(self, alarm: Alarm) -> None:
        # 异步播放（预设铃声在后台线程，UI 不冻结；播放编排保留 UI 层）
        play_alarm_sound_async(alarm, self._interface)

        # 显示通知（emoji 装饰避免 Windows 通知栏兼容问题）
        self.tray.show_notification(
            "Alarm", f" ⏰ {alarm.label} @ {alarm.time} ", "warning"
        )

        # 一次性闹钟触发后自动禁用（经接口翻转，保持管理器状态单一路径）
        if alarm.is_one_time():
            self._interface.toggle_alarm(alarm.id)
            self.alarm_panel.save_and_refresh()

    # ------------------- 主题 -------------------

    def toggle_theme(self) -> None:
        # 翻转状态、应用样式并持久化（FIX001.11：主题选择经接口写回配置）
        self.is_dark_theme = not self.is_dark_theme
        self.apply_theme()
        self._interface.save_theme("dark" if self.is_dark_theme else "light")

    def apply_theme(self) -> None:
        # 深浅主题三处联动：窗口样式、进度条、按钮图标（QSS 构造期已按主题缓存）
        if self.is_dark_theme:
            self.setStyleSheet(self._qss_dark)
            self.clock_panel.set_progress_style(self._progress_dark)
        else:
            self.setStyleSheet(self._qss_light)
            self.clock_panel.set_progress_style(self._progress_light)
        self.weather_panel.set_theme_button(self.is_dark_theme)

    # ------------------- 快捷键 -------------------

    def _install_shortcuts(self) -> None:
        # 窗口级三快捷键：保存/退出/主题切换（键位常量经接口读取，T004.1）
        shortcuts = self._base["shortcuts"]
        bindings = (
            (shortcuts["save"], self.save_settings),
            (shortcuts["quit"], self.quit_app),
            (shortcuts["theme"], self.toggle_theme),
        )
        for key, slot in bindings:
            shortcut = QShortcut(QKeySequence(key), self)
            shortcut.activated.connect(slot)

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
        # 汇总各面板当前状态经接口逐项落盘（倍率走 apply_rate 内聚动作）；
        # 失败上浮托盘提示（FIX001.19：磁盘满/只读等静默失败不再无感）
        save_ok = self._interface.apply_rate(self._interface.get_rate())
        save_ok = (
            self._interface.save_last_city(self.weather_panel.current_city_name())
            and self._interface.save_last_timezone(
                self.world_clock_panel.current_timezone()
            )
            and self._interface.save_countdown_target(
                self.countdown_panel.get_target_text()
            )
            and save_ok
        )
        # 窗口几何经接口 base64 封装单独落盘（QByteArray 运行时支持 bytes()，stub 未标注 Buffer 协议）
        save_ok = (
            self._interface.save_window_geometry(bytes(self.saveGeometry()))  # pyright: ignore[reportArgumentType]
            and save_ok
        )
        if not save_ok:
            self.tray.show_notification(
                "保存失败", "配置写入失败，请检查磁盘空间或文件权限", "warning"
            )

    def apply_startup_args(
        self,
        rate: float | None = None,
        theme: str | None = None,
        city: str | None = None,
    ) -> None:
        # 应用倍率（面板 set_rate 触发 rate_changed → 接口生效+去抖保存+托盘更新，F1）
        if rate is not None:
            self.clock_panel.set_rate(rate)

        # 应用默认城市
        if city:
            self.weather_panel.set_city(city)

        # 应用启动主题（light/dark 双分支均生效并持久化；FIX002.10：此前 light 在
        # 深色持久化下被静默忽略）
        if theme in ("dark", "light"):
            self.is_dark_theme = theme == "dark"
            self.apply_theme()
            self._interface.save_theme(theme)


def main_gui(interface: AppInterface, **kwargs: Any) -> None:
    # 创建应用与窗口（接口由装配方注入），应用启动参数后进入事件循环
    app = QApplication([])
    window = AcceleratedWorldGUI(interface)

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
#   __init__(interface): 接口注入（plan#UI2.0 三大块装配点）→ 偏好快照 → 装配 6 面板 →
#     连接信号 → 恢复时区/倒计时/闹钟 → 定时器（周期随倍率）→ 主题 → 快捷键 → 托盘
#     设计理由：窗口自身零后端 import（后端访问全部经 self._interface，PL001.08）；
#     主题 QSS 经 ui.themes.build_theme 参数化构建（构造期一次，切换零开销）
#   update_clock(): tick 经接口拉取 TimeInfo 分发时钟/日期/倒计时/世界时钟面板，推送托盘悬停
#   _on_rate_changed(rate): 倍率信号 → 接口生效 + 托盘更新
#   _update_acceleration_rate(rate): 接口 set_rate 实时生效（越界拒绝）+ 定时器重启 +
#     去抖定时器（FIX001.23；去抖属 UI 编排保留在此）
#   _flush_pending_rate(): 去抖回调，接口 apply_rate 内聚落盘（校验+重建+持久化）
#   _save_alarms(): 接口 save_alarm_dicts 落盘（失败上浮托盘提示 FIX001.19）
#   _on_alarm_triggered(alarm): 播放/通知/一次性禁用（接口 toggle_alarm 保持状态单一路径）
#   toggle_theme()/apply_theme(): 主题切换（缓存 QSS + 进度条 + 按钮图标；FIX001.11 持久化）
#   _install_shortcuts(): 挂载窗口级快捷键（Ctrl+S 保存/Ctrl+Q 退出/Ctrl+T 主题，T004.1）
#   hide_to_tray()/show_normal()/quit_app(): 托盘交互（SystemTray 信号回调）
#   closeEvent(): 托盘可见时隐藏而非退出
#   save_settings(): 倍率/城市/时区/倒计时/几何经接口逐项落盘（每项即写即存；
#     相比 E11 合并单写多几次小文件 IO，换取接口逐域契约的清晰性，PL001 权衡）
#   apply_startup_args(rate/theme/city): 启动参数应用
# main_gui(interface, **kwargs): 创建应用/窗口/启动参数/显示/事件循环
#   设计理由：主窗口只做装配与调度，业务 UI 全部内聚在面板（signal/slot 解耦）
#   关联配置：AppInterface（后端唯一入口）；ui/audio_player.py 闹钟播放；
#     ui/system_tray.py 托盘；ui/themes.py 主题工厂
