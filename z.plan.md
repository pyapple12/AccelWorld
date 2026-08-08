# AccelWorld 项目审计与重构方案报告

> 审计日期：2026-08-08
> 审计范围：全部 7 个 Python 源文件（accelworld.py、accelworld_calc.py、accelworld_date.py、accelworld_config.py、accelworld_weather.py、accelworld_alarm.py、accelworld_gui.py，共约 4700 行）
> 参考基准：AGENTS.md 代码规范；DeepTransHub 项目结构（utils/ 基础设施 → modules/ 核心逻辑 → tools/ 业务层 + 数据外置 + main.py 壳）
> 状态：本报告仅为审计与方案，**未修改任何代码**

---

## 一、审计结论摘要

当前项目存在三类问题，按严重度排列：

1. **规范性缺失（全面）**：7 个文件**均未遵守** AGENTS.md 的两条核心注释规则（函数下方 `#` 注释、文件末尾 `# =====` 说明区），且存在 dataclass 未使用、pathlib 未使用、裸 `except:`、函数内重复 import、VERSION 双副本等问题。
2. **功能缺陷（2 个真实 Bug）**：`main_gui()` 忽略 `rate`/`city` 启动参数；设置项在托盘退出路径不保存。
3. **结构性问题（架构债）**：`accelworld_gui.py` 单文件 1425 行（职责混杂：主题、主窗口、托盘、闹钟对话框、6 个面板）；`get_custom_time()` 返回 7 元组、`get_chinese_lunar_calendar()` 返回 10 元组；配置无缓存、天气请求阻塞 GUI 线程。

项目本身规划了 M08 架构优化（GUI 重构、主题分离、单元测试、类型注解），本报告的方案可与其合并执行。

---

## 二、AGENTS.md 规范符合性审计

### 2.1 逐项审计表

| 规范要求                                       | 现状                                                                                                                                                                           | 判定                                                                                                                                                               |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 函数定义下方紧跟 `#` 注释（1-3 行）            | 全部文件**均未遵守**：现有注释是 docstring（`"""..."""`），部分函数/方法（如 gui.py 的 `update_weather`、`on_tray_activated` 等回调）连 docstring 也没有                       | ✗ 严重                                                                                                                                                             |
| 文件末尾 `# =====` 函数逻辑说明区              | **7 个文件全部缺失**                                                                                                                                                           | ✗ 严重                                                                                                                                                             |
| 配置聚合优先 `@dataclass`                      | 仅 `accelworld_alarm.py` 的 `Alarm` 符合；`accelworld_config.py` 的 `DEFAULT_CONFIG` 是裸 dict；`calc`/`date` 用超长元组返回值                                                 | ✗                                                                                                                                                                  |
| 强制 `pathlib` 替代 `os.path`                  | `accelworld_config.py` 大量使用 `os.path`（CONFIG_DIR/CONFIG_FILE/exists/join/makedirs）；gui.py `get_sound_display` 用 `os.path.basename`                                     | ✗                                                                                                                                                                  |
| 禁止裸 `except:`                               | `accelworld_gui.py:792`（`show_time_picker` 内）存在裸 `except:`                                                                                                               | ✗                                                                                                                                                                  |
| import 顺序（标准库→第三方→本地）              | 文件头部大体符合；但**函数体内重复导入**（gui.py 中 `import datetime/os/time/traceback` 共 10+ 处）违反组织规范                                                                | ✗                                                                                                                                                                  |
| `_` 前缀标记私有                               | 部分符合（`_update_acceleration_rate`）；但 `get_repeat_display`、`get_sound_display`、`update_clock` 等仅供内部使用的方法未加前缀                                             | ✗ 部分                                                                                                                                                             |
| `main()` + `if __name__ == "__main__": main()` | `accelworld.py` 符合；`calc.py` 的 `__main__` 块会直接启动 GUI（与 accelworld.py 入口职责重复）；config/weather/alarm 的 `__main__` 是测试代码块（含写配置、联网请求等副作用） | ✗ 部分                                                                                                                                                             |
| 类型注解（含 `                                 | None`）                                                                                                                                                                        | 顶层函数大多有；gui.py 的类方法普遍缺失（`closeEvent(a0)`、`AlarmEditDialog.__init__`、`update_countdown` 等）；gui.py 中 `List`/`Dict`/`Any` 来自 `typing` 而非 ` | ` 语法 | ✗ 部分 |
| 常量 `UPPER_CASE` + 提升为模块级               | 常量命名基本符合；但时区列表（gui.py:476-485）、节日翻译字典（date.py:113-116）定义在函数内，每次调用重复构建                                                                  | ✗                                                                                                                                                                  |
| 版本号单一来源                                 | `VERSION` 在 `accelworld_calc.py:10` 和 `accelworld_gui.py:19` 各有一份副本                                                                                                    | ✗                                                                                                                                                                  |
| 行宽 ≤100 字符                                 | 大体符合（少数 docstring 行长略超）                                                                                                                                            | ✓                                                                                                                                                                  |
| f-string 优先                                  | 符合                                                                                                                                                                           | ✓                                                                                                                                                                  |
| 布尔/空值判断（`if x:`、`is None`）            | 符合（`if args.rate:` 为 None 判断，应改为 `is not None` 以精确）                                                                                                              | ✓ 部分                                                                                                                                                             |

