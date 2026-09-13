# 玻璃药丸导航栏模块（GlassNavRail，PL009.01）：UI3.0 GL 路径专属侧栏，
# 替换 qfw NavigationInterface（方案定案：唯一替换的 qfw 组件）；
# 栅格降级路径保留 qfw 导航（本模块仅在 GL 模式被装配）
# 玻璃面注册 glass_scene 由 GL 画布着色器统一绘制（SDF 胶囊 + 选中金描边 +
# hover 呼吸经 u_sel/u_time 着色器化）；icon+文字为 Qt 叠层（子控件位于画布之上）

import logging

from PyQt6.QtCore import Qt, QTimer, pyqtSignal
from PyQt6.QtCore import QPoint, QPointF, QSizeF, QRectF
from PyQt6.QtGui import QColor
from PyQt6.QtWidgets import QLabel, QVBoxLayout, QWidget

from qfluentwidgets import CaptionLabel, FluentIconBase, Theme, isDarkTheme

from ui.gl.glass_scene import SCENE, GlassSurface

_BREATHE_MS = 33  # hover 呼吸重绘节拍（≈30fps；停止 hover 即停表回零重绘）


class GlassNavPill(QWidget):
    # 单个玻璃药丸：玻璃面（胶囊 SDF）由画布绘制，icon+文字 Qt 叠层；
    # hover 时 selected=0.35（rim 微亮 + 轻呼吸），选中 selected=1.0（金描边呼吸）
    def __init__(self, page_id: str, icon: FluentIconBase, text: str,
                 tokens: dict, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._page_id = page_id
        self._icon = icon
        self._icon_size = int(tokens["nav_icon_size"])
        self._selected = False
        self._hover = False
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setObjectName(page_id)

        body = QVBoxLayout(self)
        body.setContentsMargins(0, 0, 0, 0)
        body.setSpacing(2)
        body.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._icon_label = QLabel(self)
        self._icon_label.setFixedSize(self._icon_size, self._icon_size)
        self._icon_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._text_label = CaptionLabel(text, self)
        body.addWidget(self._icon_label, 0, Qt.AlignmentFlag.AlignCenter)
        body.addWidget(self._text_label, 0, Qt.AlignmentFlag.AlignCenter)

        self._surface_id = f"nav-pill-{page_id}"
        self._refresh_icon()
        # hover 呼吸节拍表：进入 hover 后高频 touch 场景驱动画布重绘
        # （u_time 在画布侧取单调钟，着色器 sin 项即连续动画）
        self._breathe_timer = QTimer(self)
        self._breathe_timer.setInterval(_BREATHE_MS)
        self._breathe_timer.timeout.connect(SCENE.touch)

    def _refresh_icon(self) -> None:
        # 图标渲染（主题色跟随 qfw AUTO；按设备像素比放大取样再降比，缩放屏不糊）
        theme = Theme.DARK if isDarkTheme() else Theme.LIGHT
        pm = self._icon.icon(theme=theme).pixmap(
            self._icon_size * 2, self._icon_size * 2)
        pm.setDevicePixelRatio(2.0)
        self._icon_label.setPixmap(pm)
        self._text_label.repaint()

    def page_id(self) -> str:
        # 药丸对应页面标识（宿主据此接 switchTo）
        return self._page_id

    def set_selected(self, selected: bool) -> None:
        # 选中态切换：写场景面选中强度（1.0 金描边呼吸 / hover 0.35 / 其余 0）
        self._selected = selected
        self._apply_selection()

    def _apply_selection(self) -> None:
        # 选中强度合成：选中优先；hover 加成（着色器侧 rim 染金强度与呼吸幅度联动）；
        # 强度无变化不写场景（防重复脏标记），touch 交由调用语义保证
        strength = 1.0 if self._selected else (0.35 if self._hover else 0.0)
        if strength != self._last_written_strength():
            for s in SCENE.surfaces():
                if s.surface_id == self._surface_id:
                    s.selected = strength
        SCENE.touch()

    def _last_written_strength(self) -> float:
        # 读取场景内当前选中强度（避免重复写触发无谓脏标记；找不到按 -1 处理）
        for s in SCENE.surfaces():
            if s.surface_id == self._surface_id:
                return s.selected
        return -1.0

    def on_theme_changed(self) -> None:
        # 主题切换（宿主转发）：图标/文字色随深浅重取
        self._refresh_icon()

    def mousePressEvent(self, event) -> None:
        # 左键点击发导航请求（父控件即 GlassNavRail，直接上浮信号）
        if event.button() == Qt.MouseButton.LeftButton:
            rail = self.parent()
            if isinstance(rail, GlassNavRail):
                rail.navigate.emit(self._page_id)
        super().mousePressEvent(event)

    def enterEvent(self, event) -> None:
        # hover 进入：呼吸节拍表启动（着色器 sin(u_time) 需连续重绘帧）
        self._hover = True
        self._apply_selection()
        self._breathe_timer.start()
        super().enterEvent(event)

    def leaveEvent(self, event) -> None:
        # hover 离开：停表回零重绘（回归静态帧缓存零重绘态）
        self._hover = False
        self._breathe_timer.stop()
        self._apply_selection()
        super().leaveEvent(event)

    def _window_rect(self) -> QRectF:
        # 药丸在窗口坐标系的矩形（GL 画布铺整窗，场景几何一律窗口坐标）
        origin = self.mapTo(self.window(), QPoint(0, 0))
        return QRectF(QPointF(origin), QSizeF(self.size()))

    def _sync_geometry(self) -> None:
        # 场景几何同步（show/move/resize 驱动）；注册缺失（首次 show 前）则注册
        rect = self._window_rect()
        known = any(s.surface_id == self._surface_id for s in SCENE.surfaces())
        if known:
            SCENE.update_geometry(self._surface_id, rect)
        else:
            SCENE.register(GlassSurface(
                surface_id=self._surface_id,
                rect=rect,
                radius=max(self.height() / 2, 1.0),  # 胶囊：端帽半径 = 高度一半
            ))
        SCENE.touch()

    def showEvent(self, event) -> None:
        self._sync_geometry()
        super().showEvent(event)

    def moveEvent(self, event) -> None:
        self._sync_geometry()
        super().moveEvent(event)

    def resizeEvent(self, event) -> None:
        # Qt 回调防护（AGENTS.md 约定）：几何同步异常仅记录不上抛
        try:
            self._sync_geometry()
        except Exception:  # noqa: BLE001 — 防御：同步失败不外抛
            logging.getLogger(__name__).exception("GlassNavPill 场景几何同步失败")
        super().resizeEvent(event)


class GlassNavRail(QWidget):
    # 竖排玻璃药丸导航栏：六个页面药丸顶部对齐排列；navigate(page_id) 信号由
    # main_window 现有切换逻辑（switchTo）接收——只换皮不换交互（方案定案）
    navigate = pyqtSignal(str)

    def __init__(self, interface, items: list, parent: QWidget | None = None) -> None:
        # items: [(page_id, FluentIconBase, text)]，顺序即展示顺序（设置页在末位）
        super().__init__(parent)
        tokens = interface.get_ui_static()["layout"]
        self.setFixedWidth(int(tokens["nav_rail_width"]))
        self.setObjectName("glass-nav-rail")

        body = QVBoxLayout(self)
        body.setContentsMargins(0, int(tokens["nav_rail_margin_top"]), 0, 0)
        body.setSpacing(int(tokens["nav_pill_spacing"]))
        self._pills: list[GlassNavPill] = []
        for page_id, icon, text in items:
            pill = GlassNavPill(page_id, icon, text, tokens, self)
            pill.setFixedSize(int(tokens["nav_rail_width"]) - 2 * int(tokens["nav_pill_pad"]),
                              int(tokens["nav_pill_height"]))
            body.addWidget(pill, 0, Qt.AlignmentFlag.AlignHCenter)
            self._pills.append(pill)
        body.addStretch(1)

    def set_current(self, page_id: str) -> None:
        # 选中态切换：仅目标药丸置金（其余复位），场景重绘由药丸 touch 驱动
        for pill in self._pills:
            pill.set_selected(pill.page_id() == page_id)

    def on_theme_changed(self) -> None:
        # 主题切换（宿主转发）：全部药丸图标/文字色重取
        for pill in self._pills:
            pill.on_theme_changed()

    def pill_for_page(self, page_id: str) -> GlassNavPill | None:
        # 按页面标识取药丸（宿主切页后回填选中态用）
        for pill in self._pills:
            if pill.page_id() == page_id:
                return pill
        return None


# ===== ui/panels/glass_nav.py 函数/类说明 =====
# _BREATHE_MS: hover 呼吸重绘节拍（33ms ≈ 30fps，动画期外零重绘）
# GlassNavPill(QWidget): 玻璃药丸（icon+文字 Qt 叠层，玻璃面场景注册画布绘制）
#   __init__(page_id, icon, text, tokens, parent): objectName=page_id（切页断言用）；
#     hover 呼吸节拍表构造不启动（enterEvent 启动/leaveEvent 停止）
#   _refresh_icon(): FluentIcon 按主题深浅渲染位图（2x 取样 + DPR 降比，缩放屏不糊）
#   set_selected(selected)/_apply_selection(): 选中强度写场景面（1.0 选中金 /
#     0.35 hover / 0 复位）+ touch 场景触发重绘
#   _rail_navigate(): 经 window().findChild(GlassNavRail) 上浮导航信号
#     （药丸自身不持 rail 引用，避免装配期环依赖）
#   enterEvent/leaveEvent: hover 呼吸启停（Qt 回调；离开即回归静态零重绘）
#   _window_rect()/_sync_geometry(): 窗口坐标换算与场景注册/更新
#     （show/move/resize 驱动；注册缺失时按 胶囊半径=高/2 补注册）
#   showEvent/moveEvent/resizeEvent: Qt 回调（resizeEvent 防护包裹，AGENTS 约定）
# GlassNavRail(QWidget): 竖排玻璃药丸导航栏
#   __init__(interface, items, parent): 固定宽 nav_rail_width；药丸尺寸/间距/
#     图标尺寸/顶部留白均读 ui.json layout 导航键（零硬编码）
#   set_current(page_id): 选中态切换（宿主切页后调用）
#   on_theme_changed(): 主题联动（宿主 _apply_theme_preference 转发）
#   pill_for_page(page_id): 按页面标识取药丸
#   navigate(str) 信号: 宿主接现有 switchTo 切页逻辑（只换皮不换交互）
#   异常处理：仅 Qt 回调 resizeEvent 防护包裹；其余路径不吞异常正常上抛
#   关联配置：ui.json layout 节导航键（nav_rail_width/nav_pill_height/
#     nav_pill_spacing/nav_pill_pad/nav_icon_size/nav_rail_margin_top）；
#     玻璃视觉由 ui/gl 画布与着色器（u_sel/u_accent/u_time）承担
