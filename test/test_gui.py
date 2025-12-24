import tkinter as tk
from tkinter import ttk
import datetime

# 简单的测试GUI
root = tk.Tk()
root.title("测试GUI")
root.geometry("800x350")

# 创建主框架
main_frame = ttk.Frame(root, padding="20")
main_frame.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))

# 设置权重
root.columnconfigure(0, weight=1)
root.rowconfigure(0, weight=1)
main_frame.columnconfigure(0, weight=1)
main_frame.columnconfigure(1, weight=1)
main_frame.rowconfigure(0, weight=1)
main_frame.rowconfigure(1, weight=1)
main_frame.rowconfigure(2, weight=1)
main_frame.rowconfigure(3, weight=1)

# 时钟显示区域
clock_frame = ttk.LabelFrame(main_frame, text="实时时钟", padding="10")
clock_frame.grid(row=0, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=(0, 10))
clock_frame.columnconfigure(0, weight=1)
clock_frame.columnconfigure(1, weight=1)
clock_frame.rowconfigure(0, weight=1)

standard_time_label = ttk.Label(clock_frame, text="标准时间: 00:00:00", font=('Arial', 16))
standard_time_label.grid(row=0, column=0, sticky=tk.E, padx=(0, 20))

accelerated_time_label = ttk.Label(clock_frame, text="加速时间: 00:00:00", font=('Arial', 16))
accelerated_time_label.grid(row=0, column=1, sticky=tk.W)

# 更新函数
def update_time():
    now = datetime.datetime.now()
    standard_time = now.strftime("%H:%M:%S")
    accelerated_time = now.strftime("%H:%M:%S")
    
    standard_time_label.config(text=f"标准时间: {standard_time}")
    accelerated_time_label.config(text=f"加速时间: {accelerated_time}")
    
    root.after(1000, update_time)

# 启动更新
update_time()

# 启动主循环
root.mainloop()