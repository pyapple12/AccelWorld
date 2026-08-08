# AccelWorld 重构方案与进度（z.plan.md）

> 依据：2026-08-08 项目审计（对照 AGENTS.md 规范，参考 DeepTransHub 分层结构）
> 状态：**S1-S9 全部完成并发布（ver 0.45）**；S10 第三轮审计修复 P0-P4 已完成（ver 0.46），P5 待处理（见下文）
> 进度明细见 `x.progress.md`，本文件保留执行要点与历史方案记录

---

## 已完成 ✅（S1-S9 全部完成）

| 阶段            | 完成内容                                                                                                                                                   |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1 结构骨架     | 建立 `utils/ → config/ → modules/ → ui/ → data/` 五层包结构；`main.py` 收编入口与 VERSION 单一来源                                                         |
| S2 数据规范化   | 引入 `TimeInfo`/`LunarInfo`/`WeatherData`/`UserConfig` 四个 dataclass；天气表合并为 `WEATHER_CODE_INFO` 单表；配置走 file_utils 缓存单例 + base64 窗口几何 |
| S3 Bug 修复     | B1（启动参数生效）、B2（托盘退出保存设置）、B3（裸 except）；D1/D2/D9 及死代码清理                                                                         |
| S4 GUI 面板化   | 6 个面板（ui/panels/）+ `SystemTray` 独立类 + `AlarmEditDialog` 完善；signal/slot 解耦（rate_changed/theme_toggled/alarm_saved/alarm_triggered）           |
| S5 后台化       | 天气查询移入 QThreadPool + 30 分钟缓存 + 网络重试；闹钟预设铃声异步播放（`play_alarm_sound_async`）                                                        |
| S6 规范补齐     | 全文件函数 `#` 注释 + 末尾 `# =====` 说明区；类型注解补齐；行宽 ≤100；函数内 import 清零                                                                   |
| S8 审计修正     | 跨天闹钟去重键、配置反序列化容错、天气首次加载、一次性闹钟语义、低优先级 7 项、规范性 4 项、全量回归                                                       |
| S9 代码质量优化 | 异常上抛与窄捕获、死代码清理、效率优化（秒级缓存/轮询降频）、抽象合并、应用静态配置层（零硬编码）、pytest 41 用例测试引入                                  |
| 收尾            | workingboard M08 勾选、README/AGENTS.md 更新、版本升级 ver 0.45                                                                                            |

---

## 历史方案记录（已实施，供查阅）

以下为各轮审计/设计方案的摘要，均已实施完成；详细方案与过程见 git 提交历史：

### 第一轮审计修正方案（S8）

- 正确性问题 11 项全部修复：跨天闹钟去重键、配置反序列化容错、天气首次加载、一次性闹钟语义、rate 上限校验、倒计时恢复、年份边界降级、副本隔离、去重键清理等
- 规范性问题 6 项全部修复：import 顺序、# 注释补齐、陈旧注释清理、类型注解等
- 已排除项 4 项验证无问题：时间膨胀计算、列表刷新、base64 回退、缓存单例

### S7 测试引入（对应 M08c）——已实施（并入 S9.7）

- pytest 41 用例覆盖 time_dilation/chinese_calendar/settings/alarm_service/weather_service 五核心模块，全部通过
- 测试依赖 `tests/requirements-dev.txt`（与运行时依赖分离）

### 收尾事项

- workingboard M08 勾选、README 启动说明核对、版本升级 ver 0.45

### 第二轮代码质量审计方案（S9）

- 抽象合并 5 项（dataclass 序列化通用化/_trigger_key/TimeInfo 属性/gui_args 推导式/PresetSound 辅助）
- 效率优化 4 项（秒级缓存/CLI 轮询降频/世界时钟缓存/闹钟空转短路）
- 异常上抛 5 项（窄捕获+上抛/回调防护/CLI 兜底/播放日志化/单通道日志）
- 死代码 13 项清理、硬编码 8 类常量化
- 全部实施并验证（S9.1-S9.6）

### S9.5 应用静态配置层方案

- 定案要点：代码零硬编码、config/static 命名（static 前缀区分用户配置）、用户配置移入项目内、日志集中 logs/ 每日独立文件
- 设计构成：static_config.py 加载器（StaticConfig + get_static_config 单例）+ base.json 参数 + ui.json 字体颜色 + file_utils.get_project_root + 日志每日文件与过期清理
- 已实施并回归验证

---

## 第三轮审计发现（2026-08-08，S10 待修复）

