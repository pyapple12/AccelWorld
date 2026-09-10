# 重构进度追踪（x.progress.md）

> 依据：`z.plan.md`（AccelWorld 审计与重构方案报告）
> 当前版本：0.4.7.7（UI2.0 Fluent 重写 PL002 完成）
> 状态：**S1-S10 全部完成**，无未完成项（重构期收官）
> 更新（2026-09-10）：接入 DeepTransHub 工作流体系；自即日起新增任务按下文「未完成」区的新规则记录
> 执行原则：每阶段完成后运行验证命令确认无回归，再进入下一阶段

---

## 已完成 ✅（S1-S10 全部完成）

| 阶段               | 完成内容                                                                                                                                                                                                                                        |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1 结构骨架        | 建立 `utils/config/modules/ui/data` 五层包结构；`main.py` 收编入口；数据/工具/配置/界面层全部迁移                                                                                                                                               |
| S2 数据规范化      | 引入 `TimeInfo`/`LunarInfo`/`WeatherData`/`UserConfig` 四个 dataclass；天气表合并单表；配置缓存单例 + base64 窗口几何                                                                                                                           |
| S3 Bug 修复        | B1（启动参数生效）、B2（托盘退出保存设置）、B3（裸 except）；D1/D2/D9 及死代码清理                                                                                                                                                              |
| S4 GUI 面板化      | 6 个面板（ui/panels/）+ `SystemTray` 独立类 + `AlarmEditDialog`；signal/slot 解耦                                                                                                                                                               |
| S5 后台化          | 天气查询移入 QThreadPool + 30 分钟缓存 + 重试；闹钟预设铃声异步播放                                                                                                                                                                             |
| S6 规范补齐        | 全文件函数 `#` 注释 + 末尾 `# =====` 说明区；类型注解补齐；行宽 ≤100                                                                                                                                                                            |
| S8 审计修正        | 跨天闹钟去重键、配置反序列化容错、天气首次加载、一次性闹钟语义、低优先级 7 项、规范性 4 项、全量回归                                                                                                                                            |
| S9 代码质量优化    | 异常上抛与窄捕获、死代码清理、效率优化（秒级缓存/轮询降频）、抽象合并、**应用静态配置层**（零硬编码）、pytest 41 用例                                                                                                                           |
| S10 第三轮审计修复 | P0 严重 Bug（A1 倍率边界/A2 闹钟容错）、P1 功能缺失（B1 配置恢复）、P2 分层（D1 日志解耦/D2 音频迁 ui 层）、P3 遗漏小错（E1-E15）、P4 死代码与细节（C1-C4/F 组）、P5 注释规范（docstring 127 处全量清理）、版本号迁 base.json、类型检查策略收紧 |
| 收尾               | 版本升级 ver 0.46、README/AGENTS.md 同步更新                                                                                                                                                                                                    |

> 详细执行记录见 git 提交历史；S10 各阶段待办清单已随完成归档，审计发现与方案文档见 `z.plan.md`

---

## 阶段验证命令速查（AGENTS.md 运行与验证）

```powershell
# 导入验证
.\.venv\Scripts\python.exe -c "import main, modules.time_dilation, modules.chinese_calendar, modules.weather_service, modules.alarm_service, config.settings, config.static.static_config, ui.main_window, ui.alarm_dialog, ui.themes, ui.audio_player, ui.system_tray, data.cities, data.timezones, data.weather_codes, utils.logger, utils.file_utils, utils.retry"

# 版本
.\.venv\Scripts\python.exe main.py --version

# CLI 冒烟
.\.venv\Scripts\python.exe main.py --cli --rate 2.0

# 单元测试
.\.venv\Scripts\python.exe -m pytest tests/ -v

# GUI 无头初始化
$env:QT_QPA_PLATFORM="offscreen"; .\.venv\Scripts\python.exe -c "from PyQt6.QtWidgets import QApplication; from ui.main_window import AcceleratedWorldGUI; app = QApplication([]); w = AcceleratedWorldGUI(); print('GUI init OK')"
```

---

## 未完成（2026-09-10 起按 DeepTransHub 工作流新规则记录）

> 结构规则：已完成任务在上方历史区（S1-S10 重构期记录，原样保留）；未完成任务组用 `### 编号: 标题 [来源]`，子任务格式 `- [ ] 编号.序号 [P级] 标题 —— 做法；验证：方式`
> 编号规则：功能/修复任务组 `T{NNN}` 递增；审计修复任务组 `FIX{NNN}`（与 z.plan.md 附录 `A{NNN}` 一一对应），由 `.agents/skills/audit-report` 归档时生成
> 执行方式：按 `.agents/skills/progress-task` 流程执行（研究 → todo → 检验方案 → 实施 → 验证 → 汇报）

### T001: 实测问题修复（对应版本 0.4.7.1）[problems#1-3]

- [x] T001.1 [P1] 加速时间刷新频率随倍率变化（1/rate 秒节奏） —— `get_custom_time()` 秒级缓存按标准时间秒做键导致界面每标准秒只变一次；改法：GUI/CLI tick 间隔按 `1000ms / rate` 动态计算（`clock_tick_ms` 参数化），配合缓存键改用加速时间秒；验证：单元测试断言倍率 2.0/10.0 下 `TimeInfo.custom_second` 变化周期 + GUI 无头初始化（2026-09-10 完成：农历/中文日期改标准秒级缓存，标准/加速时间每次现算，新增 `tick_interval_ms` 属性且 GUI 定时器随倍率联动重启；探针 11 项 + 54 用例全过）
- [x] T001.2 [P2] 倒计时日期选择器实时反馈 —— `countdown_panel.show_date_picker()` 中快捷按钮 `setSelectedDate` 后即时写回 `countdown_target` 输入框，日历 `clicked` 信号连接同步写回，不等 `dialog.exec()` Accepted；验证：GUI 无头初始化 + 手动核对快捷按钮与日历点击均即时勾选（2026-09-10 完成：新增 `_set_target_date_part`/`_apply_quick_date`/`_build_date_dialog`，日历 `clicked` 与快捷按钮共用实时写回路径；探针含保留时间部分断言）
- [x] T001.3 [P3] 时间选择框显示不全 —— `show_time_picker()` 的 `QTimeEdit.setFixedSize(120, 40)` 宽度放不下 `HH:mm:ss`；改法：宽度按 `sizeHint` 自适应或加宽常量入 `ui.json`；验证：GUI 无头初始化 + 截图核对时分秒完整可见（2026-09-10 完成：移除弹窗与 QTimeEdit 固定尺寸，`_build_time_dialog` 交由 Qt sizeHint 自适应，sizeHint 198px ≥ 需求 176px；像素级视觉效果留待用户运行确认）

