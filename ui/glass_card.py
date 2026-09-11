# 玻璃材质组件模块（plan#UI2.0「时之砂」材质系统——GlassCard 与窗内光场渲染）
# T005 真机观感返工定案（2026-09-12 用户走查反馈）：视觉设计不得向测试环境妥协
# （AGENTS.md 操作注意），桌面端恢复原设计——真实 QLinearGradient/QRadialGradient
# 渐变、纹理按设备像素比渲染（消圆角毛刺）、柔投影效果弃用（毛刺来源之一）；
# offscreen 仅作功能参考：卡面不绘制、光场以实底近似（该平台栅格器有随机崩溃缺陷）
# 零 DWM 调用、零新线程、零样式表（plan#UI2.0 铁律；backdrop.py 定案不涉本模块）

import os

from PyQt6.QtCore import pyqtSignal, QRectF, Qt
from PyQt6.QtGui import QColor, QImage, QLinearGradient, QPainter, QPainterPath, QPen, QPixmap, QRadialGradient
from PyQt6.QtWidgets import QWidget

from qfluentwidgets import Theme, isDarkTheme, qconfig, setCustomStyleSheet

from interface import AppInterface

# offscreen 判定（仅测试/探针环境）：与 backdrop.py 的短路约定同源
_IS_OFFSCREEN = os.environ.get("QT_QPA_PLATFORM") == "offscreen"


def _rounded_path(w: int, h: int, radius: int) -> QPainterPath:
    # 圆角矩形路径：玻璃纹理的裁剪与描边基础
    path = QPainterPath()
    path.addRoundedRect(1.0, 1.0, max(w - 2.0, 0), max(h - 2.0, 0), radius, radius)
    return path


def _theme_tokens(tokens: dict, dark: bool) -> dict:
    # 按生效主题取深/浅参数组（token 节内固定为 dark/light 两键）
    return tokens["dark"] if dark else tokens["light"]