### 2.2 按文件统计

| 文件                  | 行数 | 函数数   | 缺 # 注释 | 缺说明区 | 其他主要问题                                                               |
| --------------------- | ---- | -------- | --------- | -------- | -------------------------------------------------------------------------- |
| accelworld.py         | 119  | 1        | 是        | 是       | CLI 传参用改 `sys.argv` 的 hack；`--cli` 判断重复                          |
| accelworld_calc.py    | 175  | 2+1 类   | 是        | 是       | `__main__` 块启动 GUI 职责重复；`start_time` 死属性；末尾重复 `import sys` |
| accelworld_date.py    | 154  | 3        | 是        | 是       | 返回 10 元组；翻译字典函数内构建                                           |
| accelworld_config.py  | 211  | 11       | 是        | 是       | 裸 dict 配置；os.path；latin1 编码 hack；无缓存；测试块写配置              |
| accelworld_weather.py | 203  | 4        | 是        | 是       | 两张重复键的天气表；`DEFAULT_CITY` 未使用；同步请求无重试；测试块联网      |
| accelworld_alarm.py   | 337  | 7+2 类   | 是        | 是       | 函数内 `import time`；winsound 阻塞主线程；`print` 代替日志；测试块        |
| accelworld_gui.py     | 1425 | 30+ 方法 | 是        | 是       | 见 3.1 缺陷清单                                                            |

---

## 三、缺陷与 Bug 清单

### 3.1 功能 Bug（影响用户）

| 编号 | 位置                          | 描述                                                                                                                                                                                                                     | 严重度 |
| ---- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------ |
| B1   | gui.py:1398-1421 `main_gui()` | **`rate`、`city` 启动参数被忽略**。`accelworld.py` 将 `--rate`、`--city` 传入 `main_gui(**gui_args)`，但 `main_gui` 只处理 `theme` 和 `hidden`，用户用 `--rate 3.0` 启动无效                                             | 高     |
| B2   | gui.py:1231-1239 `closeEvent` | **托盘退出路径不保存设置**。托盘菜单"退出"直接 `QApplication.quit()`，而 `save_settings()` 只在 `closeEvent` 且托盘不可见分支调用；常驻托盘场景下倍率/城市/时区/倒计时修改全部丢失（且 `on_slider_change` 时未触发保存） | 高     |
| B3   | gui.py:792 `show_time_picker` | 裸 `except:` 吞掉所有异常（含 `ValueError`），时间解析失败时静默回退                                                                                                                                                     | 中     |
| B4   | gui.py:457-459                | `self.countdown_time = QTimeEdit()` 创建后**从未加入布局**（死代码，M07 残留）                                                                                                                                           | 低     |

### 3.2 设计缺陷（非致命但应修复）

