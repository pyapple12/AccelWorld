//! 图标库（PL019）：Tabler Icons 3.50 子集字体——24 网格、2px 描线。
//! 字形来自 `rs/assets/fonts/TablerIcons-sub.ttf`（MIT，子集命令见 fonts/README.md）；
//! 调用点保持 `draw(painter, center, size, icon, color)` 签名不变，内部改字体绘制。

use egui::{Color32, FontFamily, FontId, Painter, Pos2};

/// 子集字体（29 个字形，约 9KB；fonttools 子集，见 `rs/assets/fonts/README.md`）
pub const ICON_FONT: &[u8] = include_bytes!("../../../../assets/fonts/TablerIcons-sub.ttf");

/// 图标集（导航五页 + 通用操作 + 天气/昼夜 + 表单；29 变体）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// 时钟（时钟页）
    Clock,
    /// 沙漏（倒计时）
    Hourglass,
    /// 地球（世界时钟）
    Globe,
    /// 云（天气：阴/默认）
    Cloud,
    /// 雨
    Rain,
    /// 雪
    Snow,
    /// 雾
    Fog,
    /// 雷暴
    Storm,
    /// 铃铛（闹钟）
    Bell,
    /// 齿轮（设置）
    Gear,
    /// 加号
    Plus,
    /// 减号
    Minus,
    /// 垃圾桶（删除）
    Trash,
    /// 太阳（昼）
    Sun,
    /// 月牙（夜）
    Moon,
    /// 刷新
    Refresh,
    /// 对勾（确认）
    Check,
    /// 关闭
    Close,
    /// 定位/基准
    Pin,
    /// 风
    Wind,
    /// 下拉箭头（Select）
    ChevronDown,
    /// 左箭头
    ChevronLeft,
    /// 右箭头
    ChevronRight,
    /// 日历
    Calendar,
    /// 搜索
    Search,
    /// 水滴（湿度）
    Droplet,
    /// 温度计（体感）
    Thermometer,
    /// 音量
    Volume,
    /// 静音
    VolumeOff,
}

impl Icon {
    /// 全部变体（冒烟/覆盖率测试；新增变体必须同步本数组）
    pub const ALL: [Icon; 29] = [
        Icon::Clock,
        Icon::Hourglass,
        Icon::Globe,
        Icon::Cloud,
        Icon::Rain,
        Icon::Snow,
        Icon::Fog,
        Icon::Storm,
        Icon::Bell,
        Icon::Gear,
        Icon::Plus,
        Icon::Minus,
        Icon::Trash,
        Icon::Sun,
        Icon::Moon,
        Icon::Refresh,
        Icon::Check,
        Icon::Close,
        Icon::Pin,
        Icon::Wind,
        Icon::ChevronDown,
        Icon::ChevronLeft,
        Icon::ChevronRight,
        Icon::Calendar,
        Icon::Search,
        Icon::Droplet,
        Icon::Thermometer,
        Icon::Volume,
        Icon::VolumeOff,
    ];
}

/// 码点表（Tabler Icons 3.50 `dist/tabler-icons.css`；改名/升级时同步）
pub fn glyph(icon: Icon) -> char {
    match icon {
        Icon::Clock => '\u{ea70}',
        Icon::Hourglass => '\u{ef93}',
        Icon::Globe => '\u{eb54}',
        Icon::Cloud => '\u{ea76}',
        Icon::Rain => '\u{ea72}',
        Icon::Snow => '\u{ea73}',
        Icon::Fog => '\u{ecd9}',
        Icon::Storm => '\u{ea74}',
        Icon::Bell => '\u{ea35}',
        Icon::Gear => '\u{eb20}',
        Icon::Plus => '\u{eb0b}',
        Icon::Minus => '\u{eaf2}',
        Icon::Trash => '\u{eb41}',
        Icon::Sun => '\u{eb30}',
        Icon::Moon => '\u{eaf8}',
        Icon::Refresh => '\u{eb13}',
        Icon::Check => '\u{ea5e}',
        Icon::Close => '\u{eb55}',
        Icon::Pin => '\u{ec9c}',
        Icon::Wind => '\u{ec34}',
        Icon::ChevronDown => '\u{ea5f}',
        Icon::ChevronLeft => '\u{ea60}',
        Icon::ChevronRight => '\u{ea61}',
        Icon::Calendar => '\u{ea53}',
        Icon::Search => '\u{eb1c}',
        Icon::Droplet => '\u{ea97}',
        Icon::Thermometer => '\u{ef67}',
        Icon::Volume => '\u{eb51}',
        Icon::VolumeOff => '\u{f1c3}',
    }
}

/// 绘制图标：Tabler 字形在 24 网格内留 2px 边距，字号 ×1.2 让视觉框≈size；
/// 颜色由调用方给语义色（不在此处做状态判断）
pub fn draw(painter: &Painter, center: Pos2, size: f32, icon: Icon, color: Color32) {
    let font = FontId::new(size * 1.3, FontFamily::Name("icons".into()));
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        glyph(icon),
        font,
        color,
    );
}

// ===== src/widgets/icons.rs 说明区 =====
// 输入：Painter + 中心 + 视觉尺寸 + Icon + 颜色；输出：单字形绘制
// 逻辑步骤：glyph(icon) 查码点 → FontId(size×1.2, "icons") → CENTER_CENTER 绘制
// 设计理由（PL019）：手绘 567 行只覆盖 20 个图标且扩编成本高；Tabler 子集
//   9KB 提供 29 个 24 网格字形，调用点零改动，线宽/圆角由字体保证一致
// 异常处理：字体未注册时 egui 回退默认族（表现为缺字），由启动期注册 + 测试兜底
// 关联依赖：assets/fonts/TablerIcons-sub.ttf（MIT）；消费方：dock/page_header/各页

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_unique_glyph() {
        // PL019：枚举全覆盖 + 码点互不重复（新增变体漏码点会在此失败）
        let mut seen = std::collections::HashSet::new();
        for icon in Icon::ALL {
            let ch = glyph(icon);
            assert_ne!(ch, '\0', "{icon:?} 缺少码点");
            assert!(seen.insert(ch), "{icon:?} 与已有图标码点重复");
        }
        assert_eq!(seen.len(), Icon::ALL.len());
    }

    #[test]
    fn all_icons_draw_without_panic() {
        // 全图标冒烟：注册子集字体后任意尺寸/位置不 panic
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "icons".into(),
            egui::FontData::from_static(ICON_FONT).into(),
        );
        fonts
            .families
            .insert(FontFamily::Name("icons".into()), vec!["icons".into()]);
        ctx.set_fonts(fonts);
        // 字体在首帧后可用（Painter::text 需要已加载的字体图集）
        let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
        out.textures_delta.clear();
        let painter = egui::Painter::new(
            ctx,
            egui::LayerId::background(),
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0)),
        );
        for icon in Icon::ALL {
            for size in [16.0, 20.0, 24.0, 32.0] {
                draw(&painter, egui::pos2(50.0, 50.0), size, icon, Color32::WHITE);
            }
            draw(&painter, egui::pos2(0.0, 0.0), 4.0, icon, Color32::GRAY);
        }
    }
}
