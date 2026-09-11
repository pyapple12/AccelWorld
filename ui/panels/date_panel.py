# 日期面板模块（S4 GUI 面板化拆分，中文日期 + 农历信息）
# PL002 Fluent 重写：SubtitleLabel/CaptionLabel 自带主题字体与深浅色适配；
# PL001（plan#UI2.0）：纯展示化——类型走 interface.types；
# PL007.01（plan#UI2.0）：农历/月相/财神升三枚玻璃 chips（数据源 TimeInfo 不动，
# lunar_info 单串按既有分隔标记拆解，纯 UI 重排）

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import QHBoxLayout, QLabel, QVBoxLayout, QWidget

from qfluentwidgets import CaptionLabel, SubtitleLabel

from interface import AppInterface
from interface.types import TimeInfo
from ui.glass_card import GlassCard

# 农历串的拆解标记（modules 产出的既有格式：主体…月相：X 财神：Y）
_LUNAR_PHASE_MARK = "月相："
_LUNAR_GOD_MARK = "财神："


class DatePanel(QWidget):
    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 日期大字 + 农历/月相/财神三枚玻璃 chips（PL007.01 仪表化）
        super().__init__(parent)

        date_layout = QVBoxLayout(self)
        date_layout.setContentsMargins(
            *interface.get_ui_static()["layout"]["panel_margin"]
        )
        self._interface = interface

        # 中文日期标签（中性占位，首帧 tick 后刷新为真实日期，F4）
        self.date_label = SubtitleLabel("----年--月--日")
        self.date_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.date_label)

        # 农历 chips 行：农历主体 / 月相 / 财神（值首帧 tick 后填充）
        chips_row = QHBoxLayout()
        chips_row.addStretch()
        self._chip_values: dict[str, QLabel] = {}
        for key in ("农历", "月相", "财神"):
            chip = GlassCard(interface, radius_key="md")
            chip_layout = QVBoxLayout(chip)
            chip_layout.setContentsMargins(12, 8, 12, 8)
            chip_layout.addWidget(CaptionLabel(key))
            value_label = QLabel("--")
            value_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
            chip_layout.addWidget(value_label)
            self._chip_values[key] = value_label
            chips_row.addWidget(chip)
        chips_row.addStretch()
        self._chips_host = QWidget(self)
        self._chips_host.setLayout(chips_row)
        date_layout.addWidget(self._chips_host)

    def update_time(self, info: TimeInfo) -> None:
        # 由主窗口时钟 tick 传入 TimeInfo：日期大字 + 农历串拆解填充 chips
        self.date_label.setText(info.chinese_date)
        lunar, phase, god = self._split_lunar(info.lunar_info)
        self._chip_values["农历"].setText(lunar)
        self._chip_values["月相"].setText(phase)
        self._chip_values["财神"].setText(god)

    @staticmethod
    def _split_lunar(text: str) -> tuple[str, str, str]:
        # 拆既有农历串（主体…月相：X 财神：Y）→ (主体, 月相值, 财神值)；
        # 标记缺失时整体归入农历主体、其余置空（格式由 modules 单源保证，容错兜底）
        if _LUNAR_PHASE_MARK in text and _LUNAR_GOD_MARK in text:
            head, _, rest = text.partition(_LUNAR_PHASE_MARK)
            phase, _, god = rest.partition(_LUNAR_GOD_MARK)
            return head.strip(), phase.strip(), god.strip()
        return text.strip(), "", ""


# ===== ui/panels/date_panel.py 函数/类说明 =====
# _LUNAR_PHASE_MARK/_LUNAR_GOD_MARK: 农历串拆解标记（modules 既有格式单源）
# DatePanel(QWidget): 日期大字 + 农历/月相/财神玻璃 chips（PL007.01）
#   __init__(interface, parent): 三枚 GlassCard md chips，值标签存 _chip_values
#   update_time(info): 日期与农历串刷新（TimeInfo 不变，纯 UI 重排）
#   _split_lunar(text): 按既有标记拆农历串；标记缺失兜底整体归主体
#   设计理由：农历信息从单行小字升为玻璃 chips（意见书 G2，仪表化）；数据源不动
#   （TimeInfo.lunar_info 单串，拆解为本面板展示逻辑，铁律 1）
#   异常处理：拆解有标记缺失兜底分支，无异常路径
#   关联配置：无直接配置依赖（chips 材质经 GlassCard → ui.json glass 节）
