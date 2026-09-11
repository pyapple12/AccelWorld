# 主窗口模块（S4 重构为面板装配器；PL002 FluentWindow 基座 + 三态主题；
# PL003 视觉迭代：单页拆分为侧栏六导航页 + Acrylic 背板 + 主题控制迁设置页）
# PL001（plan#UI2.0）：后端访问一律经 AppInterface 注入，本文件零后端 import；
# 保留 UI 编排：去抖定时器、闹钟播放/通知编排、快捷键、托盘接线、系统主题侦听

import logging
import os
from typing import Any

# 配置日志
logger = logging.getLogger(__name__)

from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QCloseEvent, QColor, QKeySequence, QShortcut
from PyQt6.QtWidgets import QApplication, QVBoxLayout, QWidget

from qfluentwidgets import (
    FluentIcon,
    FluentWindow,
    NavigationItemPosition,
    SystemThemeListener,
    Theme,
    isDarkTheme,
    setTheme,
    setThemeColor,
)

from interface import AppInterface
from interface.types import Alarm
from ui.audio_player import play_alarm_sound_async
from ui.backdrop import enable_acrylic
from ui.system_tray import SystemTray
from ui.panels.clock_panel import ClockPanel
from ui.panels.date_panel import DatePanel
from ui.panels.countdown_panel import CountdownPanel
from ui.panels.world_clock_panel import WorldClockPanel
from ui.panels.weather_panel import WeatherPanel
from ui.panels.alarm_panel import AlarmPanel
from ui.panels.settings_panel import SettingsPanel

# 主题偏好 → qfw Theme 映射与三态循环顺序（跟随系统 → 浅色 → 深色 → 跟随系统，PL002.03）
_THEME_SWITCH = {
    "auto": Theme.AUTO,
    "light": Theme.LIGHT,
    "dark": Theme.DARK,
}
_THEME_NEXT = {"auto": "light", "light": "dark", "dark": "auto"}


