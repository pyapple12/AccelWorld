# 重构进度追踪（x.progress.md）

> 依据：`z.plan.md`（AccelWorld 审计与重构方案报告）
> 当前版本：ver 0.45（M08 架构优化 + 代码质量优化已发布）
> 记录格式：状态 [⏳ 待开发, ✅ 已完成] / 优先级 [高, 中, 低]
> 执行原则：每阶段完成后运行验证命令确认无回归，再进入下一阶段

---

## 已完成 ✅（S1-S6、S8、收尾）

| 阶段          | 完成内容                                                                                                                                                                                                                               |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1 结构骨架   | 建立 `utils/config/modules/ui/data` 五层包结构；`main.py` 收编入口与 VERSION 单一来源；`accelworld.py` 兼容转发壳；数据/工具/配置/界面层全部迁移                                                                                       |
| S2 数据规范化 | 引入 `TimeInfo`/`LunarInfo`/`WeatherData`/`AppConfig` 四个 dataclass；天气表合并为 `WEATHER_CODE_INFO` 单表；配置走 file_utils 缓存单例 + base64 窗口几何                                                                              |
| S3 Bug 修复   | B1（启动参数生效 `apply_startup_args`）、B2（托盘退出保存 `quit_app` + 倍率同步保存）、B3（裸 except）；D1/D2/D9 及死代码清理（部分于 S1 提前完成）                                                                                    |
| S4 GUI 面板化 | 6 个面板（ui/panels/）+ `SystemTray` 独立类 + `AlarmEditDialog` 完善（`get_alarm()` 返回数据类）；signal/slot 解耦（rate_changed/theme_toggled/alarm_saved/alarm_triggered）                                                           |
| S5 后台化     | 天气查询移入 QThreadPool（0ms 返回）+ 30 分钟缓存 + 网络重试 3 次；闹钟预设铃声异步播放（`play_alarm_sound_async`）                                                                                                                    |
| S6 规范补齐   | 全文件函数 `#` 注释 + 末尾 `# =====` 说明区；类型注解补齐；行宽 ≤100；函数内 import 清零                                                                                                                                               |
| S8 审计修正   | ①跨天闹钟去重键加日期维度 ②配置反序列化容错 ③天气首次加载 ④一次性闹钟"仅创建当天"语义 ⑤低优先级 7 项（rate 上限/倒计时恢复/年份降级/副本隔离/去重键清理等）⑥规范性问题 4 项（import 顺序/70 处 # 注释/陈旧注释/类型注解）⑦全量回归验证 |
| 收尾          | `workingboard/2. Missions.md` M08a/b/d 勾选 + 版本升级 ver 0.44；README 更新（main.py 启动方式、新目录结构、徽章）                                                                                                                     |

> 详细执行记录见 git 提交历史；审计发现与修复方案见 `z.plan.md`

---

## 阶段验证命令速查（AGENTS.md 运行与验证）

```powershell
# 导入验证
.\.venv\Scripts\python.exe -c "import main, modules.time_dilation, modules.chinese_calendar, modules.weather_service, modules.alarm_service, config.settings, ui.main_window, ui.alarm_dialog, ui.themes, data.cities, data.timezones, data.weather_codes, utils.logger, utils.file_utils, utils.retry"

# 版本
.\.venv\Scripts\python.exe main.py --version

# CLI 冒烟
.\.venv\Scripts\python.exe main.py --cli --rate 2.0
```

---

## 未完成 ⏳

## S9 代码质量优化（第二轮审计，2026-08-08）

> 依据：`z.plan.md`「代码质量审计修正方案（第二轮）」
> 原则：先异常与死代码（P0），再效率与抽象（P1），最后常量化（P2）；每条实现后运行验证命令

### S9.1 异常处理修复（P0）