> 依据：全量全文审计（33 源码文件 + 测试 + 静态配置，对照 AGENTS.md）
> 重点：无作用函数 / 函数存放位置 / 优化空间 / 遗漏小错误；A1/A2 已实测复现
> 修复进度见 `x.progress.md` S10 todo list

### A. 严重 Bug（2 项，已复现）

| #   | 问题                     | 位置                                                                           | 说明                                                                                                                                          |
| --- | ------------------------ | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| A1  | rate=1.0 边界崩溃        | `modules/time_dilation.py:68` 硬编码 `<= 1.0` 与 `base.json rate_min=1.0` 冲突 | 滑杆拖到最左 1.0 → `_update_acceleration_rate`（main_window.py:156）无 try 抛 ValueError；`--cli --rate 1.0` 报错退出；同时违反 S9.5 零硬编码 |
| A2  | 闹钟容错加载遇 null 崩溃 | `utils/dataclass_utils.py:19` 仅捕获 ValueError                                | 配置中 `"time": null` → `_validate_time(None)` 抛 TypeError 传播 → `from_dict_list`/`load_alarms` 崩溃，应用起不来                            |

### B. 功能缺失（1 项）

| #   | 问题                                                                                                                                             | 位置                                    |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------- |
| B1  | `last_city`/`last_timezone` 只存不读：save_settings 写入（main_window.py:241-242），启动从未恢复，每次重启丢失；weather_panel 恒显示硬编码"北京" | `ui/main_window.py:__init__` 缺恢复逻辑 |

### C. 死代码/无作用函数（4 项）

| #   | 函数                                                   | 判定                                                      |
| --- | ------------------------------------------------------ | --------------------------------------------------------- |
| C1  | `WeatherData.to_display()`（weather_service.py:48）    | 生产零调用，仅测试引用 → 删除或测试改 format_weather_info |
| C2  | `TimeInfo.standard_second` 属性（time_dilation.py:41） | 生产零调用（run_live_clock 用 now.second），仅测试引用    |
| C3  | `clear_weather_cache()`（weather_service.py:130）      | 仅测试 fixture 使用，保留（测试依赖）                     |
| C4  | `main.py:91` `run_cli = args.cli`                      | 一行转发变量，直接 `if args.cli:`                         |

**一行函数评估**：`Alarm.is_one_time()`/`current_city_name()`/`current_timezone()` 有真实调用点且语义化，保留；`get_custom_time`/`get_lunar_info` 内 4 个转发变量仅用一次，可内联

### D. 函数存放位置不当（2 项）

| #   | 问题                                                                                                                                             | 位置                               | 建议                                                            |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------- | --------------------------------------------------------------- |
| D1  | **utils 层反向依赖 config**：`logger.py:11` import `config.static.static_config`，违反 AGENTS.md"utils 无业务依赖"单向分层（utils 唯一反向依赖） | `utils/logger.py`                  | log_dir/backup_days 改由参数传入（main.py 调用处传值）          |
| D2  | **modules 层混入 UI 依赖**：`play_custom_sound`/`play_alarm_sound_async` 顶层依赖 PyQt6.QtMultimedia                                             | `modules/alarm_service.py:188-240` | 移至 `ui/audio_player.py`，service 层保留纯 winsound 参数化播放 |

### E. 遗漏小错误（15 项）

| #   | 问题                                                                                                    | 位置                                         |
| --- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| E1  | `_last_triggered` 注释 `# alarm_id -> "HH:MM"` 与实现 "YYYY-MM-DD HH:MM" 不符（S8 漏改注释）            | alarm_service.py:258                         |
| E2  | 手写枚举遍历定位与 `PresetSound.from_value(...).index()` 重复                                           | ui/alarm_dialog.py:80-83                     |
| E3  | `PresetSound.display_names()` 硬编码列表与枚举顺序强耦合                                                | alarm_service.py:36                          |
| E4  | `play_custom_sound` QMediaPlayer 函数返回后无引用，播放可能被 GC 中断                                   | alarm_service.py:200-205                     |
| E5  | `play_preset_sound` 内 `preset_config` dict 每次调用重建                                                | alarm_service.py:169                         |
| E6  | `_get_repeat_display` 内 days、`get_chinese_date` 内 weekday_map 每次调用重建                           | alarm_panel.py:229 / chinese_calendar.py:158 |
| E7  | `current_timezone` 回退 "Asia/Shanghai" 硬编码（S9.5 遗漏）                                             | world_clock_panel.py:93                      |
| E8  | weather_panel 两处硬编码 "北京"（S9.5 遗漏；修复 B1 后应读 last_city）                                  | weather_panel.py:69,85                       |
| E9  | `countdown_target_date = None` 无类型注解（S6 遗漏）                                                    | countdown_panel.py:36                        |
| E10 | `QTime.fromString` 的 try/except 无意义（不抛异常）                                                     | countdown_panel.py:258-263                   |
| E11 | `save_settings` 4 次 `set_setting` 各写一次盘                                                           | main_window.py:240-243                       |
| E12 | timezones 标注含夏令时误差：巴黎标 UTC+1（夏季+2）、纽约标 UTC-5（夏季-4）                              | data/timezones.py                            |
| E13 | 托盘通知 3 秒显示时长硬编码                                                                             | system_tray.py:105                           |
| E14 | countdown_hint_label 与 placeholder 重复文案；checkbox toolTip 与文本冗余                               | countdown_panel.py:82 / alarm_dialog.py:96   |
| E15 | `ui.json font_family: "Arial"` 中文回退；`weather_refresh_ms` 与 `weather_cache_ttl` 同值两处需手动同步 | ui.json / base.json                          |

