# 主窗口模块（S4 重构为面板装配器：QTimer 调度 + 信号连接 + 主题/托盘）

import logging
from typing import Any

# 配置日志
logger = logging.getLogger(__name__)

from PyQt6.QtWidgets import QApplication, QMainWindow, QWidget, QVBoxLayout
from PyQt6.QtCore import QTimer
from PyQt6.QtGui import QCloseEvent, QKeySequence, QShortcut

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

        # 加载配置（持久化倍率脏值/越界值回退默认并记日志，FIX001.7）
        saved_rate = get_setting("time_dilation_rate", base["default_rate"])
        try:
            self.accel_world = AcceleratedWorld(time_dilation_rate=float(saved_rate))
        except (TypeError, ValueError):
            logger.warning(f"持久化倍率非法，已回退默认值: {saved_rate!r}")
            self.accel_world = AcceleratedWorld()

        # 倍率写盘去抖状态（FIX001.23：拖动滑杆高频变化仅停止后落盘一次）
        self._rate_save_timer: QTimer | None = None
        self._pending_rate: float | None = None

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
        # 天气面板以恢复城市作为初始城市（首查即用恢复值，FIX002.18 消除启动双请求）
        self.weather_panel = WeatherPanel(
            initial_city=get_setting("last_city", base["default_city"])
        )
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

        # 恢复上次时区（S10.3 B1：修复只存不读；天气城市已并入面板初始城市，FIX002.18）
        self.world_clock_panel.set_timezone(
            get_setting("last_timezone", base["default_timezone"])
        )

        # 恢复倒计时目标（解析内部态避免下次保存被清空的跨会话丢失，S8.5/FIX001.10）
        saved_countdown = get_setting("countdown_target", "")
        if saved_countdown:
            self.countdown_panel.restore_target(saved_countdown)

        # 从配置加载闹钟
        self.alarm_panel.load_alarms(get_alarms())

        # ------------------- 时钟定时器（周期随倍率联动，T001.1） -------------------
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_clock)
        self.timer.start(self.accel_world.tick_interval_ms)

        # ------------------- 主题（持久化恢复，FIX001.11） -------------------
        self.is_dark_theme = get_setting("theme", base["default_theme"]) == "dark"
        self.apply_theme()

        # ------------------- 快捷键（键位来自静态配置，T004.1） -------------------
        self._install_shortcuts()

        # ------------------- 系统托盘 -------------------
        self.tray = SystemTray(parent=self)
        self.tray.show_requested.connect(self.show_normal)
        self.tray.hide_requested.connect(self.hide_to_tray)
        self.tray.quit_requested.connect(self.quit_app)
        # 初始倍率同步托盘菜单（持久化值 ≠ 默认值时菜单不再显示错值，FIX002.9）
        self.tray.update_rate(self.accel_world.time_dilation_rate)

    # ------------------- 时钟调度 -------------------

    def update_clock(self) -> None:
        # tick 定时器驱动（周期随倍率联动，T001.1），异常不外抛仅记录日志
        try:
            info = self.accel_world.get_custom_time()
            self.clock_panel.update_time(info)
            self.date_panel.update_time(info)
            self.countdown_panel.update_countdown()
            self.world_clock_panel.update_world_clock()
            # 托盘悬停随 tick 实时显示加速时间与倍率（文本未变时托盘内部跳过，T004.4）
            self.tray.update_tooltip(
                info.custom_time, self.accel_world.time_dilation_rate
            )
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
        # 更新加速世界实例（重建立即生效，拖动滑杆保持实时响应）
        self.accel_world = AcceleratedWorld(time_dilation_rate=rate)
        # 刷新周期随倍率联动重启（加速秒周期 1000/倍率，修复 T001.1）
        self.timer.start(self.accel_world.tick_interval_ms)
        # 持久化写盘去抖（FIX001.23：拖动 2.0→10.0 此前会写盘约 80 次，现仅停止后一次）
        self._pending_rate = rate
        if self._rate_save_timer is None:
            self._rate_save_timer = QTimer(self)
            self._rate_save_timer.setSingleShot(True)
            self._rate_save_timer.timeout.connect(self._flush_pending_rate)
        self._rate_save_timer.start(int(base["rate_save_debounce_ms"]))

    def _flush_pending_rate(self) -> None:
        # 去抖定时器回调：落盘最近一次倍率（退出路径 save_settings 仍会兜底保存）
        if self._pending_rate is not None:
            set_setting("time_dilation_rate", self._pending_rate)
            self._pending_rate = None

    # ------------------- 闹钟处理 -------------------

    def _save_alarms(self) -> None:
        # alarm_saved 信号回调，导出管理器列表写入配置；失败上浮托盘提示（FIX001.19）
        if not save_alarms(self.alarm_panel.to_dict_list()):
            self.tray.show_notification(
                "保存失败", "闹钟写入失败，请检查磁盘空间或文件权限", "warning"
            )

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
        # 翻转状态、应用样式并持久化（FIX001.11：主题选择写回配置）
        self.is_dark_theme = not self.is_dark_theme
        self.apply_theme()
        set_setting("theme", "dark" if self.is_dark_theme else "light")

    def apply_theme(self) -> None:
        # 深浅主题三处联动：窗口样式、进度条、按钮图标
        if self.is_dark_theme:
            self.setStyleSheet(DARK_THEME)
            self.clock_panel.set_progress_style(DARK_THEME_PROGRESS)
        else:
            self.setStyleSheet(LIGHT_THEME)
            self.clock_panel.set_progress_style(LIGHT_THEME_PROGRESS)
        self.weather_panel.set_theme_button(self.is_dark_theme)

    # ------------------- 快捷键 -------------------

    def _install_shortcuts(self) -> None:
        # 窗口级三快捷键：保存/退出/主题切换（键位常量来自 base.json，T004.1）
        shortcuts = get_static_config().base["shortcuts"]
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
        # 合并字段单次写盘（E11：原 4 次 set_setting 各写一次，现 load 后改字段一次 save_config）；
        # 失败上浮托盘提示（FIX001.19：磁盘满/只读等静默失败不再无感）
        config = load_config()
        config.time_dilation_rate = self.accel_world.time_dilation_rate
        config.last_city = self.weather_panel.current_city_name()
        config.last_timezone = self.world_clock_panel.current_timezone()
        config.countdown_target = self.countdown_panel.get_target_text()
        save_ok = save_config(config)
        # 窗口几何经既有 base64 封装单独落盘（QByteArray 运行时支持 bytes()，stub 未标注 Buffer 协议）
        save_ok = (
            save_window_geometry(bytes(self.saveGeometry())) and save_ok  # pyright: ignore[reportArgumentType]
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
        # 应用倍率（面板 set_rate 触发 rate_changed → 重建+保存+托盘更新，无需重复 update_rate，F1）
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
            set_setting("theme", theme)


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
#   __init__: 加载配置 → 装配 6 个面板 → 连接信号 → 闹钟加载 → 定时器（周期随倍率）→ 主题 → 托盘
#   update_clock(): tick 分发 TimeInfo 到时钟/日期/倒计时/世界时钟面板，并推送托盘悬停（T004.4）
#   _on_rate_changed(rate): 倍率信号 → 重建核心实例 + 托盘更新
#   _update_acceleration_rate(rate): 倍率验证/重建/定时器重启共用路径（周期随倍率 T001.1）；
#     写盘去抖（FIX001.23）经 _pending_rate/_rate_save_timer/_flush_pending_rate 落盘
#   _flush_pending_rate(): 去抖定时器回调，落盘最近一次倍率（FIX001.23）
#   _save_alarms(): 闹钟变更持久化（alarm_saved 信号；失败上浮托盘提示 FIX001.19）
#   _on_alarm_triggered(alarm): 播放/通知/一次性禁用（alarm_triggered 信号）
#   toggle_theme()/apply_theme(): 主题切换（窗口 QSS + 进度条样式 + 按钮图标；FIX001.11 持久化）
#   _install_shortcuts(): 挂载窗口级快捷键（Ctrl+S 保存/Ctrl+Q 退出/Ctrl+T 主题，T004.1）
#   hide_to_tray()/show_normal()/quit_app(): 托盘交互（SystemTray 信号回调）
#   closeEvent(): 托盘可见时隐藏而非退出
#   save_settings(): 汇总各面板当前状态持久化
#   apply_startup_args(rate/theme/city): 启动参数应用
# main_gui(**kwargs): 创建应用/窗口/启动参数/显示/事件循环
#   设计理由：主窗口只做装配与调度，业务 UI 全部内聚在面板（signal/slot 解耦）
#   关联配置：config/settings.py 配置读写；ui/audio_player.py 闹钟播放；
#     ui/system_tray.py 托盘；ui/themes.py 样式
