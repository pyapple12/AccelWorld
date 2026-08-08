# 重构进度追踪（x.progress.md）

> 依据：`z.plan.md`（AccelWorld 审计与重构方案报告）
> 当前版本：ver 0.43 → 重构完成后升级为 ver 0.44（M08 架构优化）
> 记录格式：状态 [⏳ 待开发, ✅ 已完成] / 优先级 [高, 中, 低]
> 执行原则：每阶段完成后运行验证命令确认无回归，再进入下一阶段

---

## S1 结构骨架（对应 z.plan.md 第四章）

> 目标：建立 `modules/config/ui/data/utils` 包结构，按 z.plan.md 4.2 迁移文件，逻辑不变仅移动+改 import

### S1.1 包结构创建

- [x] S1.1.1 创建 `modules/`、`config/`、`ui/`、`data/`、`utils/` 五个包目录及各 `__init__.py`（对应 z.plan.md 4.2 目录树）
- [x] S1.1.2 创建 `main.py` 入口，收编 accelworld.py 的 CLI/GUI 分发逻辑
- [x] S1.1.3 保留根目录 `accelworld.py` 作为兼容转发壳（`from main import main; main()`），更新启动文档
- [x] S1.1.4 根目录 `__init__.py`（包标记）
- 状态：✅ 已完成｜优先级：高

### S1.2 业务核心层迁移（modules/）

- [x] S1.2.1 `accelworld_calc.py` → `modules/time_dilation.py`（AcceleratedWorld 类 + CLI 实时钟）
- [x] S1.2.2 `accelworld_date.py` → `modules/chinese_calendar.py`（农历/干支/时辰/节气/节日）
- [x] S1.2.3 `accelworld_weather.py` → `modules/weather_service.py`（天气获取，缓存/重试在 S5 补）
- [x] S1.2.4 `accelworld_alarm.py` → `modules/alarm_service.py`（Alarm/AlarmManager/播放）
- [x] S1.2.5 `VERSION` 常量收敛到 `main.py` 单一来源（修复 D7），modules/ui 改为引用
- 状态：✅ 已完成｜优先级：高

### S1.3 配置层迁移（config/）

- [x] S1.3.1 `accelworld_config.py` → `config/settings.py`
- [x] S1.3.2 配置路径改用 `pathlib`（修复 os.path 违规）
- 状态：✅ 已完成｜优先级：高

### S1.4 GUI 层迁移（ui/）

- [x] S1.4.1 主题 QSS 常量（LIGHT_THEME/DARK_THEME/进度条样式）→ `ui/themes.py`（对应 M08b）
- [x] S1.4.2 `AlarmEditDialog` → `ui/alarm_dialog.py`
- [x] S1.4.3 主窗口类保留在 `ui/main_window.py`（面板化拆分在 S4 完成）
- [x] S1.4.4 消除函数内延迟 import（结构分层后全部收敛到文件头）
- 状态：✅ 已完成｜优先级：高

### S1.5 数据层迁移（data/）

- [x] S1.5.1 `CITIES` 城市表 → `data/cities.py`
- [x] S1.5.2 `WEATHER_CODES`/`WEATHER_DESCRIPTIONS` → `data/weather_codes.py`
- [x] S1.5.3 时区表（gui.py:476-485）→ `data/timezones.py`
- 状态：✅ 已完成｜优先级：高

### S1.6 通用工具层（utils/）

- [x] S1.6.1 `utils/logger.py`：统一日志配置（控制台+文件双 handler，修复 D12）
- [x] S1.6.2 `utils/file_utils.py`：pathlib 封装 JSON 读写 + 缓存单例（参考 DeepTransHub load_config）
- [x] S1.6.3 `utils/retry.py`：泛型重试函数（参考 DeepTransHub error_handler.retry_call）
- 状态：✅ 已完成｜优先级：高

### S1.7 验证