- [x] S9.1.1 `get_weather_by_coords` 窄捕获 `(URLError, TimeoutError, JSONDecodeError)`，其余异常上抛；失败日志改 `logger.exception`（带堆栈）
- [x] S9.1.2 `_on_weather_result` 包 try/except + `logger.exception`；`_WeatherTask.run` 加异常兜底（记录 + emit None，不卡 UI）
- [x] S9.1.3 `run_live_clock` 循环内 `except Exception` 记录后继续（含 1s 防风暴间隔，CLI 不因农历库异常崩溃）
- [x] S9.1.4 `play_preset_sound`/`play_custom_sound` 失败改 `logger.exception`（替换 print）
- [x] S9.1.5 `update_clock` 统一 `logger.exception`，删除 `traceback.print_exc` 双通道打印
- 状态：✅ 已完成｜优先级：高

### S9.2 死代码清理（P0）

- [x] S9.2.1 删除 `--minimized` 冗余参数（main.py + README 同步，保留 `--hidden`）
- [x] S9.2.2 删除 `seconds_per_day` 死属性（类注解 + 赋值）
- [x] S9.2.3 删除死 import：time_dilation.py `argparse`（S9.1 顺带）、main_window.py `load_config`
- [x] S9.2.4 删除 `get_simple_weather`（weather_service.py）
- [x] S9.2.5 删除 `get_enabled_alarms`/`mark_triggered`/`update_alarm`（alarm_service.py，AlarmManager 的 add/remove 有调用方保留）
- [x] S9.2.6 删除 settings.py 配置层 CRUD 三件套 `add_alarm`/`remove_alarm`/`update_alarm`
- [x] S9.2.7 删除 `utils.logger.get_logger`、`utils.file_utils.read_file`
- [x] S9.2.8 `retry_call` 增加 `retries < 1` 防护（抛 ValueError）
- [x] S9.2.9 删除剩余小时计算的冗余取模（rate≤20 恒等于自身）
- 状态：✅ 已完成｜优先级：高

### S9.3 效率优化（P1）

- [x] S9.3.1 `get_custom_time` 秒级缓存（同秒返回缓存 TimeInfo，农历计算 10Hz→1Hz）
- [x] S9.3.2 `run_live_clock` 秒变化检测前置（标准秒未变跳过计算，3.2s 内全量计算 4 次，原 ~320 次）
- [x] S9.3.3 `world_clock_panel` 缓存 pytz 时区对象 + 秒级刷新（时区切换强制刷新，新时区才构建对象）
- [x] S9.3.4 `alarm_panel.check_alarms` 空列表短路（无闹钟不建 datetime 不遍历）
- 状态：✅ 已完成｜优先级：中

### S9.4 抽象合并（P1）

- [x] S9.4.1 新增 `utils/dataclass_utils.py`（`dataclass_to_dict`/`dataclass_from_dict(cls, data, tolerant)`），`AppConfig` 与 `Alarm` 改调用（消除两套重复序列化）
- [x] S9.4.2 去重键 helper `_trigger_key(check_time)` 抽取（`check_alarms` 使用；`mark_triggered` 已于 S9.2 删除，helper 保留语义命名化防错位）
- [x] S9.4.3 `TimeInfo` 加计算属性 `standard_time`/`standard_second`/`custom_hour`/`custom_second`，CLI 与 clock_panel 调用点改用（split 仅存于属性内部）
- 状态：✅ 已完成｜优先级：中

### S9.5 应用静态配置层（P2，定案 2026-08-08）

> 方案详见 `z.plan.md`「S9.5 应用静态配置层方案」：代码零硬编码，全部参数读 json；用户配置移入项目内；日志集中 logs/ 每日独立文件；静态配置统一 `static` 前缀命名（get_static_config）与用户配置区分

