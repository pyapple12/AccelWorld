from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QSlider, QLineEdit, QPushButton, QFrame, QMessageBox,
    QGridLayout, QProgressBar, QComboBox, QButtonGroup, QRadioButton
)
from PyQt6.QtCore import Qt, QTimer, QSize
from PyQt6.QtGui import QFont, QDoubleValidator

# 程序版本号
VERSION = "ver 0.20"

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

        self.setWindowTitle(f"加速世界 - 时间膨胀时钟 {VERSION}")
        self.setGeometry(100, 100, 900, 500)

        # 创建加速世界核心实例
        self.accel_world = AcceleratedWorld(time_dilation_rate=2.0)

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
        self.apply_theme()

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
        self.theme_button.setFont(QFont("Arial", 14))
        self.theme_button.setFixedSize(40, 35)
        self.theme_button.setToolTip("切换主题")
        self.theme_button.clicked.connect(self.toggle_theme)
        weather_layout.addWidget(self.theme_button)

        # 刷新天气按钮
        self.refresh_weather_button = QPushButton("刷新")
        self.refresh_weather_button.setFont(QFont("Arial", 10))
        self.refresh_weather_button.clicked.connect(self.update_weather)
        weather_layout.addWidget(self.refresh_weather_button)

        self.main_layout.addWidget(weather_frame)

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
        self.slider.setValue(20)  # 2.0 * 10
        self.slider.setFixedHeight(30)
        self.slider.valueChanged.connect(self.on_slider_change)
        input_layout.addWidget(self.slider, 1, 0, 1, 3)

        # 滑杆值显示
        self.slider_value_label = QLabel("2.0x")
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
            print(f"更新天气时出错: {e}")
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

        except Exception as e:
            print(f"更新时钟时出错: {e}")
            import traceback
            traceback.print_exc()


def main_gui() -> None:
    """图形界面主函数"""
    app = QApplication([])
    window = AcceleratedWorldGUI()
    window.show()
    app.exec()


if __name__ == "__main__":
    main_gui()
