# 闹钟面板模块（S4 GUI 面板化拆分，闹钟列表 + 增删改入口）

import datetime
import os
from typing import List, Dict, Any

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

from modules.alarm_service import AlarmManager, Alarm, PresetSound
from ui.alarm_dialog import AlarmEditDialog
from config.static.static_config import get_static_config

# 静态配置（检查周期/字体/颜色）
_BASE = get_static_config().base
_UI = get_static_config().ui


class AlarmPanel(QWidget):
    """闹钟面板：闹钟列表展示与增删改，内部每秒检查触发"""

    alarm_saved = pyqtSignal()  # 列表变更，主窗口负责持久化
    alarm_triggered = pyqtSignal(object)  # 闹钟触发（携带 Alarm 对象）

    def __init__(self, parent: QWidget | None = None):
        """初始化闹钟列表、管理器与每秒检查定时器"""
        # 构建列表 UI 并启动每秒触发检查定时器
        super().__init__(parent)

        self.alarm_manager = AlarmManager()

        alarm_frame = QFrame()
        alarm_frame.setFrameShape(QFrame.Shape.StyledPanel)
        alarm_layout = QVBoxLayout(alarm_frame)

        # 标题行
        alarm_title_layout = QHBoxLayout()
        alarm_title = QLabel("闹钟:")
        alarm_title.setFont(QFont(_UI["font_family"], 12))
        alarm_title_layout.addWidget(alarm_title)
        alarm_title_layout.addStretch()

        self.add_alarm_button = QPushButton("+ 添加闹钟")
        self.add_alarm_button.setFont(QFont(_UI["font_family"], 10))
        self.add_alarm_button.clicked.connect(self.show_add_alarm_dialog)
        alarm_title_layout.addWidget(self.add_alarm_button)
        alarm_layout.addLayout(alarm_title_layout)

        # 闹钟列表
        self.alarm_list = QListWidget()
        self.alarm_list.setFont(QFont(_UI["font_family"], 11))
        self.alarm_list.setFixedHeight(120)
        alarm_layout.addWidget(self.alarm_list)

        outer = QVBoxLayout(self)
        outer.addWidget(alarm_frame)

        # 每秒检查一次闹钟触发（周期来自静态配置）
        self.check_timer = QTimer(self)
        self.check_timer.timeout.connect(self.check_alarms)
        self.check_timer.start(int(_BASE["alarm_check_ms"]))

    def load_alarms(self, data: list) -> None:
        """从配置数据加载闹钟并刷新列表"""
        # 委托管理器反序列化（容错）后重建列表
        self.alarm_manager.from_dict_list(data)
        self.refresh_list()

    def to_dict_list(self) -> List[Dict[str, Any]]:
        """导出闹钟字典列表（供配置持久化）"""
        # 委托管理器逐闹钟转字典
        return self.alarm_manager.to_dict_list()

    def refresh_list(self) -> None:
        """刷新闹钟列表显示"""
        # 清空后逐闹钟构建行（开关/时间/标签/重复/声音/编辑/删除）
        self.alarm_list.clear()

        for alarm in self.alarm_manager.alarms:
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
            time_label.setFont(QFont(_UI["font_family"], 12, QFont.Weight.Bold))
            time_label.setFixedWidth(60)
            layout.addWidget(time_label)

            # 标签
            label_label = QLabel(alarm.label)
            label_label.setFont(QFont(_UI["font_family"], 11))
            label_label.setFixedWidth(150)
            layout.addWidget(label_label)

            # 重复信息
            repeat_label = QLabel(self._get_repeat_display(alarm.repeat_days))
            repeat_label.setFont(QFont(_UI["font_family"], 10))
            repeat_label.setStyleSheet("color: " + _UI["colors"]["text_secondary"])
            layout.addWidget(repeat_label)

            # 声音信息
            sound_label = QLabel(self._get_sound_display(alarm))
            sound_label.setFont(QFont(_UI["font_family"], 10))
            sound_label.setStyleSheet("color: " + _UI["colors"]["text_tertiary"])
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
        """每秒检查闹钟触发，命中后发出 alarm_triggered 信号"""
        # 空列表短路：无闹钟时不建 datetime 不遍历（S9.3）
        if not self.alarm_manager.alarms:
            return
        now = datetime.datetime.now()
        for alarm in self.alarm_manager.check_alarms(now):
            self.alarm_triggered.emit(alarm)

    def save_and_refresh(self) -> None:
        """列表变更后通知主窗口持久化并刷新显示"""
        # 先发 alarm_saved 信号持久化，再重建列表
        self.alarm_saved.emit()
        self.refresh_list()

    def show_add_alarm_dialog(self) -> None:
        """显示添加闹钟对话框"""
        # 确认后构造 Alarm 加入管理器，失败（上限/重复）弹窗提示用户
        dialog = AlarmEditDialog(self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            if self.alarm_manager.add_alarm(dialog.get_alarm()):
                self.save_and_refresh()
            else:
                QMessageBox.warning(
                    self,
                    "提示",
                    f"添加失败：已达最大数量（{_BASE['max_alarms']}）或存在相同时间与标签的闹钟",
                )

    def show_edit_alarm_dialog(self, alarm_id: str) -> None:
        """显示编辑闹钟对话框"""
        # 确认后用保留 ID 的新对象整体替换
        alarm = self.alarm_manager.get_alarm(alarm_id)
        if not alarm:
            return

        dialog = AlarmEditDialog(self, alarm)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            if self.alarm_manager.replace_alarm(dialog.get_alarm()):
                self.save_and_refresh()

    def toggle_alarm(self, alarm_id: str) -> bool:
        """切换闹钟启用状态"""
        # 成功切换后保存刷新
        result = self.alarm_manager.toggle_alarm(alarm_id)
        if result:
            self.save_and_refresh()
        return result

    def delete_alarm(self, alarm_id: str) -> None:
        """删除闹钟（带确认弹窗）"""
        # 二次确认后删除并保存刷新
        alarm = self.alarm_manager.get_alarm(alarm_id)
        if not alarm:
            return

        reply = QMessageBox.question(
            self,
            "确认删除",
            f"确定要删除闹钟「{alarm.label}」吗？",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
        )

        if reply == QMessageBox.StandardButton.Yes:
            if self.alarm_manager.remove_alarm(alarm_id):
                self.save_and_refresh()

    def _get_repeat_display(self, repeat_days: list) -> str:
        """获取重复天数的显示文本（内部辅助）"""
        # 空列表显示"一次"，否则按星期数字映射拼接
        if not repeat_days:
            return "一次"
        days = ["一", "二", "三", "四", "五", "六", "日"]
        return "周" + "".join(days[d] for d in repeat_days)

    def _get_sound_display(self, alarm: Alarm) -> str:
        """获取声音的显示文本（内部辅助）"""
        # 预设铃声显示名称（经 display_name），自定义显示文件名（截断 15 字符）
        if alarm.sound_type == "preset":
            preset = PresetSound.from_value(alarm.sound_value)
            return f"🔔 {preset.display_name}"
        return f"📁 {os.path.basename(alarm.sound_value)[:15]}"


# ===== ui/panels/alarm_panel.py 函数/类说明 =====
# AlarmPanel(QWidget): 闹钟面板
#   信号：alarm_saved 列表变更（主窗口持久化）；alarm_triggered(Alarm) 触发（主窗口播放/通知）
#   load_alarms(data): 启动时从配置加载
#   to_dict_list(): 导出列表供持久化
#   refresh_list(): 重建列表控件（每行含开关/时间/标签/重复/声音/编辑/删除）
#   check_alarms(): 每秒定时检查，命中发信号；一次性闹钟禁用由主窗口处理
#   save_and_refresh(): 变更后统一保存+刷新入口
#   show_add_alarm_dialog()/show_edit_alarm_dialog()/delete_alarm()/toggle_alarm(): 增删改
#   _get_repeat_display()/_get_sound_display(): 显示格式化辅助
#   设计理由：闹钟状态与管理器内聚于面板；与主窗口仅通过信号交互
#   关联配置：闹钟持久化经 alarm_saved → 主窗口 save_alarms