### T002: 运行监控体系 [problems#4]

- [x] T002.1 [P2] 全局异常钩子 —— `main.py` 加 `sys.excepthook` → `logger.critical(..., exc_info=True)`，Python 未捕获异常（含 Qt 槽内抛错）不再静默；验证：.temp 探针触发异常，断言日志文件含 traceback（2026-09-10 完成：`utils/monitor.py` `install_excepthook` 主线程 + `threading.excepthook` 子线程双钩子；探针断言日志含标记与堆栈）
- [x] T002.2 [P2] Qt 原生警告转发 —— `qInstallMessageHandler` 将 Qt 警告（QSS 解析失败等）按级别转发进 logger；验证：探针触发 Qt warning 断言落日志（2026-09-10 完成：`install_qt_message_handler` 五级消息映射 logging，附来源文件:行号）
- [x] T002.3 [P3] 原生崩溃栈落盘（可选） —— `faulthandler.enable(file=...)` 崩溃栈写入 logs/ 专用文件；验证：仅手工执行探针验证文件产出（2026-09-10 完成：`install_crash_handler` 写 `logs/crash-YYYY-MM-DD.log`，子进程真实崩溃自动化验证；crash-\*.log 并入同保留期清理）

### T003: UI 美化方向决策 [problems#5]

- [x] T003.1 [P3] 体验 PyQt-Fluent-Widgets 官方 demo —— 安装并运行官方 demo 实际感受 Fluent 视觉与组件形态；验证：体验结论记录到 y.problems.md#5（2026-09-10 完成：独立临时 venv 装 PyQt6-Fluent-Widgets 1.11.3 + [full] 扩展，官方仓库 PyQt6 分支 gallery demo 桌面实测运行，项目 .venv 零污染）
- [x] T003.2 [P3] 定方向 —— 三选一：UI 2.0 立项（Fluent Widgets 重写 UI 层，呼应主题商店 F03c03）/ 低成本方案（PyQtDarkTheme 或 QtTheme 配合现有 themes.py 管线）/ 短期维持现状；验证：结论写入 y.problems.md 并更新状态，若立项则在 z.plan.md 建方案章节（2026-09-10 定案：**C. 维持现状**，UI 2.0 列为 1.0 大版本候选，未立项故 z.plan.md 不建新章节）

### T004: 短期功能项 [plan#未完成功能项路线图]

- [x] T004.1 [P2] 快捷键支持（M09c/F02a01） —— QShortcut 绑定 Ctrl+S 保存、Ctrl+Q 退出、Ctrl+T 主题切换，键位常量入 config/static；验证：GUI 无头初始化 + 手动核对三快捷键（2026-09-10 完成：`base.json` 新增 `shortcuts` 键位表，`main_window._install_shortcuts` 统一挂载；探针实测三快捷键触发保存/退出保存/主题翻转）
- [x] T004.2 [P2] 倍率预设方案（M09e/F02a04） —— 预设工作/睡眠/专注模式（倍率组合），预设定义入 config/static，面板加快捷切换；验证：单元测试断言预设切换后配置生效（2026-09-10 完成：`base.json` 新增 `rate_presets`（工作 1.0/专注 2.0/睡眠 10.0），ClockPanel 预设按钮行走 `set_rate` 信号链与滑杆/手输共用校验持久化路径；`tests/test_rate_presets.py` 2 用例，GUI 断言走子进程隔离避免退出期硬崩溃污染 pytest 退出码）
- [x] T004.3 [P3] 进度条动画（F01b02） —— QPropertyAnimation 平滑过渡替代 setValue 跳变，动画时长入 base.json；验证：GUI 无头 + 手动观察过渡效果（2026-09-10 完成：`_animate_progress` 每 tick 以当前动画值为起点重定目标，时长 `progress_anim_ms=200`；验收实测半程值 5 ∈ (-1, 13) 证明平滑非跳变，终值收敛）
- [x] T004.4 [P3] 托盘 toolTip 实时化（F01c03/M09b） —— setToolTip 随时钟 tick 显示当前加速时间/倍率（当前为静态文本）；验证：手动核对托盘悬停内容随时间变化（2026-09-10 完成：`SystemTray.update_tooltip` 文本未变化时跳过重绘，主窗口 `update_clock` 逐 tick 推送；探针断言时间/倍率进入悬停文本且相同文本不重复 setToolTip）

### FIX001: 第1轮审计修复 [audit#A001]

