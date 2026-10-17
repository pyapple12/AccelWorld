//! 自绘控件总装（Python 侧自绘控件的 Rust 对等物）
//! PL013/PL014 落地：胶囊滑杆/圆环/纸片（A005 #15：glass_card.rs 生产零调用已删）

pub mod chip;
pub mod ring;
pub mod slider;

// ===== src/widgets/mod.rs 说明区 =====
// 对位：slider.rs ← ui/glass_card.py CapsuleSlider（杆/橙槽/旋钮三层同心几何）；
//   ring.rs ← 圆弧进度类自绘；chip.rs ← 节日/城市纸片
// A005 #15：原 glass_card.rs（GlassCard 容器）生产零调用（页卡面统一经
//   window::register_page_surface 注册），按"不长期保留废弃方案"删除
// 自绘圆弧留边纪律延续：可见图形四周 ≥1px 透明边距（2026-09-12 定案）
