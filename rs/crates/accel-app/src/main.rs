//! AccelWorld RS 应用入口（Python main.py 的 Rust 对等物）
//! PL011 占位：仅冒烟输出；PL013 起装载 AppHandle/eframe 运行时

mod bus;
mod glass;
mod nav;
mod pages;
mod platform;
mod theme;
mod tools;
mod widgets;
mod window;

fn main() {
    // PL013 落地顺序：tracing 初始化（级别读 base.json token）→ 构造 AppInterface
    // → tokio runtime spawn 后台 → eframe::run_native 装载 AppHandle
    // 占位期输出一行冒烟标记，保证 cargo run 可验证
    println!("accel-app 占位（PL011 工程骨架；PL013 装载 egui 外壳）");
}

// ===== src/main.rs 说明区 =====
// 模块声明一次列全（结构镜像铁律：禁止大文件后拆，z.plan 修订一）
// main()：PL011 仅冒烟；版本号单一来源 base.json，代码中不得出现版本字符串
// 模块对位：window ← ui/main_window.py；bus ← 信号总线（新增，前后端解耦通道）；
//   theme ← qss/token 体系；nav ← ui/panels/glass_nav.py；
//   glass ← ui/gl/；pages ← ui/panels/；widgets ← 自绘控件；
//   tools ← ui/tools/；platform ← backdrop/system_tray/notify/hotkey
