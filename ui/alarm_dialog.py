# 闹钟编辑对话框模块（S4 完善：类型注解 + get_alarm 返回 Alarm dataclass）

import os
from typing import Optional, List, Literal

from PyQt6.QtWidgets import (
    QDialog,
    QDialogButtonBox,
    QFormLayout,
    QLineEdit,
    QTimeEdit,
    QComboBox,
    QPushButton,
    QHBoxLayout,
    QWidget,
    QFileDialog,
    QCheckBox,
)
from PyQt6.QtCore import QTime

from modules.alarm_service import PresetSound, SUPPORTED_AUDIO_FORMATS, Alarm


class AlarmEditDialog(QDialog):
    """闹钟编辑对话框"""

    def __init__(self, parent: Optional[QWidget] = None, alarm: Optional[Alarm] = None):
        """
        初始化对话框

        :param parent: 父窗口
        :param alarm: 要编辑的闹钟（None 表示新建）
        """
        super().__init__(parent)
        self.alarm = alarm
        self.sound_type: Literal["preset", "custom"] = "preset"
        self.sound_value: str = "classic"
        self.repeat_checkboxes: List[QCheckBox] = []

        self.setWindowTitle("编辑闹钟" if alarm else "添加闹钟")
        self.setFixedWidth(400)

        layout = QFormLayout(self)

        # 标签
        self.label_edit = QLineEdit()
        self.label_edit.setPlaceholderText("闹钟名称")
        self.label_edit.setText(alarm.label if alarm else "Alarm")
        layout.addRow("标签:", self.label_edit)

        # 时间
        self.time_edit = QTimeEdit()
        self.time_edit.setDisplayFormat("HH:mm")
        if alarm:
            time_parts = alarm.time.split(":")
            self.time_edit.setTime(QTime(int(time_parts[0]), int(time_parts[1])))
        else:
            self.time_edit.setTime(QTime.currentTime().addSecs(3600))  # 默认1小时后
        layout.addRow("时间:", self.time_edit)

        # 声音选择
        sound_layout = QHBoxLayout()
        self.sound_combo = QComboBox()
        self.sound_combo.addItems(PresetSound.display_names())
        sound_layout.addWidget(self.sound_combo)

        self.custom_sound_button = QPushButton("自定义...")
        self.custom_sound_button.clicked.connect(self.select_custom_sound)
        sound_layout.addWidget(self.custom_sound_button)

        # 根据已有闹钟初始化声音设置
        if alarm:
            if alarm.sound_type == "custom":
                self.sound_type = "custom"
                self.sound_value = alarm.sound_value
            else:
                # 预设声音：根据 sound_value 定位下拉框索引
                self.sound_value = alarm.sound_value
                for idx, preset in enumerate(list(PresetSound)):
                    if preset.value == alarm.sound_value:
                        self.sound_combo.setCurrentIndex(idx)
                        break

        sound_widget = QWidget()
        sound_widget.setLayout(sound_layout)
        layout.addRow("声音:", sound_widget)

        # 重复设置
        repeat_layout = QHBoxLayout()
        repeat_layout.setSpacing(5)
        days = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"]
        for i, day in enumerate(days):
            checkbox = QCheckBox(day)
            checkbox.setFixedWidth(45)
            checkbox.setToolTip(days[i])
            if alarm and i in alarm.repeat_days:
                checkbox.setChecked(True)
            repeat_layout.addWidget(checkbox)
            self.repeat_checkboxes.append(checkbox)

        repeat_widget = QWidget()
        repeat_widget.setLayout(repeat_layout)
        layout.addRow("重复:", repeat_widget)

        # 按钮
        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addRow(buttons)

    def select_custom_sound(self) -> None:
        """选择自定义音频文件"""
        file_path, _ = QFileDialog.getOpenFileName(
            self, "选择闹钟声音", "", SUPPORTED_AUDIO_FORMATS
        )
        if file_path:
            self.sound_type = "custom"
            self.sound_value = file_path
            self.custom_sound_button.setText(f"📁 {os.path.basename(file_path)[:15]}")

    def get_alarm(self) -> Alarm:
        """获取对话框内容构造的 Alarm 对象（编辑模式保留原 ID/创建时间/启用状态）"""
        # 获取时间
        time_obj = self.time_edit.time()
        time_str = f"{time_obj.hour():02d}:{time_obj.minute():02d}"

        # 获取重复天数
        repeat_days = [
            i for i, cb in enumerate(self.repeat_checkboxes) if cb.isChecked()
        ]

        # 获取声音值
        if self.sound_type == "preset":
            sound_value = list(PresetSound)[self.sound_combo.currentIndex()].value
        else:
            sound_value = self.sound_value

        alarm = Alarm(
            label=self.label_edit.text().strip() or "新闹钟",
            time=time_str,
            sound_type=self.sound_type,
            sound_value=sound_value,
            repeat_days=repeat_days,
            enabled=self.alarm.enabled if self.alarm else True,
        )

        # 编辑模式保留原 ID 与创建时间（ID 是列表定位依据）
        if self.alarm:
            alarm.id = self.alarm.id
            alarm.created_at = self.alarm.created_at
        return alarm


# ===== ui/alarm_dialog.py 函数/类说明 =====
# AlarmEditDialog(QDialog): 闹钟添加/编辑对话框
#   __init__(parent, alarm): 构建表单（标签/时间/声音/重复），编辑模式预填数据
#   select_custom_sound(): 文件选择器设置自定义铃声
#   get_alarm(): 从表单构造 Alarm dataclass；编辑模式继承原 id/created_at/enabled
#   设计理由：直接返回数据类避免 dict 魔法键；ID 保留保证 replace_alarm 定位正确
#   关联配置：预设枚举与音频格式来自 modules/alarm_service.py