| 编号 | 位置                                | 描述                                                                                                                                          |
| ---- | ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| D1   | accelworld.py:96-98                 | CLI 模式通过改写 `sys.argv` 传参给 `main_cli`，应改为直接函数调用 `main_cli(rate=args.rate)`                                                  |
| D2   | accelworld_calc.py:167-175          | `if __name__ == "__main__"` 块直接启动 GUI，与 `accelworld.py` 入口职责重复；直接运行 calc 模块会弹窗口                                       |
| D3   | config.py:115-119                   | `save_window_geometry` 用 latin1 编解码绕行 `QByteArray`，脆弱且不可移植；建议直接存 QByteArray 的 hex/base64                                 |
| D4   | config.py:41-59 + 81-103            | `get_setting`/`set_setting` 每次调用都全量读文件（无缓存），且每次 `set_setting` 都全量写盘；高频调用浪费 IO                                  |
| D5   | weather.py:110-146                  | `get_weather_by_coords` 用 `urllib` 同步请求，GUI 在 `__init__` 和 30 分钟定时器中直接调用 → 网络慢时（10s 超时）**卡死界面**；无重试、无缓存 |
| D6   | gui.py:1001-1019 + alarm.py:121-148 | 闹钟触发时 `winsound.Beep` 循环 + `time.sleep` 在 GUI 主线程执行 → 播放期间界面冻结                                                           |
| D7   | calc.py:10 / gui.py:19              | `VERSION` 双副本，改版本易漏改导致不一致                                                                                                      |
| D8   | date.py:113-116                     | `holiday_translation` 字典每次调用重建                                                                                                        |
| D9   | weather.py:18-22                    | `DEFAULT_CITY` 常量定义了但从未使用                                                                                                           |
| D10  | gui.py:1412-1414                    | `window.is_dark_theme = False; window.toggle_theme()` 靠翻转实现暗色，逻辑绕且依赖内部状态                                                    |
| D11  | alarm.py:145-146                    | `import time` 在函数循环内                                                                                                                    |
| D12  | 日志系统                            | 各模块 `logging.getLogger(__name__)` 但**从未配置 handler**，`logger.error` 实际静默输出不到任何地方                                          |

### 3.3 规范遗漏（代码风格）

- 全部函数缺"函数定义下方 # 注释"，全部文件缺末尾 `# =====` 说明区（对照 AGENTS.md 函数注释规则）
- gui.py 方法缺类型注解（`closeEvent(a0)`、`get_sound_display(alarm)`、`trigger_alarm(alarm)` 等）
- gui.py:476-485 时区表应为模块级常量；get_repeat_display 内 `"".join([...])` 多余方括号
- config.py 未用 pathlib

---

## 四、结构重构方案（参考 DeepTransHub）

### 4.1 参考项目的架构模式

DeepTransHub 的架构核心（已在参考项目验证有效）：

```
依赖方向严格单向：业务层 → 核心层 → 工具层 → 标准库/第三方
utils/    通用基础设施（IO、配置缓存、错误重试、日志格式化）——不依赖业务
modules/  核心能力层（单一领域：LLM 交互）
tools/    业务流水线（各自独立 main() + if __name__）
+ 数据与代码分离（prompt/规则/对照表外置为 md/json 文件）
+ Context dataclass 聚合模式（每个业务域一个 dataclass + _load_xxx_context() 工厂）
+ 缓存单例（模块级 _json_cache 字典，避免重复 IO）
+ 泛型 retry_call（*args/**kwargs + 异常元组参数化，与业务解耦）
+ 列映射常量集中（columns.py 的 Col 类避免硬编码）
```

### 4.2 AccelWorld 目标结构

AccelWorld 是 GUI 桌面应用（非流水线），将 DeepTransHub 模式映射如下：

