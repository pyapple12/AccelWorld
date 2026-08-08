# AccelWorld 重构方案与进度（z.plan.md）

> 依据：2026-08-08 项目审计（对照 AGENTS.md 规范，参考 DeepTransHub 分层结构）
> 状态：S1-S6 已完成并提交（ver 0.45 / M08 + 代码质量优化）；剩余 pytest 测试引入（已并入 S9.7 完成）
> 进度明细见 `x.progress.md`，本文件仅保留执行要点

---

## 已完成 ✅（S1-S6）

| 阶段          | 完成内容                                                                                                                                                  |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1 结构骨架   | 建立 `utils/ → config/ → modules/ → ui/ → data/` 五层包结构；`main.py` 收编入口与 VERSION 单一来源；`accelworld.py` 兼容转发壳                            |
| S2 数据规范化 | 引入 `TimeInfo`/`LunarInfo`/`WeatherData`/`AppConfig` 四个 dataclass；天气表合并为 `WEATHER_CODE_INFO` 单表；配置走 file_utils 缓存单例 + base64 窗口几何 |
| S3 Bug 修复   | B1（启动参数生效）、B2（托盘退出保存设置）、B3（裸 except）；D1/D2/D9 及死代码清理                                                                        |
| S4 GUI 面板化 | 6 个面板（ui/panels/）+ `SystemTray` 独立类 + `AlarmEditDialog` 完善；signal/slot 解耦（rate_changed/theme_toggled/alarm_saved/alarm_triggered）          |
| S5 后台化     | 天气查询移入 QThreadPool + 30 分钟缓存 + 网络重试；闹钟预设铃声异步播放（`play_alarm_sound_async`）                                                       |
| S6 规范补齐   | 全文件函数 `#` 注释 + 末尾 `# =====` 说明区；类型注解补齐；行宽 ≤100；函数内 import 清零                                                                  |

---

## 未完成 ⏳

### 审计修正方案（2026-08-08 全量审计）

> 审计范围：全部 24 个 Python 文件逐行通读 + 疑点运行验证（未修改代码）
> 完整问题清单与修复 todo 见 `x.progress.md` S8，以下为方案摘要

#### 正确性问题（11 项）

| 严重度 | 问题                                                                                                   | 位置                                 | 修复方案                                      |
| ------ | ------------------------------------------------------------------------------------------------------ | ------------------------------------ | --------------------------------------------- |
| 高     | 每日重复闹钟跨天不再触发：`_last_triggered` 只记 `"HH:MM"` 无日期，周二 07:00 误判为已触发（验证确认） | alarm_service.py `check_alarms`      | 去重键加入日期维度 `"YYYY-MM-DD HH:MM"`       |
| 中     | 配置反序列化零容错：非法 `time` 抛 ValueError、未知键抛 TypeError → GUI 启动即崩（验证确认）           | alarm_service.py `Alarm.from_dict`   | 非法条目跳过 + 未知键过滤；单条失败不阻断加载 |
| 中     | `update_alarm` 绕过 `__post_init__` 校验，注入非法 time 后 `check_alarms` 崩溃（验证确认）             | alarm_service.py `update_alarm`      | 复用 `_validate_time` 拒绝非法值              |
| 中     | 天气首次加载缺失：面板构造与主窗口装配后均不触发查询，启动显示"获取天气中..."（验证确认）              | weather_panel/main_window            | 装配后触发一次 `update_weather()`             |
| 中     | 一次性闹钟注释与实现矛盾：声称"仅创建当天触发"，实际每天时分匹配即触发（靠触发后禁用兜底）             | alarm_service.py `should_trigger_on` | 用 `created_at` 日期判断当天，与注释一致      |
| 低     | `--rate` 超 20 被 QSlider clamp 静默改为 20，无提示（验证确认）                                        | main.py + clock_panel                | main.py 增加上限校验报错退出                  |
| 低     | 倒计时 `countdown_target` 只存不读，启动不恢复                                                         | main_window/settings                 | 启动时加载填充输入框（仅显示）                |
| 低     | 农历年份边界：`get_holiday_detail` 仅支持 [2004, 2026]，异常年份每秒 tick 崩溃（验证确认）             | chinese_calendar.py                  | 捕获 `NotImplementedError` 降级跳过           |
| 低     | `"-c"` 判断不可达（argparse 未定义）                                                                   | main.py:87                           | 删除冗余判断                                  |
| 低     | `get_alarms()` 返回共享 list 引用，外部修改污染缓存                                                    | settings.py                          | 返回副本                                      |
| 低     | `_last_triggered` 跨天/删除后残留；`hide_to_tray` 未走 `show_notification` 封装                        | alarm_service/main_window            | 删除闹钟时清理键；统一封装                    |

