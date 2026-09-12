# ui/gl：UI3.0 GLSL 液态玻璃渲染包（PL008）
# 结构：glass_canvas（整窗 GL 画布）/ glass_scene（玻璃面注册表）/ capability（能力探测与开关）
# 铁律：GL 画布只画"玻璃光学"，文字与交互仍由 Qt 控件承担；栅格路径保留为降级