```
AccelWorld/
├── main.py                      # 入口（由 accelworld.py 演化）：CLI/GUI 分发 + 版本号 VERSION 单一来源
├── __init__.py                  # 包标记
│
├── modules/                     # 核心业务层（无 GUI 依赖，可独立测试）★ 对应 DeepTransHub modules/
│   ├── __init__.py
│   ├── time_dilation.py         # ← accelworld_calc.py：AcceleratedWorld 类 + 时间计算 + CLI 实时钟
│   ├── chinese_calendar.py      # ← accelworld_date.py：农历/干支/时辰/节气/节日
│   ├── weather_service.py       # ← accelworld_weather.py：天气获取（+缓存 +重试）
│   └── alarm_service.py         # ← accelworld_alarm.py：Alarm dataclass / AlarmManager / 播放
│
├── config/                      # 配置管理（独立域，参考 DeepTransHub config.json 分层）
│   ├── __init__.py
│   └── settings.py              # ← accelworld_config.py：AppConfig dataclass + pathlib + 缓存单例
│
├── ui/                          # GUI 层（由 accelworld_gui.py 1425 行拆分）
│   ├── __init__.py
│   ├── main_window.py           # 主窗口：装配各面板 + QTimer 调度 + 主题切换入口
│   ├── system_tray.py           # 托盘：图标绘制、菜单、通知（← gui.py 托盘相关方法）
│   ├── alarm_dialog.py          # AlarmEditDialog（← gui.py:1267-1395）
│   ├── themes.py                # LIGHT/DARK 主题 QSS（★ 对应 M08b 主题分离）
│   └── panels/                  # 子面板组件（每个面板一个类，signal 与主窗口解耦）
│       ├── __init__.py
│       ├── clock_panel.py       # 时钟显示 + 参数标签 + 进度条
│       ├── date_panel.py        # 中文日期 + 农历标签
│       ├── countdown_panel.py   # 倒计时输入 + 日期/时间选择器（← M07 成果收编）
│       ├── world_clock_panel.py # 时区下拉 + 世界时间
│       ├── weather_panel.py     # 城市选择 + 天气显示 + 主题按钮
│       └── alarm_panel.py       # 闹钟列表 + 增删改入口
│
├── data/                        # 静态数据（参考 DeepTransHub translate/ 数据与代码分离）
│   ├── __init__.py
│   ├── cities.py                # ← weather.py CITIES 城市经纬度表
│   ├── timezones.py             # ← gui.py 时区表（当前硬编码在 setup_ui）
│   └── weather_codes.py         # ← weather.py WEATHER_CODES/WEATHER_DESCRIPTIONS（合并去重）
│
├── utils/                       # 通用工具（★ 对应 DeepTransHub utils/）
│   ├── __init__.py
│   ├── logger.py                # 统一日志配置（修复 D12：配置 handler/格式/文件输出）
│   ├── file_utils.py            # pathlib 封装的 json 读写 + 缓存单例（← config.py 骨架）
│   └── retry.py                 # 泛型 retry_call（weather 网络重试复用，参考 DeepTransHub error_handler）
│
├── accelworld.py                # ← 兼容转发壳（可选保留：`from main import main; main()`，避免破坏现有启动习惯）
├── requirements.txt
├── pyproject.toml
└── (AGENTS.md / openspec/ / workingboard/ 保持不变)
```

### 4.3 结构设计原则（对齐 DeepTransHub）

1. **依赖单向无环**：`ui → modules/config`；`modules → utils`；`config → utils`；`data` 无依赖（纯常量）。消除现存的函数内延迟 import（循环导入的根源是单文件耦合，分层后可全部收敛到文件头）
2. **模块拆分粒度**：按"横切关注点"拆（与 DeepTransHub 一致），UI 按"面板"拆
3. **可测试性**：modules/config/data 不依赖 PyQt，可用 pytest 直接测试（★ 对应 M08c）；`main.py` 保持薄壳
4. **保留 CLI 兼容**：`python main.py --cli` 与 `python accelworld.py` 均可运行

---

## 五、模块拆分与通用抽象方案

### 5.1 通用工具抽象（新增 utils/）

| 工具            | 职责                                                                                                   | 来源/参考                                         | 核心签名                                                                  |
| --------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------- | ------------------------------------------------------------------------- |
| `logger.py`     | 统一日志：控制台+文件双 handler、统一格式、`get_logger(name)`                                          | 修复 D12                                          | `get_logger(name: str) -> logging.Logger`                                 |
| `file_utils.py` | pathlib 封装 JSON 读写 + **缓存单例**（模块级 `_json_cache`，与 DeepTransHub `load_config.py` 同模式） | 修复 D4；参考 DeepTransHub file_utils/load_config | `read_json(path) / write_json(path, data) / clear_cache()`                |
| `retry.py`      | 泛型重试装饰器/函数，异常元组参数化                                                                    | 参考 DeepTransHub error_handler.retry_call        | `retry_call(func, *args, retries=3, exceptions=(...), delay=1, **kwargs)` |

### 5.2 各模块拆分要点

