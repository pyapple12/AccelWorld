# 主题样式模块（迁移自 accelworld_gui.py，S1 结构骨架阶段）
# 集中管理浅色/深色主题 QSS 与进度条动画样式

# 浅色主题样式
LIGHT_THEME = """
QMainWindow, QWidget {
    background-color: #f5f5f5;
    color: #333333;
}

QFrame {
    background-color: #ffffff;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
}

QLabel {
    color: #333333;
}

QComboBox {
    background-color: #ffffff;
    border: 1px solid #cccccc;
    border-radius: 4px;
    padding: 4px 8px;
    color: #333333;
}

QComboBox::drop-down {
    border: none;
}

QLineEdit {
    background-color: #ffffff;
    border: 1px solid #cccccc;
    border-radius: 4px;
    padding: 4px 8px;
    color: #333333;
}

QSlider::groove:horizontal {
    background-color: #e0e0e0;
    height: 8px;
    border-radius: 4px;
}

QSlider::handle:horizontal {
    background-color: #4CAF50;
    width: 20px;
    margin: -6px 0;
    border-radius: 10px;
}

QSlider::sub-page:horizontal {
    background-color: #81C784;
    border-radius: 4px;
}

QProgressBar {
    border: 1px solid #cccccc;
    border-radius: 4px;
    text-align: center;
    background-color: #ffffff;
    color: #333333;
}

QProgressBar::chunk {
    background-color: #4CAF50;
    border-radius: 2px;
}

QPushButton {
    background-color: #4CAF50;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 6px 12px;
}

QPushButton:hover {
    background-color: #45a049;
}

QPushButton:pressed {
    background-color: #3d8b40;
}
"""

# 深色主题样式
DARK_THEME = """
QMainWindow, QWidget {
    background-color: #1e1e1e;
    color: #e0e0e0;
}

QFrame {
    background-color: #2d2d2d;
    border: 1px solid #3d3d3d;
    border-radius: 8px;
}

QLabel {
    color: #e0e0e0;
}

QComboBox {
    background-color: #3d3d3d;
    border: 1px solid #4d4d4d;
    border-radius: 4px;
    padding: 4px 8px;
    color: #e0e0e0;
}

QComboBox::drop-down {
    border: none;
}

QComboBox QAbstractItemView {
    background-color: #3d3d3d;
    color: #e0e0e0;
    selection-background-color: #4CAF50;
}

QLineEdit {
    background-color: #3d3d3d;
    border: 1px solid #4d4d4d;
    border-radius: 4px;
    padding: 4px 8px;
    color: #e0e0e0;
}

QSlider::groove:horizontal {
    background-color: #4d4d4d;
    height: 8px;
    border-radius: 4px;
}

QSlider::handle:horizontal {
    background-color: #4CAF50;
    width: 20px;
    margin: -6px 0;
    border-radius: 10px;
}

QSlider::sub-page:horizontal {
    background-color: #81C784;
    border-radius: 4px;
}

QProgressBar {
    border: 1px solid #4d4d4d;
    border-radius: 4px;
    text-align: center;
    background-color: #3d3d3d;
    color: #e0e0e0;
}

QProgressBar::chunk {
    background-color: #4CAF50;
    border-radius: 2px;
}

QPushButton {
    background-color: #4CAF50;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 6px 12px;
}

QPushButton:hover {
    background-color: #5CBF60;
}

QPushButton:pressed {
    background-color: #3d8b40;
}

QPushButton:disabled {
    background-color: #888888;
}
"""

# 浅色进度条动画样式
LIGHT_THEME_PROGRESS = """
QProgressBar {
    border: 1px solid #cccccc;
    border-radius: 4px;
    text-align: center;
    background-color: #ffffff;
    color: #333333;
}

QProgressBar::chunk {
    background-color: qlineargradient(
        x1:0, y1:0, x2:1, y2:0,
        stop:0 #4CAF50,
        stop:0.5 #81C784,
        stop:1 #4CAF50
    );
    border-radius: 2px;
}
"""

# 深色进度条动画样式
DARK_THEME_PROGRESS = """
QProgressBar {
    border: 1px solid #4d4d4d;
    border-radius: 4px;
    text-align: center;
    background-color: #3d3d3d;
    color: #e0e0e0;
}

QProgressBar::chunk {
    background-color: qlineargradient(
        x1:0, y1:0, x2:1, y2:0,
        stop:0 #4CAF50,
        stop:0.5 #81C784,
        stop:1 #4CAF50
    );
    border-radius: 2px;
}
"""

# ===== ui/themes.py 函数/常量说明 =====
# LIGHT_THEME: str，浅色主题 QSS 样式表
# DARK_THEME: str，深色主题 QSS 样式表
# LIGHT_THEME_PROGRESS: str，浅色主题进度条动画样式
# DARK_THEME_PROGRESS: str，深色主题进度条动画样式
#   输入：无（纯常量）；输出：QSS 字符串
#   设计理由：主题样式从主窗口剥离，主窗口与未来 themes 管理只需引用常量
#   关联配置：由 ui/main_window.py apply_theme 使用