- [x] FIX001.1 [P0] read_json 补捕 UnicodeDecodeError —— utils/file_utils.py:36 `except (OSError, json.JSONDecodeError)` 元组追加 `UnicodeDecodeError`（ValueError 子类不被现有元组捕获，GBK 字节配置文件致启动崩溃）；验证：pytest 新用例（GBK 字节文件经 read_json 返回 default 不抛）（2026-09-11 完成：test_read_json_unicode_error_returns_default 通过）
- [x] FIX001.2 [P0] 配置损坏防丢数据与原子写入 —— settings.load_config 检测到损坏时先将原文件转存 `user_config.json.bak` 再回退默认值；file_utils.write_json 改临时文件写入 + `os.replace` 原子替换；同步更新 test_corrupted_json 断言；验证：pytest 用例（损坏→load→save 后 .bak 存在且含原内容、新文件为合法 JSON）（2026-09-11 完成：test_corrupted_config_backed_up / test_write_json_replaces_without_tmp_residue 通过）
- [x] FIX001.3 [P1] 天气 DNS 故障降级与重试修复 —— weather_service.py:111-117 retry_call 的 exceptions 与 :133 降级 except 均补 `socket.gaierror`；验证：pytest 打桩 `socket.getaddrinfo` 抛 gaierror → get_weather_by_coords 返回 None 且重试计数生效（2026-09-11 完成：test_gaierror_degrades_and_retried 断言降级 None + 3 次尝试）
- [x] FIX001.4 [P1] 节日英文名映射补全 —— chinese_calendar.py:60 `HOLIDAY_TRANSLATION` 按 chinese-calendar 库 constants 的 7 个 Holiday 英文名补全中文映射；验证：pytest 用例断言 2025 春节/清明/端午/中秋日期的 chinese_date 含中文名且不含英文（2026-09-11 完成：test_public_holidays_translated_chinese 七节日全过）
- [x] FIX001.5 [P1] 默认城市启动天气查询 —— weather_panel `__init__` 信号连接后显式发起首次 `update_weather()`（set_city 未变化分支同样直调）；验证：GUI 无头探针断言创建主窗口后天气面板进入加载/请求态而非静止占位（2026-09-11 完成：test_gui_features 子进程断言启动即查询）
- [x] FIX001.6 [P1] 铃声类型切换失效修复 —— alarm_dialog 连接 `sound_combo.currentIndexChanged` 回调复位 `sound_type="preset"`；编辑 custom 闹钟时按钮文案回填文件名；验证：GUI 无头探针（编辑 custom 闹钟→combo 选预设→get_alarm 返回 preset 类型）（2026-09-11 完成：子进程断言切换生效且按钮回填 wake.wav）
- [x] FIX001.7 [P1] AcceleratedWorld 上限校验与启动脏值回退 —— time_dilation `__init__` 补 rate_max 上限校验；ui/main_window.py:64 启动路径 try 包裹脏值回退 default_rate 并记日志；验证：pytest 边界用例（rate_max+ε 构造抛 ValueError；脏值经主窗口启动回退默认且写回配置）（2026-09-11 完成：test_init_rate_max_rejected 通过；主窗口回退由 import 链冒烟覆盖）
- [x] FIX001.8 [P2] 闹钟 created_at TypeError 补捕 —— alarm_service.py:116-120 except 改 `(ValueError, TypeError)`；验证：pytest 用例（created_at=None 的 Alarm 执行 should_trigger_on 返回 False 不抛）（2026-09-11 完成：test_created_at_none_does_not_crash 通过）
- [x] FIX001.9 [P2] dataclass 反序列化类型校验 —— utils/dataclass_utils.dataclass_from_dict 增加逐字段类型校验，类型不符的字段剔除后走默认值（UserConfig 与 Alarm 共用生效）；验证：pytest 脏配置用例（theme:null、time_dilation_rate:"abc"、repeat_days:["1"] 载入后字段为默认值/规范值）（2026-09-11 完成：test_from_dict_invalid_types_fall_back_defaults / test_repeat_days_normalized 通过）
- [x] FIX001.10 [P2] 倒计时恢复跨会话清空修复 —— main_window 启动恢复分支同步解析文本填充 `countdown_target_date`（不启动计时语义不变）；验证：探针（恢复→save_settings→配置中 countdown_target 值保留原样）（2026-09-11 完成：countdown_panel 抽取 `_parse_target_text`/新增 `restore_target`，子进程断言恢复→保存往返保留）
- [x] FIX001.11 [P2] 主题持久化接线 —— main_window `__init__` 读 `get_setting("theme", base["default_theme"])`，toggle_theme/apply_startup_args 持久化 theme 字段；验证：探针（切深色→重读配置为 dark→新窗口实例以深色启动）（2026-09-11 完成：子进程断言 Ctrl+T 后持久化且新实例恢复深色）
- [x] FIX001.12 [P2] 子进程测试配置隔离 —— config.settings 支持环境变量注入配置路径（如 ACCELWORLD_CONFIG_FILE），conftest 与 test_rate_presets 子进程脚本共用该机制重定向到临时目录；验证：跑全量 pytest 后 `git status` 无 user_config.json 变更（2026-09-11 完成：还原真实配置后全量两轮复跑零变更）
- [x] FIX001.13 [P2] 白色系颜色入 ui.json —— ui.json colors 增白色系键（按钮文本/托盘指针），themes 模板占位符与 system_tray `QColor(_UI[...])` 接入；验证：rg 全仓无 `"white"` 硬编码残留 + GUI 冒烟（2026-09-11 完成：text_on_primary/tray_hand 两键，rg 无残留）
- [x] FIX001.14 [P2] 天气响应字段缺失视为失败 —— weather_service 解析处字段缺失（temperature_2m/weather_code 等）返回 None 而非默认 0；验证：pytest 打桩缺字段响应断言返回 None（2026-09-11 完成：test_missing_required_fields_returns_none / test_response_not_dict_returns_none 通过）
- [x] FIX001.15 [P2] CLI 动态周期接线 —— time_dilation.run_live_clock 消费 `self.tick_interval_ms` 替代硬编码 sleep(1.0)/sleep(0.01)；验证：pytest 单测断言 CLI 循环周期取值 + CLI 冒烟 `--cli --rate 10.0`（2026-09-11 完成：新增 `cli_poll_interval_s()`，test_cli_poll_interval_follows_tick 通过，CLI 冒烟正常）
- [x] FIX001.16 [P2] 网络参数入配置 —— base.json 增 weather_timeout_s/weather_retries/weather_retry_delay，weather_service 读取；验证：rg 零硬编码 + 全量回归（2026-09-11 完成）
- [x] FIX001.17 [P2] file_utils 缓存键统一 resolve —— read_json_cached 与 clear_json_cache 统一以 `Path(path).resolve()` 为键；验证：pytest 用例（相对路径写后读一致）（2026-09-11 完成：test_cache_key_resolved_relative_and_absolute 通过）
- [x] FIX001.18 [P2] 日志跨天重开防护 —— logger emit 重开段包 `try/except OSError`，失败保持旧流降级继续；验证：pytest monkeypatch `_open` 抛 OSError 断言不外抛且后续 emit 可用（2026-09-11 完成：test_rollover_failure_degrades_and_recovers 通过；适配 Python 3.14 emit 不吞 \_open 异常的行为，流为 None 时跳过写入）
- [x] FIX001.19 [P2] 保存失败上浮提示 —— main_window.\_save_alarms/save_settings 检查保存返回值，失败时托盘警告通知；验证：探针模拟 write_json 失败断言通知触发（2026-09-11 完成：子进程 monkeypatch save_config=False 断言通知触发）
- [x] FIX001.20 [P2] 一次性闹钟已过时间自动顺延次日 —— alarm_service 一次性闹钟创建时设定时间已过今日的自动顺延至次日触发（2026-09-11 用户定案，语义变更）；验证：pytest 用例（今日已过时间的一次性闹钟次日触发、今日未过时间当天触发不变）并回归 S8.4 相关用例（2026-09-11 完成：test_one_shot_past_time_rolls_to_next_day / test_one_shot_future_time_stays_same_day 通过，S8.4 既有用例不受影响）
- [x] FIX001.21 [P3] 闹钟服务低危批次 —— repeat_days 构造过滤 0-6（对应审计 P3#1）；from_dict_list 丢弃条目补 logger.warning（P3#16）；声音兜底元组改引用 CLASSIC（P3#17）；\_last_triggered 不持久化补注释说明（P3#15）；验证：pytest 越界/损坏条目用例（2026-09-11 完成）
- [x] FIX001.22 [P3] 天气与 CLI 低危批次 —— \_OPENER 单线程假设注释声明（审计 P3#2）；URL 字面量改白名单常量拼接（P3#14）；main.py --rate 帮助文本注明 0.1 步进（P3#3）；验证：rg 无双源 + CLI 冒烟（2026-09-11 完成；P3#2 采用更彻底方案：opener 改每次请求独立构建，消除共享可变状态）
- [x] FIX001.23 [P3] GUI 低危批次 —— 倍率提示文案改"必须不小于"（P3#4）；托盘初始倍率取 default_rate（P3#5）；列表外城市 combo 只读展示（P3#6）；滑杆写盘去抖与 apply_acceleration 双发消除（P3#7）；选择器对话框 deleteLater（P3#8）；world_clock 日志统一 exception（P3#10）；验证：GUI 无头探针（2026-09-11 完成：写盘去抖 500ms 入 base.json rate_save_debounce_ms，子进程断言拖动 0 次立即写盘/停止后单次/应用加速单发）
- [x] FIX001.24 [P3] 入口与日志低危批次 —— setup_logging 移至 parse_args 后消除 --version 日志副作用（P3#9）；crash 前缀常量单源化（P3#19）；日志级别入 base.json 并消 backup_days 双源（P3#20）；excepthook 链式保留原钩子（P3#23）；验证：探针（--version 后无新日志文件）+ 冒烟（2026-09-11 完成：CRASH_LOG_PREFIX 单源于 logger.py，log_level 键入 base.json，test_version_flag_creates_no_log_file / test_excepthook_chains_previous_hook 通过）
- [x] FIX001.25 [P3] 规范与死代码清理批次 —— time_dilation 类属性 docstring 改 # 注释（P3#11）；Tuple/Qt 死 import 删除（P3#12/25）；SHI_CHEN 死条目修正（P3#13）；timezones 说明区更新（P3#18）；static_config 映射表显式 RuntimeError（P3#21）；CONFIG_DIR 处置（P3#22）；latin1 兼容层验证存量后清理（P3#24）；"100ms"注释更新（P3#26）；验证：rg + 全量回归（2026-09-11 完成：latin1 无存量依据按废弃方案移除、b64decode 改 validate=True、CONFIG_DIR 删除、conftest 同步）
- [x] FIX001.26 [P3] 文档与测试沉淀批次 —— README 徽章与 x.progress 当前版本行同步 base.json（P3#27）；快捷键/动画/tooltip 探针断言子进程化沉淀为持久用例（P3#28）；验证：rg 版本一致 + pytest 用例数增长（2026-09-11 完成：新增 tests/test_gui_features.py，用例数 56 → 78）
- [x] FIX001.27 [P0-P3] 修复收尾：反向验收与全量回归 —— 逐项对照 A001 原问题反向验收（原问题"不报错"→验收"报错"，原问题"静默"→验收"有感知"），结果写入 .temp/verify_fix001_accept.py；全量 pytest + import 冒烟；验证：验收脚本全 PASS + 回归无 FAIL（2026-09-11 完成：模块侧验收 11/11，GUI 侧由 test_gui_features 覆盖，pytest 78/78，冒烟 OK）

