# 闹钟编辑对话框模块（S4 完善：类型注解 + get_alarm 返回 Alarm dataclass）
# PL002 Fluent 重写：MessageBoxBase 基座 + qfw LineEdit/TimePicker/ComboBox/CheckBox；
# PL001（plan#UI2.0）：Alarm/PresetSound 类型改经 interface.types 转出（铁律 2），
# 自定义铃声按钮文案迁 ui/tools/alarm_text（铁律 1），本文件零后端 import

from typing import List, Literal, Optional

from PyQt6.QtCore import QTime
from PyQt6.QtWidgets import QFileDialog, QHBoxLayout, QFormLayout, QWidget

from qfluentwidgets import (
    CheckBox,
    ComboBox,
    LineEdit,
    MessageBoxBase,
    PushButton,
    SubtitleLabel,
    TimePicker,
)

from interface.app_interface import AppInterface
from interface.types import Alarm, PresetSound
from ui.tools.alarm_text import format_sound_button_name

# 支持的音频文件格式（Qt 文件对话框过滤器串；FIX002.17 自业务层迁入 UI 层）
SUPPORTED_AUDIO_FORMATS = (
    "Audio Files (*.wav *.mp3 *.ogg *.flac *.m4a *.wma *.aac);;All Files (*)"
)

# 重复星期复选框文案（下标即 weekday() 数字 0-6）
_WEEKDAY_LABELS = ("周一", "周二", "周三", "周四", "周五", "周六", "周日")