- [x] S1.7.1 导入验证：`python -c "import main, modules.time_dilation, modules.chinese_calendar, modules.weather_service, modules.alarm_service, config.settings, ui.main_window, data.cities, data.timezones, data.weather_codes, utils.logger, utils.file_utils, utils.retry"`
- [x] S1.7.2 CLI 冒烟：`python main.py --version`、`python main.py --cli --rate 2.0`（短暂运行 Ctrl+C 退出）
- [x] S1.7.3 旧入口兼容：`python accelworld.py --version`
- 状态：✅ 已完成｜优先级：高

### S1 备注（提前完成的 S3 项）

- [x] S3.2.1（提前完成）D1：CLI 传参改直接函数调用 `main_cli(rate=...)`，删除改写 `sys.argv` 的 hack
- [x] S3.2.2（提前完成）D2：删除 time_dilation 模块 `__main__` 启动 GUI 的入口（职责归 main.py）
- [x] S3.3.1（提前完成）B4：删除 `countdown_time` QTimeEdit 死代码
- [x] S3.3.4（提前完成）删除 config/weather/alarm 的 `__main__` 测试块（测试职责移交 S7 pytest）

---

## S2 数据规范化（对应 z.plan.md 第六/五章）

> 目标：引入 dataclass 消除超长元组与裸 dict，数据表外置，配置缓存单例

### S2.1 dataclass 引入

- [x] S2.1.1 `TimeInfo` dataclass（modules/time_dilation.py），替代 `get_custom_time()` 7 元组返回
- [x] S2.1.2 `LunarInfo` dataclass（modules/chinese_calendar.py），替代 `get_chinese_lunar_calendar()` 10 元组返回
- [x] S2.1.3 `WeatherData` dataclass（modules/weather_service.py），含 `to_display()`，替代裸 dict
- [x] S2.1.4 `AppConfig` dataclass（config/settings.py），替代 `DEFAULT_CONFIG` 裸 dict，含 `to_dict()/from_dict()`
- 状态：✅ 已完成｜优先级：高

### S2.2 数据表规范化

- [x] S2.2.1 天气表合并：`WEATHER_CODE_INFO` 单表（含中文/英文/图标/描述四字段），消除两表重复维护（修复 M06 遗留）
- [x] S2.2.2 `holiday_translation` 提升为模块级常量（修复 D8）——已在 S1 迁移时完成（`HOLIDAY_TRANSLATION`）
- [x] S2.2.3 常量表命名统一 `UPPER_CASE`
- 状态：✅ 已完成｜优先级：中

### S2.3 配置层规范化

- [x] S2.3.1 `config/settings.py` 走 `utils/file_utils.py` 缓存单例，`load_config` 只读一次文件（修复 D4）
- [x] S2.3.2 删除无意义的 `get_config_dir()/get_config_file()` 包装
- [x] S2.3.3 `save_window_geometry` 改 base64 存储（修复 D3，含旧 latin1 格式兼容）
- 状态：✅ 已完成｜优先级：中

### S2.4 验证

- [x] S2.4.1 导入验证 + CLI 冒烟（同 S1.7）
- [x] S2.4.2 手动验证配置读写往返一致（旧 config.json 可加载）
- 状态：✅ 已完成｜优先级：高

---

## S3 Bug 修复（对应 z.plan.md 第三/七章 P0）

> 目标：修复审计发现的全部功能 Bug 与死代码

### S3.1 功能 Bug

- [x] S3.1.1 修复 B1：`main_gui` 应用 `rate`/`city` 启动参数（含 `is_dark_theme` 初始化顺序）——提取 `apply_startup_args()` 方法
- [x] S3.1.2 修复 B2：托盘"退出"前调用 `save_settings()`（新增 `quit_app()`）；倍率变化经 `_update_acceleration_rate()` 同步保存
- [x] S3.1.3 修复 B3：`show_time_picker` 裸 `except:` 改为 `except (ValueError, TypeError)`
- 状态：✅ 已完成｜优先级：高

### S3.2 设计缺陷

- [x] S3.2.1 修复 D1：CLI 传参改直接函数调用 `main_cli(rate=...)`，删除改写 `sys.argv` 的 hack（S1 提前完成）
- [x] S3.2.2 修复 D2：删除 calc 模块 `__main__` 启动 GUI 的入口（职责归 main.py）（S1 提前完成）
- 状态：✅ 已完成｜优先级：中

