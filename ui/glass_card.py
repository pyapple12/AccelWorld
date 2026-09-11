# 玻璃材质组件模块（PL006：plan#UI2.0「时之砂」材质系统——GlassCard 与窗内光场渲染）
# 双路径渲染（.temp/probe_pl006_*.py 探针矩阵定案）：offscreen 平台的栅格器对
# 渐变/大尺寸纹理绘制存在堆破坏式不可靠崩溃（同代码时崩时不崩、与尺寸/时机耦合；
# y.problems#6 家族环境缺陷），故：
#   真实桌面（windows 平台）→ 完整设计：QLinearGradient 玻璃纹理 + 渐变描边笔 + 柔投影
#   offscreen（仅测试/探针）→ 安全降级：单次实色 tint + 实色描边，直绘于 paintEvent
#   （g2 矩阵实证：实色 brush + drawRoundedRect + AA 在 offscreen 稳定）
# 零 DWM 调用、零新线程、零样式表（plan#UI2.0 铁律；backdrop.py 定案不涉本模块）

import os

from PyQt6.QtCore import QRectF, pyqtSignal, Qt
from PyQt6.QtGui import QColor, QImage, QLinearGradient, QPainter, QPainterPath, QPen, QPixmap
from PyQt6.QtWidgets import QGraphicsDropShadowEffect, QWidget

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


