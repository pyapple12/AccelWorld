# AccelWorld 重构方案与进度（z.plan.md）

> 依据：2026-08-08 项目审计（对照 AGENTS.md 规范，参考 DeepTransHub 分层结构）
> 状态：S1-S6 已完成并提交（ver 0.44 / M08）；剩余 S7 测试引入
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

- [ ] 更新 `workingboard/2. Missions.md`：M08 各子项勾选（M08a/M08b/M08c/M08d）
- [ ] 核对 README 启动说明（`python main.py` / 新目录结构）