class AcceleratedWorldGUI(FluentWindow):
    def __init__(self, interface: AppInterface):
        # 接口注入→偏好恢复→主题应用→多页装配→信号→恢复→定时器→快捷键→托盘→材质→侦听
        super().__init__()
        self._interface = interface

        # 静态配置（窗口默认几何/快捷键/去抖周期/主题色等 UI 编排参数，经接口只读引用）
        self._base = interface.get_app_static()
        self._colors = interface.get_ui_static()["colors"]

        # 启动偏好快照（主题三态/城市/时区/倒计时，非法主题已在接口内归一化回退）
        prefs = interface.get_ui_preferences()

        # 倍率写盘去抖状态（FIX001.23：拖动滑杆高频变化仅停止后落盘一次；
        # 实时生效经接口 set_rate，落盘经接口 apply_rate，去抖定时器属 UI 编排保留在此）
        self._rate_save_timer: QTimer | None = None
        self._pending_rate: float | None = None

        self.setWindowTitle(f"加速世界 - 时间膨胀时钟 {interface.get_version()}")

        # 恢复窗口位置和大小（默认几何来自静态配置；FluentWindow 无边框窗口同样支持）
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

        # ------------------- 主题（三态偏好，qfw 内建深浅样式，PL002.03/09） -------------------
        # 先应用 qfw 全局主题（qfw 组件样式在绘制期解析，早应用保证装配期语义一致）；
        # 设置页选中态在面板装配后由 _apply_theme_preference 的收尾段同步
        setTheme(_THEME_SWITCH[prefs.theme])
        setThemeColor(QColor(self._colors["primary"]))
        self.theme_pref = prefs.theme
        self.is_dark_theme = bool(isDarkTheme())

        # ------------------- 面板装配（接口注入，单页拆多页，PL003.01） -------------------
        self.clock_panel = ClockPanel(interface)
        self.date_panel = DatePanel(interface)
        self.countdown_panel = CountdownPanel(interface)
        self.world_clock_panel = WorldClockPanel(interface)
        # 天气面板以恢复城市作为初始城市（首查即用恢复值，FIX002.18 消除启动双请求）
        self.weather_panel = WeatherPanel(interface, initial_city=prefs.last_city)
        self.alarm_panel = AlarmPanel(interface)
        self.settings_panel = SettingsPanel(interface)

        # 每页容器（统一页边距；addSubInterface 要求非空 objectName）
        page_tokens = interface.get_ui_static()["layout"]
        self._layout_tokens = page_tokens  # 导航展开宽度等延后触发布局所需（PL005.05）
        self.addSubInterface(
            self._make_page("page-clock", page_tokens, self.clock_panel, self.date_panel),
            FluentIcon.HOME,
            "时钟",
        )
        self.addSubInterface(
            self._make_page("page-countdown", page_tokens, self.countdown_panel),
            FluentIcon.STOP_WATCH,
            "倒计时",
        )
        self.addSubInterface(
            self._make_page("page-world", page_tokens, self.world_clock_panel),
            FluentIcon.GLOBE,
            "世界时钟",
        )
        self.addSubInterface(
            self._make_page("page-weather", page_tokens, self.weather_panel),
            FluentIcon.CLOUD,
            "天气",
        )
        self.addSubInterface(
            self._make_page("page-alarm", page_tokens, self.alarm_panel),
            FluentIcon.RINGER,
            "闹钟",
        )
        self.addSubInterface(
            self._make_page("page-settings", page_tokens, self.settings_panel),
            FluentIcon.SETTING,
            "设置",
            position=NavigationItemPosition.BOTTOM,
        )

        # 面板就绪后同步设置页选中态（auto=跟随系统/light/dark）
        self.settings_panel.sync_theme(self.theme_pref)

        # 导航常开（PL005.05）：延后到事件循环首拍（show 之后）触发；探针定案
        # （.temp/probe_pl005_nav6）——__init__ 内 pre-show 调用会污染 qfw
        # NavigationPanel 状态机（displayMode 卡 MENU，内容区不让位），show 后调用
        # 则正常内联展开；窗口过窄时 qfw 自行走 MENU 覆盖模式，不顶开内容
        QTimer.singleShot(0, self._expand_navigation)

        # ------------------- 信号连接 -------------------
        self.clock_panel.rate_changed.connect(self._on_rate_changed)
        self.settings_panel.theme_selected.connect(self._on_theme_selected)
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

        # ------------------- 快捷键（键位来自静态配置，T004.1） -------------------
        self._install_shortcuts()

        # ------------------- 系统托盘 -------------------
        self.tray = SystemTray(interface, parent=self)
        self.tray.show_requested.connect(self.show_normal)
        self.tray.hide_requested.connect(self.hide_to_tray)
        self.tray.quit_requested.connect(self.quit_app)
        # 初始倍率同步托盘菜单（持久化值 ≠ 默认值时菜单不再显示错值，FIX002.9）
        self.tray.update_rate(interface.get_rate())

        # ------------------- 系统深浅色变更侦听（PL002.10：AUTO 模式实时跟随） -------------------
        self._theme_listener = SystemThemeListener(self)
        self._theme_listener.systemThemeChanged.connect(self._on_system_theme_changed)
        self._theme_listener.start()

    def _expand_navigation(self) -> None:
        # 导航图标+文字常开（PL005.05）：由 __init__ 的 singleShot(0) 在事件循环
        # 首拍（show 之后）调用；展开宽度入 ui.json layout 节 token
        self.navigationInterface.setExpandWidth(int(self._layout_tokens["nav_expanded_width"]))
        self.navigationInterface.expand(False)

    @staticmethod
    def _make_page(object_name: str, layout_tokens: dict, *widgets: QWidget) -> QWidget:
        # 包装导航页容器：统一页边距并设 objectName（addSubInterface 硬要求，PL003.01）；
        # 边距/间距经 layout tokens（PL004.01）
        page = QWidget()
        page.setObjectName(object_name)
        page_layout = QVBoxLayout(page)
        page_layout.setContentsMargins(*layout_tokens["page_margin"])
        page_layout.setSpacing(int(layout_tokens["page_spacing"]))
        for widget in widgets:
            # 显式顶锚（PL005.04）：页面容器不再与尾部 spacer 均分富余空间，
            # 消除各页内容锚点漂移（闹钟页居中/世界时钟页 1/4 高度等异型）
            page_layout.addWidget(widget, 0, Qt.AlignmentFlag.AlignTop)
        page_layout.addStretch()
        return page

    # ------------------- 主题（三态） -------------------

    def _apply_theme_preference(self, theme_pref: str) -> None:
        # 应用主题偏好三态：setTheme 映射（AUTO 由 qfw 解析系统深浅）+ 主题色 +
        # 生效深浅状态 + Acrylic 背板按新深浅重铺 + 设置页选中态同步
        setTheme(_THEME_SWITCH[theme_pref])
        setThemeColor(QColor(self._colors["primary"]))
        self.theme_pref = theme_pref
        self.is_dark_theme = bool(isDarkTheme())
        enable_acrylic(self)  # 失败内部静默降级（无头/不支持环境）
        if getattr(self, "settings_panel", None) is not None:
            self.settings_panel.sync_theme(theme_pref)

    def _on_theme_selected(self, theme_pref: str) -> None:
        # 设置页选择器路径（PL003.02）：与应用/持久化；与快捷键循环路径并存
        if theme_pref == self.theme_pref:
            return
        self._apply_theme_preference(theme_pref)
        self._interface.save_theme(theme_pref)

    def toggle_theme(self) -> None:
        # 三态循环（跟随系统→浅→深→跟随系统，PL002.03）、应用并持久化（FIX001.11 接线保持）
        next_pref = _THEME_NEXT[self.theme_pref]
        self._apply_theme_preference(next_pref)
        self._interface.save_theme(next_pref)

    def _on_system_theme_changed(self) -> None:
        # 系统深浅色变更（侦听器线程信号）：AUTO 模式下重新解析生效主题并同步状态
        if self.theme_pref == "auto":
            setTheme(Theme.AUTO)
            self.is_dark_theme = bool(isDarkTheme())

    # ------------------- 时钟调度 -------------------

    def update_clock(self) -> None:
        # tick 定时器驱动（周期随倍率联动，T001.1），异常不外抛仅记录日志；
        # 时间信息经接口拉取（plan#UI2.0 铁律 3）；隐藏页照常更新（开销可忽略，保持简单）
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

        # 应用启动主题（auto/light/dark 三态均生效并持久化；PL002.02 语义升级）
        if theme in ("auto", "light", "dark"):
            self._apply_theme_preference(theme)
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
    # 事件循环正常结束后必须绕过 Python 退出析构（y.problems#6：实测 Qt 对象在
    # 解释器退出阶段析构顺序错误导致 0xC0000409 硬崩，温和清理/deleteLater 均无效，
    # 唯一有效缓解为 os._exit，PL004.03 矩阵实验定案）。数据无丢失风险：
    # 配置/闹钟/几何已在 quit 前经接口落盘（write_json 原子写、日志逐条 flush）
    os._exit(0)