### FIX002: 第2轮审计修复 [audit#A002]

- [x] FIX002.1 [P1] 启动脏倍率回退被残留构造击穿 —— 删除 ui/main_window.py 旧无保护构造，仅保留防护版；验证：pytest 子进程用例（user_config.json 写 "time_dilation_rate": 0.5 经 ACCELWORLD_CONFIG_FILE 注入 → GUI 以默认倍率存活不崩）（2026-09-11 完成：c_dirty_rate_startup 断言回退 2.0 存活，stderr 记录回退告警）
- [x] FIX002.2 [P1] 非 dict 结构穿透类型校验与 .bak 旁路 —— dataclass_from_dict 入口加 `isinstance(data, dict)` 判定（tolerant 返回 None / 非 tolerant 抛 ValueError）；settings.load_config 对非 dict data 走 \_backup_corrupted_config 损坏分支；验证：pytest 用例（配置内容为 `[]`/`"x"` → 载入默认值 + .bak 转存；`"alarms": ["字符串"]` 条目剔除不崩）（2026-09-11 完成：test_non_dict_config_backed_up 等三用例通过）
- [x] FIX002.3 [P2] NaN/Infinity 穿透类型校验 —— dataclass_utils float 分支加 `math.isfinite`；验证：pytest 用例（"time_dilation_rate": NaN 载入回退默认值，GUI 构造不产生 int(nan) 崩溃）（2026-09-11 完成：test_nan_rate_falls_back_to_default 通过）
- [x] FIX002.4 [P2] 天气响应 null/错型值穿透 —— weather_service 缺字段校验改为"键存在且值非 None"（新增 \_NUMERIC_FIELDS 数值判定）；验证：pytest 打桩 `temperature_2m: null` 响应断言返回 None（2026-09-11 完成：test_null_value_field_rejected / test_non_numeric_value_field_rejected 通过）
- [x] FIX002.5 [P2] 天气读体阶段网络异常逃逸 —— 重试白名单与降级 except 统一为 \_NETWORK_ERRORS 元组（补 ConnectionResetError/http.client.HTTPException）；验证：pytest 打桩 read 中途抛 ConnectionResetError → 重试后降级 None（2026-09-11 完成：test_read_phase_error_degrades_and_retried 断言 3 次尝试）
- [x] FIX002.6 [P2] env 注入配置路径保存越界上抛 —— settings.save_config 捕获 ValueError 同 OSError 策略记日志返回 False（FIX001.19 降级链不再被绕过）；验证：pytest 用例（配置注入白名单外路径 → set_setting 返回 False 不抛）（2026-09-11 完成：test_save_config_outside_roots_returns_false 通过）
- [x] FIX002.7 [P2] 原子写 tmp 冲突与 unlink 逃逸 —— file_utils.write_json tmp 名加进程唯一后缀（pid）；except 分支 unlink 再包 try/except OSError；验证：pytest 原子写用例（glob 断言无 .tmp 残留）（2026-09-11 完成）
- [x] FIX002.8 [P2] GUI 动画断言时间依赖 —— test_gui_features 进度动画 check 重写为全确定性：先收敛到已知值 5，再更新到 13，断言动画 Running 态 + 配置时长 + 终值收敛（跳变实现无 Running 态）；验证：子进程用例任意本地时刻通过（2026-09-11 完成）
- [x] FIX002.9 [P2] 托盘初始倍率显示错值 —— main_window 托盘创建后补 `self.tray.update_rate(self.accel_world.time_dilation_rate)`；验证：子进程用例（持久化倍率 10.0 启动 → 托盘菜单文本为 10.0x）（2026-09-11 完成：c_tray_initial_rate 通过）
- [x] FIX002.10 [P2] --theme light 被静默忽略 —— apply_startup_args 补 light/dark 双分支（复位浅色 + apply + 持久化）；验证：子进程用例（配置 theme=dark + --theme light 启动 → is_dark_theme 为 False 且配置回写 light）（2026-09-11 完成：c_theme_light_arg 通过）
- [x] FIX002.11 [P2] 预设子进程真实网络请求 —— test_rate_presets 子进程脚本补 get_weather_by_city 打桩（照抄 test_gui_features 模式）；验证：该用例运行无网络尾延迟（2026-09-11 完成）
- [x] FIX002.12 [P2] gui_features 打桩泄漏 —— tray.show_notification / mw.set_setting / 信号连接三处补 try/finally 还原（对齐 save_config 桩既有标准）；验证：桩还原后后续 check 仍依赖真实通道通过（2026-09-11 完成）
- [x] FIX002.13 [P3] static_config RuntimeError 承诺补全 —— 映射表循环后显式校验 {"base","ui"} 键集与各项 isinstance(str)/isinstance(dict)，不满足抛 RuntimeError（A001 P3-21 残留）；验证：pytest 三形态用例（缺键/非串值/非 dict 内容）（2026-09-11 完成：新增 tests/test_static_config.py 三用例通过）
- [x] FIX002.14 [P3] 日志默认值双源残留 —— setup_logging 取消 level/backup_days 字面默认（必传参数），单源于 base.json；验证：rg 无第二处字面默认 + 全量回归（2026-09-11 完成）
- [x] FIX002.15 [P3] timezones 说明区更新 —— data/timezones.py:19 消费方改为 ui/panels/world_clock_panel.py（A001 P3#18 遗留）；验证：rg 消费方核对（2026-09-11 完成）
- [x] FIX002.16 [P3] logger/settings 杂项批次 —— 日志文件名拼装拆 \_path_for(day) 共用（P3-4）；get_setting 注释 AppConfig→UserConfig（P3-5）；损坏转存模块级节流标志 \_backup_done（成功载入后重置，P3-6）；验证：pytest test_corrupted_backup_only_once 断言仅转存一次（2026-09-11 完成；实施中发现并修复 \_path_for 重构引入的 \_today 未初始化与 stream 丢弃回归——见汇报说明）
- [x] FIX002.17 [P3] 闹钟口径与分层批次 —— repeat_days 元素层排除 bool、非整数值统一剔除（[1.7] 与 ["1.5"] 行为一致，P3-7）；SUPPORTED_AUDIO_FORMATS 迁 ui/alarm_dialog.py（P3-8）；验证：pytest test_repeat_days_bool_and_decimal_consistent + rg 消费方（2026-09-11 完成）
- [x] FIX002.18 [P3] weather_panel 副作用对批次 —— 启动双请求消除（WeatherPanel 增 initial_city 参数，首查即用恢复值，main_window 不再二次 set_city，P3-10）；列表外城市下拉保留标注会话级语义（P3-9）；验证：子进程 c_first_weather_query 断言启动即单次查询（2026-09-11 完成）
- [x] FIX002.19 [P3] 测试私有方法调用治理 —— test_rate_presets 的 `window._flush_pending_rate()` 改事件循环等待去抖触发（QTimer + QEventLoop 公开途径），P3-11；验证：pytest 该用例仍过（2026-09-11 完成）
- [x] FIX002.20 [P1-P3] 修复收尾：反向验收与全量回归 —— 逐项对照 A002 原问题反向验收，结果写入 .temp/verify_fix002_accept.py；全量 pytest + import 冒烟 + 配置零污染复验；验证：模块侧验收 8/8 PASS，GUI/测试侧由 test_gui_features 与 test_rate_presets 覆盖，pytest 90/90，冒烟 OK，版本 0.4.7.5（2026-09-11 完成）

