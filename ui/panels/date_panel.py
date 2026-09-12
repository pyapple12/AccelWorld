# 日期面板模块（标准时计卡：日期星期 + 标准时间 + 农历 chips；热更新减法轮拆出）
# 数据源 TimeInfo 不动（chinese_date/standard_time/lunar_info），纯 UI 重排；
# 农历串按既有标记拆解（月相：/财神：），格式由 modules 单源保证

from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont
from PyQt6.QtWidgets import QHBoxLayout, QVBoxLayout, QLabel, QWidget

from qfluentwidgets import CaptionLabel

from interface import AppInterface
from interface.types import TimeInfo
from ui.glass_card import GlassCard

# 农历串的拆解标记（modules 产出的既有格式：主体…月相：X 财神：Y）
_LUNAR_PHASE_MARK = "月相："
_LUNAR_GOD_MARK = "财神："


class DatePanel(QWidget):
    def __init__(self, interface: AppInterface, parent: QWidget | None = None):
        # 标准时计卡（置顶）：上方日期星期、中央标准时间大字；农历 chips 独立成行居底
        super().__init__(parent)
        self._interface = interface
        ui = interface.get_ui_static()
        page = QVBoxLayout(self)
        page.setContentsMargins(*ui["layout"]["panel_margin"])
        page.setSpacing(int(ui["layout"]["page_spacing"]))

        # 标准时计卡：日期星期 + 标准时间（居中大字，Bahnschrift 仪表字族）
        self.std_card = GlassCard(interface, radius_key="xl")
        card_lay = QVBoxLayout(self.std_card)
        card_lay.setContentsMargins(24, 18, 24, 16)

        self.date_label = CaptionLabel("--")
        self.date_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        card_lay.addWidget(self.date_label)

        self.std_time_label = QLabel("00:00:00")
        self.std_time_label.setFont(
            QFont(
                ui["font_family_digits"],
                int(ui["scale"]["std_time"]),
                QFont.Weight.Bold,
            )
        )
        self.std_time_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        card_lay.addWidget(self.std_time_label)
        page.addWidget(self.std_card)

        # 农历 chips 行（保留现状元素，独立一行居页面底部）
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
        self.chips_host = QWidget()
        self.chips_host.setLayout(chips_row)
        page.addWidget(self.chips_host)

    def update_time(self, info: TimeInfo) -> None:
        # 由主窗口时钟 tick 传入 TimeInfo：日期/标准时间/农历串拆解填充
        self.date_label.setText(info.chinese_date)
        self.std_time_label.setText(info.standard_time)
        lunar, phase, god = self._split_lunar(info.lunar_info)
        self._chip_values["农历"].setText(lunar)
        self._chip_values["月相"].setText(phase)
        self._chip_values["财神"].setText(god)

    @staticmethod
    def _split_lunar(text: str) -> tuple[str, str, str]:
        # 拆既有农历串（主体…月相：X 财神：Y）→ (主体, 月相值, 财神值)；
        # 标记缺失时整体归入农历主体、其余置空（格式由 modules 单源保证，容错兜底）
        if _LUNAR_PHASE_MARK in text and _LUNAR_GOD_MARK in text:
            lunar, _, rest = text.partition(_LUNAR_PHASE_MARK)
            phase, _, god = rest.partition(_LUNAR_GOD_MARK)
            return lunar.strip(), phase.strip(), god.strip()
        return text.strip(), "", ""


# ===== ui/panels/date_panel.py 函数/类说明 =====
# _LUNAR_PHASE_MARK/_LUNAR_GOD_MARK: 农历串拆解标记（modules 既有格式单源）
# DatePanel(QWidget): 标准时计卡（热更新减法轮自英雄区拆出，置页面最上）
#   __init__(interface, parent): GlassCard xl 卡内日期星期在上、标准时间居中大字
#   （Bahnschrift 仪表字族）；三枚农历玻璃 chips 独立行居卡片下方（保留现状）
#   update_time(info): 日期/标准时间/农历 chips 刷新（TimeInfo 数据源不动）
#   _split_lunar(text): 农历串拆解；标记缺失兜底整体归主体，无异常路径
#   设计理由：标准时间是持续对照读数，独立成卡并置顶（用户减法轮定案）；
#   数据源 TimeInfo 不变，纯 UI 重排（plan#UI2.0 铁律 1）
#   关联配置：字体族/仪表字族/类型尺度（scale.std_time）经 AppInterface（ui.json）
