# AccelWorld 项目分析报告

---

## 1. 项目概述

单包 Python 桌面应用：基于加速倍率的时间膨胀时钟（PyQt6，中文界面为主）。

```
[输入]   用户设定加速倍率（rate_min 1.0 - rate_max 20.0，来自 base.json）
[引擎]   AcceleratedWorld 按倍率换算：加速时钟 1 秒 = 现实 1/rate 秒
[展示]   GUI（6 面板 + 系统托盘，QTimer 100ms tick）/ CLI（--cli 轮询输出）
[周边]   农历干支节气 / 世界时钟 / 倒计时 / 天气（Open-Meteo）/ 闹钟 / 主题切换
[持久化] 用户配置 config/user_config.json（倍率/主题/城市/时区/倒计时/窗口几何/闹钟）
```

- **时间膨胀**：`AcceleratedWorld` 引擎，标准一天总秒数 × rate = 加速一天总秒数，整数运算拆分时分秒
- **中华元素**：lunar-python 农历/干支/生肖/节气 + 公历节日 + 拜财神方向
- **天气**：Open-Meteo 免费 API，18 城市经纬度表驱动，WMO 天气代码映射
- **闹钟**：多闹钟（上限 max_alarms）、预设/自定义铃声、系统通知、跨天去重键触发
- **双界面**：GUI 默认模式，`--cli` 命令行实时模式；`main.py` 收编分发与版本读取

## 2. 目录结构与模块职责

```
main.py                    # CLI/GUI 分发入口，--version 从 config/static/base.json 读取
modules/                   # 业务核心层（无 GUI 依赖，可独立测试）
  time_dilation.py         # TimeInfo dataclass + AcceleratedWorld 引擎 + CLI 实时时钟
  chinese_calendar.py      # LunarInfo dataclass + 农历/干支/生肖/节气/节日
  weather_service.py       # WeatherData dataclass + Open-Meteo 查询（TTL 缓存 + 重试）
  alarm_service.py         # PresetSound/Alarm + AlarmManager（去重键触发匹配）
config/
  settings.py              # 用户配置读写（UserConfig dataclass + load/save/get/set）
  user_config.json         # 用户配置文件（项目内，随项目走）
  static/                  # 应用静态配置层（零硬编码）
    config.json            # 引导映射表（分类名 → json 相对路径）
    base.json              # 应用参数（version/倍率范围/定时器周期/日志路径等）
    ui.json                # 字体/颜色常量
    static_config.py       # StaticConfig + get_static_config() 单例加载器
ui/                        # GUI 层（PyQt6）
  main_window.py           # 主窗口装配器（面板装配 + QTimer 调度 + 信号连接 + 主题/托盘）
  panels/                  # 6 个功能面板（时钟/日期/倒计时/世界时钟/天气/闹钟）
  system_tray.py           # 系统托盘（图标/菜单/通知）
  alarm_dialog.py          # 闹钟编辑对话框
  audio_player.py          # 闹钟音频播放（自定义铃声 + 异步分发，防 GC 中断）
  themes.py                # 浅色/深色主题 QSS 模板 + ui.json 颜色注入
data/                      # 静态数据表
  cities.py                # 城市经纬度表
  timezones.py             # 时区表（含夏令时标注）
  weather_codes.py         # WMO 天气代码映射表
utils/                     # 通用工具层（无业务依赖）
  logger.py                # 统一日志（每日独立文件 + 过期清理）
  file_utils.py            # JSON 读写 + 缓存单例 + 项目根定位
  dataclass_utils.py       # dataclass 反序列化通用工具（字段白名单过滤）
  retry.py                 # 泛型重试函数 retry_call
tests/                     # pytest 单元测试（44 用例，依赖 tests/requirements-dev.txt）
logs/                      # 运行日志（app-YYYY-MM-DD.log 每日独立文件）
```

架构一句话：依赖单向分层 `utils(L0) ← config(L1) ← modules(L2) ← ui(L3)`，`data/` 为纯数据表被各层引用；`main.py` 顶层分发，模块间顶层 import。

## 3. 核心设计模式

### 3.1 应用静态配置层 — 引导映射表 + 单例加载器

```python
STATIC_DIR = Path(__file__).resolve().parent  # 唯一结构约定：__file__ 自定位

@dataclass
class StaticConfig:
    base: Dict[str, Any]  # base.json 应用参数
    ui: Dict[str, Any]    # ui.json 字体/颜色

def get_static_config() -> StaticConfig:
    # 模块级 _static_config_cache 缓存懒加载，首次调用后不再读文件
```

- 引导链：`config.json`（映射表）→ 遍历读取分类 json → 聚合为 `StaticConfig`
- 文件缺失/损坏抛 `RuntimeError`（开发期快速暴露，不静默兜底）
- 全项目零硬编码：倍率范围/定时器周期/窗口几何/通知时长/日志路径等一律 `get_static_config().base["..."]`

### 3.2 用户配置 — dataclass + 缓存单例读写

```python
@dataclass
class UserConfig:
    time_dilation_rate: float = field(
        default_factory=lambda: get_static_config().base["default_rate"]
    )  # 默认值经 default_factory 从静态配置现取，不双处硬编码
```

- 读取走 `read_json_cached()`（仅成功解析才入缓存），写入 `write_json()` 后清缓存保证一致
- 反序列化委托 `dataclass_from_dict()` 通用工具：字段白名单过滤，缺省字段由 dataclass 默认值兜底
- 窗口几何 base64 编码存储；闹钟列表存 dict 结构

### 3.3 时间膨胀引擎 — 秒级缓存 + 整数运算