class AlarmEditDialog(MessageBoxBase):
    def __init__(
        self,
        parent: QWidget,
        alarm: Optional[Alarm] = None,
        *,
        interface: AppInterface,
    ):
        # 构建表单并预填数据；编辑模式回填时间/声音/重复（MessageBoxBase 需父窗口）；
        # interface 必填（关键字传参）：读取 ui.json 间距 token（FIX003.11：消除双源硬编码）
        super().__init__(parent)
        self.alarm = alarm
        self.sound_type: Literal["preset", "custom"] = "preset"
        self.sound_value: str = "classic"
        self.repeat_checkboxes: List[CheckBox] = []
        dialog_layout = interface.get_ui_static()["layout"]

        self.titleLabel = SubtitleLabel("编辑闹钟" if alarm else "添加闹钟", self)
        self.viewLayout.addWidget(self.titleLabel)

        form_host = QWidget(self)
        form = QFormLayout(form_host)
        form.setSpacing(int(dialog_layout["dialog_form_spacing"]))

        # 标签
        self.label_edit = LineEdit()
        self.label_edit.setPlaceholderText("闹钟名称")
        self.label_edit.setText(alarm.label if alarm else "Alarm")
        form.addRow("标签:", self.label_edit)

        # 时间（qfw TimePicker，HH:mm）
        self.time_edit = TimePicker()
        if alarm:
            time_parts = alarm.time.split(":")
            self.time_edit.setTime(QTime(int(time_parts[0]), int(time_parts[1])))
        else:
            self.time_edit.setTime(QTime.currentTime().addSecs(3600))  # 默认1小时后
        form.addRow("时间:", self.time_edit)

        # 声音选择
        sound_layout = QHBoxLayout()
        self.sound_combo = ComboBox()
        self.sound_combo.addItems(PresetSound.display_names())
        # 下拉框选择预设时复位声音类型（FIX001.6：此前自定义闹钟选任何预设都被静默忽略）
        self.sound_combo.currentTextChanged.connect(self._on_sound_preset_selected)
        sound_layout.addWidget(self.sound_combo)

        self.custom_sound_button = PushButton("自定义...")
        self.custom_sound_button.clicked.connect(self.select_custom_sound)
        sound_layout.addWidget(self.custom_sound_button)

        # 根据已有闹钟初始化声音设置
        if alarm:
            if alarm.sound_type == "custom":
                self.sound_type = "custom"
                self.sound_value = alarm.sound_value
                # 回填文件名到按钮文案（FIX001.6：修复打开自定义闹钟时当前铃声不可见）
                self.custom_sound_button.setText(format_sound_button_name(alarm.sound_value))
            else:
                # 预设声音：经 from_value 定位枚举（大小写不敏感兜底 CLASSIC），避免手写遍历（E2）
                self.sound_value = alarm.sound_value
                self.sound_combo.setCurrentIndex(
                    PresetSound.from_value(alarm.sound_value).index()
                )

        sound_widget = QWidget(self)
        sound_widget.setLayout(sound_layout)
        form.addRow("声音:", sound_widget)

        # 重复设置
        repeat_layout = QHBoxLayout()
        repeat_layout.setSpacing(int(dialog_layout["dialog_repeat_spacing"]))
        for i, day in enumerate(_WEEKDAY_LABELS):
            checkbox = CheckBox(day)
            if alarm and i in alarm.repeat_days:
                checkbox.setChecked(True)
            repeat_layout.addWidget(checkbox)
            self.repeat_checkboxes.append(checkbox)

        repeat_widget = QWidget(self)
        repeat_widget.setLayout(repeat_layout)
        form.addRow("重复:", repeat_widget)

        self.viewLayout.addWidget(form_host)

        # 底部按钮（MessageBoxBase 自带 yesButton/cancelButton，文案本地化并接线）
        self.yesButton.setText("确定")
        self.cancelButton.setText("取消")
        self.yesButton.clicked.connect(self.accept)
        self.cancelButton.clicked.connect(self.reject)

    def _on_sound_preset_selected(self, text: str) -> None:
        # 下拉框选择预设时复位声音类型为 preset（FIX001.6：自定义→预设切换不再被静默忽略）
        self.sound_type = "preset"

    def select_custom_sound(self) -> None:
        # 文件选择器成功后切换声音类型并更新按钮文案
        file_path, _ = QFileDialog.getOpenFileName(
            self, "选择闹钟声音", "", SUPPORTED_AUDIO_FORMATS
        )
        if file_path:
            self.sound_type = "custom"
            self.sound_value = file_path
            self.custom_sound_button.setText(format_sound_button_name(file_path))

    def get_alarm(self) -> Alarm:
        # 获取时间（qfw TimePicker → HH:MM 字符串）
        time_obj = self.time_edit.getTime()
        time_str = f"{time_obj.hour():02d}:{time_obj.minute():02d}"

        # 获取重复天数
        repeat_days = [
            i for i, cb in enumerate(self.repeat_checkboxes) if cb.isChecked()
        ]

        # 获取声音值（下拉框索引经 from_index 反查枚举，S9.6 封装）
        if self.sound_type == "preset":
            sound_value = PresetSound.from_index(self.sound_combo.currentIndex()).value
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
# _WEEKDAY_LABELS: 重复星期复选框文案（下标即 weekday 数字）
# SUPPORTED_AUDIO_FORMATS: 本文件 UI 常量（Qt 文件对话框过滤器串，FIX002.17）
# AlarmEditDialog(MessageBoxBase): 闹钟添加/编辑对话框（PL002 Fluent 重写）
#   __init__(parent, alarm): parent 必传（MessageBoxBase 遮罩依赖父窗口）；
#     表单（标签 LineEdit/时间 TimePicker/声音 ComboBox+PushButton/重复 CheckBox）；
#     编辑模式预填数据（自定义铃声回填文件名到按钮文案，FIX001.6，文案经 tools）
#     设计理由：qfw ComboBox 的 currentTextChanged（str）替代原 currentIndexChanged，
#     回调语义不变（选择预设即复位 sound_type）
#   _on_sound_preset_selected(text): 选预设复位 sound_type（FIX001.6）
#   select_custom_sound(): 文件选择器设置自定义铃声
#   get_alarm(): 从表单构造 Alarm dataclass；编辑模式继承原 id/created_at/enabled
#   设计理由：直接返回数据类避免 dict 魔法键；ID 保留保证 replace_alarm 定位正确；
#   类型经 interface.types 转出（plan#UI2.0 铁律 2，本文件零后端 import）
#   关联配置：预设枚举经 interface.types；铃声按钮文案经 ui.tools.alarm_text
