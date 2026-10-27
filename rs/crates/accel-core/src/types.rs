//! 跨层 DTO 类型（interface/types.py 的 Rust 对等物）
//! TimeInfo/WeatherData 随 PL011.10/.11 落地；命令/事件枚举与偏好聚合随 PL011.13 落地

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::alarm::{Alarm, AlarmDraft};

/// 时间信息快照（Python TimeInfo dataclass 镜像）
/// 时间域字段随 PL011.10 落地；chinese_date/lunar_info 农历域字段随 PL012 扩入
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeInfo {
    /// 标准日期时间字符串（"YYYY-MM-DD HH:MM:SS"，本地墙钟）
    pub standard_datetime: String,
    /// 自定义（加速）时间字符串（"HH:MM:SS"，小时可超 24，整数日长回绕）
    pub custom_time: String,
    /// 中文日期字符串（"YYYY年MM月DD日 星期X"，PL012 扩入，标准秒缓存）
    #[serde(default)]
    pub chinese_date: String,
    /// 农历展示文本（拼装契约见 chinese_calendar，PL012 扩入，标准秒缓存）
    #[serde(default)]
    pub lunar_info: String,
    /// 时间膨胀倍率百分比（rate × 100）
    pub dilation_percentage: f64,
    /// 膨胀后一天的小时数（与钟面回绕同口径的整数日长，浮点呈现）
    pub expanded_hours_per_day: f64,
    /// 加速后当天剩余小时数（整数日长口径，负值钳 0）
    pub remaining_hours: f64,
    /// 整数日长（int(24×rate)，钟面回绕模数）
    pub day_hours: i64,
    /// 刷新周期毫秒（min(clock_tick_ms, 1000/rate)，下钳 1）
    pub tick_ms: i64,
}

impl TimeInfo {
    /// 标准时间的 HH:MM:SS 部分（Python property 对等：显示用避免调用点 split）
    pub fn standard_time(&self) -> &str {
        self.standard_datetime.split(' ').nth(1).unwrap_or("")
    }

    /// 自定义时间小时数（Python property 对等：进度条用）
    pub fn custom_hour(&self) -> i64 {
        self.custom_time
            .split(':')
            .next()
            .and_then(|h| h.parse().ok())
            .unwrap_or(0)
    }
}

/// 逐小时点（PL014.11）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HourlyPoint {
    /// "YYYY-MM-DDTHH:MM"
    pub time: String,
    /// 温度（℃）
    pub temperature: f64,
    /// 降水概率（%）
    pub precip_prob: f64,
}

/// 七日概要点（PL014.11）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyPoint {
    /// "YYYY-MM-DD"
    pub date: String,
    /// 最高温（℃）
    pub t_max: f64,
    /// 最低温（℃）
    pub t_min: f64,
    /// 日出（"YYYY-MM-DDTHH:MM"）
    pub sunrise: String,
    /// 日落
    pub sunset: String,
}

/// 天气信息快照（Python WeatherData dataclass 镜像 + PL014.11 扩展）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeatherData {
    /// 温度（℃）
    pub temperature: f64,
    /// 相对湿度（%）
    pub humidity: f64,
    /// 风速（km/h）
    pub wind_speed: f64,
    /// 体感温度（℃）
    pub apparent_temperature: f64,
    /// WMO 天气代码（查表键；非整数标量按未知码归一，见 weather.rs 说明）
    pub weather_code: i64,
    /// 中文短名（如"晴"；未知码回退"未知"）
    pub weather: String,
    /// 中文完整描述
    pub description: String,
    /// emoji 图标
    pub icon: String,
    /// 逐小时 24 点（PL014.11）
    #[serde(default)]
    pub hourly: Vec<HourlyPoint>,
    /// 七日概要（PL014.11）
    #[serde(default)]
    pub daily: Vec<DailyPoint>,
}

/// UI 会话偏好快照（Python UiPreferences dataclass 镜像）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiPreferences {
    /// 主题偏好三态（auto=跟随系统/light/dark；非法值由接口归一化回退默认）
    pub theme: String,
    /// 上次选择的城市（缺省回退 default_city）
    pub last_city: String,
    /// 上次选择的时区（缺省回退 default_timezone）
    pub last_timezone: String,
    /// 上次设置的倒计时目标文本（未设置为空串）
    pub countdown_target: String,
}

