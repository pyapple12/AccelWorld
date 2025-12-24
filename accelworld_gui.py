import tkinter as tk
from tkinter import ttk
from accelworld_calc import AcceleratedWorld, VERSION
from accelworld_date import get_chinese_date


class AcceleratedWorldGUI:
    """加速世界图形界面类 - 使用Tkinter实现的可视化时钟应用"""
    
    def __init__(self, root: tk.Tk):
        """
        初始化图形界面
        :param root: Tkinter根窗口对象
        """
        self.root = root
        self.root.title(f"加速世界 - 时间膨胀时钟 {VERSION}")
        # 设置窗口大小和位置（居中显示）
        screen_width = self.root.winfo_screenwidth()
        screen_height = self.root.winfo_screenheight()
        width = 900
        height = 500
        x = (screen_width - width) // 2
        y = (screen_height - height) // 2
        self.root.geometry(f"{width}x{height}+{x}+{y}")
        self.root.resizable(True, True)
        self.root.lift()  # 确保窗口在最上层
        
        # 创建加速世界核心实例
        self.accel_world = AcceleratedWorld(time_dilation_rate=2.0)
        
        # 初始化UI组件
        self.setup_ui()
        
        # 启动实时时钟更新
        self.update_clock()
    
    def setup_ui(self) -> None:
        """设置UI界面布局和组件"""
        # 创建主容器
        main_frame = ttk.Frame(self.root, padding="20")
        main_frame.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))
        
        # 设置列和行的权重，使界面可以自适应大小
        self.root.columnconfigure(0, weight=1)
        self.root.rowconfigure(0, weight=1)
        main_frame.columnconfigure(0, weight=1)
        main_frame.columnconfigure(1, weight=1)
        main_frame.columnconfigure(2, weight=1)
        main_frame.rowconfigure(0, weight=1)
        main_frame.rowconfigure(1, weight=1)
        main_frame.rowconfigure(2, weight=1)
        main_frame.rowconfigure(3, weight=1)
        
        # ------------------- 时钟显示区域 -------------------
        clock_frame = ttk.LabelFrame(main_frame, text="实时时钟", padding="10")
        clock_frame.grid(row=0, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=(0, 10))
        clock_frame.columnconfigure(0, weight=1)
        clock_frame.columnconfigure(1, weight=1)
        clock_frame.rowconfigure(0, weight=1)
        
        # 标准时间标签
        self.standard_time_label = ttk.Label(clock_frame, text="标准时间: 00:00:00", font=('Arial', 16))
        self.standard_time_label.grid(row=0, column=0, sticky=tk.E, padx=(0, 20))
        
        # 加速时间标签
        self.accelerated_time_label = ttk.Label(clock_frame, text="加速时间: 00:00:00", font=('Arial', 16))
        self.accelerated_time_label.grid(row=0, column=1, sticky=tk.W)
        
        # ------------------- 加速参数显示区域 -------------------
        params_frame = ttk.LabelFrame(main_frame, text="加速参数", padding="10")
        params_frame.grid(row=1, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=10)
        # 设置params_frame的列权重
        params_frame.columnconfigure(0, weight=1)
        params_frame.columnconfigure(1, weight=1)
        params_frame.columnconfigure(2, weight=1)
        params_frame.columnconfigure(3, weight=1)
        params_frame.columnconfigure(4, weight=1)
        params_frame.columnconfigure(5, weight=1)
        params_frame.rowconfigure(0, weight=1)
        
        # 加速后一天小时数
        ttk.Label(params_frame, text="加速后一天小时数:", font=('Arial', 12)).grid(row=0, column=0, sticky=tk.E, padx=(0, 10))
        self.hours_per_day_label = ttk.Label(params_frame, text="48.00小时", font=('Arial', 12, 'bold'))
        self.hours_per_day_label.grid(row=0, column=1, sticky=tk.W, padx=(0, 40))
        
        # 加速倍率
        ttk.Label(params_frame, text="加速倍率:", font=('Arial', 12)).grid(row=0, column=2, sticky=tk.E, padx=(0, 10))
        self.acceleration_rate_label = ttk.Label(params_frame, text="200%", font=('Arial', 12, 'bold'))
        self.acceleration_rate_label.grid(row=0, column=3, sticky=tk.W, padx=(0, 40))
        
        # 加速后剩余小时数
        ttk.Label(params_frame, text="加速后剩余小时数:", font=('Arial', 12)).grid(row=0, column=4, sticky=tk.E, padx=(0, 10))
        self.remaining_hours_label = ttk.Label(params_frame, text="45.00小时", font=('Arial', 12, 'bold'))
        self.remaining_hours_label.grid(row=0, column=5, sticky=tk.W)
        
        # ------------------- 日期显示区域 -------------------
        date_frame = ttk.LabelFrame(main_frame, text="当前日期", padding="10")
        date_frame.grid(row=2, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=10)
        date_frame.rowconfigure(0, weight=1)
        date_frame.rowconfigure(1, weight=1)
        date_frame.columnconfigure(0, weight=1)
        
        # 中文日期标签
        self.date_label = ttk.Label(date_frame, text="2025年12月25日 星期四", font=('Arial', 14), anchor=tk.CENTER)
        self.date_label.grid(row=0, column=0, sticky=(tk.W, tk.E))  # 居中显示
        
        # 农历信息标签
        self.lunar_info_label = ttk.Label(date_frame, text="乙巳年（蛇年）冬月初六丑时 月相：夕月 公历节日：圣诞节 拜财神：正北方向（坎位）", 
                                          font=('Arial', 12), anchor=tk.CENTER, foreground="#555555")
        self.lunar_info_label.grid(row=1, column=0, sticky=(tk.W, tk.E), pady=(5, 0))  # 居中显示，位于日期下方
        
        # ------------------- 用户交互区域 -------------------
        input_frame = ttk.LabelFrame(main_frame, text="加速设置", padding="10")
        input_frame.grid(row=3, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=10)
        # 设置列配置
        input_frame.columnconfigure(0, weight=0)  # 标签列
        input_frame.columnconfigure(1, weight=0)  # 输入框列
        input_frame.columnconfigure(2, weight=1)  # 说明文本列
        input_frame.columnconfigure(3, weight=0)  # 按钮列
        
        # 设置行配置
        input_frame.rowconfigure(0, weight=1)
        input_frame.rowconfigure(1, weight=1)
        input_frame.rowconfigure(2, weight=1)
        
        # 加速倍率输入框
        ttk.Label(input_frame, text="加速倍率:", font=("Arial", 12)).grid(row=0, column=0, sticky=tk.E, padx=(0, 5), pady=(0, 5))
        
        self.rate_var = tk.StringVar(value="2.0")
        self.rate_entry = ttk.Entry(input_frame, textvariable=self.rate_var, width=10, font=("Arial", 12))
        self.rate_entry.grid(row=0, column=1, sticky=tk.W, padx=(0, 5), pady=(0, 5))
        
        # 加速倍率说明
        ttk.Label(input_frame, text="（必须大于1.0，默认值2.0，最大值20.0）", font=("Arial", 10)).grid(row=0, column=2, sticky=tk.W, pady=(0, 5))
        
        # 加速倍率滑杆
        self.slider_var = tk.DoubleVar(value=2.0)
        self.rate_slider = ttk.Scale(input_frame, from_=1.0, to=20.0, orient=tk.HORIZONTAL, 
                                      variable=self.slider_var, command=self.on_slider_change)
        self.rate_slider.grid(row=1, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=(0, 10))
        
        # 滑杆值显示
        self.slider_value_label = ttk.Label(input_frame, text="2.0x", font=("Arial", 12))
        self.slider_value_label.grid(row=2, column=1, pady=(0, 10))
        
        # 确认按钮
        self.confirm_button = ttk.Button(input_frame, text="应用加速", command=self.apply_acceleration, style="Accent.TButton")
        self.confirm_button.grid(row=0, column=3, rowspan=2, sticky=(tk.N, tk.S, tk.E), padx=(20, 0), pady=(0, 5))
        
        # ------------------- 样式设置 -------------------
        style = ttk.Style()
        style.configure("Accent.TButton", font=("Arial", 12, "bold"))
    
    def on_slider_change(self, value: str) -> None:
        """滑杆值变化时的回调函数"""
        slider_value = float(value)
        self.slider_value_label.config(text=f"{slider_value:.1f}x")
    
    def apply_acceleration(self) -> None:
        """应用加速倍率"""
        try:
            # 优先使用输入框的值，如果输入框为空或无效，则使用滑杆的值
            if self.rate_var.get().strip():
                rate = float(self.rate_var.get())
                # 精确到小数点后2位
                rate = round(rate, 2)
            else:
                rate = self.slider_var.get()
            
            # 验证倍率是否在有效范围内
            if not (1.0 <= rate <= 20.0):
                raise ValueError("加速倍率必须在1.0到20.0之间")
            
            # 更新加速世界实例
            self.accel_world = AcceleratedWorld(time_dilation_rate=rate)
            
            # 同步输入框和滑杆的值
            self.rate_var.set("")  # 清空文本框
            self.slider_var.set(rate)
            self.slider_value_label.config(text=f"{rate:.1f}x")
            
        except ValueError as e:
            # 显示错误信息
            error_window = tk.Toplevel(self.root)
            error_window.title("错误")
            error_window.geometry("300x100")
            error_window.transient(self.root)
            error_window.grab_set()
            
            ttk.Label(error_window, text=str(e)).pack(pady=20)
            ttk.Button(error_window, text="确定", command=error_window.destroy).pack()
    
    def update_clock(self) -> None:
        """更新时钟显示"""
        try:
            # 获取当前时间信息
            standard_datetime, custom_time, chinese_date, lunar_info, dilation_percentage, expanded_hours_per_day, remaining_hours = self.accel_world.get_custom_time()
            
            # 更新标签内容
            self.standard_time_label.config(text=f"标准时间: {standard_datetime.split()[1]}")
            self.accelerated_time_label.config(text=f"加速时间: {custom_time}")
            self.hours_per_day_label.config(text=f"{expanded_hours_per_day:.2f}小时")
            self.acceleration_rate_label.config(text=f"{dilation_percentage:.0f}%")
            self.remaining_hours_label.config(text=f"{remaining_hours:.2f}小时")
            
            # 更新日期显示
            self.date_label.config(text=chinese_date)
            
            # 更新农历信息显示
            self.lunar_info_label.config(text=lunar_info)
            
        except Exception as e:
            print(f"更新时钟时出错: {e}")
            import traceback
            traceback.print_exc()
        
        # 100毫秒后再次更新
        self.root.after(100, self.update_clock)


def main_gui() -> None:
    """图形界面主函数"""
    root = tk.Tk()
    app = AcceleratedWorldGUI(root)
    root.mainloop()


if __name__ == "__main__":
    main_gui()
