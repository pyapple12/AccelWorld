# 重构进度追踪（x.progress.md）

> 依据：`z.plan.md`（AccelWorld 审计与重构方案报告）
> 当前版本：ver 0.46（S10 第三轮审计修复 P0-P4 已发布；P5 docstring 合规待处理）
> 状态：**S1-S9 全部完成**；**S10 第三轮审计修复（进行中）**——待办见下
> 执行原则：每阶段完成后运行验证命令确认无回归，再进入下一阶段

---

## 已完成 ✅（S1-S9 全部完成）

| 阶段            | 完成内容                                                                                                                                                                                                           |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| S1 结构骨架     | 建立 `utils/config/modules/ui/data` 五层包结构；`main.py` 收编入口与 VERSION 单一来源；数据/工具/配置/界面层全部迁移                                                                                               |
| S2 数据规范化   | 引入 `TimeInfo`/`LunarInfo`/`WeatherData`/`UserConfig` 四个 dataclass；天气表合并为 `WEATHER_CODE_INFO` 单表；配置走 file_utils 缓存单例 + base64 窗口几何                                                         |
| S3 Bug 修复     | B1（启动参数生效）、B2（托盘退出保存设置）、B3（裸 except）；D1/D2/D9 及死代码清理                                                                                                                                 |
| S4 GUI 面板化   | 6 个面板（ui/panels/）+ `SystemTray` 独立类 + `AlarmEditDialog` 完善；signal/slot 解耦                                                                                                                             |
| S5 后台化       | 天气查询移入 QThreadPool + 30 分钟缓存 + 网络重试；闹钟预设铃声异步播放                                                                                                                                            |
| S6 规范补齐     | 全文件函数 `#` 注释 + 末尾 `# =====` 说明区；类型注解补齐；行宽 ≤100；函数内 import 清零                                                                                                                           |
| S8 审计修正     | 跨天闹钟去重键、配置反序列化容错、天气首次加载、一次性闹钟语义、低优先级 7 项、规范性 4 项、全量回归                                                                                                               |
| S9 代码质量优化 | 异常上抛与窄捕获、死代码清理（9 处）、效率优化（秒级缓存/轮询降频 80 倍）、抽象合并、**应用静态配置层**（config/static json 零硬编码 + 用户配置移入项目内 + 日志每日独立文件）、小优化、**pytest 41 用例测试引入** |
| 收尾            | workingboard M08 勾选、README/AGENTS.md 更新、版本升级 ver 0.45                                                                                                                                                    |

> 详细执行记录见 git 提交历史；审计发现与方案文档见 `z.plan.md`

---

## S10 第三轮审计修复 todo list（2026-08-08，进行中）

> 审计详情（问题描述/位置/判定）见 `z.plan.md`「第三轮审计发现」章节

### P0 严重 Bug（必修）

- [x] S10.1 A1：`AcceleratedWorld` 倍率下限校验改为读取 `base.rate_min`（消除 1.0 边界崩溃 + 硬编码；main.py/main_cli/滑杆校验路径回归，滑杆拖到最左 1.0 不抛异常）
- [x] S10.2 A2：`dataclass_from_dict` 容错模式捕获 `(ValueError, TypeError)`（`"time": null` 闹钟条目跳过而非崩溃）；补测试

### P1 功能缺失

- [x] S10.3 B1：GUI 启动恢复 `last_city`（weather_panel.set_city）与 `last_timezone`（下拉按值定位），消除"只存不读"；weather_panel "北京" 硬编码改读配置

### P2 分层修正

- [x] S10.4 D1：`utils/logger.py` 解除对 config 的反向依赖——`setup_logging(log_dir, backup_days)` 改参数传入，main.py 调用处传值
- [x] S10.5 D2：`play_custom_sound`/`play_alarm_sound_async` 移至 `ui/audio_player.py`，`alarm_service` 保留纯 winsound 播放（模块间引用同步更新）

### P3 遗漏小错（E1-E15）

- [x] S10.6 E1 注释修正（`_last_triggered` 键格式）+ E2 用 `from_value().index()` 消除重复 + E3 display_names 自动生成
- [x] S10.7 E4 QMediaPlayer 持有引用集合防 GC 中断 + E5 preset_config 提为模块常量
- [x] S10.8 E6 weekday_map/days 提为模块常量 + E7/E8 "Asia/Shanghai"/"北京" 硬编码改读静态配置
- [x] S10.9 E9 类型注解补齐 + E10 去掉无意义 try/except + E11 save_settings 合并单次写盘
- [x] S10.10 E12 timezones 夏令时标注修正 + E13 通知时长参数化 + E14 冗余文案/toolTip 清理 + E15 字体/双配置同步

