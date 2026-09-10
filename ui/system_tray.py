# 系统托盘模块（S4 GUI 面板化拆分，托盘图标绘制、菜单、通知）
# PL002 Fluent 化：菜单改 qfw RoundMenu（右键 Context 激活时弹出，PL002.08）；
# 通知保留原生 showMessage（qfw 无系统托盘组件，todo 明示可保留原生）；
# PL001（plan#UI2.0）：版本号/颜色/通知时长/默认倍率经 AppInterface 读取，零后端 import

from PyQt6.QtCore import pyqtSignal, Qt
from PyQt6.QtGui import QCursor, QIcon, QAction, QPixmap, QPainter, QPen, QColor, QBrush
from PyQt6.QtWidgets import QSystemTrayIcon

from qfluentwidgets import RoundMenu

from interface import AppInterface


class SystemTray(QSystemTrayIcon):
    show_requested = pyqtSignal()  # 请求显示窗口
    hide_requested = pyqtSignal()  # 请求隐藏到托盘
    quit_requested = pyqtSignal()  # 请求退出程序

    def __init__(self, interface: AppInterface, parent=None):
        # 初始化图标/菜单/激活监听后显示托盘（版本/颜色/倍率/时长经接口读取）
        super().__init__(parent)
        self._interface = interface
        self._ui = interface.get_ui_static()
        self.setToolTip(f"加速世界 - {interface.get_version()}")
        self._create_icon()
        self._create_menu()
        self.activated.connect(self._on_activated)
        self.show()

    def _create_icon(self) -> None:
        # QPainter 画圆底+指针，透明背景（颜色来自接口提供的 ui.json 颜色表）
        pixmap = QPixmap(32, 32)
        pixmap.fill(Qt.GlobalColor.transparent)  # 透明背景

        tray_color = QColor(self._ui["colors"]["tray_blue"])
        painter = QPainter(pixmap)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.setPen(QPen(tray_color, 2))  # 蓝色边框
        painter.setBrush(QBrush(tray_color))
        painter.drawEllipse(2, 2, 28, 28)  # 圆形背景

        # 时钟指针（颜色经 ui.json 配置，FIX001.13）
        painter.setPen(
            QPen(QColor(self._ui["colors"]["tray_hand"]), 2, Qt.PenStyle.SolidLine,
                 Qt.PenCapStyle.RoundCap)
        )
        painter.drawLine(16, 16, 16, 8)  # 分针
        painter.drawLine(16, 16, 22, 16)  # 时针
        painter.end()

        self.setIcon(QIcon(pixmap))

    def _create_menu(self) -> None:
        # 菜单改 qfw RoundMenu（Fluent 风格弹层）；动作经信号转发给主窗口处理；
        # 未 setContextMenu，右键经 Context 激活在光标处弹出（PL002.08）
        self.tray_menu = RoundMenu(parent=self.parent())

        self.show_action = QAction("显示窗口", self)
        self.show_action.triggered.connect(self.show_requested.emit)
        self.tray_menu.addAction(self.show_action)

        self.hide_action = QAction("隐藏到托盘", self)
        self.hide_action.triggered.connect(self.hide_requested.emit)
        self.tray_menu.addAction(self.hide_action)

        self.tray_menu.addSeparator()

        # 当前倍率显示（只读；初始值取配置默认倍率，FIX001.23 P3#5）
        default_rate = float(self._interface.get_app_static()["default_rate"])
        self.rate_action = QAction(f"当前倍率: {default_rate:.1f}x", self)
        self.rate_action.setEnabled(False)
        self.tray_menu.addAction(self.rate_action)

        self.tray_menu.addSeparator()

        self.quit_action = QAction("退出", self)
        self.quit_action.triggered.connect(self.quit_requested.emit)
        self.tray_menu.addAction(self.quit_action)

    def _on_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        # 右键（Context）弹出 Fluent 菜单；双击显示窗口；其余忽略
        if reason == QSystemTrayIcon.ActivationReason.Context:
            self.tray_menu.popup(QCursor.pos())
        elif reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self.show_requested.emit()

    def update_rate(self, rate: float) -> None:
        # 倍率变化时同步只读菜单项文本
        self.rate_action.setText(f"当前倍率: {rate:.1f}x")

    def update_tooltip(self, accelerated_time: str, rate: float) -> None:
        # tick 推送悬停文本（加速时间+倍率，T004.4）；文本未变化时跳过，避免托盘重复重绘
        text = f"加速世界 - {accelerated_time} @ {rate:.1f}x"
        if text != self.toolTip():
            self.setToolTip(text)

    def show_notification(
        self, title: str, message: str, icon_kind: str = "info"
    ) -> None:
        # 保留原生 showMessage（qfw 无托盘通知能力，PL002.08 决策）；时长经接口读取（E13）
        icon_map = {
            "info": QSystemTrayIcon.MessageIcon.Information,
            "warning": QSystemTrayIcon.MessageIcon.Warning,
        }
        duration_ms = int(self._interface.get_app_static()["notification_duration_ms"])
        self.showMessage(
            title,
            message,
            icon_map.get(icon_kind, QSystemTrayIcon.MessageIcon.Information),
            duration_ms,
        )


# ===== ui/system_tray.py 函数/类说明 =====
# SystemTray(QSystemTrayIcon): 系统托盘类
#   信号：show_requested/hide_requested/quit_requested（主窗口连接并处理）
#   __init__(interface, parent): 版本号经 AppInterface.get_version()、颜色经 get_ui_static()
#   _create_icon(): 用 QPainter 绘制蓝色圆形时钟图标（颜色来自 ui.json tray_blue/tray_hand）
#   _create_menu(): qfw RoundMenu 菜单重建（显示/隐藏/倍率只读/退出，PL002.08）；
#     不调 setContextMenu，由 _on_activated 在右键时 popup 到光标处
#   _on_activated(reason): Context 弹菜单、DoubleClick 显示窗口
#   update_rate(rate): 倍率变化时更新菜单文本（主窗口经 rate 信号调用）
#   update_tooltip(accelerated_time, rate): tick 推送悬停文本（T004.4；文本未变跳过重绘）
#   show_notification(title, message, icon_kind): 保留原生 showMessage（时长经接口读取）
#   设计理由：托盘职责独立成类；PL002 决策——通知与图标保留原生（qfw 无系统托盘能力，
#   todo 明示可保留），仅菜单 Fluent 化；PL001 起零后端 import（plan#UI2.0 铁律 2）
#   关联配置：版本号/默认倍率/通知时长经接口读取（base.json）；颜色经接口（ui.json）