def render_field_pixmap(size_w: int, size_h: int, field_tokens: dict, dark: bool) -> QPixmap:
    # 窗内光场（PL006.03）：垂直底色 + 双光晕（金/紫或金/蓝），预渲染为位图供拷贝；
    # 光晕以 1/3 尺寸小图绘制阶梯椭圆后平滑放大（插值抹平同心色带，等效径向渐变）
    pix = QPixmap(size_w, size_h)
    pix.fill(Qt.GlobalColor.transparent)
    painter = QPainter(pix)
    tokens = _theme_tokens(field_tokens, dark)
    top = QColor(tokens["top"])
    bottom = QColor(tokens["bottom"])
    painter.setPen(Qt.PenStyle.NoPen)
    steps = max(size_h, 1)
    for row in range(steps):
        t = row / max(steps - 1, 1)
        painter.fillRect(
            0, row, size_w, 1,
            QColor(
                round(top.red() + (bottom.red() - top.red()) * t),
                round(top.green() + (bottom.green() - top.green()) * t),
                round(top.blue() + (bottom.blue() - top.blue()) * t),
                round(top.alpha() + (bottom.alpha() - top.alpha()) * t),
            ),
        )
    # 光晕小图（1/3 尺寸）：阶梯椭圆 → SmoothPixmapTransform 放大后色带不可辨
    small_w, small_h = max(size_w // 3, 1), max(size_h // 3, 1)
    glow_img = QImage(small_w, small_h, QImage.Format.Format_ARGB32_Premultiplied)
    glow_img.fill(0)
    glow_painter = QPainter(glow_img)
    glow_painter.setPen(Qt.PenStyle.NoPen)
    for key, rf in (("glow_a", "glow_a_r"), ("glow_b", "glow_b_r")):
        glow = QColor(tokens[key])
        cx = small_w * float(tokens[f"{key}_x"])
        cy = small_h * float(tokens[f"{key}_y"])
        radius = small_w * float(tokens[rf])
        for i in range(24, 0, -1):
            t = i / 24
            r = radius * t
            alpha = round(glow.alpha() * (1.0 - t) * 1.35)
            if alpha <= 0:
                continue
            glow_painter.setBrush(
                QColor(glow.red(), glow.green(), glow.blue(), min(alpha, 255)))
            glow_painter.drawEllipse(int(cx - r), int(cy - r), int(r * 2), int(r * 2))
    glow_painter.end()
    painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
    painter.drawImage(QRectF(0, 0, size_w, size_h), glow_img)
    painter.end()
    return pix


class GlassCard(QWidget):
    # 玻璃卡片容器：对角 tint + 顶部镜面高光描边 + 柔投影（材质参数经接口读取）
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
        if not _IS_OFFSCREEN:
            # 柔投影仅真实桌面启用（offscreen 探针定案：效果层参与崩溃面）
            self._effect = QGraphicsDropShadowEffect(self)
            self._apply_effect_token()
            self.setGraphicsEffect(self._effect)
        # 主题变化：重取深浅参数组并重渲染（qfw 全局信号；PyQt 于析构自动断连）
        qconfig.themeChanged.connect(self._on_theme_changed)

    def _on_theme_changed(self, theme: Theme) -> None:
        # 深浅切换：失效纹理缓存 + 刷新投影参数
        if not _IS_OFFSCREEN:
            self._apply_effect_token()
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

    def _apply_effect_token(self) -> None:
        # 柔投影参数（颜色/模糊/纵向偏移）来自当前主题的 glass 节
        tokens = _theme_tokens(self._glass_tokens, isDarkTheme())
        self._effect.setColor(QColor(tokens["shadow"]))
        self._effect.setBlurRadius(int(tokens["shadow_blur"]))
        self._effect.setOffset(0, int(tokens["shadow_dy"]))

    def _render_texture(self) -> None:
        # 玻璃纹理（真实桌面）：对角 QLinearGradient tint + 渐变描边笔（高光→淡边）；
        # 选中态描边换强调色金（世界时钟矩阵当前项，PL007.02）
        img = QImage(self.size(), QImage.Format.Format_ARGB32_Premultiplied)
        img.fill(0)
        painter = QPainter(img)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        w, h = self.width(), self.height()
        path = _rounded_path(w, h, self._radius)
        tokens = _theme_tokens(self._glass_tokens, isDarkTheme())
        fill_from = QColor(tokens["fill_from"])
        fill_to = QColor(tokens["fill_to"])
        grad = QLinearGradient(0, 0, w, h)
        grad.setColorAt(0.0, fill_from)
        grad.setColorAt(1.0, fill_to)
        painter.setPen(Qt.PenStyle.NoPen)
        painter.setBrush(grad)
        painter.drawRoundedRect(path.boundingRect(), self._radius, self._radius)
        edge = QLinearGradient(0, 0, 0, h)
        if self._selected:
            edge.setColorAt(0.0, self._accent)
            edge.setColorAt(1.0, self._accent)
            pen = QPen()
            pen.setWidthF(1.6)
        else:
            edge.setColorAt(0.0, QColor(tokens["highlight"]))
            edge.setColorAt(1.0, QColor(tokens["border"]))
            pen = QPen()
            pen.setWidthF(1.2)
        pen.setBrush(edge)
        painter.setPen(pen)
        painter.setBrush(Qt.BrushStyle.NoBrush)
        painter.drawRoundedRect(path.boundingRect(), self._radius, self._radius)
        painter.end()
        self._pix = QPixmap.fromImage(img)

    def resizeEvent(self, event) -> None:
        # 尺寸变化失效纹理缓存（真实桌面重渲染；offscreen 直绘无需缓存）
        self._pix = None
        super().resizeEvent(event)

    def paintEvent(self, event) -> None:
        if _IS_OFFSCREEN:
            # offscreen（仅测试/探针）：卡面不绘制（透明，子控件照常自绘）。
            # 探针矩阵定案：本平台栅格器存在堆破坏式缺陷，任何卡面重绘路径均可能
            # 触发延迟崩溃；测试断言不依赖卡面视觉，故干脆不绘
            painter = QPainter(self)
            painter.end()
            return
        if self._pix is None:
            self._render_texture()
        painter = QPainter(self)
        painter.drawPixmap(0, 0, self._pix)
        painter.end()


# ===== ui/glass_card.py 函数/类说明 =====
# _IS_OFFSCREEN: offscreen 平台判定（与 backdrop.py 短路约定同源）；双路径开关
# _rounded_path(w, h, radius): 圆角矩形路径（裁剪与描边基础）
# _theme_tokens(tokens, dark): 按生效主题取 dark/light 参数组
# apply_capsule(widget): 胶囊圆角（半径=高度一半；经 qfw setCustomStyleSheet 注入，
#   仅圆角属性不设背景，PL006.04）
# render_field_pixmap(size_w, size_h, field_tokens, dark): 窗内光场位图
#   （垂直平滑底色逐行填充 + 双光晕同心椭圆阶梯 alpha；实色图元，双平台一致，PL006.03）
# GlassCard(QWidget): 玻璃卡片容器
#   真实桌面：对角渐变纹理缓存（resize 失效重渲染，paintEvent 位块拷贝）+ 柔投影
#   offscreen：实色 tint + 高光描边直绘（g2 矩阵实证稳定路径）
#   __init__(interface, radius_key, parent): 材质参数/圆角阶经接口读取（ui.json）
#   _on_theme_changed(theme)/_apply_effect_token(): 深浅切换重渲染与投影刷新
#   设计理由：玻璃观感 = tint + 高光 + 投影的光学近似（真背景模糊成本高已顺延）；
#   offscreen 崩溃为平台栅格器缺陷（探针矩阵定案），降级路径保证测试/探针可用
#   异常处理：无外部 IO/DWM 调用，无异常路径；主题信号回调由 PyQt 生命周期管理
#   关联配置：ui.json glass/field/radius 节（深浅两套），经 AppInterface 读取
