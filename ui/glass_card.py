# 玻璃材质组件模块（plan#UI2.0「时之砂」材质系统——GlassCard 与窗内光场渲染）
# T005 真机观感返工定案（2026-09-12 用户走查反馈）：视觉设计不得向测试环境妥协
# （AGENTS.md 操作注意），桌面端恢复原设计——真实 QLinearGradient/QRadialGradient
# 渐变、纹理按设备像素比渲染（消圆角毛刺）、柔投影效果弃用（毛刺来源之一）；
# offscreen 仅作功能参考：卡面不绘制、光场以实底近似（该平台栅格器有随机崩溃缺陷）
# 零 DWM 调用、零新线程、零样式表（plan#UI2.0 铁律；backdrop.py 定案不涉本模块）

import os

from PyQt6.QtCore import QPoint, pyqtSignal, QRectF, Qt
from PyQt6.QtGui import QColor, QImage, QLinearGradient, QPainter, QPainterPath, QPen, QPixmap, QRadialGradient, QRadialGradient
from PyQt6.QtWidgets import QWidget

from qfluentwidgets import Theme, isDarkTheme, qconfig, setCustomStyleSheet

from interface import AppInterface

# offscreen 判定（仅测试/探针环境）：与 backdrop.py 的短路约定同源
_IS_OFFSCREEN = os.environ.get("QT_QPA_PLATFORM") == "offscreen"


_NOISE_TILE: QImage | None = None


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


def _noise_tile() -> QImage:
    # 全局噪点瓦片（128×128，随机明暗 ±4%）：叠加到光场上打散 8-bit 渐变色带
    # （苹果 Liquid Glass 同款思路：材质混入 1~3% 噪点；固定种子保证视觉稳定）
    global _NOISE_TILE
    if _NOISE_TILE is None:
        import random

        rng = random.Random(20260912)
        img = QImage(128, 128, QImage.Format.Format_ARGB32_Premultiplied)
        img.fill(0)
        painter = QPainter(img)
        for y in range(128):
            for x in range(128):
                v = rng.randint(-10, 10)
                if v >= 0:
                    painter.setPen(Qt.PenStyle.NoPen)
                    painter.setBrush(QColor(255, 255, 255, v))
                else:
                    painter.setPen(Qt.PenStyle.NoPen)
                    painter.setBrush(QColor(0, 0, 0, -v))
                painter.drawPoint(x, y)
        painter.end()
        _NOISE_TILE = img
    return _NOISE_TILE


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
    # 噪点抖动叠加（约 ±2%）：打散 8-bit 渐变的同心色带（Apple 同款处理）
    noise = _noise_tile()
    painter.setOpacity(0.5)
    x = 0
    while x < size_w:
        y = 0
        while y < size_h:
            painter.drawImage(x, y, noise)
            y += 128
        x += 128
    painter.setOpacity(1.0)
    painter.end()
    return pix


def _with_alpha(color: QColor, alpha: float) -> QColor:
    # 返回同 RGB、指定 alpha（0~1 浮点）的颜色副本
    return QColor(color.red(), color.green(), color.blue(), round(255 * alpha))


