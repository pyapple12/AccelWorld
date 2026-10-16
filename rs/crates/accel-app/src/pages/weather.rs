//! 天气页（ui/panels/weather_panel.py 的 Rust 对等物，PL014 增强）
//! 用户产品定案：多城市天气卡 + 美化呈现

// ===== src/pages/weather.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL014 进入
// 数据流：RefreshWeather 命令 → WeatherUpdated/WeatherFailed 事件回推；
//   city 坐标解析经 core::data::cities（零硬编码城市）