- **time_dilation.py**：`AcceleratedWorld` 保留；`get_custom_time()` 改返回 `TimeInfo` dataclass（见 6.2）；CLI 实时钟 `run_live_clock()` 迁入并以 `main_cli(rate: float)` 直接接受参数（修复 D1）
- **chinese_calendar.py**：`get_chinese_lunar_calendar()` 改返回 `LunarInfo` dataclass；`holiday_translation` 提升为模块级常量（修复 D8）
- **weather_service.py**：新增 30 分钟内存缓存（★ 对应 M09a）；网络请求改走 `utils/retry.py`；保留 `get_weather_by_city/format_weather_info/get_simple_weather`
- **alarm_service.py**：`AlarmManager` 增加 `max_alarms` 可配置；播放函数迁出（供 ui 层在后台线程调用，修复 D6）；`print` 改日志
- **settings.py**：`AppConfig` dataclass + `load_config()/save_config()` 走 `utils/file_utils.py` 缓存；`save_window_geometry` 改 base64 存储（修复 D3）；删除 `get_config_dir/get_config_file` 两个无意义包装

---

## 六、数据结构规范化方案

### 6.1 统一原则

对齐 DeepTransHub 的"Context dataclass 聚合"模式：**所有跨函数传递的多值返回 → dataclass；所有配置聚合 → dataclass；所有查询表 → 模块级常量或 data/ 模块**。

### 6.2 新增 dataclass 清单

| dataclass                                   | 字段                                                                                                                                                                                        | 替代对象                                                                                     |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `TimeInfo`（modules/time_dilation.py）      | `standard_datetime: str`、`custom_time: str`、`chinese_date: str`、`lunar_info: str`、`dilation_percentage: float`、`expanded_hours_per_day: float`、`remaining_hours: float`               | `get_custom_time()` 的 7 元组返回                                                            |
| `LunarInfo`（modules/chinese_calendar.py）  | `lunar_year: str`、`shengxiao: str`、`lunar_month: str`、`lunar_day: str`、`shichen: str`、`yue_phase: str`、`jieqi: str`、`public_holiday: str`、`cai_shen_dir: str`、`position: str`      | `get_chinese_lunar_calendar()` 的 10 元组返回                                                |
| `WeatherData`（modules/weather_service.py） | `temperature: float`、`humidity: float`、`wind_speed: float`、`apparent_temperature: float`、`weather_code: int`、`weather: str`、`description: str`、`icon: str`；附带 `to_display()` 方法 | weather 的裸 dict 返回（消除 `weather['icon']` 魔法键）                                      |
| `AppConfig`（config/settings.py）           | `time_dilation_rate: float = 2.0`、`theme: str = "light"`、`last_city: str = "北京"`、`last_timezone: str = "Asia/Shanghai"`、`countdown_target: str = ""`、`window_geometry: str           | None = None`、`alarms: list[dict] = field(default_factory=list)`；含 `to_dict()/from_dict()` | `DEFAULT_CONFIG` 裸 dict |

### 6.3 常量与数据外置

| 数据                                                         | 迁往                                    | 说明                                                                                                                               |
| ------------------------------------------------------------ | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `CITIES`（weather.py）                                       | `data/cities.py` 的 `CITIES` 常量       | 含城市显示名                                                                                                                       |
| `WEATHER_CODES` + `WEATHER_DESCRIPTIONS`（weather.py）       | `data/weather_codes.py`                 | 两表键一致，合并为单一 `WEATHER_CODE_INFO: dict[int, WeatherCodeInfo]`（含中文/英文/图标/描述四字段），修复 M06 遗留的重复维护问题 |
| 时区表（gui.py:476-485）                                     | `data/timezones.py` 的 `TIMEZONES` 列表 | 消除函数内硬编码（修复 D8 同类问题）                                                                                               |
| `SHI_CHEN`/`CAI_SHEN_DIRECTION`/`CUSTOM_HOLIDAYS`（date.py） | 保留在 `chinese_calendar.py` 模块级     | 已是常量                                                                                                                           |
| 版本号                                                       | `main.py` 单一 `VERSION = "ver 0.44"`   | modules/ui 从 main 或 `__init__.py` 引用（修复 D7）                                                                                |

---

## 七、优化建议汇总

按优先级排列（P0 立即 / P1 重构期 / P2 后续）：