### S3.3 死代码清理

- [x] S3.3.1 删除 B4：`countdown_time` QTimeEdit 死代码（S1 提前完成）
- [x] S3.3.2 删除 D9：`DEFAULT_CITY` 未使用常量
- [x] S3.3.3 删除 `AcceleratedWorld.start_time` 未使用属性（S1 迁移时已删除）
- [x] S3.3.4 删除 config/weather/alarm 的 `__main__` 测试块（测试职责移交 S7 pytest）（S1 提前完成）
- 状态：✅ 已完成｜优先级：低

### S3.4 验证

- [x] S3.4.1 手动 GUI 验证：`--rate 3.0 --city 上海` 生效；调倍率→托盘退出→重启后设置保留
- [x] S3.4.2 CLI 冒烟验证
- 状态：✅ 已完成｜优先级：高

---

## S4 GUI 面板化（对应 z.plan.md 4.2 ui/panels/ 与 M08a）

> 目标：`ui/main_window.py` 只做装配，六个面板独立文件，托盘/对话框独立

### S4.1 面板拆分

- [x] S4.1.1 `ui/panels/clock_panel.py`：时钟显示 + 参数标签 + 进度条（时钟刷新逻辑迁入）——倍率设置区（滑杆/输入框/应用按钮）一并迁入
- [x] S4.1.2 `ui/panels/date_panel.py`：中文日期 + 农历标签
- [x] S4.1.3 `ui/panels/countdown_panel.py`：倒计时输入 + 日期/时间选择器（收编 M07 成果）
- [x] S4.1.4 `ui/panels/world_clock_panel.py`：时区下拉 + 世界时间
- [x] S4.1.5 `ui/panels/weather_panel.py`：城市选择 + 天气显示 + 主题切换按钮
- [x] S4.1.6 `ui/panels/alarm_panel.py`：闹钟列表 + 增删改入口
- 状态：✅ 已完成｜优先级：高

### S4.2 主窗口瘦身

- [x] S4.2.1 `ui/main_window.py` 改为装配各面板 + QTimer 调度 + 主题切换入口
- [x] S4.2.2 面板间通信用 signal/slot 解耦：`rate_changed`/`theme_toggled`/`alarm_saved`/`alarm_triggered`
- 状态：✅ 已完成｜优先级：高

### S4.3 托盘与对话框

- [x] S4.3.1 `ui/system_tray.py`：托盘图标绘制、菜单、通知（独立类，show/hide/quit 信号）
- [x] S4.3.2 `ui/alarm_dialog.py` 完善：补类型注解、`get_alarm()` 返回 `Alarm` dataclass（保留原 ID）
- 状态：✅ 已完成｜优先级：中

### S4.4 验证

- [x] S4.4.1 GUI 冒烟：启动、切主题、设倒计时、加闹钟、托盘双击/退出
- 状态：✅ 已完成｜优先级：高

---

## S5 后台化（对应 z.plan.md 第七章 P1/P2 与 M09a）

> 目标：网络与声音播放不阻塞 GUI 线程

### S5.1 天气后台化

- [x] S5.1.1 天气请求移入 `QThreadPool`（`_WeatherTask(QRunnable)`，启动加载与 30 分钟定时刷新均走后台）
- [x] S5.1.2 30 分钟内存缓存（`weather_service` 层实现，修复 D5，仅缓存成功结果）
- [x] S5.1.3 网络请求接入 `utils/retry.py` 重试（总尝试 3 次，URLError/TimeoutError）
- 状态：✅ 已完成｜优先级：高

### S5.2 闹钟播放后台化

- [x] S5.2.1 闹钟声音播放移入后台线程（`play_alarm_sound_async`：预设铃声 daemon 线程，自定义铃声主线程，修复 D6）
- [x] S5.2.2 `play_preset_sound` 函数内 `import time` 移出（修复 D11，`time_module` 别名避免与 datetime.time 冲突）
- 状态：✅ 已完成｜优先级：中

### S5.3 验证

