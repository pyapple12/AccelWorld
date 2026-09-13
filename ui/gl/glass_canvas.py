# 整窗 GL 玻璃画布（GlassCanvas）：单 pass 绘制程序化光场背景 + 场景玻璃面
# 静态帧缓存：轮询场景脏标记，仅在玻璃面注册/几何/选中变化或动画 touch 期重绘
# 配色注入：光场深浅两套 tokens 与强调色经构造传入（ui.json field/colors，零硬编码）
# Qt 回调防护（AGENTS.md 约定）：paintGL/initializeGL 全部 try/except 包裹——
# 回调内未捕获异常触发 PyQt6 fail-fast 终止进程，任何异常只降级记录、绝不外抛

import logging
import struct
import time

from PyQt6.QtCore import QRectF, QTimer
from PyQt6.QtGui import QColor
from PyQt6.QtOpenGL import (
    QOpenGLBuffer,
    QOpenGLShader,
    QOpenGLShaderProgram,
    QOpenGLVersionFunctionsFactory,
    QOpenGLVersionProfile,
)
from PyQt6.QtOpenGLWidgets import QOpenGLWidget

from ui.gl.glass_scene import GlassScene
from ui.gl.shaders import fragment_shader_source, vertex_shader_source

_MAX_SURFACES = 16  # 单帧玻璃面上限（与 shaders.py 的 uniform 数组长度一致；六药丸+七卡）
_GL_FLOAT = 5126  # GL_FLOAT（Qt6 C++ 宏不透出到 PyQt6，需数值字面量）
_GL_COLOR_BUFFER_BIT = 0x4000  # 同上
_DEGRADE_COLOR = (0.051, 0.075, 0.133, 1.0)  # 降级纯色（深空底，含 alpha）
_DIRTY_POLL_MS = 150  # 场景脏标记轮询周期（空闲期空转开销可忽略）