#### 规范性问题（6 项）

| 问题                                                                 | 位置                                           | 修复方案               |
| -------------------------------------------------------------------- | ---------------------------------------------- | ---------------------- |
| import 顺序违规：本地 `from main import VERSION` 在第三方 PyQt6 之前 | main_window.py:10-14                           | 调整顺序               |
| S6.1.1 未完全执行：十余处函数只有 docstring 无 `#` 注释              | 各 panels/main_window/system_tray/alarm_dialog | 补齐                   |
| 8 个文件头"迁移自 accelworld_xxx.py，S1 结构骨架阶段"陈旧注释        | time_dilation/chinese_calendar/themes/data 等  | 清理                   |
| `main_gui(**kwargs)` 无 `kwargs: Any` 注解                           | main_window.py:247                             | 补齐                   |
| `AlarmPanel.to_dict_list() -> list` 元素类型可精确化                 | alarm_panel.py:74                              | `List[Dict[str, Any]]` |
| `winsound` 顶层导入未声明平台限制（Windows 目标，可接受）            | alarm_service.py:7                             | 保留，注释声明         |

#### 已排除项（验证无问题）

- 时间膨胀计算正确（含非整数倍率）；剩余小时取模冗余但无害
- 闹钟列表刷新无自动禁用死循环（`setChecked` 先于 `connect`）
- `load_window_geometry` base64 回退正确（`binascii.Error` 属 ValueError 子类）
- 配置缓存单例、天气缓存/重试、GUI 信号链、启动参数

### S7 测试引入（对应 M08c）

> 目标：pytest 覆盖核心模块（modules/config 不依赖 PyQt，可直接测试）

#### 环境准备

- [ ] 添加 pytest 依赖 + `tests/` 目录

#### 测试用例清单

| 模块               | 覆盖点                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------- |
| `time_dilation`    | 倍率校验（≤1.0 抛错）、自定义时间计算（如 2.0x→48 小时制）、24h 边界、`TimeInfo` 字段完整性 |
| `chinese_calendar` | `LunarInfo` 各字段（干支/生肖/时辰/节气/节日）、已知日期断言（如 2026-08-08 丙午年）        |
| `config.settings`  | 默认配置、读写往返、损坏 JSON 容错、缓存命中/清理                                           |
| `alarm_service`    | 重复规则匹配、一次性闹钟、同分钟触发去重、max_alarms 上限、`replace_alarm` 保留 ID          |
| `weather_service`  | 格式化空值容错、缓存命中/过期、重试耗尽返回 None（mock 网络层）                             |

#### 验证

- [ ] `pytest` 全部通过

### 收尾事项

- [x] 更新 `workingboard/2. Missions.md`：M08 各子项勾选（M08a/M08b/M08d 完成，M08c 随 S7），版本升级 ver 0.44
- [x] 核对 README 启动说明（`python main.py` / 新目录结构）

---

## 代码质量审计修正方案（第二轮，2026-08-08）

> 审计范围：全部代码文件逐行审阅（未修改代码）
> 维度：抽象合并 / 效率 / 异常上抛 / 无效代码 / 硬编码 / 默认值 / 防御性代码

### 一、可抽象成函数的代码（仅大幅消除重复者）

