# 整窗 GL 玻璃画布（GlassCanvas）：单 pass 绘制程序化光场背景 + 场景玻璃面
# 静态帧缓存：paintGL 仅在 update() 调度时运行（GL 模式下无定时器即零重绘）
# Qt 回调防护（AGENTS.md 约定）：paintGL/initializeGL 全部 try/except 包裹——
# 回调内未捕获异常触发 PyQt6 fail-fast 终止进程，任何异常只降级记录、绝不外抛

import logging
import struct

from PyQt6.QtCore import QRectF
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

_MAX_SURFACES = 8  # 单帧玻璃面上限（与 shaders.py 的 uniform 数组长度一致，超出截断）
_GL_FLOAT = 5126  # GL_FLOAT（Qt6 C++ 宏不透出到 PyQt6，需数值字面量）
_GL_COLOR_BUFFER_BIT = 0x4000  # 同上
_DEGRADE_COLOR = (0.051, 0.075, 0.133, 1.0)  # 降级纯色（深空底，含 alpha）


class GlassCanvas(QOpenGLWidget):
    # 整窗 GL 画布：所有玻璃面（SDF 圆角 + 折射/色散/rim）单 pass 绘制；
    # 高度由调用方设为 轨高 + 2×TRACK_PAD，控件透明底叠 Acrylic
    def __init__(self, scene: GlassScene, dark: bool = True,
                 parent=None) -> None:
        # 持场景引用与主题；预建着色器程序与 VBO 对象（真初始化在 initializeGL，
        # 该阶段尚无 GL 上下文，不可编译/上传）
        super().__init__(parent)
        self._scene = scene
        self._dark = dark
        self._prog = QOpenGLShaderProgram(self)
        self._funcs = None
        self._ready = False  # 着色器链路可用标记（编译/链接任一失败即 False 走降级）
        self._vbo = QOpenGLBuffer(QOpenGLBuffer.Type.VertexBuffer)

    def set_dark(self, dark: bool) -> None:
        # 主题切换：更新配色并标记重绘（PL009 主题联动入口）
        self._dark = dark
        self.update()

    def _gl(self):
        # 版本函数对象（懒获取并缓存）：PyQt6 的 QOpenGLContext 无 functions()，
        # 经 QOpenGLVersionFunctionsFactory 取 2.1 口径函数集
        if self._funcs is None:
            vp = QOpenGLVersionProfile()
            vp.setVersion(2, 1)
            self._funcs = QOpenGLVersionFunctionsFactory.get(vp, self.context())
            self._funcs.initializeOpenGLFunctions()
        return self._funcs

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
        self._prog.bind()
        self._prog.setUniformValue("u_res", float(w_px), float(h_px))
        surfaces = self._scene.surfaces()[:_MAX_SURFACES]
        self._prog.setUniformValue("u_count", len(surfaces))
        for i, s in enumerate(surfaces):
            rect = s.rect
            dpr_rect = QRectF(rect.x() * dpr, rect.y() * dpr,
                              rect.width() * dpr, rect.height() * dpr)
            self._prog.setUniformValue(f"u_rect[{i}]", float(dpr_rect.x()),
                                       float(dpr_rect.y()),
                                       float(dpr_rect.width()),
                                       float(dpr_rect.height()))
            self._prog.setUniformValue(f"u_radius[{i}]", float(s.radius) * dpr)
            self._prog.setUniformValue(f"u_refr[{i}]", float(s.refraction))
            tint = s.tint
            self._prog.setUniformValue(f"u_tint[{i}]",
                                       float(tint[0]), float(tint[1]), float(tint[2]))
        if self._dark:
            self._prog.setUniformValue("u_top", 0.051, 0.075, 0.133)
            self._prog.setUniformValue("u_bottom", 0.035, 0.051, 0.086)
            self._prog.setUniformValue("u_glow_a", 0.89, 0.70, 0.42)
            self._prog.setUniformValue("u_glow_b", 0.20, 0.32, 0.48)
        else:
            self._prog.setUniformValue("u_top", 0.906, 0.933, 0.969)
            self._prog.setUniformValue("u_bottom", 0.949, 0.925, 0.863)
            self._prog.setUniformValue("u_glow_a", 0.79, 0.60, 0.31)
            self._prog.setUniformValue("u_glow_b", 0.25, 0.42, 0.66)
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
# GlassCanvas(QOpenGLWidget): 整窗 GL 画布
#   __init__(scene, dark, parent): 持场景与主题；预建着色器程序/VBO 对象
#     （无 GL 上下文，不可编译上传）；_ready 初始 False
#   set_dark(dark): 主题切换——更新配色标记并 update() 触发重绘（PL009 联动入口）
#   _gl(): 懒取并缓存 2.1 版本函数对象；PyQt6 的 QOpenGLContext 已移除 functions()，
#     必须 经 QOpenGLVersionFunctionsFactory.get(profile, context)（PL008.10 定案）；
#     失败抛异常，由调用方回调防护捕获
#   initializeGL(): Qt 回调（上下文首帧就绪）——编译链接顶点/片段着色器 +
#     bindAttributeLocation("a_pos", 0) + 全屏四边形 VBO 上传；try/except 全包，
#     失败置 _ready=False 并记录
#   paintGL(): Qt 回调——try/except 全包调 _paint_glass；异常 → exception 级日志 +
#     _clear_solid 降级纯色（帧缓冲未定义内容必须覆盖）
#   _paint_glass(): 设 uniforms（u_res/u_count/u_rect/u_radius/u_refr/u_tint/
#     u_top/u_bottom/u_glow_a/u_glow_b）→ VBO 绑定 + setAttributeBuffer(location 0,
#     GL_FLOAT, 偏移 0, 2 分量, 步长 0) → glDrawArrays(TRIANGLE_STRIP, 4 顶点)
#     单 pass；_ready=False 时纯色清屏直接返回（静默降级不刷日志）
#   _clear_solid(): 降级清屏实现；自身再失败仅 debug 记录（降级路径禁止二次抛出）
#   异常处理：paintGL/initializeGL 两个 Qt 回调 try/except 全包——PyQt6 回调内
#     未捕获 Python 异常触发 fail-fast 终止进程（0xC0000409 零输出，即 GL 探针
#     127 硬崩根因；AGENTS.md「Qt 回调防护约定」）；非回调函数不吞异常正常上抛
#   设计理由：静态帧缓存由调用方 update() 驱动；程序化光场单 pass 避免 FBO 翻转
#     复杂度；VBO + setAttributeBuffer 一次性上传顶点数据（替代逐帧 Python 序列
#     组包，且规避 PyQt6 三参数 setAttributeArray 重载差异）
#   关联配置：ui.json gl_enabled × capability.gl_available 决定画布是否被装配；
#     着色器源码 ui/gl/shaders.py；场景注册表 ui/gl/glass_scene.py（SCENE 单例）
