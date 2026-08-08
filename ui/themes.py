# 主题样式模块
# 集中管理浅色/深色主题 QSS 与进度条动画样式
# S9.5：颜色经占位符 @{key}@ 从 config/static/ui.json 生成（零硬编码，保持 QSS 可读）

from config.static.static_config import get_static_config

# 颜色表（来自静态配置）
_COLORS = get_static_config().ui["colors"]


def _apply_colors(qss: str) -> str:
    # 用 @{key}@ 占位符替换颜色值（str.replace 不解析 QSS 花括号，安全）
    result = qss
    for key, value in _COLORS.items():
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
    color: white;
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
    color: white;
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

# 生成最终 QSS（颜色单源：config/static/ui.json）
LIGHT_THEME = _apply_colors(_LIGHT_THEME_TPL)
DARK_THEME = _apply_colors(_DARK_THEME_TPL)
LIGHT_THEME_PROGRESS = _apply_colors(_LIGHT_THEME_PROGRESS_TPL)
DARK_THEME_PROGRESS = _apply_colors(_DARK_THEME_PROGRESS_TPL)


# ===== ui/themes.py 函数/常量说明 =====
# _COLORS: dict，颜色表（来自 config/static/ui.json colors）
# _apply_colors(qss) -> str: 用 @{key}@ 占位符替换颜色值
#   设计理由：str.replace 不解析 QSS 的花括号语法（.format/f-string 会冲突），
#   且模板保持 QSS 可读性（S9.5 决策点 2 方式 A 改良版）
# LIGHT_THEME/DARK_THEME/LIGHT_THEME_PROGRESS/DARK_THEME_PROGRESS:
#   由模板经 _apply_colors 生成的最终 QSS 字符串
#   设计理由：颜色单源（ui.json），改主题色只需改 json
#   关联配置：颜色来自 config/static/ui.json；由 ui/main_window.py apply_theme 使用
