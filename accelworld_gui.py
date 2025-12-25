import sys
from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QPushButton, QLineEdit, QSlider, QFrame, QGroupBox
)
from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QFont
from qt_material import apply_stylesheet

# 导入核心计算模块
from accelworld_calc import AcceleratedWorld, VERSION
from accelworld_date import get_chinese_date


class AcceleratedWorldGUI(QMainWindow):
    """加速世界图形界面类 - 使用PyQt6实现的可视化时钟应用"""
    
    def __init__(self):
        """
        初始化图形界面
        """
        super().__init__()
        
        # 设置窗口标题和基本属性
        self.setWindowTitle(f"加速世界 - 时间膨胀时钟 {VERSION}")
        self.resize(900, 700)
        self.setMinimumSize(700, 700)
        
        # 创建加速世界核心实例
        self.accel_world = AcceleratedWorld(time_dilation_rate=2.0)
        
        # 初始化UI组件
        self.setup_ui()
        
        # 启动实时时钟更新
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_clock)
        self.timer.start(100)  # 每100毫秒更新一次
    
    def setup_ui(self):
        """设置UI界面布局和组件"""
        # 创建中心部件
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        
        # 创建主布局
        main_layout = QVBoxLayout(central_widget)
        main_layout.setSpacing(20)
        main_layout.setContentsMargins(20, 20, 20, 20)
        
        # ------------------- 时钟显示区域 -------------------
        clock_group = QGroupBox("实时时钟")
        clock_layout = QHBoxLayout(clock_group)
        clock_layout.setSpacing(30)
        clock_layout.setContentsMargins(20, 15, 20, 15)
        
        # 标准时间标签
        self.standard_time_label = QLabel("标准时间: 00:00:00")
        self.standard_time_label.setFont(QFont("Arial", 24, QFont.Weight.Bold))
        self.standard_time_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.standard_time_label.setStyleSheet("font-size: 24px; font-weight: bold;")
        clock_layout.addWidget(self.standard_time_label)
        
        # 加速时间标签
        self.accelerated_time_label = QLabel("加速时间: 00:00:00")
        self.accelerated_time_label.setFont(QFont("Arial", 24, QFont.Weight.Bold))
        self.accelerated_time_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.accelerated_time_label.setStyleSheet("font-size: 24px; font-weight: bold;")
        clock_layout.addWidget(self.accelerated_time_label)
        
        main_layout.addWidget(clock_group)
        
        # ------------------- 加速参数显示区域 -------------------
        params_group = QGroupBox("加速参数")
        params_layout = QHBoxLayout(params_group)
        params_layout.setSpacing(30)
        params_layout.setContentsMargins(20, 15, 20, 15)
        
        # 设置三列等宽
        params_layout.addStretch(1)
        
        # 加速后一天小时数
        hours_label = QLabel("加速后一天小时数:")
        hours_label.setFont(QFont("Arial", 18))
        hours_label.setStyleSheet("font-size: 18px;")
        params_layout.addWidget(hours_label)
        
        self.hours_per_day_label = QLabel("48.00小时")
        self.hours_per_day_label.setFont(QFont("Arial", 18, QFont.Weight.Bold))
        self.hours_per_day_label.setStyleSheet("font-size: 18px; font-weight: bold;")
        params_layout.addWidget(self.hours_per_day_label)
        
        params_layout.addStretch(1)
        
        # 加速倍率
        rate_label = QLabel("加速倍率:")
        rate_label.setFont(QFont("Arial", 18))
        rate_label.setStyleSheet("font-size: 18px;")
        params_layout.addWidget(rate_label)
        
        self.acceleration_rate_label = QLabel("200%")
        self.acceleration_rate_label.setFont(QFont("Arial", 18, QFont.Weight.Bold))
        self.acceleration_rate_label.setStyleSheet("font-size: 18px; font-weight: bold;")
        params_layout.addWidget(self.acceleration_rate_label)
        
        params_layout.addStretch(1)
        
        # 加速后剩余小时数
        remaining_label = QLabel("加速后剩余小时数:")
        remaining_label.setFont(QFont("Arial", 18))
        remaining_label.setStyleSheet("font-size: 18px;")
        params_layout.addWidget(remaining_label)
        
        self.remaining_hours_label = QLabel("45.00小时")
        self.remaining_hours_label.setFont(QFont("Arial", 18, QFont.Weight.Bold))
        self.remaining_hours_label.setStyleSheet("font-size: 18px; font-weight: bold;")
        params_layout.addWidget(self.remaining_hours_label)
        
        params_layout.addStretch(1)
        
        main_layout.addWidget(params_group)
        
        # ------------------- 日期显示区域 -------------------
        date_group = QGroupBox("当前日期")
        date_layout = QVBoxLayout(date_group)
        date_layout.setSpacing(10)
        date_layout.setContentsMargins(20, 15, 20, 15)
        
        # 中文日期标签
        self.date_label = QLabel("2025年12月25日 星期四")
        self.date_label.setFont(QFont("Arial", 20))
        self.date_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.date_label.setStyleSheet("font-size: 20px;")
        date_layout.addWidget(self.date_label)
        
        # 农历信息标签
        self.lunar_info_label = QLabel(
            "乙巳年（蛇年）冬月初六丑时 月相：夕月 公历节日：圣诞节 拜财神：正北方向（坎位）"
        )
        self.lunar_info_label.setFont(QFont("Arial", 18))
        self.lunar_info_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.lunar_info_label.setStyleSheet("font-size: 18px; color: #555555")
        self.lunar_info_label.setWordWrap(True)
        date_layout.addWidget(self.lunar_info_label)
        
        main_layout.addWidget(date_group)
        
        # ------------------- 用户交互区域 -------------------
        input_group = QGroupBox("加速设置")
        input_layout = QVBoxLayout(input_group)
        input_layout.setSpacing(15)
        input_layout.setContentsMargins(20, 15, 20, 15)
        
        # 输入框行
        input_row = QHBoxLayout()
        input_row.setSpacing(10)
        
        # 加速倍率标签
        rate_input_label = QLabel("加速倍率:")
        rate_input_label.setFont(QFont("Arial", 12))
        input_row.addWidget(rate_input_label)
        
        # 加速倍率输入框
        self.rate_entry = QLineEdit()
        self.rate_entry.setFont(QFont("Arial", 12))
        self.rate_entry.setPlaceholderText("2.0")
        self.rate_entry.setMaximumWidth(100)
        input_row.addWidget(self.rate_entry)
        
        # 说明文本
        info_label = QLabel("（必须大于1.0，默认值2.0，最大值20.0）")
        info_label.setFont(QFont("Arial", 10))
        input_row.addWidget(info_label)
        
        # 应用按钮
        self.apply_button = QPushButton("应用加速")
        self.apply_button.setFont(QFont("Arial", 12, QFont.Weight.Bold))
        self.apply_button.clicked.connect(self.apply_acceleration)
        input_row.addWidget(self.apply_button)
        
        input_row.addStretch(1)
        input_layout.addLayout(input_row)
        
        # 滑杆行
        slider_row = QHBoxLayout()
        slider_row.setSpacing(10)
        
        # 滑杆标签
        slider_label = QLabel("滑杆调节:")
        slider_label.setFont(QFont("Arial", 12))
        slider_row.addWidget(slider_label)
        
        # 加速倍率滑杆
        self.rate_slider = QSlider(Qt.Orientation.Horizontal)
        self.rate_slider.setRange(10, 200)
        self.rate_slider.setValue(20)
        self.rate_slider.setTickInterval(10)
        self.rate_slider.setTickPosition(QSlider.TickPosition.TicksBelow)
        self.rate_slider.valueChanged.connect(self.on_slider_change)
        slider_row.addWidget(self.rate_slider)
        
        # 滑杆值显示
        self.slider_value_label = QLabel("2.0x")
        self.slider_value_label.setFont(QFont("Arial", 12))
        self.slider_value_label.setMinimumWidth(50)
        slider_row.addWidget(self.slider_value_label)
        
        input_layout.addLayout(slider_row)
        
        main_layout.addWidget(input_group)
    
    def on_slider_change(self):
        """滑杆值变化时的回调函数"""
        slider_value = self.rate_slider.value() / 10.0
        self.slider_value_label.setText(f"{slider_value:.1f}x")
    
    def apply_acceleration(self):
        """应用加速倍率"""
        try:
            # 优先使用输入框的值，如果输入框为空或无效，则使用滑杆的值
            if self.rate_entry.text().strip():
                rate = float(self.rate_entry.text())
                # 精确到小数点后2位
                rate = round(rate, 2)
            else:
                rate = self.rate_slider.value() / 10.0
            
            # 验证倍率是否在有效范围内
            if not (1.0 <= rate <= 20.0):
                raise ValueError("加速倍率必须在1.0到20.0之间")
            
            # 更新加速世界实例
            self.accel_world = AcceleratedWorld(time_dilation_rate=rate)
            
            # 同步输入框和滑杆的值
            self.rate_entry.clear()  # 清空文本框
            self.rate_slider.setValue(int(rate * 10))
            self.slider_value_label.setText(f"{rate:.1f}x")
            
        except ValueError as e:
            # 显示错误信息
            error_dialog = QWidget()
            error_dialog.setWindowTitle("错误")
            error_dialog.resize(300, 100)
            
            error_layout = QVBoxLayout(error_dialog)
            error_label = QLabel(str(e))
            error_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
            error_layout.addWidget(error_label)
            
            ok_button = QPushButton("确定")
            ok_button.clicked.connect(error_dialog.close)
            error_layout.addWidget(ok_button, 0, Qt.AlignmentFlag.AlignCenter)
            
            error_dialog.exec()
    
    def update_clock(self):
        """更新时钟显示"""
        try:
            # 获取当前时间信息
            standard_datetime, custom_time, chinese_date, lunar_info, dilation_percentage, expanded_hours_per_day, remaining_hours = self.accel_world.get_custom_time()
            
            # 更新标签内容
            self.standard_time_label.setText(f"标准时间: {standard_datetime.split()[1]}")
            self.accelerated_time_label.setText(f"加速时间: {custom_time}")
            self.hours_per_day_label.setText(f"{expanded_hours_per_day:.2f}小时")
            self.acceleration_rate_label.setText(f"{dilation_percentage:.0f}%")
            self.remaining_hours_label.setText(f"{remaining_hours:.2f}小时")
            
            # 更新日期显示
            self.date_label.setText(chinese_date)
            
            # 更新农历信息显示
            self.lunar_info_label.setText(lunar_info)
            
        except Exception as e:
            print(f"更新时钟时出错: {e}")
            import traceback
            traceback.print_exc()


def main_gui():
    """图形界面主函数"""
    app = QApplication(sys.argv)
    
    # 应用qt-material明亮主题
    apply_stylesheet(app, theme='light_cyan.xml')
    
    window = AcceleratedWorldGUI()
    window.show()
    
    sys.exit(app.exec())


if __name__ == "__main__":
    main_gui()