| #   | 重复模式                                                                                                                                                      | 位置                                             | 建议                                                                                                        | 预估省行                |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- | ----------------------- |
| 1   | dataclass 序列化逻辑重复：`to_dict`=asdict、`from_dict`=字段白名单过滤+构造，`AppConfig` 与 `Alarm` 两套几乎相同（唯一差异：Alarm 捕获 ValueError 返回 None） | settings.py:33-44、alarm_service.py:120-134      | 抽 `utils` 通用函数 `dataclass_to_dict(obj)` / `dataclass_from_dict(cls, data, tolerant=False)`，两处改调用 | ~15                     |
| 2   | 去重键格式化重复：`check_time.strftime("%Y-%m-%d %H:%M")` 两处一致（S8.1 曾因不一致埋 bug）                                                                   | alarm_service.py:320、342                        | 抽 `_trigger_key(check_time)` 私有函数共用                                                                  | ~3（防错位价值 > 行数） |
| 3   | TimeInfo 时间字段重复解析：`split()/split(":")` 在 CLI 与 GUI 多处                                                                                            | time_dilation.py:136-138、clock_panel.py:130/139 | 给 `TimeInfo` 加计算属性（`standard_second`/`custom_hour`）                                                 | ~6                      |
| 4   | gui_args 构建：4 个 `if x is not None`                                                                                                                        | main.py:102-110                                  | 推导式过滤 + hidden 单独处理                                                                                | ~5                      |
| 5   | PresetSound 枚举↔列表互转模式                                                                                                                                 | alarm_panel.py:224、alarm_dialog.py:136          | 枚举加 `display_name` property 与 `index` 方法                                                              | ~4                      |

**不建议抽取**：6 面板的 `QFrame+StyledPanel` 包装模式、20+ 处 `QLabel+setFont` 模式（各面板差异大，收益低侵入大）。

### 二、效率优化

| #   | 问题                                                                                                        | 位置                       | 建议                                                  |
| --- | ----------------------------------------------------------------------------------------------------------- | -------------------------- | ----------------------------------------------------- |
| 1   | 农历计算 10 次/秒重复：GUI tick 每 100ms 调 `get_custom_time()`（内含 lunar-python 全量计算），显示秒级才变 | time_dilation.py:53-116    | `get_custom_time` 加秒级缓存（同秒返回上次 TimeInfo） |
| 2   | CLI 实时钟 10ms 轮询 × 全量计算：先调 `get_custom_time` 再判秒变化                                          | time_dilation.py:130-158   | 循环内秒未变直接 sleep（配合 #1 缓存）                |
| 3   | 世界时钟 10Hz：每 tick `pytz.timezone()` + `now(tz)`                                                        | world_clock_panel.py:52-64 | 面板内缓存 tz 对象 + 秒级刷新                         |
| 4   | 闹钟检查空转：无闹钟时每秒建 datetime+遍历                                                                  | alarm_panel.py:152-157     | 前置 `if not self.alarm_manager.alarms: return`       |

### 三、异常上抛与 try/catch 建议（错误被隐藏点）

| #   | 问题                                                                                                     | 位置                     | 风险 | 建议                                                                                        |
| --- | -------------------------------------------------------------------------------------------------------- | ------------------------ | ---- | ------------------------------------------------------------------------------------------- |
| 1   | `except Exception` 吞编程错误（KeyError/TypeError 等）→ 返回 None → UI 只显示"获取失败"，真实 bug 被隐藏 | weather_service.py:98    | 高   | 窄捕获 `(URLError, TimeoutError, JSONDecodeError)`，其余上抛；失败日志改 `logger.exception` |
| 2   | `_on_weather_result` 无防护，格式化异常冒泡进 Qt 槽                                                      | weather_panel.py         | 中   | 包 try/except + `logger.exception`                                                          |
| 3   | CLI 无兜底：`run_live_clock` 只捕 KeyboardInterrupt，农历库异常直接崩 CLI                                | time_dilation.py:130     | 中   | 循环内 `except Exception` 记录后继续                                                        |
| 4   | 播放失败用 print 吞异常（GUI 中不可见、无堆栈）                                                          | alarm_service.py:145-193 | 中   | 改 `logger.exception`                                                                       |
| 5   | `update_clock` 双通道打印（logger.error + traceback.print_exc）                                          | main_window.py:130-132   | 低   | 统一 `logger.exception`                                                                     |

### 四、无效 / 不起作用的代码（建议删除，约省 60+ 行）

