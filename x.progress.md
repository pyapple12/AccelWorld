# 重构进度追踪（x.progress.md）

> 依据：`z.plan.md`（AccelWorld 审计与重构方案报告）
> 当前版本：0.4.7.0（四段式首版；S10 第三轮审计修复 P0-P5 全部完成）
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

- [ ] T001.1 [P1] 加速时间刷新频率随倍率变化（1/rate 秒节奏） —— `get_custom_time()` 秒级缓存按标准时间秒做键导致界面每标准秒只变一次；改法：GUI/CLI tick 间隔按 `1000ms / rate` 动态计算（`clock_tick_ms` 参数化），配合缓存键改用加速时间秒；验证：单元测试断言倍率 2.0/10.0 下 `TimeInfo.custom_second` 变化周期 + GUI 无头初始化
- [ ] T001.2 [P2] 倒计时日期选择器实时反馈 —— `countdown_panel.show_date_picker()` 中快捷按钮 `setSelectedDate` 后即时写回 `countdown_target` 输入框，日历 `clicked` 信号连接同步写回，不等 `dialog.exec()` Accepted；验证：GUI 无头初始化 + 手动核对快捷按钮与日历点击均即时勾选
- [ ] T001.3 [P3] 时间选择框显示不全 —— `show_time_picker()` 的 `QTimeEdit.setFixedSize(120, 40)` 宽度放不下 `HH:mm:ss`；改法：宽度按 `sizeHint` 自适应或加宽常量入 `ui.json`；验证：GUI 无头初始化 + 截图核对时分秒完整可见

### T002: 运行监控体系 [problems#4]

- [ ] T002.1 [P2] 全局异常钩子 —— `main.py` 加 `sys.excepthook` → `logger.critical(..., exc_info=True)`，Python 未捕获异常（含 Qt 槽内抛错）不再静默；验证：.temp 探针触发异常，断言日志文件含 traceback
- [ ] T002.2 [P2] Qt 原生警告转发 —— `qInstallMessageHandler` 将 Qt 警告（QSS 解析失败等）按级别转发进 logger；验证：探针触发 Qt warning，断言落日志
- [ ] T002.3 [P3] 原生崩溃栈落盘（可选） —— `faulthandler.enable(file=...)` 崩溃栈写入 logs/ 专用文件；验证：仅手工执行探针验证文件产出

### T003: UI 美化方向决策 [problems#5]

- [ ] T003.1 [P3] 体验 PyQt-Fluent-Widgets 官方 demo —— 安装并运行官方 demo 实际感受 Fluent 视觉与组件形态；验证：体验结论记录到 y.problems.md#5
- [ ] T003.2 [P3] 定方向 —— 三选一：UI 2.0 立项（Fluent Widgets 重写 UI 层，呼应主题商店 F03c03）/ 低成本方案（PyQtDarkTheme 或 QtTheme 配合现有 themes.py 管线）/ 短期维持现状；验证：结论写入 y.problems.md 并更新状态，若立项则在 z.plan.md 建方案章节

### T004: 短期功能项 [plan#未完成功能项路线图]

- [ ] T004.1 [P2] 快捷键支持（M09c/F02a01） —— QShortcut 绑定 Ctrl+S 保存、Ctrl+Q 退出、Ctrl+T 主题切换，键位常量入 config/static；验证：GUI 无头初始化 + 手动核对三快捷键
- [ ] T004.2 [P2] 倍率预设方案（M09e/F02a04） —— 预设工作/睡眠/专注模式（倍率组合），预设定义入 config/static，面板加快捷切换；验证：单元测试断言预设切换后配置生效
- [ ] T004.3 [P3] 进度条动画（F01b02） —— QPropertyAnimation 平滑过渡替代 setValue 跳变，动画时长入 base.json；验证：GUI 无头 + 手动观察过渡效果
- [ ] T004.4 [P3] 托盘 toolTip 实时化（F01c03/M09b） —— setToolTip 随时钟 tick 显示当前加速时间/倍率（当前为静态文本）；验证：手动核对托盘悬停内容随时间变化