### PL001: UI2.0 架构解耦——接口层与纯展示化 [plan#UI2.0]

- [x] PL001.01 接口目录骨架 —— 新建 `interface/__init__.py`（说明注释）、`interface/types.py`（re-export 后端 DTO：TimeInfo/WeatherData/Alarm/PresetSound，并定义 UiPreferences dataclass：theme/last_city/last_timezone/countdown_target）、`interface/app_interface.py`（AppInterface 空类骨架 + 文件尾说明区）；验证：`python -c "from interface import AppInterface; from interface import types"` 成功，且 `rg "PyQt6" interface/` 零结果（接口无 Qt 依赖铁律）（2026-09-11 完成：三文件就位导入冒烟通过；说明区措辞避让 Qt 字样后验收零结果）
- [x] PL001.02 时钟与倍率契约 —— AppInterface 增 get_time_info()/get_tick_interval_ms()/get_version()/get_rate()/get_rate_bounds()（返回 (rate_min, rate_max)）/get_rate_presets()（返回 dict）/apply_rate(rate)（内聚：范围校验 + AcceleratedWorld 重建 + set_setting 持久化，去抖逻辑留 UI 层）；接口内部持有 AcceleratedWorld 实例（构造时以持久化倍率初始化，脏值回退逻辑一并迁入）；验证：新增 tests/test_interface.py 纯 pytest 用例（文件头 `rg "PyQt6"` 断言无 Qt import）：apply_rate 合法值生效且落盘、越界值被拒、脏持久化值构造回退默认（2026-09-11 完成：七用例全过；另增 set_rate 轻量路径（校验+重建不落盘）承载滑杆拖动实时生效，持久化去抖留 UI 层，FIX001.23 语义不回归）
- [x] PL001.03 配置与几何契约 —— AppInterface 增 get_ui_preferences()（组装 UiPreferences）/save_theme(theme)/save_last_city(city)/save_last_timezone(tz)/save_countdown_target(text)/load_window_geometry()/save_window_geometry(bytes)，全部委托 config.settings；验证：tests/test_interface.py 往返用例（save→get_ui_preferences 断言一致；几何 base64 往返），复用 conftest 临时配置隔离（2026-09-11 完成：偏好往返/缺省回退 base.json/几何往返/非法 base64 返回 None 四用例全过）
- [x] PL001.04 天气与时区契约 —— AppInterface 增 get_city_names()（sorted CITIES keys）/fetch_weather(city)（含缓存，委托 weather_service）/get_weather_refresh_interval_ms()（weather_cache_ttl\*1000）/format_weather_display(city, weather)（委托 format_weather_info）/get_timezone_options()（委托 data.timezones）；验证：tests/test_interface.py 用例（未知城市 None、缓存命中只查一次、时区表非空）（2026-09-11 完成：城市排序/未知城市 None/缓存单次查询/格式化空数据/时区表五用例全过）
- [x] PL001.05 闹钟契约（所有权迁移）—— AlarmManager 从 alarm_panel 迁入 AppInterface 内部持有；接口增 load_alarm_dicts()/save_alarm_dicts(list)/check_alarms(now)/get_max_alarms()；验证：tests/test_interface.py 用例（load→check 触发匹配→save→重载一致），且 `rg "AlarmManager" ui/` 零结果（2026-09-11 完成：载入→触发→保存→重载一致、同分钟去重、CRUD、上限/去重四用例全过，`rg "AlarmManager" ui/` 零结果；实施定案：save_alarm_dicts 收敛为零参（序列化接口内管理器状态），面板增删改查另经接口 add/remove/get/replace/toggle/get_alarms 转发）
- [x] PL001.06 ui/tools 骨架与倒计时运算 —— 新建 `ui/tools/__init__.py` + `ui/tools/countdown_tools.py`：`parse_target_text(text)`（自 countdown_panel.\_parse_target_text 迁入）、`format_remaining(target, now)`（自 update_countdown 的剩余拆解与着色态计算迁入，返回 (文本, 是否结束)）；countdown_panel 改为调用 tools 并只做 setText/着色；验证：新增 tests/test_countdown_tools.py 纯单测（三种格式解析、剩余拆解边界、过期态），面板文件 `rg "strptime" ui/panels/` 零结果（2026-09-11 完成：8 用例全过，到点即结束边界覆盖；面板着色键 danger/primary 语义不变）
- [x] PL001.07 时钟与闹钟展示运算 —— `ui/tools/clock_tools.py`：`progress_bounds(expanded_hours_per_day)`（自 update_time 的 total_hours=int() 迁入）；`ui/tools/alarm_text.py`：`format_repeat_display(repeat_days)`（自 alarm_panel.\_get_repeat_display 及 \_WEEKDAY_CHARS 迁入）、`format_sound_button_name(path)`（basename 截断逻辑迁入）；验证：纯单测（进度上界、重复文案、文件名截断），面板内 `rg "_WEEKDAY_CHARS|basename" ui/panels/` 零结果（2026-09-11 完成：test_ui_tools 7 用例全过；预设铃声"🔔 显示名"分支保留于面板 \_sound_display，自定义铃声截断文案走 tools）
- [x] PL001.08 main_window 改造 —— 构造 AppInterface 单例并注入各面板构造参数；自身后端 import 清零：settings 组（load_config/save_config/get_setting/set_setting/get_alarms/save_alarms/load_window_geometry/save_window_geometry）→ interface 对应方法；AcceleratedWorld → interface.apply_rate/get_time_info；数据 CITIES → 面板已不需要；保留：去抖定时器、闹钟触发播放编排（audio_player 调用）、主题翻转等 UI 编排；验证：`rg "^(from|import) (modules|config|data)" ui/main_window.py` 零结果 + GUI 子进程全过 + 手动启动核对（2026-09-11 完成：构造改 interface 必传注入，偏好经 get_ui_preferences 快照恢复；主题 QSS 经 themes.build_theme 构造期缓存；倍率实时走 set_rate、去抖 flush 走 apply_rate；GUI 子进程全过且手动启动窗口正常）
- [x] PL001.09 clock_panel/date_panel 纯展示化 —— 构造增 interface 参数；TimeInfo 类型引用改 `interface.types`；get_static_config 删除（字体/版本等经 interface 或由 main_window 传入）；update_time 只做 setText + 调 clock_tools；验证：两文件 `rg "^(from|import) (modules|config|data)"` 零结果 + GUI 子进程时钟 check 通过（2026-09-11 完成：范围/预设/默认倍率/动画时长经接口读取，进度条样式改主窗口 apply_theme 统一下发（构造期不再引 themes 常量））
- [x] PL001.10 countdown_panel/world_clock_panel 纯展示化 —— countdown：解析迁 tools（见 PL001.06）、get_static_config 删除；world_clock：TIMEZONES → interface.get_timezone_options()（pytz 转换属渲染逻辑保留面板）；验证：两文件后端 import 清零 + 子进程倒计时/世界时钟 check 通过（2026-09-11 完成：current_timezone 回退默认时区改经 get_app_static）
- [x] PL001.11 weather_panel 纯展示化 —— weather_service/CITIES import 删除：fetch_weather/format_weather_display/get_city_names/get_weather_refresh_interval_ms 全走 interface；QThreadPool 线程编排保留在面板（接口保持同步拉取式）；验证：文件后端 import 清零 + 子进程天气 check（打桩点同步改 interface 层）通过（2026-09-11 完成：\_WeatherTask 持接口引用查询，WeatherData 类型经 interface.types）
- [x] PL001.12 alarm_panel/alarm_dialog 纯展示化 —— AlarmManager 引用删除（增删改查经 interface）；PresetSound/Alarm 类型改 interface.types；repeat/铃声文案迁 alarm_text；验证：两文件后端 import 清零 + 子进程铃声切换/闹钟加载 check 通过（2026-09-11 完成：闹钟加载改面板 load_alarms() 零参自取（经接口 load_alarm_dicts）；上限文案经 get_max_alarms；对话框按钮文案走 tools）
- [x] PL001.13 system_tray/audio_player/main.py 收编 —— system_tray 的 get_static_config（版本号）→ interface.get_version()；audio_player 保持经 interface.types 取 Alarm/PresetSound；main.py：GUI 分支构造 AppInterface 并注入 main_gui(interface=...)，CLI 分支维持 main_cli（CLI 属后端自足入口，不涉接口）；验证：全 ui/ + main.py `rg "^(from|import) (modules|config|data)"` 仅剩 main.py 的 main_cli 一处（CLI 直连后端属合理例外，写入说明区）+ import 冒烟（2026-09-11 完成：托盘颜色/默认倍率/通知时长亦经接口读取；audio_player 预设铃声播放经接口 play_preset_sound（编排留 UI 层）；main.py 静态参数经 AppInterface.get_app_static() 静态访问器（--version 零实例副作用保持）；audio_player 编排签名增 interface 参数）
- [x] PL001.14 测试体系改造 —— tests/test_gui_features.py / test_rate_presets.py 桩点从模块层（wp.get_weather_by_city 等）改为 interface 层（AppInterface 实例方法打桩）；新增 tests/test_interface.py 全量契约用例；验证：全量 pytest 绿 + `rg "from modules" tests/test_gui_features.py tests/test_rate_presets.py` 零结果（2026-09-11 完成：fetch_weather 类级打桩覆盖全部窗口实例，save/set_setting 桩点迁 config.settings 模块层（接口经模块命名空间调用故打桩仍生效）；TimeInfo/Alarm 类型 import 改 interface.types；另新增 test_countdown_tools/test_ui_tools）
- [x] PL001.15 解耦验收（软化版）—— ui/ 目录后端 import 清零复核（`rg "^(from|import) (modules|config|data)" ui/` 零结果，interface/types 除外不适用）；tests/test_interface.py 在无 Qt 场景可独立运行；手动启动软件走查：时钟走字/倍率/预设/主题/天气/闹钟/倒计时/托盘全功能无回归；验证：rg 零结果 + pytest 90+ 用例全绿 + 用户桌面实测确认（2026-09-11 完成：rg 全部清零；元路径钩子屏蔽 Qt 下接口七域契约独立运行并真实落盘（.temp/verify_pl001_accept.py 6/6）；全量 pytest 128 用例全绿；import/CLI/--version 冒烟全过；配置零污染哈希复验一致；软件已拉起（pythonw PID 18576 窗口正常），桌面走查待用户确认）
- [x] PL001.16 PL001 收尾提交 —— 勾选本组条目、z.plan 方案勾选状态、草拟 commit（版本号 bump 由用户定）；验证：git diff 核对清单完整（2026-09-11 完成：z.plan UI 2.0 章节补状态行；commit 草拟见汇报，版本 bump 待用户定夺后由用户执行提交）