class GlassCanvas(QOpenGLWidget):
    # 整窗 GL 画布：所有玻璃面（SDF 圆角 + 折射/色散/rim/选中呼吸）单 pass 绘制；
    # 装配于主窗口最底层（lower），半透明输出透出 DWM Acrylic，页内容子控件位于其上
    def __init__(self, scene: GlassScene, dark: bool, field_tokens: dict,
                 accent: str, glass_tokens: dict | None = None,
                 parent=None) -> None:
        # 持场景引用与主题；field_tokens 为 ui.json field 节（dark/light 双套），
        # accent 为 colors.primary（选中金描边），glass_tokens 为 glass 节（lift 提亮）；
        # 预建着色器程序与 VBO 对象（真初始化在 initializeGL，该阶段尚无 GL 上下文）
        super().__init__(parent)
        self._scene = scene
        self._dark = dark
        self._field_tokens = field_tokens
        self._accent = accent
        self._glass_tokens = glass_tokens or {}
        self._prog = QOpenGLShaderProgram(self)
        self._funcs = None
        self._ready = False  # 着色器链路可用标记（编译/链接任一失败即 False 走降级）
        self._vbo = QOpenGLBuffer(QOpenGLBuffer.Type.VertexBuffer)
        self._theme_colors = self._parse_theme_colors()
        # 场景脏轮询：玻璃面注册/几何/选中变化或 hover 呼吸 touch 后重绘一帧
        self._poll = QTimer(self)
        self._poll.setInterval(_DIRTY_POLL_MS)
        self._poll.timeout.connect(self._poll_dirty)

    def _parse_theme_colors(self) -> dict:
        # 解析当前主题的 field 配色组（hex 含 alpha，QColor 按 #AARRGGBB 读取）与
        # glass.gl_lift 白色 veil 提亮值（缺省回退 lift）；异常回退深空底色
        # （配置缺键不致装配失败，行为与降级一致）
        try:
            theme = "dark" if self._dark else "light"
            tokens = self._field_tokens[theme]
            keys = ("top", "bottom", "glow_a", "glow_b")
            colors = {k: QColor(tokens[k]) for k in keys}
            glass = self._glass_tokens.get(theme, {})
            colors["lift"] = int(glass.get("gl_lift", glass.get("lift", 8)))
            return colors
        except Exception:  # noqa: BLE001 — 防御：配置缺键回退默认深空配色
            logging.getLogger(__name__).exception("field 配色解析失败，回退默认配色")
            fallback = QColor(13, 19, 34, 140)
            colors = {k: fallback for k in ("top", "bottom", "glow_a", "glow_b")}
            colors["lift"] = 26
            return colors

    def set_dark(self, dark: bool) -> None:
        # 主题切换（PL009.07 联动入口）：更新主题态并重解析配色，标记重绘
        self._dark = dark
        self._theme_colors = self._parse_theme_colors()
        self._scene.touch()

    def _gl(self):
        # 版本函数对象（懒获取并缓存）：PyQt6 的 QOpenGLContext 无 functions()，
        # 经 QOpenGLVersionFunctionsFactory 取 2.1 口径函数集
        if self._funcs is None:
            vp = QOpenGLVersionProfile()
            vp.setVersion(2, 1)
            self._funcs = QOpenGLVersionFunctionsFactory.get(vp, self.context())
            self._funcs.initializeOpenGLFunctions()
        return self._funcs

    def _poll_dirty(self) -> None:
        # 场景脏轮询：有玻璃面变化或动画 touch 才调度一帧重绘（静态期零重绘）
        if self._scene.dirty:
            self._scene.clear_dirty()
            self.update()

    def start(self) -> None:
        # 启动画布轮询（装配完成后由宿主调用；构造期启动会在画布未显示时空转）
        self._poll.start()

    def paintGL(self) -> None:
        # 防护性回调：GL 管线任何异常不外抛（fail-fast 防护），记录后降级纯色清屏
        # （GL 帧缓冲内容在绘制失败时未定义，必须覆盖，不能只跳过）
        try:
            self._paint_glass()
        except Exception as e:  # noqa: BLE001 — 防御：渲染失败降级为纯色，不上抛
            logging.getLogger(__name__).exception("GlassCanvas 渲染失败，降级纯色: %s", e)
            self._clear_solid()

    def _paint_glass(self) -> None:
        dpr = self.devicePixelRatioF()
        w_px, h_px = int(self.width() * dpr), int(self.height() * dpr)
        f = self._gl()
        f.glViewport(0, 0, w_px, h_px)
        if not self._ready:
            # 着色器链路未就绪（初始化/链接失败）：静默纯色清屏（不刷异常日志）
            f.glClearColor(*_DEGRADE_COLOR)
            f.glClear(_GL_COLOR_BUFFER_BIT)
            return
        top = self._theme_colors["top"]
        bottom = self._theme_colors["bottom"]
        glow_a = self._theme_colors["glow_a"]
        glow_b = self._theme_colors["glow_b"]
        tokens = self._field_tokens["dark" if self._dark else "light"]
        accent = QColor(self._accent)
        self._prog.bind()
        self._prog.setUniformValue("u_res", float(w_px), float(h_px))
        # uv 归一化空间（PL009 返工）：场景矩形换算 uv 系（y 底为原点，Qt 顶底
        # 语义翻转），折射/rim 等效果参数量纲随 shaders.py uv 体系生效
        w_log, h_log = max(self.width(), 1), max(self.height(), 1)
        surfaces = [s for s in self._scene.surfaces() if s.visible][:_MAX_SURFACES]
        self._prog.setUniformValue("u_count", len(surfaces))
        self._prog.setUniformValue("u_time", time.monotonic() % 3600.0)
        self._prog.setUniformValue("u_accent", float(accent.redF()),
                                   float(accent.greenF()), float(accent.blueF()))
        self._prog.setUniformValue("u_lift", float(self._theme_colors["lift"]) / 255.0)
        for i, s in enumerate(surfaces):
            rect = s.rect
            u_rect = QRectF(rect.x() / w_log,
                            (h_log - rect.y() - rect.height()) / h_log,
                            rect.width() / w_log, rect.height() / h_log)
            self._prog.setUniformValue(f"u_rect[{i}]", float(u_rect.x()),
                                       float(u_rect.y()),
                                       float(u_rect.width()),
                                       float(u_rect.height()))
            self._prog.setUniformValue(f"u_radius[{i}]", float(s.radius) / h_log)
            self._prog.setUniformValue(f"u_refr[{i}]", float(s.refraction))
            self._prog.setUniformValue(f"u_sel[{i}]", float(s.selected))
            tint = s.tint
            self._prog.setUniformValue(f"u_tint[{i}]",
                                       float(tint[0]), float(tint[1]), float(tint[2]))
        for name, color, px, py, rad in (
            ("u_top", top, None, None, None),
            ("u_bottom", bottom, None, None, None),
            ("u_glow_a", glow_a, tokens["glow_a_x"], tokens["glow_a_y"], tokens["glow_a_r"]),
            ("u_glow_b", glow_b, tokens["glow_b_x"], tokens["glow_b_y"], tokens["glow_b_r"]),
        ):
            self._prog.setUniformValue(name, float(color.redF()), float(color.greenF()),
                                       float(color.blueF()), float(color.alphaF()))
            if px is not None:
                # ui.json 的 y 值直传 uv（y=0 → uv 底/窗口底，与用户走查观感一致；
                # PL009 返工曾误加 1-y 翻转致光晕上下颠倒，已回退）
                self._prog.setUniformValue(f"{name}_pos", float(px), float(py))
                self._prog.setUniformValue(f"{name}_rad", float(rad))
        self._vbo.bind()
        self._prog.enableAttributeArray(0)
        self._prog.setAttributeBuffer(0, _GL_FLOAT, 0, 2, 0)
        f.glDrawArrays(5, 0, 4)  # 5 = GL_TRIANGLE_STRIP
        self._prog.disableAttributeArray(0)
        self._vbo.release()
        self._prog.release()

    def initializeGL(self) -> None:
        # 着色器编译链接 + 全屏四边形 VBO 上传（上下文就绪后一次性）；Qt 回调：
        # try/except 全包，失败置 _ready=False 记录，绝不外抛（fail-fast 防护）
        try:
            self._gl()
            self._prog.addShaderFromSourceCode(
                QOpenGLShader.ShaderTypeBit.Vertex, vertex_shader_source())
            self._prog.addShaderFromSourceCode(
                QOpenGLShader.ShaderTypeBit.Fragment, fragment_shader_source())
            self._prog.bindAttributeLocation("a_pos", 0)
            self._ready = self._prog.link()
            if not self._ready:
                logging.getLogger(__name__).error(
                    "GlassCanvas 着色器链接失败: %s", self._prog.log())
            quad = struct.pack("8f", -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0)
            self._vbo.create()
            self._vbo.bind()
            self._vbo.allocate(quad, len(quad))
            self._vbo.release()
        except Exception:  # noqa: BLE001 — 防御：初始化失败降级，不上抛
            self._ready = False
            logging.getLogger(__name__).exception("GlassCanvas 初始化失败，渲染降级纯色")

    def _clear_solid(self) -> None:
        # 降级纯色清屏；清屏自身再失败（如上下文已失效）仅 debug 记录，
        # 降级路径绝不允许二次抛出（否则仍会触发 fail-fast）
        try:
            f = self._gl()
            f.glClearColor(*_DEGRADE_COLOR)
            f.glClear(_GL_COLOR_BUFFER_BIT)
        except Exception:  # noqa: BLE001 — 防御：降级失败不外抛
            logging.getLogger(__name__).debug(
                "GlassCanvas 降级清屏失败（上下文不可用）", exc_info=True)


