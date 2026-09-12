# 玻璃面注册表（glass_scene）：GL 模式下各 GlassCard 把几何注册进来，
# 画布按列表统一绘制玻璃光学；z 序按 elevation 与注册顺序（FIX/PL008.04）

from dataclasses import dataclass, field

from PyQt6.QtCore import QRectF


@dataclass
class GlassSurface:
    # 单块玻璃面的描述：矩形 + 端帽半径 + tint 色与折射强度（PL008 场景模型）
    surface_id: str
    rect: QRectF
    radius: float = 12.0
    tint: tuple[float, float, float] = (0.16, 0.14, 0.20)
    refraction: float = 0.5
    elevation: int = 0


class GlassScene:
    # 玻璃面注册表：register/unregister/update_geometry 三操作 + dirty 标记
    # （dirty 驱动画布静态帧缓存失效；几何以画布坐标系为准）
    def __init__(self) -> None:
        self._surfaces: dict[str, GlassSurface] = {}
        self._dirty = True

    def register(self, surface: GlassSurface) -> None:
        # 注册或整体替换（同 id 幂等）
        self._surfaces[surface.surface_id] = surface
        self._dirty = True

    def unregister(self, surface_id: str) -> None:
        # 注销（控件销毁时调用），无 id 时静默
        if self._surfaces.pop(surface_id, None) is not None:
            self._dirty = True

    def update_geometry(self, surface_id: str, rect: QRectF) -> None:
        # 仅更新几何（resize/move 场景），保持其他属性
        if surface_id in self._surfaces:
            self._surfaces[surface_id].rect = QRectF(rect)
            self._dirty = True

    def surfaces(self) -> list[GlassSurface]:
        # 按 elevation 升序（低在前高在后），供画布顺序绘制
        return sorted(self._surfaces.values(), key=lambda s: s.elevation)

    @property
    def dirty(self) -> bool:
        return self._dirty

    def clear_dirty(self) -> None:
        self._dirty = False


# 进程级单例：GL 模式下各 GlassCard 向其注册几何（画布读取绘制）
SCENE = GlassScene()