def apply_capsule(widget: QWidget) -> None:
    # 胶囊圆角：半径取控件高度一半；经 qfw 官方自定义样式接口注入
    # （仅圆角属性，不设背景，不影响 Acrylic 分层；PL006.04）
    name = widget.__class__.__name__
    radius = max(widget.height() // 2, 0)
    qss = f"{name}{{border-radius: {radius}px}}"
    setCustomStyleSheet(widget, qss, qss)


def render_field_pixmap(size_w: int, size_h: int, field_tokens: dict, dark: bool,
                        dpr: float = 1.0) -> QPixmap:
    # 窗内光场（PL006.03；T005 恢复原设计）：垂直线性底色 + 双径向光晕（金/紫或金/蓝），
    # 纹理按设备像素比渲染（缩放屏无锯齿）；offscreen 栅格器有随机崩溃缺陷，仅作实底参考
    dpr = max(dpr, 1.0)
    pix = QPixmap(round(size_w * dpr), round(size_h * dpr))
    pix.setDevicePixelRatio(dpr)
    pix.fill(Qt.GlobalColor.transparent)
    painter = QPainter(pix)
    tokens = _theme_tokens(field_tokens, dark)
    if _IS_OFFSCREEN:
        # 测试参考近似：实底色（不做渐变，规避该平台栅格器随机崩溃）
        base = QColor(tokens["top"])
        painter.fillRect(0, 0, size_w, size_h, base)
        painter.end()
        return pix
    tokens_dark_top = QColor(tokens["top"])
    tokens_bottom = QColor(tokens["bottom"])
    lin = QLinearGradient(0, 0, 0, size_h)
    lin.setColorAt(0.0, tokens_dark_top)
    lin.setColorAt(1.0, tokens_bottom)
    painter.setPen(Qt.PenStyle.NoPen)
    painter.setBrush(lin)
    painter.drawRect(0, 0, size_w, size_h)
    for key, rf in (("glow_a", "glow_a_r"), ("glow_b", "glow_b_r")):
        glow = QColor(tokens[key])
        cx = size_w * float(tokens[f"{key}_x"])
        cy = size_h * float(tokens[f"{key}_y"])
        radius = size_w * float(tokens[rf])
        radial = QRadialGradient(cx, cy, radius)
        radial.setColorAt(0.0, glow)
        transparent = QColor(glow)
        transparent.setAlpha(0)
        radial.setColorAt(1.0, transparent)
        painter.setBrush(radial)
        painter.drawRect(0, 0, size_w, size_h)
    painter.end()
    return pix


class GlassCard(QWidget):
    # 玻璃卡片容器：对角 tint + 顶部镜面高光描边（材质参数经接口读取）
    # 用法：GlassCard(interface, radius_key="xl|lg|md")，向其 layout 内加内容控件；
    # clicked 信号供可点击卡（如世界时钟矩阵，PL007.02）；set_selected 切换金描边
    clicked = pyqtSignal()

    def __init__(self, interface: AppInterface, radius_key: str = "lg",
                 parent: QWidget | None = None):
        super().__init__(parent)
        ui = interface.get_ui_static()
        self._glass_tokens = ui["glass"]
        self._radius = int(ui["radius"][radius_key])
        self._accent = QColor(ui["colors"]["primary"])
        self._selected = False
        self._pix: QPixmap | None = None
        # 主题变化：失效纹理缓存（重渲染惰性于 paintEvent；析构时由 PyQt 自动断连）
        qconfig.themeChanged.connect(self._on_theme_changed)

    def _on_theme_changed(self, theme: Theme) -> None:
        # 深浅切换：失效纹理缓存（重渲染惰性于 paintEvent）
        self._pix = None
        self.update()

    def set_selected(self, selected: bool) -> None:
        # 选中态（金描边）：世界时钟矩阵等可点卡的当前项指示（PL007.02）
        if self._selected != selected:
            self._selected = selected
            self._pix = None
            self.update()

    def mousePressEvent(self, event) -> None:
        # 左键点击发 clicked（可点卡契约；不可点卡无监听者无副作用）
        if event.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit()
        super().mousePressEvent(event)

    def _render_texture(self) -> None:
        # 玻璃纹理（T005 恢复原设计）：对角 QLinearGradient tint + 渐变描边笔
        # （顶部镜面高光→底部淡边；选中态整圈换强调色金）；按设备像素比渲染消毛刺
        dpr = max(self.devicePixelRatioF(), 1.0)
        img = QImage(round(self.width() * dpr), round(self.height() * dpr),
                     QImage.Format.Format_ARGB32_Premultiplied)
        img.setDevicePixelRatio(dpr)
        img.fill(0)
        painter = QPainter(img)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        w, h = self.width(), self.height()
        path = _rounded_path(w, h, self._radius)
        tokens = _theme_tokens(self._glass_tokens, isDarkTheme())
        fill_from = QColor(tokens["fill_from"])
        fill_to = QColor(tokens["fill_to"])
        tint = QLinearGradient(0, 0, w, h)
        tint.setColorAt(0.0, fill_from)
        tint.setColorAt(1.0, fill_to)
        painter.setPen(Qt.PenStyle.NoPen)
        painter.setBrush(tint)
        painter.drawPath(path)
        edge = QLinearGradient(0, 0, 0, h)
        if self._selected:
            edge.setColorAt(0.0, self._accent)
            edge.setColorAt(1.0, self._accent)
            pen = QPen()
            pen.setWidthF(1.4)
        else:
            edge.setColorAt(0.0, QColor(tokens["highlight"]))
            edge.setColorAt(1.0, QColor(tokens["border"]))
            pen = QPen()
            pen.setWidthF(1.0)
        pen.setBrush(edge)
        painter.setPen(pen)
        painter.setBrush(Qt.BrushStyle.NoBrush)
        painter.drawPath(path)
        painter.end()
        self._pix = QPixmap.fromImage(img)

    def resizeEvent(self, event) -> None:
        # 尺寸变化失效纹理缓存（重渲染惰性于 paintEvent）
        self._pix = None
        super().resizeEvent(event)

    def paintEvent(self, event) -> None:
        if _IS_OFFSCREEN:
            # offscreen 仅作功能参考：卡面不绘制（透明，子控件照常自绘）——
            # 该平台栅格器有随机崩溃缺陷，视觉实测以用户桌面为准（T005 定案）
            return
        if self._pix is None:
            self._render_texture()
        painter = QPainter(self)
        painter.drawPixmap(0, 0, self._pix)
        painter.end()


# ===== ui/glass_card.py 函数/类说明 =====
# _IS_OFFSCREEN: offscreen 平台判定（与 backdrop.py 短路约定同源）；
#   仅作测试参考降级开关，桌面端一律按原设计渲染（T005 定案）
# _rounded_path(w, h, radius): 圆角矩形路径（裁剪与描边基础）
# _theme_tokens(tokens, dark): 按生效主题取 dark/light 参数组
# apply_capsule(widget): 胶囊圆角（半径=高度一半；经 qfw setCustomStyleSheet 注入，
#   仅圆角属性不设背景，PL006.04）
# render_field_pixmap(size_w, size_h, field_tokens, dark, dpr): 窗内光场位图
#   （垂直线性底色 + QRadialGradient 双光晕；按设备像素比渲染；供主窗口
#   paintEvent 位块拷贝，PL006.03/T005）
# GlassCard(QWidget): 玻璃卡片容器
#   纹理：对角 tint 渐变 + 渐变描边笔（顶部镜面高光→底部淡边；选中整圈强调色金），
#   resize/主题变化失效重渲染，paintEvent 位块拷贝；按 DPR 渲染消缩放毛刺；
#   柔投影效果弃用（QGraphicsDropShadowEffect 中间缓冲为圆角毛刺来源，T005 定案）
#   设计理由：玻璃观感 = tint + 高光 + 透出的 Acrylic（field 低不透明度）；
#   零 DWM 调用零新线程零样式表（plan#UI2.0 铁律）
#   异常处理：无外部 IO/DWM 调用，无异常路径；主题信号回调由 PyQt 生命周期管理
#   关联配置：ui.json glass/field/radius 节（深浅两套）+ colors.primary（选中描边），
#   经 AppInterface 读取