# ===== ui/gl/glass_canvas.py 函数/模块级常量说明 =====
# 模块级常量：
#   _MAX_SURFACES: 单帧玻璃面上限（与 shaders.py uniform 数组长度 8 一致，超出截断）
#   _GL_FLOAT / _GL_COLOR_BUFFER_BIT: OpenGL 固定枚举数值（Qt6 宏不透出到 PyQt6）
#   _DEGRADE_COLOR: 渲染失败降级纯色（深空底 RGBA，与暗色主题 u_top 同源）
#   _DIRTY_POLL_MS: 场景脏标记轮询周期（空闲期仅布尔比较，开销可忽略）
# GlassCanvas(QOpenGLWidget): 整窗 GL 画布
#   __init__(scene, dark, field_tokens, accent, parent): 持场景/主题/ui.json field
#     双套配色与强调色（零硬编码注入）；预建着色器程序/VBO 对象（无 GL 上下文，
#     不可编译上传）；_ready 初始 False；轮询定时器构造不启动（宿主 start()）
#   _parse_theme_colors(): 解析当前主题 field 四色（hex 含 alpha，#AARRGGBB）；
#     配置缺键回退默认深空配色（异常仅记录，装配不被打断）
#   set_dark(dark): 主题切换联动（PL009.07）——重解析配色并 touch 场景触发重绘
#   _gl(): 懒取并缓存 2.1 版本函数对象；PyQt6 的 QOpenGLContext 已移除 functions()，
#     必须 经 QOpenGLVersionFunctionsFactory.get(profile, context)（PL008.10 定案）
#   _poll_dirty()/start(): 轮询场景脏标记 → 调度单帧重绘（静态帧缓存；hover 呼吸
#     由导航高频 touch 驱动连续重绘，动画停止即回归零重绘）
#   initializeGL(): Qt 回调（上下文首帧就绪）——编译链接顶点/片段着色器 +
#     bindAttributeLocation("a_pos", 0) + 全屏四边形 VBO 上传；try/except 全包，
#     失败置 _ready=False 并记录
#   paintGL(): Qt 回调——try/except 全包调 _paint_glass；异常 → exception 级日志 +
#     _clear_solid 降级纯色（帧缓冲未定义内容必须覆盖）
#   _paint_glass(): 设 uniforms（分辨率/时间/强调色/玻璃面矩形圆角折射选中 tint/
#     深浅配色四组 vec4 含 alpha + 光晕位置半径）→ VBO 绑定 + setAttributeBuffer
#     (location 0, GL_FLOAT, 偏移 0, 2 分量, 步长 0) → glDrawArrays(TRIANGLE_STRIP,
#     4 顶点) 单 pass；_ready=False 时纯色清屏直接返回（静默降级不刷日志）
#   _clear_solid(): 降级清屏实现；自身再失败仅 debug 记录（降级路径禁止二次抛出）
#   异常处理：paintGL/initializeGL 两个 Qt 回调 try/except 全包——PyQt6 回调内
#     未捕获 Python 异常触发 fail-fast 终止进程（0xC0000409 零输出，即 GL 探针
#     127 硬崩根因；AGENTS.md「Qt 回调防护约定」）；非回调函数不吞异常正常上抛
#   设计理由：静态帧缓存（轮询脏标记）+ 动画期 touch 高频驱动；程序化光场单 pass
#     避免 FBO 翻转复杂度；VBO + setAttributeBuffer 一次性上传顶点数据
#   关联配置：ui.json gl_enabled × capability.gl_available 决定画布是否被装配；
#     field 节（深浅双套 top/bottom/glow_x/y/r）+ colors.primary（选中金）经宿主注入；
#     着色器源码 ui/gl/shaders.py；场景注册表 ui/gl/glass_scene.py（SCENE 单例）
