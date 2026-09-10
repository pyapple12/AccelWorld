# 闹钟面板模块（S4 GUI 面板化拆分，闹钟列表 + 增删改入口）
# PL002 Fluent 重写：qfw ListWidget/SwitchButton/PushButton/InfoBar/MessageBox；
# PL001（plan#UI2.0）：闹钟管理器所有权迁入 AppInterface（铁律 2，本文件零后端 import），
# 增删改查/触发检查全部经接口；重复/铃声文案迁 ui/tools/alarm_text（铁律 1）

import datetime

from PyQt6.QtCore import pyqtSignal, QTimer, Qt
from PyQt6.QtWidgets import QHBoxLayout, QListWidgetItem, QVBoxLayout, QLabel, QWidget

from qfluentwidgets import (
    BodyLabel,
    CaptionLabel,
    FluentIcon,
    InfoBar,
    InfoBarPosition,
    ListWidget,
    MessageBox,
    PushButton,
    SwitchButton,
    ToolButton,
)

from interface import AppInterface
from interface.types import Alarm, PresetSound
from ui.alarm_dialog import AlarmEditDialog
from ui.tools.alarm_text import format_repeat_display, format_sound_button_name

# 列表行内控件尺寸（行高紧凑，px 常量属布局细节，PL003.01 设计 tokens 收编候选）
_ROW_WIDGET_HEIGHT = 24


