from typing import List, Dict, Any
import logging

# 配置日志
logger = logging.getLogger(__name__)

from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QSlider, QLineEdit, QPushButton, QFrame, QMessageBox,
    QGridLayout, QProgressBar, QComboBox, QButtonGroup, QRadioButton,
    QSystemTrayIcon, QMenu, QListWidget, QListWidgetItem, QCheckBox,
    QDialog, QDialogButtonBox, QFormLayout, QTimeEdit, QFileDialog,
    QGroupBox, QDateEdit, QCalendarWidget
)
from PyQt6.QtCore import Qt, QTimer, QSize, QTime
from PyQt6.QtGui import QFont, QDoubleValidator, QIcon

# 程序版本号
VERSION = "ver 0.43"

# ------------------- 主题样式定义 -------------------
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

# 进度条动画样式
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


class AcceleratedWorldGUI(QMainWindow):
    """加速世界图形界面类 - 使用PyQt6实现的可视化时钟应用"""

    def __init__(self):
        """初始化图形界面"""
        super().__init__()

        # 延迟导入，避免循环导入问题
        from accelworld_calc import AcceleratedWorld
        from accelworld_config import load_config, get_setting

        # 加载配置
        config = load_config()
        saved_rate = get_setting("time_dilation_rate", 2.0)

        self.setWindowTitle(f"加速世界 - 时间膨胀时钟 {VERSION}")

        # 恢复窗口位置和大小
        from accelworld_config import load_window_geometry
        geometry = load_window_geometry()
        if geometry:
            self.restoreGeometry(geometry)
        else:
            self.setGeometry(100, 100, 900, 500)

        # 创建加速世界核心实例
        self.accel_world = AcceleratedWorld(time_dilation_rate=saved_rate)

        # 设置中心部件和主布局
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        self.main_layout = QVBoxLayout(central_widget)
        self.main_layout.setContentsMargins(20, 20, 20, 20)
        self.main_layout.setSpacing(10)

        # 初始化UI组件
        self.setup_ui()

        # 启动实时时钟更新（100毫秒刷新一次）
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_clock)
        self.timer.start(100)

        # 启动天气更新定时器（每30分钟更新一次）
        self.weather_timer = QTimer(self)
        self.weather_timer.timeout.connect(self.update_weather)
        self.weather_timer.start(30 * 60 * 1000)  # 30分钟

        # 初始化天气数据
        self.current_city = "北京"
        self.update_weather()

        # 初始化主题（默认浅色）
        self.is_dark_theme = False

        # 闹钟管理器占位符（在 init_alarms 中初始化）
        self.alarm_manager = None
        self.apply_theme()

        # 初始化系统托盘
        self.setup_system_tray()

    def setup_ui(self) -> None:
        """设置UI界面布局和组件"""

        # ------------------- 时钟显示区域 -------------------
        clock_frame = QFrame()
        clock_frame.setFrameShape(QFrame.Shape.StyledPanel)
        clock_layout = QVBoxLayout(clock_frame)

        # 时间标签行
        time_label_layout = QHBoxLayout()

        # 标准时间标签
        self.standard_time_label = QLabel("标准时间: 00:00:00")
        self.standard_time_label.setFont(QFont("Arial", 16))
        time_label_layout.addWidget(self.standard_time_label)

        # 加速时间标签
        self.accelerated_time_label = QLabel("加速时间: 00:00:00")
        self.accelerated_time_label.setFont(QFont("Arial", 16))
        time_label_layout.addWidget(self.accelerated_time_label)

        clock_layout.addLayout(time_label_layout)

        # 加速时间进度条
        self.progress_bar = QProgressBar()
        self.progress_bar.setFont(QFont("Arial", 10))
        self.progress_bar.setFixedHeight(25)
        self.progress_bar.setFormat("%v / %m 小时")
        self.progress_bar.setStyleSheet(LIGHT_THEME_PROGRESS)
        clock_layout.addWidget(self.progress_bar)

        self.main_layout.addWidget(clock_frame)

        # ------------------- 加速参数显示区域 -------------------
        params_frame = QFrame()
        params_frame.setFrameShape(QFrame.Shape.StyledPanel)
        params_layout = QGridLayout(params_frame)

        # 加速后一天小时数
        hours_per_day_label = QLabel("加速后一天小时数:")
        hours_per_day_label.setFont(QFont("Arial", 12))
        params_layout.addWidget(hours_per_day_label, 0, 0)
        self.hours_per_day_value_label = QLabel("48.00小时")
        self.hours_per_day_value_label.setFont(QFont("Arial", 12, QFont.Weight.Bold))
        params_layout.addWidget(self.hours_per_day_value_label, 0, 1)

        # 加速倍率
        rate_label = QLabel("加速倍率:")
        rate_label.setFont(QFont("Arial", 12))
        params_layout.addWidget(rate_label, 0, 2)
        self.rate_value_label = QLabel("200%")
        self.rate_value_label.setFont(QFont("Arial", 12, QFont.Weight.Bold))
        params_layout.addWidget(self.rate_value_label, 0, 3)

        # 加速后剩余小时数
        remaining_label = QLabel("加速后剩余小时数:")
        remaining_label.setFont(QFont("Arial", 12))
        params_layout.addWidget(remaining_label, 0, 4)
        self.remaining_hours_value_label = QLabel("45.00小时")
        self.remaining_hours_value_label.setFont(QFont("Arial", 12, QFont.Weight.Bold))
        params_layout.addWidget(self.remaining_hours_value_label, 0, 5)

        self.main_layout.addWidget(params_frame)

        # ------------------- 日期显示区域 -------------------
        date_frame = QFrame()
        date_frame.setFrameShape(QFrame.Shape.StyledPanel)
        date_layout = QVBoxLayout(date_frame)

        # 中文日期标签
        self.date_label = QLabel("2025年12月25日 星期四")
        self.date_label.setFont(QFont("Arial", 14))
        self.date_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.date_label)

        # 农历信息标签
        self.lunar_info_label = QLabel("农历信息...")
        self.lunar_info_label.setFont(QFont("Arial", 12))
        self.lunar_info_label.setStyleSheet("color: #555555")
        self.lunar_info_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        date_layout.addWidget(self.lunar_info_label)

        self.main_layout.addWidget(date_frame)

        # ------------------- 倒计时区域 -------------------
        countdown_frame = QFrame()
        countdown_frame.setFrameShape(QFrame.Shape.StyledPanel)
        countdown_layout = QHBoxLayout(countdown_frame)

        # 倒计时标签
        countdown_title_label = QLabel("倒计时:")
        countdown_title_label.setFont(QFont("Arial", 12))
        countdown_layout.addWidget(countdown_title_label)

        # 目标时间输入框和日期选择器（水平排列）
        countdown_input_layout = QHBoxLayout()
        countdown_input_layout.setSpacing(5)

        # 目标时间输入
        self.countdown_target = QLineEdit()
        self.countdown_target.setPlaceholderText("YYYY-MM-DD HH:MM:SS")
        self.countdown_target.setFont(QFont("Arial", 9))
        self.countdown_target.setFixedWidth(200)
        countdown_input_layout.addWidget(self.countdown_target)

        # 日期选择器按钮
        self.date_picker_button = QPushButton("📅")
        self.date_picker_button.setFixedSize(32, 32)
        self.date_picker_button.setToolTip("选择日期")
        self.date_picker_button.setFont(QFont("Arial", 12))
        self.date_picker_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.date_picker_button.clicked.connect(self.show_date_picker)
        countdown_input_layout.addWidget(self.date_picker_button)

        # 时间选择器按钮
        self.time_picker_button = QPushButton("🕐")
        self.time_picker_button.setFixedSize(32, 32)
        self.time_picker_button.setToolTip("选择时间")
        self.time_picker_button.setFont(QFont("Arial", 12))
        self.time_picker_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.time_picker_button.clicked.connect(self.show_time_picker)
        countdown_input_layout.addWidget(self.time_picker_button)

        countdown_layout.addLayout(countdown_input_layout)

        # 倒计时显示
        self.countdown_label = QLabel("--天 --:--:--:--")
        self.countdown_label.setFont(QFont("Arial", 14, QFont.Weight.Bold))
        self.countdown_label.setStyleSheet("color: #4CAF50;")
        countdown_layout.addWidget(self.countdown_label)

        # 格式提示
        countdown_hint_label = QLabel("YYYY-MM-DD HH:MM:SS")
        countdown_hint_label.setFont(QFont("Arial", 9))
        countdown_hint_label.setStyleSheet("color: #888888")
        countdown_layout.addWidget(countdown_hint_label)

        countdown_layout.addStretch()

        # 设置倒计时按钮
        self.set_countdown_button = QPushButton("设置")
        self.set_countdown_button.setFont(QFont("Arial", 10))
        self.set_countdown_button.clicked.connect(self.set_countdown)
        countdown_layout.addWidget(self.set_countdown_button)

        # 清除倒计时按钮
        self.clear_countdown_button = QPushButton("清除")
        self.clear_countdown_button.setFont(QFont("Arial", 10))
        self.clear_countdown_button.clicked.connect(self.clear_countdown)
        countdown_layout.addWidget(self.clear_countdown_button)

        self.main_layout.addWidget(countdown_frame)
        self.countdown_target_date = None  # 倒计时目标时间
        self.countdown_time = QTimeEdit()
        self.countdown_time.setDisplayFormat("HH:mm:ss")
        self.countdown_time.setFixedWidth(80)

        # ------------------- 世界时钟区域 -------------------
        world_clock_frame = QFrame()
        world_clock_frame.setFrameShape(QFrame.Shape.StyledPanel)
        world_clock_layout = QHBoxLayout(world_clock_frame)

        # 世界时钟标题
        world_clock_title = QLabel("世界时钟:")
        world_clock_title.setFont(QFont("Arial", 12))
        world_clock_layout.addWidget(world_clock_title)

        # 时区选择
        self.timezone_combo = QComboBox()
        self.timezone_combo.setFont(QFont("Arial", 11))
        self.timezone_combo.setFixedWidth(150)
        # 添加常用时区
        timezones = [
            ("北京 (UTC+8)", "Asia/Shanghai"),
            ("东京 (UTC+9)", "Asia/Tokyo"),
            ("首尔 (UTC+9)", "Asia/Seoul"),
            ("伦敦 (UTC+0)", "Europe/London"),
            ("巴黎 (UTC+1)", "Europe/Paris"),
            ("纽约 (UTC-5)", "America/New_York"),
            ("洛杉矶 (UTC-8)", "America/Los_Angeles"),
            ("悉尼 (UTC+11)", "Australia/Sydney"),
        ]
        for name, tz in timezones:
            self.timezone_combo.addItem(name, tz)
        self.timezone_combo.currentIndexChanged.connect(self.update_world_clock)
        world_clock_layout.addWidget(self.timezone_combo)

        # 世界时钟显示
        self.world_clock_label = QLabel("00:00:00")
        self.world_clock_label.setFont(QFont("Arial", 14, QFont.Weight.Bold))
        self.world_clock_label.setStyleSheet("color: #2196F3;")
        world_clock_layout.addWidget(self.world_clock_label)

        world_clock_layout.addStretch()

        self.main_layout.addWidget(world_clock_frame)

        # ------------------- 天气显示区域 -------------------
        weather_frame = QFrame()
        weather_frame.setFrameShape(QFrame.Shape.StyledPanel)
        weather_layout = QHBoxLayout(weather_frame)

        # 城市选择
        city_label = QLabel("城市:")
        city_label.setFont(QFont("Arial", 12))
        weather_layout.addWidget(city_label)

        self.city_combo = QComboBox()
        self.city_combo.setFont(QFont("Arial", 12))
        self.city_combo.setFixedWidth(120)
        # 添加城市列表
        from accelworld_weather import CITIES
        self.city_combo.addItems(sorted(CITIES.keys()))
        self.city_combo.setCurrentText("北京")
        self.city_combo.currentTextChanged.connect(self.on_city_changed)
        weather_layout.addWidget(self.city_combo)

        # 天气图标
        self.weather_icon_label = QLabel("☀️")
        self.weather_icon_label.setFont(QFont("Arial", 24))
        weather_layout.addWidget(self.weather_icon_label)

        # 天气信息
        self.weather_info_label = QLabel("获取天气中...")
        self.weather_info_label.setFont(QFont("Arial", 12))
        weather_layout.addWidget(self.weather_info_label)

        weather_layout.addStretch()

        # 主题切换
        self.theme_button = QPushButton("🌙")
        self.theme_button.setFont(QFont("Arial", 12))
        self.theme_button.setFixedSize(36, 36)
        self.theme_button.setToolTip("切换主题")
        self.theme_button.setStyleSheet("padding: 0px; margin: 0px;")
        self.theme_button.clicked.connect(self.toggle_theme)
        weather_layout.addWidget(self.theme_button)

        # 刷新天气按钮
        self.refresh_weather_button = QPushButton("刷新")
        self.refresh_weather_button.setFont(QFont("Arial", 10))
        self.refresh_weather_button.clicked.connect(self.update_weather)
        weather_layout.addWidget(self.refresh_weather_button)

        self.main_layout.addWidget(weather_frame)

        # ------------------- 闹钟区域 -------------------
        alarm_frame = QFrame()
        alarm_frame.setFrameShape(QFrame.Shape.StyledPanel)
        alarm_layout = QVBoxLayout(alarm_frame)

        # 闹钟标题行
        alarm_title_layout = QHBoxLayout()

        alarm_title = QLabel("闹钟:")
        alarm_title.setFont(QFont("Arial", 12))
        alarm_title_layout.addWidget(alarm_title)

        alarm_title_layout.addStretch()

        # 添加闹钟按钮
        self.add_alarm_button = QPushButton("+ 添加闹钟")
        self.add_alarm_button.setFont(QFont("Arial", 10))
        self.add_alarm_button.clicked.connect(self.show_add_alarm_dialog)
        alarm_title_layout.addWidget(self.add_alarm_button)

        alarm_layout.addLayout(alarm_title_layout)

        # 闹钟列表
        self.alarm_list = QListWidget()
        self.alarm_list.setFont(QFont("Arial", 11))
        self.alarm_list.setFixedHeight(120)
        alarm_layout.addWidget(self.alarm_list)

        self.main_layout.addWidget(alarm_frame)

        # 初始化闹钟管理器
        self.init_alarms()

        # ------------------- 用户交互区域 -------------------
        input_frame = QFrame()
        input_frame.setFrameShape(QFrame.Shape.StyledPanel)
        input_layout = QGridLayout(input_frame)

        # 加速倍率标签
        rate_input_label = QLabel("加速倍率:")
        rate_input_label.setFont(QFont("Arial", 12))
        input_layout.addWidget(rate_input_label, 0, 0, Qt.AlignmentFlag.AlignRight)

        # 加速倍率输入框
        self.rate_entry = QLineEdit()
        self.rate_entry.setText("2.0")
        self.rate_entry.setFont(QFont("Arial", 12))
        self.rate_entry.setFixedWidth(80)
        self.rate_entry.setValidator(QDoubleValidator(1.0, 20.0, 2))
        input_layout.addWidget(self.rate_entry, 0, 1, Qt.AlignmentFlag.AlignLeft)

        # 加速倍率说明
        rate_hint_label = QLabel("（必须大于1.0，默认值2.0，最大值20.0）")
        rate_hint_label.setFont(QFont("Arial", 10))
        input_layout.addWidget(rate_hint_label, 0, 2, Qt.AlignmentFlag.AlignLeft)

        # 加速倍率滑杆
        self.slider = QSlider(Qt.Orientation.Horizontal)
        self.slider.setMinimum(10)  # 1.0 * 10
        self.slider.setMaximum(200)  # 20.0 * 10
        initial_value = int(self.accel_world.time_dilation_rate * 10)
        self.slider.setValue(initial_value)
        self.slider.setFixedHeight(30)
        self.slider.valueChanged.connect(self.on_slider_change)
        input_layout.addWidget(self.slider, 1, 0, 1, 3)

        # 滑杆值显示
        self.slider_value_label = QLabel(f"{self.accel_world.time_dilation_rate:.1f}x")
        self.slider_value_label.setFont(QFont("Arial", 12))
        input_layout.addWidget(self.slider_value_label, 2, 1, Qt.AlignmentFlag.AlignLeft)

        # 确认按钮
        self.confirm_button = QPushButton("应用加速")
        self.confirm_button.setFont(QFont("Arial", 12, QFont.Weight.Bold))
        self.confirm_button.setFixedSize(120, 50)
        self.confirm_button.clicked.connect(self.apply_acceleration)
        input_layout.addWidget(self.confirm_button, 0, 3, 2, 1, Qt.AlignmentFlag.AlignVCenter | Qt.AlignmentFlag.AlignRight)

        self.main_layout.addWidget(input_frame)

    def on_slider_change(self, value: int) -> None:
        """滑杆值变化时的回调函数 - 实时更新加速倍率"""
        slider_value = value / 10.0
        self.slider_value_label.setText(f"{slider_value:.1f}x")
        # 同步更新输入框值
        self.rate_entry.setText(f"{slider_value:.2f}")

        # 实时更新加速倍率，无需点击应用按钮
        self._update_acceleration_rate(slider_value)

    def _update_acceleration_rate(self, rate: float) -> None:
        """更新加速倍率（内部方法）"""
        from accelworld_calc import AcceleratedWorld
        # 验证倍率是否在有效范围内
        if not (1.0 <= rate <= 20.0):
            return
        # 更新加速世界实例
        self.accel_world = AcceleratedWorld(time_dilation_rate=rate)

    def apply_acceleration(self) -> None:
        """应用加速倍率 - 仅响应文字输入框"""
        rate_text = self.rate_entry.text().strip()
        if not rate_text:
            # 如果输入框为空，使用滑杆当前值
            rate_text = str(self.slider.value() / 10.0)

        try:
            rate = float(rate_text)
            # 精确到小数点后2位
            rate = round(rate, 2)

            # 验证倍率是否在有效范围内
            if not (1.0 <= rate <= 20.0):
                raise ValueError("加速倍率必须在1.0到20.0之间")

            # 更新加速世界实例
            self._update_acceleration_rate(rate)

            # 同步滑杆的值（清空输入框）
            self.slider.setValue(int(rate * 10))
            self.slider_value_label.setText(f"{rate:.1f}x")
            self.rate_entry.setText("")

        except ValueError as e:
            QMessageBox.critical(self, "错误", str(e))

    def set_countdown(self) -> None:
        """设置倒计时目标时间"""
        import datetime

        target_text = self.countdown_target.text().strip()
        if not target_text:
            QMessageBox.warning(self, "警告", "请输入目标时间")
            return

        try:
            # 尝试解析时间格式
            if len(target_text) == 19:  # YYYY-MM-DD HH:MM:SS
                self.countdown_target_date = datetime.datetime.strptime(target_text, "%Y-%m-%d %H:%M:%S")
            elif len(target_text) == 16:  # YYYY-MM-DD HH:MM
                self.countdown_target_date = datetime.datetime.strptime(target_text, "%Y-%m-%d %H:%M")
            elif len(target_text) == 10:  # YYYY-MM-DD
                self.countdown_target_date = datetime.datetime.strptime(target_text, "%Y-%m-%d")
                # 如果只有日期，设置时间为当天23:59:59
                self.countdown_target_date = self.countdown_target_date.replace(hour=23, minute=59, second=59)
            else:
                raise ValueError("时间格式不正确")
        except ValueError as e:
            QMessageBox.critical(self, "错误", f"时间格式不正确，请使用 YYYY-MM-DD HH:MM:SS 格式\n{e}")
            return

        # 检查时间是否已过期
        if self.countdown_target_date <= datetime.datetime.now():
            QMessageBox.warning(self, "警告", "目标时间已过期，请选择未来时间")
            self.countdown_target_date = None
            return

        self.update_countdown()

    def clear_countdown(self) -> None:
        """清除倒计时"""
        self.countdown_target_date = None
        self.countdown_label.setText("--天 --:--:--:--")
        self.countdown_target.clear()


    def show_date_picker(self) -> None:
        """显示日期选择器对话框"""
        from PyQt6.QtWidgets import QDialog
        from PyQt6.QtCore import QDate

        dialog = QDialog(self)
        dialog.setWindowTitle('选择日期')
        dialog.setFixedSize(320, 340)  # 调整高度

        layout = QVBoxLayout(dialog)
        layout.setSpacing(5)

        calendar = QCalendarWidget()
        calendar.setGridVisible(True)
        calendar.setSelectedDate(QDate.currentDate())
        calendar.setFixedSize(310, 250)  # 确保显示5周
        layout.addWidget(calendar)

        # 预设快捷按钮
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(5)

        today_btn = QPushButton('今天')
        today_btn.setFont(QFont("Arial", 10))
        today_btn.clicked.connect(lambda: calendar.setSelectedDate(QDate.currentDate()))
        btn_layout.addWidget(today_btn)

        tomorrow_btn = QPushButton('明天')
        tomorrow_btn.setFont(QFont("Arial", 10))
        tomorrow_btn.clicked.connect(lambda: calendar.setSelectedDate(QDate.currentDate().addDays(1)))
        btn_layout.addWidget(tomorrow_btn)

        week_btn = QPushButton('一周后')
        week_btn.setFont(QFont("Arial", 10))
        week_btn.clicked.connect(lambda: calendar.setSelectedDate(QDate.currentDate().addDays(7)))
        btn_layout.addWidget(week_btn)

        layout.addLayout(btn_layout)

        # 确定/取消按钮
        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.setFont(QFont("Arial", 10))
        buttons.accepted.connect(dialog.accept)
        buttons.rejected.connect(dialog.reject)
        layout.addWidget(buttons)

        if dialog.exec() == QDialog.DialogCode.Accepted:
            selected_date = calendar.selectedDate()
            current_text = self.countdown_target.text().strip()
            # 保留当前时间部分，只更新日期
            if current_text and len(current_text) >= 10:
                # 尝试保留时间部分
                time_part = current_text[10:] if len(current_text) > 10 else ' 00:00:00'
                self.countdown_target.setText(f"{selected_date.toString('yyyy-MM-dd')}{time_part}")
            else:
                self.countdown_target.setText(f"{selected_date.toString('yyyy-MM-dd')} 00:00:00")

    def show_time_picker(self) -> None:
        """显示时间选择器"""
        from PyQt6.QtCore import QTime, QDate

        # 使用输入框中的日期，如果为空则使用今天
        current_text = self.countdown_target.text().strip()
        if current_text and len(current_text) >= 10:
            date_str = current_text[:10]
        else:
            date_str = QDate.currentDate().toString('yyyy-MM-dd')

        # 解析当前时间（如果有）
        current_time = QTime.currentTime()
        if current_text and len(current_text) >= 16:
            try:
                time_str = current_text[11:16]
                current_time = QTime.fromString(time_str, 'HH:mm')
            except:
                pass

        # 显示时间对话框
        dialog = QDialog(self)
        dialog.setWindowTitle('选择时间')
        dialog.setFixedSize(220, 140)

        layout = QVBoxLayout(dialog)
        layout.setSpacing(10)

        time_edit = QTimeEdit()
        time_edit.setDisplayFormat('HH:mm:ss')
        time_edit.setTime(current_time)
        time_edit.setFont(QFont("Arial", 14))
        time_edit.setFixedSize(120, 40)
        layout.addWidget(time_edit)

        # 快捷按钮
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(5)

        now_btn = QPushButton('现在')
        now_btn.setFont(QFont("Arial", 10))
        now_btn.clicked.connect(lambda: time_edit.setTime(QTime.currentTime()))
        btn_layout.addWidget(now_btn)

        layout.addLayout(btn_layout)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.setFont(QFont("Arial", 10))
        buttons.accepted.connect(dialog.accept)
        buttons.rejected.connect(dialog.reject)
        layout.addWidget(buttons)

        if dialog.exec() == QDialog.DialogCode.Accepted:
            selected_time = time_edit.time()
            time_str = selected_time.toString('HH:mm:ss')
            self.countdown_target.setText(f'{date_str} {time_str}')

    # ------------------- 闹钟相关方法 -------------------

    def init_alarms(self) -> None:
        """初始化闹钟管理器并加载保存的闹钟"""
        from accelworld_alarm import AlarmManager
        from accelworld_config import get_alarms

        self.alarm_manager = AlarmManager()

        # 从配置加载闹钟
        saved_alarms = get_alarms()
        if saved_alarms:
            self.alarm_manager.from_dict_list(saved_alarms)

        # 更新闹钟列表显示
        self.refresh_alarm_list()

    def refresh_alarm_list(self) -> None:
        """刷新闹钟列表显示"""
        self.alarm_list.clear()

        for alarm in self.alarm_manager.alarms:
            item = QListWidgetItem()
            item.setData(Qt.ItemDataRole.UserRole, alarm.id)

            # 创建自定义 widget 来显示闹钟信息
            widget = QWidget()
            layout = QHBoxLayout(widget)
            layout.setContentsMargins(5, 2, 5, 2)

            # 启用开关
            checkbox = QCheckBox()
            checkbox.setChecked(alarm.enabled)
            checkbox.setFixedWidth(30)
            checkbox.toggled.connect(lambda checked, a_id=alarm.id: self.toggle_alarm(a_id))
            layout.addWidget(checkbox)

            # 时间
            time_label = QLabel(alarm.time)
            time_label.setFont(QFont("Arial", 12, QFont.Weight.Bold))
            time_label.setFixedWidth(60)
            layout.addWidget(time_label)

            # 标签
            label_label = QLabel(alarm.label)
            label_label.setFont(QFont("Arial", 11))
            label_label.setFixedWidth(150)
            layout.addWidget(label_label)

            # 重复信息
            repeat_str = self.get_repeat_display(alarm.repeat_days)
            repeat_label = QLabel(repeat_str)
            repeat_label.setFont(QFont("Arial", 10))
            repeat_label.setStyleSheet("color: #888888")
            layout.addWidget(repeat_label)

            # 声音信息
            sound_str = self.get_sound_display(alarm)
            sound_label = QLabel(sound_str)
            sound_label.setFont(QFont("Arial", 10))
            sound_label.setStyleSheet("color: #666666")
            layout.addWidget(sound_label)

            layout.addStretch()

            # 编辑按钮
            edit_btn = QPushButton("编辑")
            edit_btn.setFixedSize(50, 25)
            edit_btn.clicked.connect(lambda checked, a_id=alarm.id: self.show_edit_alarm_dialog(a_id))
            layout.addWidget(edit_btn)

            # 删除按钮
            delete_btn = QPushButton("删除")
            delete_btn.setFixedSize(50, 25)
            delete_btn.clicked.connect(lambda checked, a_id=alarm.id: self.delete_alarm(a_id))
            layout.addWidget(delete_btn)

            widget.setLayout(layout)
            item.setSizeHint(widget.sizeHint())

            self.alarm_list.addItem(item)
            self.alarm_list.setItemWidget(item, widget)

    def get_repeat_display(self, repeat_days: List[int]) -> str:
        """获取重复显示文本"""
        if not repeat_days:
            return "一次"
        days = ["一", "二", "三", "四", "五", "六", "日"]
        return "周" + "".join([days[d] for d in repeat_days])

    def get_sound_display(self, alarm) -> str:
        """获取声音显示文本"""
        from accelworld_alarm import PresetSound
        if alarm.sound_type == "preset":
            preset = PresetSound.from_value(alarm.sound_value)
            return f"🔔 {preset.display_names()[list(PresetSound).index(preset)]}"
        else:
            # 显示文件名
            import os
            return f"📁 {os.path.basename(alarm.sound_value)[:15]}"

    def show_add_alarm_dialog(self) -> None:
        """显示添加闹钟对话框"""
        dialog = AlarmEditDialog(self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            alarm_data = dialog.get_alarm_data()
            from accelworld_alarm import Alarm
            alarm = Alarm(**alarm_data)
            if self.alarm_manager.add_alarm(alarm):
                self.save_alarms()
                self.refresh_alarm_list()

    def show_edit_alarm_dialog(self, alarm_id: str) -> None:
        """显示编辑闹钟对话框"""
        alarm = self.alarm_manager.get_alarm(alarm_id)
        if not alarm:
            return

        dialog = AlarmEditDialog(self, alarm)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            alarm_data = dialog.get_alarm_data()
            self.alarm_manager.update_alarm(alarm_id, **alarm_data)
            self.save_alarms()
            self.refresh_alarm_list()

    def toggle_alarm(self, alarm_id: str) -> bool:
        """切换闹钟启用状态"""
        result = self.alarm_manager.toggle_alarm(alarm_id)
        if result:
            self.save_alarms()
            self.refresh_alarm_list()
        return result

    def delete_alarm(self, alarm_id: str) -> None:
        """删除闹钟"""
        from PyQt6.QtWidgets import QMessageBox
        alarm = self.alarm_manager.get_alarm(alarm_id)
        if not alarm:
            return

        reply = QMessageBox.question(
            self, "确认删除",
            f"确定要删除闹钟「{alarm.label}」吗？",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No
        )

        if reply == QMessageBox.StandardButton.Yes:
            if self.alarm_manager.remove_alarm(alarm_id):
                self.save_alarms()
                self.refresh_alarm_list()

    def save_alarms(self) -> None:
        """保存闹钟列表到配置"""
        from accelworld_config import save_alarms
        save_alarms(self.alarm_manager.to_dict_list())

    def check_alarms(self) -> None:
        """检查是否需要触发闹钟"""
        if self.alarm_manager is None:
            return
        import datetime
        now = datetime.datetime.now()
        triggered = self.alarm_manager.check_alarms(now)

        for alarm in triggered:
            self.trigger_alarm(alarm)

    def trigger_alarm(self, alarm) -> None:
        """触发闹钟"""
        from accelworld_alarm import play_alarm_sound

        # 播放声音
        play_alarm_sound(alarm)

        # 显示通知（使用 emoji 装饰避免 Windows 通知栏兼容问题）
        self.show_notification(
            "Alarm",
            f" ⏰ {alarm.label} @ {alarm.time} ",
            QSystemTrayIcon.MessageIcon.Warning
        )

        # 如果是一次性闹钟，自动禁用
        if alarm.is_one_time():
            alarm.enabled = False
            self.save_alarms()
            self.refresh_alarm_list()

    def update_countdown(self) -> None:
        """更新倒计时显示"""
        import datetime

        if not self.countdown_target_date:
            return

        now = datetime.datetime.now()
        remaining = self.countdown_target_date - now

        if remaining.total_seconds() <= 0:
            self.countdown_label.setText("00天 00:00:00")
            self.countdown_label.setStyleSheet("color: #f44336;")  # 红色表示倒计时结束
            return

        days = remaining.days
        hours = remaining.seconds // 3600
        minutes = (remaining.seconds % 3600) // 60
        seconds = remaining.seconds % 60

        self.countdown_label.setText(f"{days}天 {hours:02d}:{minutes:02d}:{seconds:02d}")
        self.countdown_label.setStyleSheet("color: #4CAF50;")

    def update_world_clock(self) -> None:
        """更新世界时钟显示"""
        import datetime
        import pytz

        tz_name = self.timezone_combo.currentData()
        if not tz_name:
            return

        try:
            tz = pytz.timezone(tz_name)
            world_time = datetime.datetime.now(tz).strftime("%H:%M:%S")
            self.world_clock_label.setText(world_time)
        except Exception as e:
            logger.error(f"更新世界时钟时出错: {e}")
            self.world_clock_label.setText("00:00:00")

    def on_city_changed(self, city_name: str) -> None:
        """城市选择变更时的回调函数"""
        self.current_city = city_name
        self.update_weather()

    def update_weather(self) -> None:
        """更新天气信息"""
        try:
            from accelworld_weather import get_weather_by_city, format_weather_info
            weather = get_weather_by_city(self.current_city)
            if weather:
                self.weather_info_label.setText(format_weather_info(weather, self.current_city))
                self.weather_icon_label.setText(weather["icon"])
            else:
                self.weather_info_label.setText("天气获取失败")
                self.weather_icon_label.setText("❓")
        except Exception as e:
            logger.error(f"更新天气时出错: {e}")
            self.weather_info_label.setText("天气获取失败")
            self.weather_icon_label.setText("❓")

    def toggle_theme(self) -> None:
        """切换主题"""
        self.is_dark_theme = not self.is_dark_theme
        self.apply_theme()

    def apply_theme(self) -> None:
        """应用当前主题"""
        if self.is_dark_theme:
            self.setStyleSheet(DARK_THEME)
            self.progress_bar.setStyleSheet(DARK_THEME_PROGRESS)
            self.theme_button.setText("☀️")
            self.theme_button.setToolTip("切换到浅色主题")
        else:
            self.setStyleSheet(LIGHT_THEME)
            self.progress_bar.setStyleSheet(LIGHT_THEME_PROGRESS)
            self.theme_button.setText("🌙")
            self.theme_button.setToolTip("切换到深色主题")

    def update_clock(self) -> None:
        """更新时钟显示"""
        try:
            # 获取当前时间信息
            standard_datetime, custom_time, chinese_date, lunar_info, dilation_percentage, expanded_hours_per_day, remaining_hours = self.accel_world.get_custom_time()

            # 更新标签内容
            self.standard_time_label.setText(f"标准时间: {standard_datetime.split()[1]}")
            self.accelerated_time_label.setText(f"加速时间: {custom_time}")
            self.hours_per_day_value_label.setText(f"{expanded_hours_per_day:.2f}小时")
            self.rate_value_label.setText(f"{dilation_percentage:.0f}%")
            self.remaining_hours_value_label.setText(f"{remaining_hours:.2f}小时")

            # 计算进度并更新进度条
            total_hours = int(expanded_hours_per_day)
            current_hour = int(custom_time.split(":")[0])
            self.progress_bar.setMaximum(total_hours)
            self.progress_bar.setValue(current_hour)

            # 更新日期显示
            self.date_label.setText(chinese_date)

            # 更新农历信息显示
            self.lunar_info_label.setText(lunar_info)

            # 更新倒计时显示
            self.update_countdown()

            # 更新世界时钟显示
            self.update_world_clock()

            # 检查闹钟（每秒检查一次，在分钟变化时触发）
            if hasattr(self, 'alarm_manager'):
                self.check_alarms()

        except Exception as e:
            logger.error(f"更新时钟时出错: {e}")
            import traceback
            traceback.print_exc()

    def setup_system_tray(self) -> None:
        """设置系统托盘"""
        from PyQt6.QtGui import QIcon, QAction

        # 创建托盘图标
        self.tray_icon = QSystemTrayIcon(self)
        self.tray_icon.setToolTip(f"加速世界 - {VERSION}")

        # 创建自定义图标 (32x32 蓝色圆形时钟图标)
        from PyQt6.QtGui import QPixmap, QPainter, QPen, QColor, QBrush
        from PyQt6.QtCore import Qt

        pixmap = QPixmap(32, 32)
        pixmap.fill(Qt.GlobalColor.transparent)  # 透明背景

        painter = QPainter(pixmap)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.setPen(QPen(QColor("#4A90D9"), 2))  # 蓝色边框
        painter.setBrush(QBrush(QColor("#4A90D9")))
        painter.drawEllipse(2, 2, 28, 28)  # 圆形背景

        # 时钟指针
        painter.setPen(QPen(QColor("white"), 2, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap))
        painter.drawLine(16, 16, 16, 8)   # 分针
        painter.drawLine(16, 16, 22, 16)  # 时针
        painter.end()

        self.tray_icon.setIcon(QIcon(pixmap))

        # 创建托盘菜单
        self.tray_menu = QMenu()

        # 显示窗口动作
        self.show_action = QAction("显示窗口", self)
        self.show_action.triggered.connect(self.show_normal)
        self.tray_menu.addAction(self.show_action)

        # 隐藏窗口动作
        self.hide_action = QAction("隐藏到托盘", self)
        self.hide_action.triggered.connect(self.hide_to_tray)
        self.tray_menu.addAction(self.hide_action)

        self.tray_menu.addSeparator()

        # 当前倍率显示
        self.rate_action = QAction(f"当前倍率: {self.accel_world.time_dilation_rate:.1f}x", self)
        self.rate_action.setEnabled(False)
        self.tray_menu.addAction(self.rate_action)

        self.tray_menu.addSeparator()

        # 退出动作
        self.quit_action = QAction("退出", self)
        self.quit_action.triggered.connect(QApplication.quit)
        self.tray_menu.addAction(self.quit_action)

        # 设置托盘菜单
        self.tray_icon.setContextMenu(self.tray_menu)

        # 双击托盘图标显示窗口
        self.tray_icon.activated.connect(self.on_tray_activated)

        # 显示托盘图标
        self.tray_icon.show()

        # 初始隐藏到托盘的标志
        self.is_hidden_to_tray = False

    def on_tray_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        """托盘图标被点击"""
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self.show_normal()

    def hide_to_tray(self) -> None:
        """隐藏到系统托盘"""
        self.hide()
        self.is_hidden_to_tray = True
        self.tray_icon.showMessage(
            "加速世界",
            "程序已隐藏到系统托盘，点击托盘图标可重新显示",
            QSystemTrayIcon.MessageIcon.Information,
            2000
        )

    def show_normal(self) -> None:
        """显示窗口"""
        self.show()
        self.raise_()
        self.activateWindow()
        self.is_hidden_to_tray = False

    def closeEvent(self, a0) -> None:
        """关闭窗口事件 - 最小化到托盘而非退出"""
        if self.tray_icon.isVisible():
            self.hide_to_tray()
            a0.ignore()
        else:
            # 保存配置
            self.save_settings()
            a0.accept()

    def save_settings(self) -> None:
        """保存当前设置"""
        from accelworld_config import set_setting, save_window_geometry
        set_setting("time_dilation_rate", self.accel_world.time_dilation_rate)
        set_setting("last_city", self.current_city)
        set_setting("last_timezone", self.timezone_combo.currentData())
        if self.countdown_target_date:
            set_setting("countdown_target", self.countdown_target.text())
        else:
            set_setting("countdown_target", "")
        save_window_geometry(self.saveGeometry()) 

    def update_tray_info(self) -> None:
        """更新托盘信息"""
        # 更新托盘中显示的倍率
        for action in self.tray_menu.actions():
            if action.text().startswith("当前倍率"):
                action.setText(f"当前倍率: {self.accel_world.time_dilation_rate:.1f}x")
                break

    def show_notification(self, title: str, message: str, icon: QSystemTrayIcon.MessageIcon = QSystemTrayIcon.MessageIcon.Information) -> None:
        """显示系统通知"""
        if self.tray_icon.isVisible():
            self.tray_icon.showMessage(title, message, icon, 3000)


# ------------------- 闹钟编辑对话框 -------------------

class AlarmEditDialog(QDialog):
    """闹钟编辑对话框"""

    def __init__(self, parent=None, alarm=None):
        """
        初始化对话框

        :param parent: 父窗口
        :param alarm: 要编辑的闹钟（None 表示新建）
        """
        super().__init__(parent)
        self.alarm = alarm
        self.setWindowTitle("编辑闹钟" if alarm else "添加闹钟")
        self.setFixedWidth(400)

        layout = QFormLayout(self)

        # 标签
        self.label_edit = QLineEdit()
        self.label_edit.setPlaceholderText("闹钟名称")
        self.label_edit.setText(alarm.label if alarm else "Alarm")
        layout.addRow("标签:", self.label_edit)

        # 时间
        self.time_edit = QTimeEdit()
        self.time_edit.setDisplayFormat("HH:mm")
        if alarm:
            time_parts = alarm.time.split(":")
            self.time_edit.setTime(QTime(int(time_parts[0]), int(time_parts[1])))
        else:
            self.time_edit.setTime(QTime.currentTime().addSecs(3600))  # 默认1小时后
        layout.addRow("时间:", self.time_edit)

        # 声音选择
        sound_layout = QHBoxLayout()
        self.sound_combo = QComboBox()
        from accelworld_alarm import PresetSound
        self.sound_combo.addItems(PresetSound.display_names())
        sound_layout.addWidget(self.sound_combo)

        self.custom_sound_button = QPushButton("自定义...")
        self.custom_sound_button.clicked.connect(self.select_custom_sound)
        sound_layout.addWidget(self.custom_sound_button)

        # 根据已有闹钟初始化声音设置
        self.sound_type = "preset"
        self.sound_value = "classic"

        if alarm:
            if alarm.sound_type == "custom":
                self.sound_type = "custom"
                self.sound_value = alarm.sound_value
            else:
                # 预设声音：根据 sound_value 找到对应索引
                self.sound_value = alarm.sound_value
                preset_list = list(PresetSound)
                for idx, preset in enumerate(preset_list):
                    if preset.value == alarm.sound_value:
                        self.sound_combo.setCurrentIndex(idx)
                        break

        sound_widget = QWidget()
        sound_widget.setLayout(sound_layout)
        layout.addRow("声音:", sound_widget)

        # 重复设置
        repeat_layout = QHBoxLayout()
        repeat_layout.setSpacing(5)
        self.repeat_checkboxes = []
        days = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"]
        for i, day in enumerate(days):
            checkbox = QCheckBox(day)
            checkbox.setFixedWidth(45)
            checkbox.setToolTip(days[i])
            if alarm and i in alarm.repeat_days:
                checkbox.setChecked(True)
            repeat_layout.addWidget(checkbox)
            self.repeat_checkboxes.append(checkbox)

        repeat_widget = QWidget()
        repeat_widget.setLayout(repeat_layout)
        layout.addRow("重复:", repeat_widget)

        # 按钮
        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addRow(buttons)

    def select_custom_sound(self) -> None:
        """选择自定义音频文件"""
        from accelworld_alarm import SUPPORTED_AUDIO_FORMATS
        file_path, _ = QFileDialog.getOpenFileName(
            self, "选择闹钟声音", "", SUPPORTED_AUDIO_FORMATS
        )
        if file_path:
            self.sound_type = "custom"
            self.sound_value = file_path
            import os
            self.custom_sound_button.setText(f"📁 {os.path.basename(file_path)[:15]}")

    def get_alarm_data(self) -> Dict[str, Any]:
        """获取闹钟数据"""
        # 获取时间
        time_obj = self.time_edit.time()
        time_str = f"{time_obj.hour():02d}:{time_obj.minute():02d}"

        # 获取重复天数
        repeat_days = [i for i, cb in enumerate(self.repeat_checkboxes) if cb.isChecked()]

        # 获取声音值
        from accelworld_alarm import PresetSound
        if self.sound_type == "preset":
            sound_value = list(PresetSound)[self.sound_combo.currentIndex()].value
        else:
            sound_value = self.sound_value

        return {
            "label": self.label_edit.text().strip() or "新闹钟",
            "time": time_str,
            "sound_type": self.sound_type,
            "sound_value": sound_value,
            "repeat_days": repeat_days,
            "enabled": True,
        }


def main_gui(**kwargs) -> None:
    """
    图形界面主函数

    :param kwargs: 可选参数
        - rate: 时间膨胀倍率
        - theme: 主题 ("light" 或 "dark")
        - city: 默认城市
        - hidden: 是否隐藏到托盘
    """
    app = QApplication([])
    window = AcceleratedWorldGUI()

    # 应用启动参数
    if kwargs.get("theme") == "dark":
        window.is_dark_theme = False
        window.toggle_theme()  # 切换到暗色

    if kwargs.get("hidden"):
        window.hide_to_tray()
    else:
        window.show()

    app.exec()


if __name__ == "__main__":
    main_gui()
