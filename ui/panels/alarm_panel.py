# 闹钟面板模块（S4 GUI 面板化拆分，闹钟列表 + 增删改入口）
# PL001（plan#UI2.0）：闹钟管理器所有权迁入 AppInterface（铁律 2，本文件零后端 import），
# 增删改查/触发检查全部经接口；重复/铃声文案迁 ui/tools/alarm_text（铁律 1）

import datetime

from PyQt6.QtCore import pyqtSignal, QTimer, Qt
from PyQt6.QtWidgets import (
    QWidget,
    QHBoxLayout,
    QVBoxLayout,
    QFrame,
    QLabel,
    QPushButton,
    QListWidget,
    QListWidgetItem,
    QCheckBox,
    QMessageBox,
    QDialog,
)
from PyQt6.QtGui import QFont

from interface import AppInterface
from interface.types import Alarm, PresetSound
from ui.alarm_dialog import AlarmEditDialog
from ui.tools.alarm_text import format_repeat_display, format_sound_button_name


class AlarmPanel(QWidget):
    alarm_saved = pyqtSignal()  # 列表变更，主窗口负责经接口持久化
    alarm_triggered = pyqtSignal(object)  # 闹钟触发（携带 Alarm 对象）

    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 构建列表 UI 并启动触发检查定时器（周期/样式经接口读取）
        super().__init__(parent)
        self._interface = interface
        self._ui = ui = interface.get_ui_static()

        alarm_frame = QFrame()
        alarm_frame.setFrameShape(QFrame.Shape.StyledPanel)
        alarm_layout = QVBoxLayout(alarm_frame)

        # 标题行
        alarm_title_layout = QHBoxLayout()
        alarm_title = QLabel("闹钟:")
        alarm_title.setFont(QFont(ui["font_family"], 12))
        alarm_title_layout.addWidget(alarm_title)
        alarm_title_layout.addStretch()

        self.add_alarm_button = QPushButton("+ 添加闹钟")
        self.add_alarm_button.setFont(QFont(ui["font_family"], 10))
        self.add_alarm_button.clicked.connect(self.show_add_alarm_dialog)
        alarm_title_layout.addWidget(self.add_alarm_button)
        alarm_layout.addLayout(alarm_title_layout)

        # 闹钟列表
        self.alarm_list = QListWidget()
        self.alarm_list.setFont(QFont(ui["font_family"], 11))
        self.alarm_list.setFixedHeight(120)
        alarm_layout.addWidget(self.alarm_list)

        outer = QVBoxLayout(self)
        outer.addWidget(alarm_frame)

        # 每秒检查一次闹钟触发（周期来自静态配置，经接口读取）
        self.check_timer = QTimer(self)
        self.check_timer.timeout.connect(self.check_alarms)
        self.check_timer.start(int(self._interface.get_app_static()["alarm_check_ms"]))

    def load_alarms(self) -> None:
        # 经接口从配置载入闹钟（容错）后重建列表
        self._interface.load_alarm_dicts()
        self.refresh_list()

    def refresh_list(self) -> None:
        # 清空后逐闹钟构建行（开关/时间/标签/重复/声音/编辑/删除）；
        # 数据经接口只读遍历，重复/铃声文案经 ui/tools 格式化
        self.alarm_list.clear()

        for alarm in self._interface.get_alarms():
            item = QListWidgetItem()
            item.setData(Qt.ItemDataRole.UserRole, alarm.id)

            # 自定义 widget 展示闹钟信息
            widget = QWidget()
            layout = QHBoxLayout(widget)
            layout.setContentsMargins(5, 2, 5, 2)

            # 启用开关
            checkbox = QCheckBox()
            checkbox.setChecked(alarm.enabled)
            checkbox.setFixedWidth(30)
            checkbox.toggled.connect(
                lambda checked, a_id=alarm.id: self.toggle_alarm(a_id)
            )
            layout.addWidget(checkbox)

            # 时间
            time_label = QLabel(alarm.time)
            time_label.setFont(QFont(self._ui["font_family"], 12, QFont.Weight.Bold))
            time_label.setFixedWidth(60)
            layout.addWidget(time_label)

            # 标签
            label_label = QLabel(alarm.label)
            label_label.setFont(QFont(self._ui["font_family"], 11))
            label_label.setFixedWidth(150)
            layout.addWidget(label_label)

            # 重复信息
            repeat_label = QLabel(format_repeat_display(alarm.repeat_days))
            repeat_label.setFont(QFont(self._ui["font_family"], 10))
            repeat_label.setStyleSheet(
                "color: " + self._ui["colors"]["text_secondary"]
            )
            layout.addWidget(repeat_label)

            # 声音信息
            sound_label = QLabel(self._sound_display(alarm))
            sound_label.setFont(QFont(self._ui["font_family"], 10))
            sound_label.setStyleSheet(
                "color: " + self._ui["colors"]["text_tertiary"]
            )
            layout.addWidget(sound_label)

            layout.addStretch()

            # 编辑/删除按钮
            edit_btn = QPushButton("编辑")
            edit_btn.setFixedSize(50, 25)
            edit_btn.clicked.connect(
                lambda checked, a_id=alarm.id: self.show_edit_alarm_dialog(a_id)
            )
            layout.addWidget(edit_btn)

            delete_btn = QPushButton("删除")
            delete_btn.setFixedSize(50, 25)
            delete_btn.clicked.connect(
                lambda checked, a_id=alarm.id: self.delete_alarm(a_id)
            )
            layout.addWidget(delete_btn)

            widget.setLayout(layout)
            item.setSizeHint(widget.sizeHint())

            self.alarm_list.addItem(item)
            self.alarm_list.setItemWidget(item, widget)

    def check_alarms(self) -> None:
        # 触发检查经接口执行（同分钟去重在管理器内），命中逐个发信号
        now = datetime.datetime.now()
        for alarm in self._interface.check_alarms(now):
            self.alarm_triggered.emit(alarm)

    def save_and_refresh(self) -> None:
        # 先发 alarm_saved 信号（主窗口经接口落盘），再重建列表
        self.alarm_saved.emit()
        self.refresh_list()

    def show_add_alarm_dialog(self) -> None:
        # 确认后经接口添加，失败（上限/重复）弹窗提示用户
        dialog = AlarmEditDialog(self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            if self._interface.add_alarm(dialog.get_alarm()):
                self.save_and_refresh()
            else:
                QMessageBox.warning(
                    self,
                    "提示",
                    f"添加失败：已达最大数量（{self._interface.get_max_alarms()}）"
                    "或存在相同时间与标签的闹钟",
                )

    def show_edit_alarm_dialog(self, alarm_id: str) -> None:
        # 确认后经接口用保留 ID 的新对象整体替换
        alarm = self._interface.get_alarm(alarm_id)
        if not alarm:
            return

        dialog = AlarmEditDialog(self, alarm)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            if self._interface.replace_alarm(dialog.get_alarm()):
                self.save_and_refresh()

    def toggle_alarm(self, alarm_id: str) -> bool:
        # 经接口翻转启用态，成功后保存刷新
        result = self._interface.toggle_alarm(alarm_id)
        if result:
            self.save_and_refresh()
        return result

    def delete_alarm(self, alarm_id: str) -> None:
        # 二次确认后经接口删除并保存刷新
        alarm = self._interface.get_alarm(alarm_id)
        if not alarm:
            return

        reply = QMessageBox.question(
            self,
            "确认删除",
            f"确定要删除闹钟「{alarm.label}」吗？",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
        )

        if reply == QMessageBox.StandardButton.Yes:
            if self._interface.remove_alarm(alarm_id):
                self.save_and_refresh()

    def _sound_display(self, alarm: Alarm) -> str:
        # 预设铃声显示名称（经 display_name），自定义显示文件名（截断文案走 tools）
        if alarm.sound_type == "preset":
            preset = PresetSound.from_value(alarm.sound_value)
            return f"🔔 {preset.display_name}"
        return format_sound_button_name(alarm.sound_value)


# ===== ui/panels/alarm_panel.py 函数/类说明 =====
# AlarmPanel(QWidget): 闹钟面板
#   信号：alarm_saved 列表变更（主窗口经接口持久化）；alarm_triggered(Alarm) 触发
#   __init__(interface, parent): 检查周期/样式经接口读取（PL001.12 纯展示化）
#   load_alarms(): 启动时经接口从配置载入
#   refresh_list(): 重建列表控件（数据经接口 get_alarms；文案经 ui/tools/alarm_text）
#   check_alarms(): 定时经接口检查，命中发信号；一次性闹钟禁用由主窗口经接口处理
#   save_and_refresh(): 变更后统一保存+刷新入口
#   show_add_alarm_dialog()/show_edit_alarm_dialog()/delete_alarm()/toggle_alarm():
#     增删改全部经接口（闹钟管理器所有权在 AppInterface，plan#UI2.0 迁移要点）
#   _sound_display(alarm): 铃声文案（预设走 display_name，自定义走 tools 截断文案）
#   设计理由：面板零业务状态零后端 import（plan#UI2.0 铁律 1/2）；与主窗口仅通过信号交互
#   关联配置：检查周期 alarm_check_ms 与样式经接口读取；闹钟持久化经 alarm_saved → 主窗口