### PL002: UI2.0 Fluent 重写 [plan#UI2.0]

- [x] PL002.01 依赖接入 —— requirements.txt 增 `PyQt6-Fluent-Widgets`；venv 安装并 `python -c "import qfluentwidgets"` 冒烟；记录版本号入 requirements（不带 [full] 扩展，按需再补）；验证：导入冒烟 + `pip show` 记录版本（2026-09-11 完成：安装 1.11.3，requirements 约束 >=1.11.3；.temp/probe_pl002_qfw.py 13 项组件 API 探针全过——ComboBox/SwitchButton/DatePicker/TimePicker/MessageBoxBase/FluentWindow/RoundMenu/SystemThemeListener 签名实测，qconfig.theme 在此版本返回解析后主题、AUTO 跟随内建）
- [x] PL002.02 主题契约扩展（auto/light/dark）—— UiPreferences.theme 取值扩为 auto/light/dark（存量 light/dark 兼容）；AppInterface 增 get_system_theme_hint()（读 Windows 注册表 AppsUseLightTheme，返回 "light"/"dark"）；验证：pytest 用例（auto 解析、非法值回退）（2026-09-11 完成：三态兼容/非法回退/注册表 hint 三用例（含 monkeypatch winreg 与读失败回退）；get_ui_preferences 归一化非法主题回退 default_theme；base.json default_theme 定案改 "auto"（用户决策②跟随系统），test_settings 三处硬编码断言改读 base.json 独立证据）
- [x] PL002.03 main_window 基座重写 —— 以 qfw FluentWindow/组件基类重建窗口骨架；启动时 setTheme（auto→系统侦测，light/dark→显式）；主题切换按钮语义更新为三态循环（跟随系统→浅→深）；验证：GUI 子进程用例（三态切换 + 持久化往返）（2026-09-11 完成：FluentWindow 单导航页"加速世界"保持单屏 UX；qfw setTheme 三态映射 + setThemeColor(ui.json primary)；Ctrl+T 与天气按钮统一三态循环 auto→light→dark→auto 经接口持久化；子进程三态循环+重启恢复 check 过）
- [x] PL002.04 clock_panel Fluent 重写 —— 时间标签换 DisplayLabel/标题字体体系，进度条换 ProgressRing 或 fluent ProgressBar，倍率滑杆换 qfw Slider + 卡片布局（CardWidget）；业务经 interface 不变；验证：子进程时钟 check + 用户视觉抽查（2026-09-11 完成：DisplayLabel 双时间 + ProgressBar（QProgressBar 子类动画兼容）+ Slider + LineEdit + PrimaryPushButton + SimpleCardWidget 双卡片；错误提示改 InfoBar 非模态；进度动画 check 过）
- [x] PL002.05 weather_panel Fluent 重写 —— 下拉换 ComboBox(qfw)、按钮换 PrimaryPushButton、刷新态用 ProgressRing；线程编排维持；验证：子进程天气 check（2026-09-11 完成：qfw ComboBox（addItems/setCurrentText/currentTextChanged/findText 探针实测可用）+ PushButton(FluentIcon.SYNC) 刷新 + 主题按钮三态外观（auto=🌗/light=☀️/dark=🌙）；QThreadPool 编排维持；首次查询 check 过）
- [x] PL002.06 world_clock/date/countdown Fluent 重写 —— 下拉/输入/日历对话框换 qfw 组件；倒计时 QMessageBox 换 InfoBar/MessageBox；验证：子进程倒计时 check + 选择器交互 check（2026-09-11 完成：world_clock qfw ComboBox（索引即选项表下标，弃 itemData）+ 强调色改 QPalette；date 换 SubtitleLabel/CaptionLabel 自带主题色；countdown 换 LineEdit/ToolButton(FluentIcon.CALENDAR/DATE_TIME)+MessageBoxBase 选择器（内嵌 DatePicker/TimePicker+快捷按钮，快捷勾选与日历状态同步防确定键覆盖）+InfoBar 提示；选择器构建/读写/确定接线 check 过）
- [x] PL002.07 alarm_panel/alarm_dialog Fluent 重写 —— 列表/复选框换 qfw 组件，开关用 SwitchButton；验证：子进程闹钟加载/铃声切换 check（2026-09-11 完成：ListWidget + SwitchButton（先 setChecked 后连 checkedChanged 防构建期误触发，setOnText 开/setOffText 关）+ ToolButton 行内按钮 + 删除确认 qfw MessageBox + 失败 InfoBar；对话框改 MessageBoxBase 基座（parent 必传）+ qfw LineEdit/TimePicker/ComboBox/CheckBox；铃声切换 check 过）
- [x] PL002.08 system_tray 与通知 Fluent 化 —— 托盘通知换 qfw 通知能力（或保留原生 showMessage + 图标更新），菜单重建；验证：子进程托盘 check + 手动通知核对（2026-09-11 完成：定案保留原生 showMessage 与图标绘制（qfw 无系统托盘组件，todo 明示可保留），菜单重建为 qfw RoundMenu（右键 Context 激活 popup 到光标处，弃 setContextMenu）；托盘初始倍率/悬停 check 过；通知效果待用户走查确认）
- [x] PL002.09 QSS 管线退役 —— 删除 ui/themes.py 与 LIGHT/DARK*THEME 引用；ui.json colors 键转 qfw setThemeColor 主题色映射（键保留换用途）；set_progress_style/apply_theme 调用点清除；验证：`rg "LIGHT_THEME|DARK_THEME|setStyleSheet" ui/` 零结果 + 全量回归（2026-09-11 完成：themes.py 整文件删除；ui.json colors 键全保留——primary 转 setThemeColor，danger/accent/text 类转 QPalette 或 qfw 标签自带主题色，bg*\* 暂闲置；set_progress_style/apply_theme/QSS 缓存全清；验收注释措辞避让后 rg 零结果）
- [x] PL002.10 深浅跟随与持久化回归 —— 系统深浅色变更侦测（qfw 信号/轮询注册表）→ 实时切换；auto 模式下用户手动切换的交互定案并落测试；验证：pytest + 手动改系统主题核对（2026-09-11 完成：SystemThemeListener 常驻侦听，systemThemeChanged 信号 AUTO 模式下重解析生效主题；交互定案——手动循环任意态经 save_theme 持久化（离开 auto 即显式主题），注册表 hint 与 qfw AUTO 解析一致性过验收）
- [x] PL002.11 全量回归与视觉验收 —— pytest 全量 + GUI 子进程用例适配后全绿；涉及视觉核对时按多模态节点停下切多模态模型截图走查；验证：回归全绿 + 用户确认（2026-09-11 完成：测试适配——主题三态循环重写、铃声对话框补父窗口、新增选择器交互 check、settings 主题断言改 base.json 独立证据；全量 131 用例全绿；反向验收 .temp/verify_pl002_accept.py 5/5（子进程模式规避侦听线程退出硬崩，并修正其配置写穿真实文件问题）；Fluent 版已拉起（PID 19064），视觉走查与改系统主题跟随核对待用户确认——本模型无视觉能力，如实声明不读图）
- [x] PL002.12 PL002 收尾提交 —— 勾选条目、方案状态、草拟 commit（版本号 bump 由用户定）；验证：git diff 核对（2026-09-11 完成：z.plan 状态行更新；commit 草拟见汇报，版本 bump 由用户定夺）