| #   | 代码                                                                         | 位置                     | 说明                                    |
| --- | ---------------------------------------------------------------------------- | ------------------------ | --------------------------------------- |
| 1   | `--minimized` 参数：与 `--hidden` 行为完全相同（都设 hidden）                | main.py:69-71、109-110   | 二选一保留                              |
| 2   | `self.seconds_per_day = 86400`：从未被读取（S3 删 start_time 时遗漏）        | time_dilation.py:48      | 死属性                                  |
| 3   | `import argparse`：main_cli 已改参数直传                                     | time_dilation.py:7       | 死 import                               |
| 4   | `load_config` import：从未使用（**init** 用 get_setting）                    | main_window.py:17-25     | 死 import                               |
| 5   | `get_simple_weather`                                                         | weather_service.py:154   | 死函数                                  |
| 6   | `AlarmManager.get_enabled_alarms`                                            | alarm_service.py:307     | 死函数                                  |
| 7   | `AlarmManager.mark_triggered`                                                | alarm_service.py:339     | 死函数                                  |
| 8   | `AlarmManager.update_alarm`                                                  | alarm_service.py:273     | 死函数（UI 走 replace_alarm）           |
| 9   | settings 配置层 CRUD 三件套 `add_alarm/remove_alarm/update_alarm`            | settings.py:165-208      | 死函数（与 AlarmManager 重复两套 CRUD） |
| 10  | `utils.logger.get_logger`                                                    | logger.py:47             | 死函数                                  |
| 11  | `utils.file_utils.read_file`                                                 | file_utils.py:46         | 死函数                                  |
| 12  | `retry_call` 的 `raise last_exc`：retries<1 时 raise None 崩溃（未防护路径） | retry.py:26              | 参数防护缺口                            |
| 13  | 剩余小时取模 `% total_custom_seconds_per_day`：rate≤20 恒等于自身            | time_dilation.py:102-105 | 纯冗余运算                              |

### 五、防御性 / 硬编码 / 默认值评估

#### 防御性代码

- ✅ 必要：`Alarm.from_dict`/`should_trigger_on` 容错、`load_window_geometry` latin1 兼容、`set_setting` hasattr、`PresetSound.from_value` 兜底、`UNKNOWN_WEATHER` 兜底
- ⚠️ 过度：`get_weather_by_coords` 与 `_on_weather_result` 的 `except Exception`（吞编程错误，见三-1/2，改窄捕获+上抛）
- ⚠️ 冗余：`check_alarms` 的 `if not alarm.enabled: continue` 与 `should_trigger_on` 内 enabled 检查双重判断（保留 manager 短路即可）

#### 硬编码（建议常量化）

| #   | 硬编码                                                                                            | 位置                         |
| --- | ------------------------------------------------------------------------------------------------- | ---------------------------- |
| 1   | 字体 `"Arial"` + 尺寸 20+ 处                                                                      | 全部 panels + main_window    |
| 2   | 颜色 `#4CAF50`（≥4 处）、`#888888`/`#666666`/`#555555`/`#2196F3`/`#f44336`——与 themes.py QSS 重复 | 各 panels                    |
| 3   | 倍率范围 1.0/20.0 三处重复（main.py 校验 / `_update_acceleration_rate` / clock_panel）            | 三文件                       |
| 4   | 默认城市 `"北京"` 两处                                                                            | settings + weather_panel     |
| 5   | 默认时区 `"Asia/Shanghai"` 两处                                                                   | settings + world_clock_panel |
| 6   | `max_alarms = 10`                                                                                 | alarm_service.py:231         |
| 7   | 默认窗口 `900x500`                                                                                | main_window.py:57            |
| 8   | 定时器周期 100ms / 1000ms / 30×60×1000                                                            | 三处                         |

#### 默认值

- 合理（保留）：`rate=2.0`、Alarm 字段默认、`retry_call(retries=3, delay=1.0)`、`setup_logging(INFO)`、`format_weather_info(city_name="")`
- 冗余：`main_cli(rate: float = 2.0)`——main.py 总显式传参，双默认语义（不影响正确性，注明即可）

### 六、优先级建议

1. **P0**：三-1/2（吞异常改窄捕获+上抛）；四-1 死代码清理（`--minimized`、`seconds_per_day`、两个死 import、9 个死函数）
2. **P1**：二-1/2/3（秒级缓存、CLI 轮询降频、世界时钟缓存）；一-1/2/3（序列化抽象、去重键 helper、TimeInfo 计算属性）
3. **P2**：三-3/4/5（CLI 兜底、播放日志化）；五-硬编码常量化；一-4/5 小优化

---

## S9.5 应用静态配置层方案（定案 2026-08-08）

