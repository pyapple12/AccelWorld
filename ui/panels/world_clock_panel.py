# 世界时钟面板模块（PL007.02 仪表化：4×2 常驻城市玻璃卡矩阵，点击卡片切换基准时区）
# 常驻城市集经 AppInterface.get_world_pins（用户配置 world_pins，铁律 2）；
# pytz 换算属渲染逻辑保留面板（铁律 3）；昼夜指示按当地时刻 ☾/☀

import datetime
import logging

import pytz

from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont
from PyQt6.QtWidgets import QGridLayout, QHBoxLayout, QLabel, QVBoxLayout, QWidget

from qfluentwidgets import CaptionLabel, SubtitleLabel

from interface import AppInterface
from ui.glass_card import GlassCard

# 昼夜判定阈值（当地时间 <6 时或 ≥18 时记夜，配 ☾；其余 ☀）
_NIGHT_HOUR_START = 18
_NIGHT_HOUR_END = 6

# 配置日志
logger = logging.getLogger(__name__)


class WorldClockPanel(QWidget):
    def __init__(self, interface: AppInterface, parent=None):
        # 常驻城市矩阵：城市集经接口读取（空/缺表项兜底取全表），点击卡片切换基准时区
        super().__init__(parent)
        self._interface = interface
        self._options = interface.get_timezone_options()
        self._ui = interface.get_ui_static()
        self._layout = self._ui["layout"]
        self._pins = [p for p in interface.get_world_pins() if self._city_name(p)]
        if not self._pins:
            self._pins = [iana for _, iana in self._options]
        self._current_tz = self._interface.get_app_static()["default_timezone"]
        self._tz_cache: dict[str, Any] = {}
        self._last_second = -1
        self._cards: dict[str, GlassCard] = {}
        self._name_labels: dict[str, QLabel] = {}
        self._time_labels: dict[str, QLabel] = {}
        self._meta_labels: dict[str, QLabel] = {}
        self._daynight_labels: dict[str, QLabel] = {}

        page_layout = QVBoxLayout(self)
        page_layout.setContentsMargins(*self._layout["panel_margin"])
        page_layout.setSpacing(int(self._layout["page_spacing"]))

        # 页标题 + 操作提示
        page_title = SubtitleLabel("世界时钟")
        page_title.setFont(QFont(self._ui["font_family"], int(self._ui["scale"]["page_title"])))
        page_layout.addWidget(page_title)
        page_layout.addWidget(CaptionLabel("点击卡片切换基准时区 · 时间随当地夏令时"))

        # 4 列城市卡矩阵（行数随城市集自适应）
        grid = QGridLayout()
        grid.setSpacing(int(self._layout["page_spacing"]))
        digits_font = QFont(
            self._ui["font_family_digits"],
            int(self._ui["scale"]["city_time"]),
            QFont.Weight.Bold,
        )
        for index, iana in enumerate(self._pins):
            card = GlassCard(interface, radius_key="lg")
            card_lay = QVBoxLayout(card)
            card_lay.setContentsMargins(14, 12, 14, 12)

            name_row = QHBoxLayout()
            name_label = QLabel(self._city_name(iana))
            name_row.addWidget(name_label)
            daynight_label = QLabel("--")
            daynight_label.setAlignment(Qt.AlignmentFlag.AlignRight)
            name_row.addWidget(daynight_label)
            self._daynight_labels[iana] = daynight_label
            card_lay.addLayout(name_row)

            time_label = QLabel("--:--")
            time_label.setFont(digits_font)
            self._time_labels[iana] = time_label
            card_lay.addWidget(time_label)

            meta_label = CaptionLabel("--")
            self._meta_labels[iana] = meta_label
            card_lay.addWidget(meta_label)

            card.clicked.connect(lambda checked=False, tz=iana: self.set_timezone(tz))
            self._cards[iana] = card
            self._name_labels[iana] = name_label
            grid.addWidget(card, index // 4, index % 4)
        page_layout.addLayout(grid)
        page_layout.addStretch()
        self._refresh_selection()

    def _city_name(self, iana: str) -> str:
        # IANA → 城市短名：优先选项表显示名去括号，表外取路径尾段兜底
        for name, candidate in self._options:
            if candidate == iana:
                return name.split(" (")[0]
        return iana.rsplit("/", 1)[-1].replace("_", " ")

    def _refresh_selection(self) -> None:
        # 选中态金描边刷新（当前基准时区卡片高亮）
        for iana, card in self._cards.items():
            card.set_selected(iana == self._current_tz)

    def update_world_clock(self) -> None:
        # 同秒跳过刷新，pytz 对象按名缓存；逐卡刷新时间/星期/昼夜；
        # 选中基准卡显示秒级，其余 HH:MM；单卡失败降级 00:00 不影响他卡
        now = datetime.datetime.now()
        if now.second == self._last_second:
            return
        self._last_second = now.second
        for iana in self._pins:
            try:
                tz = self._tz_cache.get(iana)
                if tz is None:
                    tz = pytz.timezone(iana)
                    self._tz_cache[iana] = tz
                local = now.astimezone(tz)
                selected = iana == self._current_tz
                self._time_labels[iana].setText(
                    local.strftime("%H:%M:%S" if selected else "%H:%M")
                )
                weekday = "一二三四五六日"[local.weekday()]
                self._meta_labels[iana].setText(f"星期{weekday}")
                night = local.hour >= _NIGHT_HOUR_START or local.hour < _NIGHT_HOUR_END
                self._daynight_labels[iana].setText(f"{'☾' if night else '☀'} {local.hour} 时")
            except Exception as e:
                # 统一 logger.exception 带堆栈（与其他面板回调风格一致）
                logger.exception(f"更新世界时钟 {iana} 时出错: {e}")
                self._time_labels[iana].setText("00:00")

    def set_timezone(self, tz_name: str) -> None:
        # 按 IANA 标识切换基准时区（配置恢复与点击共用；矩阵外城市仅记录不描边）
        self._current_tz = tz_name
        self._last_second = -1  # 强制下一拍立即刷新
        self._refresh_selection()

    def current_timezone(self) -> str:
        # 当前基准 IANA 标识（供主窗口 save_settings 持久化）
        return self._current_tz


# ===== ui/panels/world_clock_panel.py 函数/类说明 =====
# _NIGHT_HOUR_START/_NIGHT_HOUR_END: 昼夜判定阈值（当地时刻）
# WorldClockPanel(QWidget): 世界时钟 4×2 常驻城市玻璃卡矩阵（PL007.02）
#   __init__(interface, parent): 城市集经接口读取（空集兜底全表）；卡含城市名/
#   时间（Bahnschrift）/星期 meta/昼夜指示；点击卡片切换基准时区
#   _city_name(iana): IANA → 城市短名（选项表显示名去括号，表外取路径尾段）
#   _refresh_selection(): 选中态金描边刷新（GlassCard.set_selected）
#   update_world_clock(): 主窗口 tick 调用；同秒跳过；选中卡显示秒，其余 HH:MM
#   set_timezone(tz_name)/current_timezone(): 配置恢复与持久化契约（签名不变）
#   异常处理：单卡 pytz 转换失败降级 00:00 并记录日志，不影响他卡
#   关联配置：时区表/常驻城市集（world_pins）经接口；字体/圆角经 ui.json