### F. 效率细节

- `_on_rate_changed`/`apply_startup_args` 重复调 `tray.update_rate`（冗余无害）
- `apply_acceleration` round 后 `int(rate*10)` 截断（2.05 → 滑杆 2.0 显示不一致），应 `round(rate*10)`
- `main.py` epilog 与 docstring 重复；`date_panel` 初始占位写死示例日期

### 优先级建议

A1+A2（崩溃，必修）→ B1（功能缺失）→ D1/D2（分层）→ E 组（小错）→ C 组（死代码）；A1/A2/B1 修复量均 <10 行

---

## workingboard 未完成项分析（2026-08-08，归档前快照）

> 来源：workingboard/（已归档至 archived/workingboard）
> 结论：记录的问题 bug 已全部修复；以下为尚未实施的功能计划

### 一、尚未完成的功能项

#### 短期（F01）

| 计划项        | 内容                                                                                     | 关联 |
| ------------- | ---------------------------------------------------------------------------------------- | ---- |
| F01b02        | 进度条动画效果（QPropertyAnimation 平滑过渡，当前仅 setValue 跳变）                      | 无   |
| F01c03 / M09b | 托盘图标实时更新（setToolTip 显示时间/倍率；菜单倍率项已实时，toolTip 仍静态——部分完成） | P05b |

#### 中期（F02 / M09）

| 计划项        | 内容                                                | 关联 |
| ------------- | --------------------------------------------------- | ---- |
| M09c / F02a01 | 快捷键支持（Ctrl+S 保存、Ctrl+Q 退出、Ctrl+T 主题） | P05c |
| M09d / F02a02 | 多语言界面（中/英/日）                              | P05e |
| M09e / F02a04 | 加速倍率预设方案（工作/睡眠/专注模式）              | P05f |

#### 长期（F03）

| 计划项       | 内容                                                            |
| ------------ | --------------------------------------------------------------- |
| F03a01/02/03 | 打包可执行文件（PyInstaller）、安装程序（NSIS/Inno）、发布 PyPI |
| F03b01/02    | 文档站点（MkDocs/Sphinx）、CONTRIBUTING 贡献指南                |
| F03c01       | 自定义城市（GUI 手动添加非预设城市，当前仅启动参数支持列表外）  |
| F03c02/03/04 | 数据统计可视化、主题商店、云端配置同步                          |

#### 可选功能（M99）

位置服务、主题市场、插件系统、统计、云同步、移动端适配、语音播报、农历黄历——全部未完成（长期愿景）

### 二、已实现但文档过时（归档前状态未同步）

| 项                    | 实际状态                          | 原文档状态             |
| --------------------- | --------------------------------- | ---------------------- |
| M08c（单元测试）      | ✅ 已完成（S9.7，pytest 41 用例） | 标"待 S7"              |
| M09a（天气缓存）      | ✅ 已完成（S5，30 分钟缓存+重试） | 标"⏳ 待开发"          |
| P05d / F02a03（闹铃） | ✅ 已完成（M04b，ver 0.41）       | 状态字段过时           |
| P02-P05 全部问题      | ✅ 均已修复                       | 状态字段仍写"[待修复]" |

### 三、建议后续动作

1. 可立即实施的小项：M09c 快捷键（简单）、F01b02 进度条动画（简单）、M09e 倍率预设（简单）、F01c03 托盘 toolTip 实时化（简单）
2. 中大型规划（建议 OpenSpec 提案流程）：M09d 多语言（中）、F03a 打包发布（中）、F03c 自定义城市/统计（中）
3. 长期愿景（M99/F03b/F03c 其余）：暂缓，等待版本 1.0 规划