```python
class AcceleratedWorld:
    def __init__(self, time_dilation_rate: float | None = None):
        # None 哨兵避免默认参数定义时求值；下限读 base.rate_min（消除边界矛盾）
        # 低于 rate_min 抛 ValueError

    def get_custom_time(self) -> TimeInfo:
        # (年,月,日,时,分,秒) 作缓存键：同秒内直接返回缓存，
        # 避免 GUI 10Hz tick 重复农历全量计算
```

- 换算：当前时刻总秒数（含微秒精度）× rate = 加速总秒数，`//` 与 `%` 整数运算直接拆分时分秒，避免手动进位
- `TimeInfo` dataclass 携带标准/加速时间 + 中文日期 + 农历信息 + 膨胀百分比 + 进度条属性（`standard_time`/`custom_hour`/`custom_second`）
- 已知局限（y.problems#1）：缓存键按标准时间秒，刷新节奏与倍率无关——待修复

### 3.4 闹钟服务 — 去重键触发模型

- `Alarm` dataclass（时间/标签/铃声/启用/一次性语义）+ `PresetSound` 枚举
- `_trigger_key(check_time)` 生成去重键：同一时刻同一闹钟只触发一次，防轮询期内重复弹窗
- `AlarmManager` 持有闹钟列表，按 `alarm_check_ms`（1000ms）轮询匹配；跨天闹钟按去重键正确处理

### 3.5 天气服务 — TTL 缓存 + 泛型重试

```python
def get_weather_by_city(city_name: str) -> Optional[WeatherData]:
    # 城市名 → cities.py 经纬度表 → get_weather_by_coords()
    # TTL 缓存（weather_cache_ttl=1800s）内直接命中返回
```

- 网络请求经 `utils/retry.py` 的 `retry_call()` 泛型重试
- GUI 侧查询移入 `QThreadPool` 后台线程（S5 后台化），不阻塞主线程
- 失败返回 `None`，`format_weather_info()` 统一格式化展示文案

### 3.6 GUI 分层 — 面板化 + signal/slot 解耦

- `main_window.AcceleratedWorldGUI` 为纯装配器：面板装配 + `QTimer`（clock_tick_ms=100ms）调度 + 信号连接 + 主题/托盘
- 6 个面板独立类（`ui/panels/`），跨面板通信走信号：`rate_changed`/`theme_toggled`/`alarm_saved`/`alarm_triggered`
- `SystemTray` 独立类：图标/菜单/通知，关闭按钮最小化到托盘而非退出
- 主题：`themes.py` QSS 模板 + `_apply_colors()` 注入 `ui.json` 颜色常量（浅色/深色两套）
- 闹钟音频独立 `audio_player.py`：异步分发播放（防局部变量 GC 中断播放）

### 3.7 日志 — 每日独立文件 + 过期清理

- `_DailyFileHandler` 按 `logs/app-YYYY-MM-DD.log` 切分每日文件
- `_cleanup_old_logs()` 按 `log_backup_days`（7 天）清理过期日志
- `setup_logging()` 统一初始化，控制台 + 文件双通道

## 4. 代码风格观察

| 方面       | 实际情况                                                                       |
| ---------- | ------------------------------------------------------------------------------ |
| 命名风格   | 函数/变量 `snake_case`，类 `CamelCase`，常量 `UPPER_CASE`，`_` 前缀模块内私用   |
| 注释语言   | 中文，`#` 行注释体系；函数定义下方紧跟 1-3 行用途注释；禁止 docstring 当注释用 |
| 底部文档   | 统一在文件末尾 `# =====` 函数逻辑说明区（输入/输出/逻辑/设计理由/异常/关联配置） |
| 类型注解   | 函数参数与返回值注解齐全，`typing` 模块 + `| None` 语法                         |
| import     | 三段顺序（标准库 → 第三方 → 本地），顶层 import，禁止函数内延迟 import          |
| 配置格式   | JSON；静态配置 `config/static/*.json` + 单例加载器；用户配置 dataclass 序列化   |
| 路径处理   | 强制 pathlib                                                                     |
| 字符串     | f-string 优先，普通字符串双引号                                                  |
| 临时文件   | `.temp/`（已 gitignore）                                                         |
| 类型检查   | pyproject.toml 仅 basedpyright 配置，绝大多数检查显式放宽为 none                 |

## 5. 与 DeepTransHub 对比

| 维度         | AccelWorld                              | DeepTransHub                            |
| ------------ | --------------------------------------- | --------------------------------------- |
| 应用形态     | PyQt6 桌面 GUI（默认）+ CLI             | CLI 菜单 + subprocess 调度独立脚本      |
| 配置管理     | `config/static/` 静态层 + 用户配置 dataclass | `config/static/` 静态层 + key.json 独立 |
| 配置加载     | `get_static_config()` 单例              | `get_static_config()` 单例（同源设计）  |
| 上下文聚合   | `TimeInfo`/`LunarInfo`/`WeatherData`/`UserConfig` 四 dataclass | `_XxxContext` 四工具各自 dataclass      |
| 重试         | `utils/retry.py` 泛型 `retry_call()`    | `utils/error_handler.py` `retry_call()` |
| 并发模型     | QThreadPool 后台线程 + QTimer 轮询      | 串行 + Ctrl+Q 文件信号优雅退出          |
| 数据存储     | JSON 用户配置 + data/ 静态数据表        | Excel 状态机 + SQLite 词汇库            |
| 日志系统     | 每日独立文件 + 过期清理                 | 按天+序号 usage 日志 + 分析工具链       |
| 代码规范     | AGENTS.md 成文（# 注释体系/说明区）     | AGENTS.md 成文（同源，DeepTransHub 更早） |
| 工作流体系   | 2026-09-10 接入（本文档交接）           | w/x/y/z 四件套 + skills + 审计流程      |