- [x] S9.5.1 新建 `config/static/`（config.json 映射表 + base.json + ui.json + static_config.py：StaticConfig dataclass + `get_static_config()`/`_load_static_config()` 缓存单例，`static` 前缀命名）；`utils/file_utils.py` 新增 `get_project_root()`（校验 main.py）
- [x] S9.5.2 `config/settings.py`：配置路径改项目内（`get_project_root() / user_config`，旧 `~/.config` 不迁移）；UserConfig（由 AppConfig 改名）默认值用 `default_factory` 从 base.json 现取（零硬编码；countdown_target/window_geometry/alarms 保持结构默认）
- [x] S9.5.3 `utils/logger.py`：日志路径/保留天数参数化；`_DailyFileHandler` 每天独立文件 `logs/app-YYYY-MM-DD.log`（写日志时跨天切换）；`_cleanup_old_logs` 启动时清理超过 `log_backup_days` 的旧文件
- [x] S9.5.4 基础参数改造：main.py rate 校验+help 文案、main_window（倍率范围/窗口几何/时钟 tick）、clock_panel（slider 范围/校验/默认值）、weather_service（缓存 TTL）、weather_panel（刷新周期）、alarm_panel（检查周期）、alarm_service（max_alarms）
- [x] S9.5.5 字体/颜色全量替换：各面板 `"Arial"` 与散落颜色 → ui.json（`@{key}@` 占位符方案）；themes.py QSS 模板经 `_apply_colors` 生成（颜色单源）
- [x] S9.5.6 alarm_service 两处 print → logger.warning；AlarmPanel 添加失败弹窗提示（max_alarms/重复）
- [x] S9.5.7 文档同步：AGENTS.md（结构/验证命令/零硬编码原则）、README（结构树/Q2 配置路径）
- [x] S9.5.8 回归验证：主题渲染/字体/窗口恢复/日志每日切换与清理/首启生成 user_config/GUI+CLI 全链路
- 状态：✅ 已完成｜优先级：低

### S9.6 小优化（P2）

- [x] S9.6.1 `gui_args` 构建改推导式过滤（main.py，hidden 单独处理，16 组合等价验证）
- [x] S9.6.2 `PresetSound` 增加 `display_name` property、`index()`、`from_index()`（alarm_panel/alarm_dialog 互转复用，输出逐项一致）
- 状态：✅ 已完成｜优先级：低

### S9.7 回归验证与测试引入（合并原 S7，对应 M08c）

> 目标：pytest 覆盖核心模块（modules/config 不依赖 PyQt，可直接测试）+ S9 全部改动综合回归

#### 测试引入（原 S7.1/S7.2）

- [x] S9.7.1 添加 pytest 依赖（`requirements-dev.txt`，与运行时依赖分离）+ `tests/` 目录（conftest.py 隔离 fixture：临时配置路径 + 天气打桩 + 缓存清理）
- [x] S9.7.2 `time_dilation` 测试（8 用例）：倍率校验、时间计算、24h 边界、TimeInfo 属性、秒级缓存、main_cli 默认
- [x] S9.7.3 `chinese_calendar` 测试（7 用例）：干支/生肖/月日/时辰/节日/年份边界/格式化
- [x] S9.7.4 `config.settings` 测试（7 用例）：默认值/读写往返/损坏 JSON 容错/缓存/未知键/base64 几何/副本
- [x] S9.7.5 `alarm_service` 测试（9 用例）：构造校验/重复/一次性/跨天去重/上限/容错/保留 ID/预设辅助
- [x] S9.7.6 `weather_service` 测试（8 用例）：格式化/缓存/重试/窄捕获/编程错误上抛/未知城市
- 状态：✅ 已完成｜优先级：中

#### 回归验证（原 S9.7）

- [x] S9.7.7 每项修改后运行导入验证 + 相关行为验证（S9.1-S9.6 各阶段已执行）
- [x] S9.7.8 `pytest` 全部通过（41/41，6.5s）
- [x] S9.7.9 全部完成后综合回归：py_compile / 全模块导入 / 核心行为 / GUI offscreen 全链路 / CLI 冒烟（含 rate=25 拒绝）
- 状态：✅ 已完成｜优先级：高