# ===== ui/main_window.py 函数/类说明 =====
# _THEME_SWITCH/_THEME_NEXT: 主题偏好→qfw Theme 映射与三态循环顺序表（PL002.03）
# AcceleratedWorldGUI(FluentWindow): 主窗口装配器
#   __init__(interface): 接口注入（plan#UI2.0 装配点）→ 偏好快照 → 主题应用 → 面板构建
#   → 六导航页装配（时钟/倒计时/世界时钟/天气/闹钟 + 设置置底，PL003.01 方案 A 拆页）
#   → 信号 → 恢复时区/倒计时/闹钟 → 定时器（周期随倍率）→ 快捷键 → 托盘
#   → SystemThemeListener（AUTO 深浅跟随，PL002.10）
#   设计理由：窗口自身零后端 import（后端访问全部经 self._interface，PL001.08 保持）；
#   深浅样式由 qfw 内建；Acrylic 背板经 ui/backdrop.py（PL003.03，随主题重铺）
#   _make_page(object_name, *widgets): 导航页容器工厂（统一页边距 + objectName 硬要求；
#   PL005.04 显式 AlignTop 顶锚，消除内容锚点漂移）
#   _expand_navigation(): 导航图标+文字常开（PL005.05，singleShot(0) 延后到 show 后；
#   pre-show 调用污染 qfw 状态机致内容不让位，探针定案见 .temp/probe_pl005_nav6）
#   _apply_theme_preference(theme_pref): setTheme 三态映射 + setThemeColor +
#   is_dark_theme 生效状态 + Acrylic 重铺 + 设置页选中态同步（构造期早应用经
#   getattr 守卫跳过面板同步）
#   _on_theme_selected(theme_pref): 设置页选择器路径（与快捷键循环双路径并存，PL003.02）
#   toggle_theme(): 三态循环 → 应用 → 经接口持久化
#   _on_system_theme_changed(): AUTO 模式下系统主题变更重解析（PL002.10）
#   update_clock(): tick 经接口拉取 TimeInfo 分发时钟/日期/倒计时/世界时钟面板，推送托盘悬停
#   _on_rate_changed(rate)/_update_acceleration_rate(rate)/_flush_pending_rate():
#     倍率信号 → 接口 set_rate 实时生效（越界拒绝）→ 去抖定时器 → apply_rate 内聚落盘
#   _save_alarms(): 接口 save_alarm_dicts 落盘（失败上浮托盘提示 FIX001.19）
#   _on_alarm_triggered(alarm): 播放/通知/一次性禁用（接口 toggle_alarm 保持状态单一路径）
#   _install_shortcuts(): 挂载窗口级快捷键（Ctrl+S 保存/Ctrl+Q 退出/Ctrl+T 主题，T004.1）
#   hide_to_tray()/show_normal()/quit_app()/closeEvent(): 托盘交互与退出保存
#   save_settings(): 倍率/城市/时区/倒计时/几何经接口逐项落盘
#   apply_startup_args(rate/theme/city): 启动参数应用（theme 三态）
# main_gui(interface, **kwargs): 创建应用/窗口/启动参数/显示/事件循环
#   关联配置：AppInterface（后端唯一入口）；ui/system_tray.py 托盘；
#   ui/audio_player.py 闹钟播放；ui/backdrop.py Acrylic 背板；ui/panels/settings_panel.py 设置页