> 讨论定案要点：①配置架构保持两个 py 分别管理（settings.py 用户配置 + static 静态配置）②代码侧零硬编码（默认值与常量全部读 json）③用户配置从 `~/.config/accelworld` 修正到项目目录 ④日志集中项目内 `logs/`、每天独立文件、保留天数参数化并驱动过期清理

### 一、目标结构

```
AccelWorld/
├── main.py
├── config/
│   ├── settings.py            # 用户配置（UserConfig dataclass，可读写）——文件位置不变
│   ├── user_config.json       # 生成：用户运行时配置（修正原 ~/.config/accelworld 决策）
│   ├── static/                # ★ 应用静态配置包（命名定案：候选 C "static"，强调只读）
│   │   ├── __init__.py
│   │   ├── config.json        # 引导映射表（唯一硬编码：static_config.py 内 __file__ 自定位）
│   │   ├── base.json          # 应用参数（倍率/默认值/周期/窗口/日志路径/用户配置路径）
│   │   ├── ui.json            # UI 参数（字体/颜色）
│   │   └── static_config.py   # StaticConfig dataclass + get_config() 缓存单例
│   └── logs/                  # 生成：app-YYYY-MM-DD.log（每天独立文件）
└── utils/
    └── file_utils.py          # + get_project_root()（借鉴 appdotwriter，校验 main.py）
```

### 二、base.json（应用参数）

```json
{
  "rate_min": 1.0,
  "rate_max": 20.0,
  "default_rate": 2.0,
  "default_theme": "light",
  "default_city": "北京",
  "default_timezone": "Asia/Shanghai",
  "max_alarms": 10,
  "window_x": 100,
  "window_y": 100,
  "window_width": 900,
  "window_height": 500,
  "clock_tick_ms": 100,
  "alarm_check_ms": 1000,
  "weather_refresh_ms": 1800000,
  "weather_cache_ttl": 1800,
  "user_config": "config/user_config.json",
  "logs_dir": "logs",
  "log_backup_days": 7
}
```

> 注：`default_rate`/`default_theme`/`default_city`/`default_timezone` 是 **UserConfig（用户配置）的默认值来源**——用户从未设置时的起点，属于应用参数故放 base.json；`user_config.json` 是运行时生成的用户数据（用户改过的键优先），缺失键由这些默认值兜底。

### 三、ui.json（UI 参数）

```json
{
  "font_family": "Arial",
  "colors": {
    "primary": "#4CAF50",
    "primary_hover": "#45a049",
    "primary_pressed": "#3d8b40",
    "text_secondary": "#888888",
    "text_tertiary": "#666666",
    "text_muted": "#555555",
    "accent": "#2196F3",
    "danger": "#f44336"
  }
}
```

### 四、static_config.py（加载器）

```python
# 应用静态配置加载
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict

from utils.file_utils import read_json

# 引导路径：static_config.py 所在目录（唯一结构约定，__file__ 自定位）
STATIC_DIR = Path(__file__).resolve().parent


@dataclass
class StaticConfig:
    """应用静态配置聚合（base/ui 各为 dict，只读，供全项目引用）"""
    base: Dict[str, Any]
    ui: Dict[str, Any]


def _load_static_config() -> StaticConfig:
    # 私有加载：读引导映射表 → 遍历读取各分类 json → 聚合返回；文件缺失/损坏抛错暴露
    mapping = read_json(STATIC_DIR / "config.json", default={})
    result: Dict[str, Dict[str, Any]] = {}
    for key, rel_path in mapping.items():
        data = read_json(STATIC_DIR / rel_path, default=None)
        if data is None:
            raise RuntimeError(f"静态配置文件缺失或损坏: {rel_path}")
        result[key] = data
    return StaticConfig(base=result["base"], ui=result["ui"])


_static_config_cache: StaticConfig | None = None


def get_static_config() -> StaticConfig:
    # 公开单例访问：缓存懒加载，首次调用后不再读文件
    global _static_config_cache
    if _static_config_cache is None:
        _static_config_cache = _load_static_config()
    return _static_config_cache
```

> 命名说明：统一 `static` 前缀（`get_static_config`/`_load_static_config`/`_static_config_cache`），与用户配置的 `load_config`/`save_config`/`get_setting` 明确区分，调用处零歧义（不沿用参考项目泛化的 `get_config()`）。

### 五、utils/file_utils.py 新增 get_project_root()

