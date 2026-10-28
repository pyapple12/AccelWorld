//! 自绘控件总装（PL017：实心主题控件套件）
//! digits/icons/icon_button/dock/page_header/field/switch/empty/rate_strip/slider/chip/toast

pub mod chip;
pub mod digits;
pub mod dock;
pub mod empty;
pub mod field;
pub mod icon_button;
pub mod icons;
pub mod motion;
pub mod page_header;
pub mod primary_button;
pub mod rate_strip;
pub mod select;
pub mod slider;
pub mod spinner;
pub mod switch;
pub mod toast;

// ===== src/widgets/mod.rs 说明区 =====
// PL017 套件：dock（悬浮带标签导航）/page_header（抽屉页头）/field（label+输入+焦点环）/
//   switch（语义绿开关）/empty（空态）/rate_strip（360px 短倍率条）；沿用 digits/icons/
//   icon_button/chip/slider/toast
// 自绘圆弧留边纪律延续：可见图形四周 ≥1px 透明边距（2026-09-12 定案）
// 消费方：pages/*/window.rs
