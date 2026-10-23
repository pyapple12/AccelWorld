//! 自绘控件总装（UIR 实心主题控件套件）
//! 等宽数字位/矢量图标/图标按钮/chip 纸片 + 沿用件（胶囊滑杆）
//! （A008 #7：switch.rs 已删——生产零调用，闹钟行卡内联同款绘制几何）

pub mod chip;
pub mod digits;
pub mod icon_button;
pub mod icons;
pub mod slider;

// ===== src/widgets/mod.rs 说明区 =====
// UIR 实心主题套件：digits.rs（等宽数字位——y.problems#12 根治）/icons.rs（矢量
//   图标库——零字体依赖）/icon_button.rs（图标按钮）/chip.rs（纸片徽标，纯矢量——
//   玻璃面注册已随 glass 模块删除）
// 沿用件：slider.rs（三层几何胶囊滑杆，纯函数在测）
// 已删：dock.rs（底部导航——被 window.rs 内联 sidebar 取代）/ring.rs（圆环——
//   时钟页改细条进度后零调用）/switch.rs（胶囊开关——生产零调用，A008 #7 删除；
//   闹钟行卡的开关为行内定位绘制，几何与其同构）
// 自绘圆弧留边纪律延续：可见图形四周 ≥1px 透明边距（2026-09-12 定案）
// 消费方：pages/*/window.rs