- [x] S5.3.1 GUI 冒烟：断网启动不卡 UI（0ms 返回）；闹钟触发时界面可操作
- 状态：✅ 已完成｜优先级：高

---

## S6 规范补齐（对应 z.plan.md 第二章审计与 M08d）

> 目标：全部文件符合 AGENTS.md 代码规范

### S6.1 注释规范（全部 .py 文件）

- [x] S6.1.1 每个函数定义下方补 `#` 注释（1-3 行，说明用途和核心逻辑）——补齐迁移文件（time_dilation/chinese_calendar/alarm_service/settings）
- [x] S6.1.2 每个文件末尾补 `# ===== 文件名 函数/类说明 =====` 说明区（输入/输出/逻辑步骤/设计理由/异常处理/关联配置）——补齐 main/accelworld/time_dilation/chinese_calendar/alarm_service/settings
- [x] S6.1.3 私有方法统一 `_` 前缀（`_get_repeat_display`/`_get_sound_display`；`update_clock` 等为 Qt 回调保留公开）
- 状态：✅ 已完成｜优先级：高

### S6.2 类型注解

- [x] S6.2.1 gui 层方法补齐参数/返回类型注解（`closeEvent(a0: QCloseEvent)`、`_on_alarm_triggered(alarm: Alarm)`、`_on_weather_result`、`main()`）
- [x] S6.2.2 基于 basedpyright 校验（不修改 pyproject.toml 现有放宽配置）——LSP 仅剩第三方 stub/PyQt 索引问题，运行时全部验证通过
- 状态：✅ 已完成｜优先级：中

### S6.3 代码风格

- [x] S6.3.1 `if args.rate:` 改 `is not None` 精确判断（main.py 三处）
- [x] S6.3.2 函数内重复 import 清理（datetime/os/time/traceback/winsound 全部移出）
- [x] S6.3.3 `"".join([...])` 去方括号等推导式优化
- 状态：✅ 已完成｜优先级：低

### S6.4 验证

- [x] S6.4.1 逐文件 review：注释规则/命名/import 顺序/行宽 100（修复 time_dilation 2 处、chinese_calendar 1 处超宽行）
- 状态：✅ 已完成｜优先级：高

---

## S7 测试引入（对应 z.plan.md 第八章 S7 与 M08c）

> 目标：pytest 覆盖核心模块

### S7.1 测试框架

- [ ] S7.1.1 添加 pytest 依赖 + `tests/` 目录
- 状态：⏳ 待开发｜优先级：中

### S7.2 测试用例

- [ ] S7.2.1 `time_dilation`：倍率校验、自定义时间计算、24h 边界、`TimeInfo` 字段
- [ ] S7.2.2 `chinese_calendar`：`LunarInfo` 各字段（干支/生肖/时辰/节气/节日）、已知日期断言
- [ ] S7.2.3 `config.settings`：默认配置、读写往返、损坏 JSON 容错、缓存
- [ ] S7.2.4 `alarm_service`：重复规则、一次性闹钟、触发去重、max_alarms 上限
- [ ] S7.2.5 `weather_service`：格式化为空容错、缓存命中
- 状态：⏳ 待开发｜优先级：中

### S7.3 验证

- [ ] S7.3.1 `pytest` 全部通过
- 状态：⏳ 待开发｜优先级：高

---

## 收尾

- [ ] 更新 `workingboard/2. Missions.md`：M08 各子项勾选完成，版本升级 ver 0.44
- [ ] 更新 README（启动方式 `python main.py`、新目录结构）
- [ ] 按 OpenSpec 流程归档变更（如适用）
- 状态：⏳ 待开发｜优先级：中

---

## 阶段验证命令速查（AGENTS.md 运行与验证）

```powershell
# 导入验证
.\.venv\Scripts\python.exe -c "import accelworld_calc, accelworld_date, accelworld_config, accelworld_weather, accelworld_alarm"

# 版本
.\.venv\Scripts\python.exe accelworld.py --version

# CLI 冒烟
.\.venv\Scripts\python.exe accelworld.py --cli --rate 2.0
```