```python
# 项目根：utils/file_utils.py → utils/ → 项目根
_PROJECT_ROOT = Path(__file__).resolve().parent.parent


def get_project_root() -> Path:
    # 获取项目根目录并校验 main.py 存在（防止目录层级偏移）
    if not (_PROJECT_ROOT / "main.py").exists():
        raise RuntimeError(f"项目根目录检测失败：{_PROJECT_ROOT} 下缺少 main.py")
    return _PROJECT_ROOT
```

### 六、settings.py 改造（用户配置）

- `CONFIG_FILE = get_project_root() / get_static_config().base["user_config"]`（修正原 `~/.config/accelworld` 决策；旧配置不迁移，新位置自然生成）
- UserConfig 默认值**零硬编码**：`time_dilation_rate`/`theme`/`last_city`/`last_timezone` 用 `field(default_factory=lambda: get_static_config().base[...])`——每次构造实例时现取默认值（default_factory 机制：代码无字面量、不受 import 顺序影响、改 json 即改默认）
- `countdown_target`（空串）/`window_geometry`（None）/`alarms`（空列表）保持结构语义默认（"用户未设置"的天然兜底），不 json 化
- 加载优先级：`user_config.json` 已有键以文件为准，缺失键由 dataclass 默认值（base.json）兜底

### 七、logger.py 改造（日志路径 2：每天独立文件）

- 日志目录/保留天数从 static 读取：`logs_dir`/`log_backup_days`
- 每天独立文件 `logs/app-YYYY-MM-DD.log`：写日志前检查日期，跨天切换 handler（自定义 DailyFileHandler 或惰性切换）
- 过期清理：启动时 + 每日切换时扫描 `logs/`，按文件名日期戳删除超过 `log_backup_days` 天的 `app-*.log`（约 10 行）
- 原 `~/.config/accelworld/app.log` 与配置一同迁入项目内

### 八、改造点对照（零硬编码全覆盖）

| 位置                                                | 现硬编码                                               | 改为                                                           |
| --------------------------------------------------- | ------------------------------------------------------ | -------------------------------------------------------------- |
| main.py                                             | rate 1.0/20.0 校验                                     | rate_min/rate_max                                              |
| main_window.py                                      | 倍率范围、`setGeometry(100,100,900,500)`、QTimer 100ms | rate_min/max、window_x/y/width/height、clock_tick_ms           |
| clock_panel.py                                      | slider 10/200、1.0-20.0 校验、"Arial"、颜色            | rate_min/max（×10 换算）、font_family、colors                  |
| alarm_panel.py                                      | QTimer 1000ms、"Arial"、颜色                           | alarm_check_ms、font_family、colors                            |
| weather_panel.py                                    | 30min 定时器、"Arial"、颜色                            | weather_refresh_ms、font_family、colors                        |
| weather_service.py                                  | CACHE_TTL_SECONDS=1800                                 | weather_cache_ttl                                              |
| alarm_service.py                                    | max_alarms=10、两处 print                              | max_alarms、logger.warning（+面板弹窗第二层）                  |
| date/countdown/world_clock/system_tray/alarm_dialog | "Arial"、颜色                                          | font_family、colors                                            |
| themes.py                                           | QSS 内写死颜色                                         | 从 colors 生成（生成式，保持可读，非 f-string 全转义）         |
| settings.py                                         | `~/.config/accelworld`、字面默认值                     | get_project_root + user_config 路径、default_factory 读 static |
| logger.py                                           | `~/.config/accelworld/app.log`                         | logs_dir + 每日文件 + 清理                                     |

### 九、风险与控制

1. **引导路径**：static/config.json 位置由 `__file__` 自定位（唯一结构约定，非业务参数）
2. **配置缺失**：映射/文件缺失抛 RuntimeError（开发期快速暴露，不静默兜底）
3. **性能**：get_static_config() 缓存单例，import 期一次读取
4. **循环依赖**：static_config.py 仅依赖 utils.file_utils，无环
5. **命名歧义**：静态配置统一 `static` 前缀命名，与用户配置（load_config/save_config/get_setting）明确区分
6. **回归重点**：主题渲染、字体、窗口恢复、日志每日切换与过期清理、旧配置丢弃后的首启行为
7. **文档同步**：AGENTS.md（结构/配置/日志路径）、README（Q2 配置路径）、x.progress.md
