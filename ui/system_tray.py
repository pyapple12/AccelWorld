# 系统托盘模块（S4 GUI 面板化拆分，托盘图标绘制、菜单、通知）

from PyQt6.QtCore import pyqtSignal, Qt
from PyQt6.QtWidgets import QSystemTrayIcon, QMenu
from PyQt6.QtGui import QIcon, QAction, QPixmap, QPainter, QPen, QColor, QBrush


class SystemTray(QSystemTrayIcon):
    """系统托盘：图标绘制、菜单（显示/隐藏/倍率/退出）与通知"""

    show_requested = pyqtSignal()  # 请求显示窗口
    hide_requested = pyqtSignal()  # 请求隐藏到托盘
    quit_requested = pyqtSignal()  # 请求退出程序

    def __init__(self, version: str = "", parent=None):
        """创建托盘图标与菜单"""
        # 初始化图标/菜单/双击监听后显示托盘
        super().__init__(parent)
        self.setToolTip(f"加速世界 - {version}")
        self._create_icon()
        self._create_menu()
        self.activated.connect(self._on_activated)
        self.show()

    def _create_icon(self) -> None:
        """绘制 32x32 蓝色圆形时钟图标"""
        # QPainter 画圆底+指针，透明背景
        pixmap = QPixmap(32, 32)
        pixmap.fill(Qt.GlobalColor.transparent)  # 透明背景

        painter = QPainter(pixmap)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.setPen(QPen(QColor("#4A90D9"), 2))  # 蓝色边框
        painter.setBrush(QBrush(QColor("#4A90D9")))
        painter.drawEllipse(2, 2, 28, 28)  # 圆形背景

        # 时钟指针
        painter.setPen(
            QPen(QColor("white"), 2, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap)
        )
        painter.drawLine(16, 16, 16, 8)  # 分针
        painter.drawLine(16, 16, 22, 16)  # 时针
        painter.end()

        self.setIcon(QIcon(pixmap))

    def _create_menu(self) -> None:
        """创建托盘右键菜单"""
        # 菜单动作经信号转发给主窗口处理
        self.tray_menu = QMenu()

        self.show_action = QAction("显示窗口", self)
        self.show_action.triggered.connect(self.show_requested.emit)
        self.tray_menu.addAction(self.show_action)

        self.hide_action = QAction("隐藏到托盘", self)
        self.hide_action.triggered.connect(self.hide_requested.emit)
        self.tray_menu.addAction(self.hide_action)

        self.tray_menu.addSeparator()

        # 当前倍率显示（只读）
        self.rate_action = QAction("当前倍率: 2.0x", self)
        self.rate_action.setEnabled(False)
        self.tray_menu.addAction(self.rate_action)

        self.tray_menu.addSeparator()

        self.quit_action = QAction("退出", self)
        self.quit_action.triggered.connect(self.quit_requested.emit)
        self.tray_menu.addAction(self.quit_action)

        self.setContextMenu(self.tray_menu)

    def _on_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        """托盘图标激活回调：双击显示窗口"""
        # 仅响应 DoubleClick，其他激活原因忽略
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self.show_requested.emit()

    def update_rate(self, rate: float) -> None:
        """更新菜单中显示的当前倍率"""
        # 倍率变化时同步只读菜单项文本
        self.rate_action.setText(f"当前倍率: {rate:.1f}x")

    def show_notification(
        self, title: str, message: str, icon_kind: str = "info"
    ) -> None:
        """显示系统通知（icon_kind: info/warning）"""
        # 图标类型映射后统一 3 秒展示
        icon_map = {
            "info": QSystemTrayIcon.MessageIcon.Information,
            "warning": QSystemTrayIcon.MessageIcon.Warning,
        }
        self.showMessage(
            title,
            message,
            icon_map.get(icon_kind, QSystemTrayIcon.MessageIcon.Information),
            3000,
        )


# ===== ui/system_tray.py 函数/类说明 =====
# SystemTray(QSystemTrayIcon): 系统托盘类
#   信号：show_requested/hide_requested/quit_requested（主窗口连接并处理）
#   _create_icon(): 用 QPainter 绘制蓝色圆形时钟图标
#   _create_menu(): 显示/隐藏/倍率（只读）/退出菜单
#   _on_activated(reason): 双击托盘显示窗口
#   update_rate(rate): 倍率变化时更新菜单文本（主窗口经 rate 信号调用）
#   show_notification(title, message, icon_kind): 封装 showMessage
#   设计理由：托盘职责独立成类，主窗口不再持有图标/菜单/绘制逻辑
#   关联配置：版本号由主窗口传入（来自 main.VERSION）