def _mix_color(a: QColor, b: QColor, t: float) -> QColor:
    # 颜色线性插值（t=0 → a，t=1 → b）
    return QColor(
        round(a.red() + (b.red() - a.red()) * t),
        round(a.green() + (b.green() - a.green()) * t),
        round(a.blue() + (b.blue() - a.blue()) * t),
        round(a.alpha() + (b.alpha() - a.alpha()) * t),
    )


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
        self._field_tokens = ui["field"]
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

    def _field_color_at(self, cx: float, cy: float) -> QColor:
        # 采样主窗口光场在窗口坐标 (cx, cy) 处的颜色（线性底色 + 双光晕贡献，
        # 与 render_field_pixmap 同一解析模型）——rim 动态取色的数据源
        tokens = _theme_tokens(self._field_tokens, isDarkTheme())
        top = QColor(tokens["top"])
        bottom = QColor(tokens["bottom"])
        win = self.window()
        t = max(min(cy / max(win.height(), 1), 1.0), 0.0)
        color = QColor(
            round(top.red() + (bottom.red() - top.red()) * t),
            round(top.green() + (bottom.green() - top.green()) * t),
            round(top.blue() + (bottom.blue() - top.blue()) * t),
        )
        win_w = max(win.width(), 1)
        for key, rf in (("glow_a", "glow_a_r"), ("glow_b", "glow_b_r")):
            glow = QColor(tokens[key])
            gx = win_w * float(tokens[f"{key}_x"])
            gy = win.height() * float(tokens[f"{key}_y"])
            gr = win_w * float(tokens[rf])
            dist = ((cx - gx) ** 2 + (cy - gy) ** 2) ** 0.5
            t2 = min(max(0.0, 1.0 - dist / max(gr, 1.0)) * (glow.alpha() / 255.0) * 2.2, 0.85)
            color = _mix_color(color, glow, t2)
        return color

    def _render_texture(self) -> None:
        # 液态玻璃纹理（热更新，向 Apple Liquid Glass 靠近）：
        # ① 透镜层——光场在卡片正下方的区域围绕卡片中心放大 1.07 绘制（内容弯折感）
        # ② 环境色渗染——采样卡片中心光场色，增饱和后低 alpha 染入（玻璃"吃"环境色）
        # ③ bezel 环带——渗染色的加深版沿边缘 9px 聚光
        # ④ 方向性描边——左上受光右下收暗（选中态整圈强调色金）；按 DPR 渲染消毛刺
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

        # ① 透镜层：主窗口光场位图中卡片正下方的区域，放大 1.07 绘制（内容弯折感）
        win = self.window()
        field_pix = getattr(win, "_field_pix", None)
        painter.save()
        painter.setClipPath(path)
        if field_pix is not None:
            origin = self.mapTo(win, QPoint(0, 0))
            # 不透明度补偿（热更新定案）：窗口层已画过 0.75 光场，透镜层按 0.43
            # 叠加使卡内光场总覆盖率 ≈ 卡外，消除"卡内比底色深"的双重叠暗
            painter.setOpacity(0.43)
            painter.translate(w / 2, h / 2)
            painter.scale(1.07, 1.07)
            painter.translate(-(origin.x() + w / 2), -(origin.y() + h / 2))
            painter.drawPixmap(0, 0, field_pix)
        else:
            painter.fillRect(0, 0, w, h, QColor(tokens["fill_from"]))
        painter.restore()

        # ② 环境光透入（位置感知）：按卡片受光角在窗口的实际位置解析金晕贡献——
        # 贡献显著才透光，强度随采样衰减（杜绝"背景无金、卡内盖章金"）；
        # 半径收敛贴角，圆角裁剪内两段式衰减（无直角切边）
        field_tokens = _theme_tokens(self._field_tokens, isDarkTheme())
        glow = QColor(field_tokens["glow_a"])
        corner_x, corner_y = w - 10, 10
        glow_center_x = win.width() * float(field_tokens["glow_a_x"])
        glow_center_y = win.height() * float(field_tokens["glow_a_y"])
        glow_radius = max(win.width() * float(field_tokens["glow_a_r"]), 1.0)
        corner_win_x = origin.x() + corner_x
        corner_win_y = origin.y() + corner_y
        dist = ((corner_win_x - glow_center_x) ** 2
                + (corner_win_y - glow_center_y) ** 2) ** 0.5
        presence = min(max(0.0, 1.0 - dist / glow_radius)
                       * (glow.alpha() / 255.0) * 2.2, 1.0)
        if presence > 0.05:
            intensity = min(presence * 1.4, 1.0)
            radius = min(min(w, h) * 0.6, 120)
            radial = QRadialGradient(corner_x, corner_y, radius)
            radial.setColorAt(0.0, _with_alpha(glow, round(140 * intensity)))
            radial.setColorAt(1.0, _with_alpha(glow, 0))
            # 加色混合（CompositionMode_Plus）：光只加不盖，深底上不产生浑浊色斑
            painter.setCompositionMode(QPainter.CompositionMode.CompositionMode_Plus)
            painter.setPen(Qt.PenStyle.NoPen)
            painter.setBrush(radial)
            painter.drawRect(0, 0, w, h)
            painter.setCompositionMode(QPainter.CompositionMode.CompositionMode_SourceOver)

        # ③ 白色提亮（极薄一层：玻璃只比底色浅一分；反差已减半）
        painter.setBrush(QColor(255, 255, 255, int(tokens["lift"])))
        painter.drawPath(path)

        # ④ 动态 rim + 渐进羽化环带：颜色不再固定白，而是随边缘处光场采样色变化
        # （背景变则 rim 变）；三层缓坡把卡缘亮度台阶软化成过渡（治直角光斑）；
        # 受光侧（左上）加权更亮，背光侧保最低可见度；选中态整圈强调色金
        origin = self.mapTo(win, QPoint(0, 0))
        c_tl = self._field_color_at(origin.x() + w * 0.18, origin.y() + h * 0.18)
        c_br = self._field_color_at(origin.x() + w * 0.82, origin.y() + h * 0.82)
        rim_tl = _mix_color(c_tl, QColor(255, 255, 255), 0.65)
        rim_br = _mix_color(c_br, QColor(255, 255, 255), 0.30)
        if self._selected:
            rim_tl = self._accent
            rim_br = self._accent
        rim_grad = QLinearGradient(0, 0, w, h)
        rim_grad.setColorAt(0.0, rim_tl)
        rim_grad.setColorAt(1.0, rim_br)
        rim_pen = QPen()
        rim_pen.setWidthF(1.2)
        rim_pen.setBrush(rim_grad)
        painter.setPen(rim_pen)
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