class AlarmPanel(QWidget):
    alarm_saved = pyqtSignal()  # 列表变更，主窗口负责经接口持久化
    alarm_triggered = pyqtSignal(object)  # 闹钟触发（携带 Alarm 对象）

    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 构建列表 UI 并启动触发检查定时器（周期经接口读取）
        super().__init__(parent)
        self._interface = interface
        self._notify_ms = int(interface.get_app_static()["notification_duration_ms"])

        alarm_layout = QVBoxLayout(self)
        alarm_layout.setContentsMargins(4, 2, 4, 2)

        # 标题行
        alarm_title_layout = QHBoxLayout()
        alarm_title = BodyLabel("闹钟:")
        alarm_title_layout.addWidget(alarm_title)
        alarm_title_layout.addStretch()

        self.add_alarm_button = PushButton(FluentIcon.ADD, "添加闹钟")
        self.add_alarm_button.clicked.connect(self.show_add_alarm_dialog)
        alarm_title_layout.addWidget(self.add_alarm_button)
        alarm_layout.addLayout(alarm_title_layout)

        # 闹钟列表（qfw ListWidget，随主题深浅自绘，PL002.07）
        self.alarm_list = ListWidget()
        self.alarm_list.setFixedHeight(120)
        alarm_layout.addWidget(self.alarm_list)

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
            item = self._make_item(alarm)
            row = self._make_row(alarm)
            item.setSizeHint(row.sizeHint())  # 行高随控件（qfw 开关/按钮高于默认项高）
            self.alarm_list.addItem(item)
            self.alarm_list.setItemWidget(item, row)

    def _make_item(self, alarm: Alarm) -> QListWidgetItem:
        # 构造列表项并携带闹钟 ID（UserRole）
        item = QListWidgetItem()
        item.setData(Qt.ItemDataRole.UserRole, alarm.id)
        return item

    def _make_row(self, alarm: Alarm) -> QWidget:
        # 构建单行控件（启用开关/时间/标签/重复/声音/编辑/删除）
        widget = QWidget()
        layout = QHBoxLayout(widget)
        layout.setContentsMargins(5, 2, 5, 2)

        # 启用开关（SwitchButton：先 setChecked 后连接，防构建期误触发翻转）
        switch = SwitchButton()
        switch.setOnText("开")
        switch.setOffText("关")
        switch.setChecked(alarm.enabled)
        switch.checkedChanged.connect(
            lambda checked, a_id=alarm.id: self.toggle_alarm(a_id)
        )
        layout.addWidget(switch)

        # 时间
        time_label = BodyLabel(alarm.time)
        time_label.setFixedWidth(60)
        layout.addWidget(time_label)

        # 标签
        label_label = BodyLabel(alarm.label)
        label_label.setFixedWidth(150)
        layout.addWidget(label_label)

        # 重复信息（弱化色 CaptionLabel 随主题）
        repeat_label = CaptionLabel(format_repeat_display(alarm.repeat_days))
        layout.addWidget(repeat_label)

        # 声音信息
        sound_label = CaptionLabel(self._sound_display(alarm))
        layout.addWidget(sound_label)

        layout.addStretch()

        # 编辑/删除按钮
        edit_btn = ToolButton(FluentIcon.EDIT)
        edit_btn.setFixedSize(_ROW_WIDGET_HEIGHT + 8, _ROW_WIDGET_HEIGHT)
        edit_btn.setToolTip("编辑")
        edit_btn.clicked.connect(
            lambda checked, a_id=alarm.id: self.show_edit_alarm_dialog(a_id)
        )
        layout.addWidget(edit_btn)

        delete_btn = ToolButton(FluentIcon.DELETE)
        delete_btn.setFixedSize(_ROW_WIDGET_HEIGHT + 8, _ROW_WIDGET_HEIGHT)
        delete_btn.setToolTip("删除")
        delete_btn.clicked.connect(
            lambda checked, a_id=alarm.id: self.delete_alarm(a_id)
        )
        layout.addWidget(delete_btn)

        return widget

    def check_alarms(self) -> None:
        # 触发检查经接口执行（同分钟去重在管理器内），命中逐个发信号
        now = datetime.datetime.now()
        for alarm in self._interface.check_alarms(now):
            self.alarm_triggered.emit(alarm)

    def save_and_refresh(self) -> None:
        # 先发 alarm_saved 信号（主窗口经接口落盘），再重建列表
        self.alarm_saved.emit()
        self.refresh_list()

    def _show_warning(self, title: str, content: str) -> None:
        # 非模态警告提示条（原模态警告弹窗，PL002.07 Fluent 化）
        InfoBar.warning(
            title=title,
            content=content,
            orient=Qt.Orientation.Vertical,
            isClosable=True,
            position=InfoBarPosition.TOP,
            duration=self._notify_ms,
            parent=self,
        )

    def show_add_alarm_dialog(self) -> None:
        # 确认后经接口添加，失败（上限/重复）InfoBar 提示用户
        dialog = AlarmEditDialog(self)
        if dialog.exec():
            if self._interface.add_alarm(dialog.get_alarm()):
                self.save_and_refresh()
            else:
                self._show_warning(
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
        if dialog.exec():
            if self._interface.replace_alarm(dialog.get_alarm()):
                self.save_and_refresh()

    def toggle_alarm(self, alarm_id: str) -> bool:
        # 经接口翻转启用态，成功后保存刷新
        result = self._interface.toggle_alarm(alarm_id)
        if result:
            self.save_and_refresh()
        return result

    def delete_alarm(self, alarm_id: str) -> None:
        # 二次确认（qfw MessageBox）后经接口删除并保存刷新
        alarm = self._interface.get_alarm(alarm_id)
        if not alarm:
            return

        confirm = MessageBox("确认删除", f"确定要删除闹钟「{alarm.label}」吗？", self)
        if not confirm.exec():
            return

        if self._interface.remove_alarm(alarm_id):
            self.save_and_refresh()

    def _sound_display(self, alarm: Alarm) -> str:
        # 预设铃声显示名称（经 display_name），自定义显示文件名（截断文案走 tools）
        if alarm.sound_type == "preset":
            preset = PresetSound.from_value(alarm.sound_value)
            return f"🔔 {preset.display_name}"
        return format_sound_button_name(alarm.sound_value)


# ===== ui/panels/alarm_panel.py 函数/类说明 =====
# _ROW_WIDGET_HEIGHT: 列表行内小按钮高度（布局细节，PL003.01 tokens 收编候选）
# AlarmPanel(QWidget): 闹钟面板
#   信号：alarm_saved 列表变更（主窗口经接口持久化）；alarm_triggered(Alarm) 触发
#   __init__(interface, parent): 检查周期/提示时长经接口读取（PL002.07 Fluent 重写：
#   ListWidget 列表 + SwitchButton 开关 + ToolButton 行内按钮 + InfoBar/MessageBox 提示）
#   load_alarms(): 启动时经接口从配置载入
#   refresh_list(): 重建列表（_make_item 携 ID + _make_row 构行）
#   _make_row(alarm): 单行构建；SwitchButton 先 setChecked 后连接 checkedChanged
#     （防构建期信号误触发，与原 QCheckBox 顺序语义一致）
#   check_alarms(): 定时经接口检查，命中发信号；一次性闹钟禁用由主窗口经接口处理
#   save_and_refresh(): 变更后统一保存+刷新入口
#   show_add_alarm_dialog()/show_edit_alarm_dialog()/delete_alarm()/toggle_alarm():
#     增删改全部经接口（闹钟管理器所有权在 AppInterface，plan#UI2.0 迁移要点）；
#     删除二次确认经 qfw MessageBox（exec 真值判断）
#   _show_warning(title, content): InfoBar 非模态提示（时长 notification_duration_ms）
#   _sound_display(alarm): 铃声文案（预设走 display_name，自定义走 tools 截断文案）
#   设计理由：面板零业务状态零后端 import（plan#UI2.0 铁律 1/2）；与主窗口仅通过信号交互
#   关联配置：检查周期 alarm_check_ms 与提示时长经接口读取；闹钟持久化经 alarm_saved → 主窗口