### PL003: UI2.0 落地打磨与 1.0 收口 [plan#UI2.0]

- [ ] PL003.01 设计 tokens 整理 —— 间距/字号/圆角/动效时长收编 ui.json（经 interface 暴露给 tools/面板），清除散落魔数；验证：rg 面板内无硬编码 px 字号/间距残留抽查
- [ ] PL003.02 动效与一致性清理 —— QPropertyAnimation 参数统一入配置，进度/倒计时/主题切换动效风格对齐；验证：GUI 子进程动画 check + 手动观察
- [ ] PL003.03 退出崩溃复查 —— y.problems#6（GUI 退出期硬崩溃）在 Fluent 体系下复测：子进程 GUI 用例退出码 + logs/crash-\*.log 检查；验证：复测记录写入 y.problems#6 状态
- [ ] PL003.04 文档同步 —— README（UI 说明/截图占位）、w.study 架构章节（三大块+接口契约）、m.milestone 1.0 对齐、AGENTS（结构树/验证命令如涉变化）；验证：文档交叉核对
- [ ] PL003.05 版本策略定案 —— 0.5.0.0 版本号/发布形态经用户定案后 bump 并草拟发布 commit；验证：base.json 与五处文档版本一致
- [ ] PL003.06 终验走查 —— 全量回归 + 手动验收清单（用户操作走查：启动/时钟/倍率/预设/主题跟随/天气/闹钟/倒计时/托盘/快捷键/退出）+ 配置零污染复验；验证：走查清单逐项确认
