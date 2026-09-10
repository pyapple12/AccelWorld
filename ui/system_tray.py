# 系统托盘模块（S4 GUI 面板化拆分，托盘图标绘制、菜单、通知）

from PyQt6.QtCore import pyqtSignal, Qt
from PyQt6.QtWidgets import QSystemTrayIcon, QMenu
from PyQt6.QtGui import QIcon, QAction, QPixmap, QPainter, QPen, QColor, QBrush

from config.static.static_config import get_static_config

# 静态配置（托盘图标颜色/默认倍率）
_BASE = get_static_config().base
_UI = get_static_config().ui


class SystemTray(QSystemTrayIcon):
    show_requested = pyqtSignal()  # 请求显示窗口
    hide_requested = pyqtSignal()  # 请求隐藏到托盘
    quit_requested = pyqtSignal()  # 请求退出程序

    def __init__(self, parent=None):
        # 初始化图标/菜单/双击监听后显示托盘（版本来自静态配置，单一来源）
        super().__init__(parent)
        self.setToolTip(f"加速世界 - {get_static_config().base['version']}")
        self._create_icon()
        self._create_menu()
        self.activated.connect(self._on_activated)
        self.show()

    def _create_icon(self) -> None:
        # QPainter 画圆底+指针，透明背景（颜色来自静态配置）
        pixmap = QPixmap(32, 32)
        pixmap.fill(Qt.GlobalColor.transparent)  # 透明背景

        tray_color = QColor(_UI["colors"]["tray_blue"])
        painter = QPainter(pixmap)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.setPen(QPen(tray_color, 2))  # 蓝色边框
        painter.setBrush(QBrush(tray_color))
        painter.drawEllipse(2, 2, 28, 28)  # 圆形背景

        # 时钟指针（颜色经 ui.json 配置，FIX001.13）
        painter.setPen(
            QPen(QColor(_UI["colors"]["tray_hand"]), 2, Qt.PenStyle.SolidLine,
                 Qt.PenCapStyle.RoundCap)
        )
        painter.drawLine(16, 16, 16, 8)  # 分针
        painter.drawLine(16, 16, 22, 16)  # 时针
        painter.end()

        self.setIcon(QIcon(pixmap))

    def _create_menu(self) -> None:
        # 菜单动作经信号转发给主窗口处理
        self.tray_menu = QMenu()

        self.show_action = QAction("显示窗口", self)
        self.show_action.triggered.connect(self.show_requested.emit)
        self.tray_menu.addAction(self.show_action)

        self.hide_action = QAction("隐藏到托盘", self)
        self.hide_action.triggered.connect(self.hide_requested.emit)
        self.tray_menu.addAction(self.hide_action)

        self.tray_menu.addSeparator()

        # 当前倍率显示（只读；初始值取配置默认倍率，FIX001.23 P3#5）
        self.rate_action = QAction(f"当前倍率: {float(_BASE['default_rate']):.1f}x", self)
        self.rate_action.setEnabled(False)
        self.tray_menu.addAction(self.rate_action)

        self.tray_menu.addSeparator()

        self.quit_action = QAction("退出", self)
        self.quit_action.triggered.connect(self.quit_requested.emit)
        self.tray_menu.addAction(self.quit_action)

        self.setContextMenu(self.tray_menu)

    def _on_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        # 仅响应 DoubleClick，其他激活原因忽略
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
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
        # 图标类型映射后展示，时长来自静态配置（E13 参数化）
        icon_map = {
            "info": QSystemTrayIcon.MessageIcon.Information,
            "warning": QSystemTrayIcon.MessageIcon.Warning,
        }
        self.showMessage(
            title,
            message,
            icon_map.get(icon_kind, QSystemTrayIcon.MessageIcon.Information),
            int(get_static_config().base["notification_duration_ms"]),
        )


# ===== ui/system_tray.py 函数/类说明 =====
# SystemTray(QSystemTrayIcon): 系统托盘类
#   信号：show_requested/hide_requested/quit_requested（主窗口连接并处理）
#   _create_icon(): 用 QPainter 绘制蓝色圆形时钟图标（颜色来自 config/static/ui.json tray_blue）
#   _create_menu(): 显示/隐藏/倍率（只读）/退出菜单
#   _on_activated(reason): 双击托盘显示窗口
#   update_rate(rate): 倍率变化时更新菜单文本（主窗口经 rate 信号调用）
#   update_tooltip(accelerated_time, rate): tick 推送悬停文本（T004.4）
#     输入：加速时间字符串、当前倍率；输出：无（副作用为 setToolTip）
#     设计理由：文本未变化时跳过 setToolTip（tick 高频调用，托盘悬停无需逐帧重绘）；
#     只接收基础类型参数，托盘不依赖 modules 层
#     异常处理：无
#   show_notification(title, message, icon_kind): 封装 showMessage（时长来自 base.json）
#   设计理由：托盘职责独立成类，主窗口不再持有图标/菜单/绘制逻辑
#   关联配置：版本号来自 config/static/base.json base["version"]（版本迁移方案）；
#     颜色来自静态配置 ui.json