/// 主题偏好枚举（SetTheme 命令负载；serde 小写形态与配置存储一致）
/// FIX011.14：存储串单源 as_str/parse——原 interface THEME_CHOICES/SetTheme
/// 映射/serde rename 三处字面量平行（serde 声明宏无法引用方法，靠本表锚定同步）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    Auto,
    Light,
    Dark,
}

impl ThemePref {
    /// 存储串单源（与 serde rename_all = "lowercase" 同形——改动两处必须同步）
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemePref::Auto => "auto",
            ThemePref::Light => "light",
            ThemePref::Dark => "dark",
        }
    }

    /// 存储串解析（as_str 逆映射；非法 None——装载归一化回退默认的依据）
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(ThemePref::Auto),
            "light" => Some(ThemePref::Light),
            "dark" => Some(ThemePref::Dark),
            _ => None,
        }
    }
}

// ===== 信号总线协议 v1（z.plan 修订二·C，全部变体一次定义）=====

/// UI → 后端（意图）：前端一切写操作的单一出口
#[derive(Debug, Clone)]
pub enum FrontendCommand {
    /// 实时生效（滑杆拖动，不落盘——对齐 set_rate 轻量路径）
    SetRate(f64),
    /// 内聚落盘（校验+重建+持久化——对齐 apply_rate）
    ApplyRate(f64),
    RefreshWeather {
        city: String,
        force: bool,
    },
    /// 切换天气城市（A005 #4：单城市模型下的城市选择——持久化 last_city）
    SetWeatherCity(String),
    AddAlarm(AlarmDraft),
    RemoveAlarm(String),
    ReplaceAlarm(Alarm),
    ToggleAlarm(String),
    AddCountdownTarget {
        name: String,
        at: DateTime<Local>,
    },
    RemoveCountdownTarget(String),
    AddWorldCity(String),
    RemoveWorldCity(String),
    SetBaseTimezone(String),
    SetTheme(ThemePref),
    /// 窗口几何存档（UIR：退出前保存 "x,y,w,h"，启动恢复——UIR.7 几何记忆）
    SetWindowGeometry {
        geometry: String,
    },
    /// 立即汇总落盘（退出前保存）
    SaveNow,
    Quit,
}

/// 后端 → UI（事实）：UI 无状态重建，一切以后端回推为准
#[derive(Debug, Clone)]
pub enum BackendEvent {
    /// 按倍率节奏推送的时间快照
    Tick(TimeInfo),
    WeatherUpdated {
        city: String,
        data: WeatherData,
    },
    WeatherFailed {
        city: String,
        reason: String,
    },
    AlarmTriggered(Alarm),
    /// 列表变更后回推全量（UI 无状态重建）
    AlarmsChanged(Vec<Alarm>),
    ConfigSaved(bool),
    RateChanged(f64),
    /// 倍率被拒（越界/非法）——A005 #19：失败也必须回推，UI 据此回退乐观更新
    RateRejected(f64),
}

// ===== src/types.rs 说明区 =====
// 输入：无（纯数据定义）；输出：跨层 DTO + 信号总线协议两枚举（全七域一次列全）
// 设计理由：Python 侧 types.py 是后端 DTO 的 re-export 转出层；Rust 侧本文件即
//   DTO 唯一定义处（避免 re-export 间接层）；serde 命名与 user_config.json schema
//   一致；FrontendCommand/BackendEvent 按 z.plan 修订二·C 逐变体定义——PL011
//   实现时钟/倍率/天气三域分发，闹钟/倒计时/世界城市域随 PL012/PL014 接线，
//   枚举形状定死即不再变动（协议 v1）；DateTime<Local> 负载可直接 serde 往返
// A005 #4 修订（2026-10-18）：删除从未接线的 AddWeatherCity/RemoveWeatherCity，
//   新增 SetWeatherCity（单城市模型的换城入口）；命令分发 match 已穷尽——新增
//   变体不接线即编译失败（协议演进由编译器强制）
// 协议 v1.1（2026-10-23，UIR.7）：FrontendCommand 扩 SetWindowGeometry（窗口
//   几何记忆契约，17 变体——「协议 v1 定死」自本条起为「演进须登记」口径）
// 关联依赖：serde/chrono/alarm（数据形状）；消费方：interface（分发）/ app bus
