# 设置页模块（PL003 新建：主题三段选择器 + 关于区，plan#UI2.0 视觉迭代）
# 纯展示化：主题应用与持久化归主窗口（经 AppInterface），本页只发选择信号与同步选中态

from PyQt6.QtCore import pyqtSignal
from PyQt6.QtWidgets import QHBoxLayout, QVBoxLayout, QWidget

from qfluentwidgets import BodyLabel, CaptionLabel, CardWidget, SegmentedWidget, SubtitleLabel

from interface import AppInterface

# 主题选项表（键与 UiPreferences.theme 三态一一对应，顺序即展示顺序）
_THEME_OPTIONS = (("auto", "跟随系统"), ("light", "浅色"), ("dark", "深色"))


class SettingsPanel(QWidget):
    theme_selected = pyqtSignal(str)  # 主题选择变化（携带三态键，主窗口应用并持久化）

    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 构建设置页：主题卡片（三段选择器）+ 关于卡片（版本号经接口读取）
        super().__init__(parent)
        self._interface = interface
        self._layout = interface.get_ui_static()["layout"]
        self._current_theme = ""  # 当前选中态（sync_theme/选择信号共同维护，供读取）

        layout = QVBoxLayout(self)
        layout.setContentsMargins(*self._layout["panel_margin"])
        layout.setSpacing(int(self._layout["page_spacing"]))

        layout.addWidget(SubtitleLabel("设置"))

        # ------------------- 主题卡片 -------------------
        theme_card = CardWidget(self)
        theme_layout = QHBoxLayout(theme_card)
        theme_layout.setContentsMargins(*self._layout["card_padding"])

        theme_text_col = QVBoxLayout()
        theme_text_col.addWidget(BodyLabel("主题"))
        theme_text_col.addWidget(CaptionLabel("跟随系统时随 Windows 深浅色实时切换"))
        theme_layout.addLayout(theme_text_col)
        theme_layout.addStretch()

        self.theme_switch = SegmentedWidget(parent=self)
        for key, text in _THEME_OPTIONS:
            self.theme_switch.addItem(key, text)
        self.theme_switch.setCurrentItem("auto")
        self.theme_switch.currentItemChanged.connect(self._on_theme_changed)
        theme_layout.addWidget(self.theme_switch)

        layout.addWidget(theme_card)

        # ------------------- 关于卡片 -------------------
        about_card = CardWidget(self)
        about_layout = QHBoxLayout(about_card)
        about_layout.setContentsMargins(*self._layout["card_padding"])
        about_layout.addWidget(BodyLabel("关于"))
        about_layout.addStretch()
        self.version_label = CaptionLabel(f"加速世界 · 版本 {interface.get_version()}")
        about_layout.addWidget(self.version_label)
        layout.addWidget(about_card)

        layout.addStretch()

    def _on_theme_changed(self, key: str) -> None:
        # 分段选择器切换：记录选中态并发信号（应用/持久化归主窗口单一路径）
        self._current_theme = key
        self.theme_selected.emit(key)

    def sync_theme(self, theme_pref: str) -> None:
        # 主题变化时反向同步选中态（快捷键循环/启动参数路径）；屏蔽信号防应用回环
        self._current_theme = theme_pref
        self.theme_switch.blockSignals(True)
        self.theme_switch.setCurrentItem(theme_pref)
        self.theme_switch.blockSignals(False)

    def current_theme(self) -> str:
        # 当前选中态（测试与主窗口同步核对用）
        return self._current_theme


# ===== ui/panels/settings_panel.py 函数/类说明 =====
# _THEME_OPTIONS: 主题三态选项表（键与 UiPreferences.theme 一致：auto/light/dark）
# SettingsPanel(QWidget): 设置页
#   信号：theme_selected(str) 主题选择变化（主窗口应用并经接口持久化）
#   __init__(interface, parent): 版本号经 AppInterface.get_version() 读取
#   _on_theme_changed(key): 记录选中态 + 发信号（qfw SegmentedWidget 的
#     currentItemChanged(str) 信号，成员与语义经 .temp/probe_pl003.py 实测）
#   sync_theme(theme_pref): 反向同步选中态（blockSignals 防应用回环）；
#     快捷键 Ctrl+T 三态循环与启动参数路径经此保持设置页选中态一致
#   current_theme(): 读取当前选中态
#   设计理由：主题控制的归属从天气栏（历史遗留）迁入设置页（plan#UI2.0 定案）；
#   面板零后端 import（铁律 2）
#   关联配置：版本号经接口读取（base.json）；主题键经接口归一化（auto/light/dark）