| 优先级 | 事项                                                                             | 关联    |
| ------ | -------------------------------------------------------------------------------- | ------- |
| P0     | 修复 B1：`main_gui` 应用 `rate`/`city` 参数（含恢复 `is_dark_theme` 初始化顺序） | Bug     |
| P0     | 修复 B2：托盘"退出"前调用 `save_settings()`；滑杆变化时同步保存                  | Bug     |
| P0     | 修复 B3：`show_time_picker` 裸 except 改为 `except (ValueError, TypeError)`      | 规范    |
| P1     | 按第四章结构拆分文件（gui 1425 行 → ui/ 包）                                     | 结构    |
| P1     | 按第六章引入 dataclass，消除 7/10 元组                                           | 结构    |
| P1     | 补齐函数下方 `#` 注释 + 文件末尾 `# =====` 说明区（全部文件）                    | 规范    |
| P1     | 配置缓存单例 + `set_setting` 批量保存                                            | D4      |
| P1     | 天气请求移入后台线程（`QThread`/`QThreadPool`）+ 30 分钟缓存                     | D5/M09a |
| P1     | `VERSION` 单一来源                                                               | D7      |
| P1     | 删除 `__main__` 测试块（config/weather/alarm），测试交给 pytest（M08c）          | 规范    |
| P2     | 闹钟播放移入线程，UI 不冻结                                                      | D6      |
| P2     | `save_window_geometry` 改 base64                                                 | D3      |
| P2     | 移除死代码（B4、D2、D9、`start_time` 属性）                                      | 清理    |
| P2     | 补充 gui 方法类型注解（基于 basedpyright 但不放宽现有 pyproject 配置）           | M08d    |
| P2     | `holiday_translation`、时区表提升为常量                                          | D8      |
| P2     | 统一日志输出（新增 utils/logger.py）                                             | D12     |

---

## 八、实施路线图（供审核）

> 本方案不包含实施代码，以下为建议的执行顺序。每阶段完成后可运行 AGENTS.md 中的导入验证命令确认无回归。

| 阶段          | 内容                                                                                                            | 验证方式                            | 对应里程碑 |
| ------------- | --------------------------------------------------------------------------------------------------------------- | ----------------------------------- | ---------- |
| S1 结构骨架   | 创建 `modules/config/ui/data/utils` 包结构；按 4.2 迁移文件（保持逻辑不变，仅移动+改 import）；main.py 收编入口 | 导入验证 + CLI 冒烟                 | M08 前置   |
| S2 数据规范化 | 引入 6.2 的 4 个 dataclass；数据表外置 data/；配置改 pathlib+缓存单例                                           | 导入验证 + CLI 冒烟                 | M08 前置   |
| S3 Bug 修复   | 修复 B1/B2/B3/B4、D1/D2/D7/D9 清理死代码                                                                        | 手动 GUI 验证（托盘退出后配置保留） | —          |
| S4 GUI 面板化 | 拆分 ui/panels/，主窗口只做装配；托盘/对话框独立文件                                                            | GUI 冒烟                            | M08a/M08b  |
| S5 后台化     | 天气/闹钟播放移入线程；天气缓存                                                                                 | GUI 冒烟（断网不卡 UI）             | M09a       |
| S6 规范补齐   | 全部函数 `#` 注释、文件末尾说明区、类型注解                                                                     | 人工 review                         | M08d       |
| S7 测试引入   | pytest 为核心模块（时间计算、农历、配置、闹钟逻辑）写单测                                                       | `pytest`                            | M08c       |

**注意**：S1 迁移会导致文件路径变更，`accelworld.py` 需保留为兼容转发壳或同步更新启动文档（README/AGENTS.md 中 `accelworld_calc.py` 的 VERSION 位置说明需一并更新）。

---

## 九、与现有规划的衔接

- 本方案 S4 与 workingboard M08a（GUI 重构）、M08b（主题分离）直接合并；建议 M08a 采用"面板拆分"而非 Qt Designer（.ui 文件对纯代码项目收益有限，且引入 uic 工具链成本高，可与 M08a 需求讨论）
- 本方案 S7 对应 M08c（pytest 单测）
- 本方案 P1 天气缓存对应 M09a
- 版本号建议：重构合并为 ver 0.44（M08）
