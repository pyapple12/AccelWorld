# 主题样式模块
# 集中管理浅色/深色主题 QSS 与进度条动画样式
# S9.5：颜色经占位符 @{key}@ 从颜色表生成（零硬编码，保持 QSS 可读）
# PL001（plan#UI2.0）：改参数化工厂——颜色表由主窗口经接口传入，本模块不再 import config
# （ui/ 后端 import 清零）；PL002 Fluent 重写时本管线整体退役

from typing import Any


def _apply_colors(qss: str, colors: dict[str, Any]) -> str:
    # 用 @{key}@ 占位符替换颜色值（str.replace 不解析 QSS 花括号，安全）
    result = qss
    for key, value in colors.items():
        result = result.replace(f"@{{{key}}}@", value)
    return result


# 浅色主题样式模板（占位符 @{key}@）
_LIGHT_THEME_TPL = """
QMainWindow, QWidget {
    background-color: @{bg_window_light}@;
    color: @{text_main}@;
}

QFrame {
    background-color: @{bg_frame_light}@;
    border: 1px solid @{bg_border_light}@;
    border-radius: 8px;
}

QLabel {
    color: @{text_main}@;
}

QComboBox {
    background-color: @{bg_control_light}@;
    border: 1px solid @{bg_border_light}@;
    border-radius: 4px;
    padding: 4px 8px;
    color: @{text_main}@;
}

QComboBox::drop-down {
    border: none;
}

QLineEdit {
    background-color: @{bg_control_light}@;
    border: 1px solid @{bg_border_light}@;
    border-radius: 4px;
    padding: 4px 8px;
    color: @{text_main}@;
}

QSlider::groove:horizontal {
    background-color: @{bg_border_light}@;
    height: 8px;
    border-radius: 4px;
}

QSlider::handle:horizontal {
    background-color: @{primary}@;
    width: 20px;
    margin: -6px 0;
    border-radius: 10px;
}

QSlider::sub-page:horizontal {
    background-color: @{primary_light}@;
    border-radius: 4px;
}

QProgressBar {
    border: 1px solid @{bg_border_light}@;
    border-radius: 4px;
    text-align: center;
    background-color: @{bg_control_light}@;
    color: @{text_main}@;
}

QProgressBar::chunk {
    background-color: @{primary}@;
    border-radius: 2px;
}

QPushButton {
    background-color: @{primary}@;
    color: @{text_on_primary}@;
    border: none;
    border-radius: 6px;
    padding: 6px 12px;
}

QPushButton:hover {
    background-color: @{primary_hover}@;
}

QPushButton:pressed {
    background-color: @{primary_pressed}@;
}
"""

# 深色主题样式模板
_DARK_THEME_TPL = """
QMainWindow, QWidget {
    background-color: @{bg_window_dark}@;
    color: @{text_light}@;
}

QFrame {
    background-color: @{bg_frame_dark}@;
    border: 1px solid @{bg_border_dark}@;
    border-radius: 8px;
}

QLabel {
    color: @{text_light}@;
}

QComboBox {
    background-color: @{bg_control_dark}@;
    border: 1px solid @{bg_border_dark}@;
    border-radius: 4px;
    padding: 4px 8px;
    color: @{text_light}@;
}

QComboBox::drop-down {
    border: none;
}

QComboBox QAbstractItemView {
    background-color: @{bg_control_dark}@;
    color: @{text_light}@;
    selection-background-color: @{primary}@;
}

QLineEdit {
    background-color: @{bg_control_dark}@;
    border: 1px solid @{bg_border_dark}@;
    border-radius: 4px;
    padding: 4px 8px;
    color: @{text_light}@;
}

QSlider::groove:horizontal {
    background-color: @{bg_border_dark}@;
    height: 8px;
    border-radius: 4px;
}

QSlider::handle:horizontal {
    background-color: @{primary}@;
    width: 20px;
    margin: -6px 0;
    border-radius: 10px;
}

QSlider::sub-page:horizontal {
    background-color: @{primary_light}@;
    border-radius: 4px;
}

QProgressBar {
    border: 1px solid @{bg_border_dark}@;
    border-radius: 4px;
    text-align: center;
    background-color: @{bg_control_dark}@;
    color: @{text_light}@;
}

QProgressBar::chunk {
    background-color: @{primary}@;
    border-radius: 2px;
}

QPushButton {
    background-color: @{primary}@;
    color: @{text_on_primary}@;
    border: none;
    border-radius: 6px;
    padding: 6px 12px;
}

QPushButton:hover {
    background-color: @{primary_dark_hover}@;
}

QPushButton:pressed {
    background-color: @{primary_pressed}@;
}

QPushButton:disabled {
    background-color: @{disabled}@;
}
"""

# 浅色进度条动画样式模板
_LIGHT_THEME_PROGRESS_TPL = """
QProgressBar {
    border: 1px solid @{bg_border_light}@;
    border-radius: 4px;
    text-align: center;
    background-color: @{bg_control_light}@;
    color: @{text_main}@;
}

QProgressBar::chunk {
    background-color: qlineargradient(
        x1:0, y1:0, x2:1, y2:0,
        stop:0 @{primary}@,
        stop:0.5 @{primary_light}@,
        stop:1 @{primary}@
    );
    border-radius: 2px;
}
"""

# 深色进度条动画样式模板
_DARK_THEME_PROGRESS_TPL = """
QProgressBar {
    border: 1px solid @{bg_border_dark}@;
    border-radius: 4px;
    text-align: center;
    background-color: @{bg_control_dark}@;
    color: @{text_light}@;
}

QProgressBar::chunk {
    background-color: qlineargradient(
        x1:0, y1:0, x2:1, y2:0,
        stop:0 @{primary}@,
        stop:0.5 @{primary_light}@,
        stop:1 @{primary}@
    );
    border-radius: 2px;
}
"""

# 生成主题 QSS 的参数化工厂（颜色单源：ui.json colors，经接口传入）
def build_theme(colors: dict[str, Any], dark: bool) -> tuple[str, str]:
    # 按深浅构建 (窗口 QSS, 进度条 QSS)；颜色表由调用方经 AppInterface.get_ui_static() 传入
    if dark:
        return (
            _apply_colors(_DARK_THEME_TPL, colors),
            _apply_colors(_DARK_THEME_PROGRESS_TPL, colors),
        )
    return (
        _apply_colors(_LIGHT_THEME_TPL, colors),
        _apply_colors(_LIGHT_THEME_PROGRESS_TPL, colors),
    )


# ===== ui/themes.py 函数/常量说明 =====
# _apply_colors(qss, colors) -> str: 用 @{key}@ 占位符替换颜色值
#   设计理由：str.replace 不解析 QSS 的花括号语法（.format/f-string 会冲突），
#   且模板保持 QSS 可读性（S9.5 决策点 2 方式 A 改良版）
# LIGHT/DARK 窗口与进度条模板（_LIGHT_THEME_TPL/_DARK_THEME_TPL/
#   _LIGHT_THEME_PROGRESS_TPL/_DARK_THEME_PROGRESS_TPL）: 占位符模板常量
# build_theme(colors, dark) -> tuple[str, str]: 主题构建工厂
#   输入：颜色表（AppInterface.get_ui_static()["colors"]）与深浅开关
#   输出：(窗口 QSS, 进度条 QSS)，由主窗口在构造期构建并按主题取用
#   设计理由：PL001 参数化后本模块不 import config（ui/ 后端 import 清零验收）；
#   PL002 QSS 管线退役时仅删本模块与调用点
#   关联配置：颜色键名与值来自 config/static/ui.json