### P4 死代码与细节

- [x] S10.11 C1 `WeatherData.to_display()` 删除（测试改 format_weather_info）+ C2 `standard_second` 处理 + C4 `run_cli = args.cli` 内联
- [x] S10.12 F 组：tray.update_rate 去重、`round(rate*10)`、epilog 精简、date_panel 占位中性化

### P5 注释规则合规（新规则审计：禁止 docstring 顶替 `#` 注释）

> 2026-08-08 AGENTS.md 更新规则：**禁止 docstring（三引号）写注释**，函数/类/模块文档统一走 `#` 注释体系（docstring 不承担注释职责，单行 docstring 当注释用属违规，`.temp/verify_s11.py` 自动检测）
> AST 全量扫描（排除 .venv/.history）：**127 处 docstring 违规 / 19 个文件**，全部需删除 docstring（其内容并入函数下 `#` 注释或文件末尾 `# =====` 说明区）

- [ ] S10.13 modules 层 35 处：`alarm_service.py`（20：PresetSound/Alarm/AlarmManager 类 + display_names/from_value/**post_init**/_validate_time/should_trigger_on/to_dict/from_dict/is_one_time/play_preset_sound/add_alarm/remove_alarm/get_alarm/replace_alarm/toggle_alarm/check_alarms/to_dict_list/from_dict_list）、`time_dilation.py`（6：TimeInfo/AcceleratedWorld 类 + **init**/get_custom_time/run_live_clock/main_cli）、`chinese_calendar.py`（4：LunarInfo 类 + 3 函数）、`weather_service.py`（5：WeatherData 类 + 4 函数）
- [ ] S10.14 ui 层 30 处：`main_window.py`（16：类 + 15 函数）、`alarm_dialog.py`（4）、`audio_player.py`（3）、`system_tray.py`（7）
- [ ] S10.15 ui/panels 层 46 处：`alarm_panel.py`（13）、`clock_panel.py`（7）、`countdown_panel.py`（8）、`date_panel.py`（3）、`weather_panel.py`（10）、`world_clock_panel.py`（5）
- [ ] S10.16 config/data/utils 层 14 处：`settings.py`（11）、`static_config.py`（1）、`weather_codes.py`（1）、`logger.py`（1）
- [ ] S10.17 main.py 2 处：模块级 docstring（文件头三引号）+ `main()`
- [ ] S10.18 验证：AST docstring 扫描 0 处 + `.temp/verify_s11.py`（若创建）0 违规 + pytest 全量 + 全模块导入

### 验证（每项完成后执行）

```powershell
# 导入验证
.\.venv\Scripts\python.exe -c "import main, modules.time_dilation, modules.chinese_calendar, modules.weather_service, modules.alarm_service, config.settings, config.static.static_config, ui.main_window, ui.alarm_dialog, ui.themes, data.cities, data.timezones, data.weather_codes, utils.logger, utils.file_utils, utils.retry"

# 单元测试（含新增 A1/A2 回归用例）
.\.venv\Scripts\python.exe -m pytest tests/ -v

# 边界冒烟（S10.1 后）
.\.venv\Scripts\python.exe main.py --cli --rate 1.0

# GUI 无头初始化（S10.3/S10.5 后）
$env:QT_QPA_PLATFORM="offscreen"; .\.venv\Scripts\python.exe -c "from PyQt6.QtWidgets import QApplication; from ui.main_window import AcceleratedWorldGUI; app = QApplication([]); w = AcceleratedWorldGUI(); print('GUI init OK')"
```

---

## 阶段验证命令速查（AGENTS.md 运行与验证）

```powershell
# 导入验证
.\.venv\Scripts\python.exe -c "import main, modules.time_dilation, modules.chinese_calendar, modules.weather_service, modules.alarm_service, config.settings, config.static.static_config, ui.main_window, ui.alarm_dialog, ui.themes, data.cities, data.timezones, data.weather_codes, utils.logger, utils.file_utils, utils.retry"

# 版本
.\.venv\Scripts\python.exe main.py --version

# CLI 冒烟
.\.venv\Scripts\python.exe main.py --cli --rate 2.0

# 单元测试
.\.venv\Scripts\python.exe -m pytest tests/ -v
```
