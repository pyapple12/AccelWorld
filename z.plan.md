# AccelWorld 重构方案与进度（z.plan.md）

> 依据：2026-08-08 项目审计（对照 AGENTS.md 规范，参考 DeepTransHub 分层结构）
> 状态：**S1-S10 全部完成并发布（ver 0.46）**，无未完成项（重构期收官）
> 更新（2026-09-10）：接入 DeepTransHub 工作流体系；本文件新增「审计观察项豁免定案清单」与「附录（审计报告归档区）」，旧内容原样保留
> 进度明细见 `x.progress.md`，本文件保留执行要点与历史方案记录

---

## 已完成 ✅（S1-S10 全部完成）

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
| S10 第三轮审计修复 | P0 严重 Bug（A1 倍率边界/A2 闹钟容错）、P1 配置恢复、P2 分层修正（日志解耦/音频迁 ui 层）、P3 遗漏小错 E1-E15、P4 死代码与细节、P5 注释规范（docstring 全量清理）、版本号迁 base.json、类型检查策略收紧 |

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

## 第三轮审计发现与修复（S10，2026-08-08，已全部实施）

> 全量审计（33 源码文件 + 测试 + 静态配置）发现 P0-P5 六组问题，已全部修复并通过验证；详细清单见 git 提交历史

### S10 修复摘要

- P0 严重 Bug（A1/A2）：倍率 rate_min 边界崩溃（改读静态配置）、闹钟 null 字段容错（补捕获 TypeError），补边界回归测试
- P1 功能缺失（B1）：last_city/last_timezone 只存不读恢复，weather_panel 去硬编码
- P2 分层修正（D1/D2）：utils/logger 解除 config 反向依赖（参数传入）、音频播放迁 ui/audio_player.py（防 GC 中断）
- P3 遗漏小错（E1-E15）：注释/枚举自动生成/模块常量/类型注解/单次写盘/夏令时标注/通知时长参数化/字体与双配置同步
- P4 死代码与细节（C1-C4/F 组）：to_display/standard_second/run_cli 清理、托盘去重、滑杆粒度对齐、epilog 去重、占位中性化
- P5 注释规范：全量清理 127 处 docstring（函数/类/模块统一 # 注释体系），测试替身函数补注释
- 附加：版本号迁入 base.json 单一来源、函数内延迟 import 全部顶层化、pyproject.toml 新增 [tool.pyright] 段（4 项关键检查收紧 warning + 行级 ignore）

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

---

## 审计观察项豁免定案清单（2026-09-10 随工作流体系接入初始化）

> 豁免唯一权威源：已定案项审计时不再重复报告。新定案条目由归档环节（`.agents/skills/audit-report`）经用户确认后追加。
> 分级规则：①**永久豁免**——设计定案/用户保证/容错体系覆盖，后续轮次不再报告不再讨论；②**条件豁免**——当前条件下不可达或可接受，**触发条件变化时重新评估**（每项标注触发条件）。

### ① 永久豁免（不再讨论）

| 文件:行号 | 描述 | 定案理由 | 定案日期 |
| --------- | ---- | -------- | -------- |
| modules/alarm_service.py:193-196 | 闹钟触发去重键 _last_triggered 仅内存不持久化（同分钟内重启理论可重复响一次） | 单实例桌面应用；去重键含日期维度，触发窗口极窄（仅"精确触发分钟内重启"）；代码注释已声明接受（FIX001.21 P3#15） | 2026-09-11 |

### ② 条件豁免（触发条件变化时重新评估）

| 文件:行号 | 描述 | 触发条件 | 定案日期 |
| --------- | ---- | -------- | -------- |
| （暂无）   |      |          |          |

---

## 附录（审计报告归档区）

> 格式：`## 附录 A{NNN}：全量代码审计报告（第N轮，YYYY-MM-DD）`，编号 `A001` 起递增（取号：搜索本文档 `## 附录 A\d{3}` 最大值 +1）。
> 审计执行见 `.agents/skills/audit-project`，归档流程见 `.agents/skills/audit-report`，修复任务组 `FIX{NNN}` 写入 `x.progress.md` 未完成区。

## 附录 A001：全量代码审计报告（第 1 轮，2026-09-10；2026-09-11 归档）

> 范围：main.py + config/ data/ modules/ ui/ utils/ tests/ 全部 42 个 .py/.json 文件全文通读（三路并行子审计 + 主会话高严重度逐条亲核与行号抽查）
> 方式：只读审计，未修改任何代码；豁免定案清单当时为空，无已定案豁免项
> 状态：✅ 已修复（2026-09-11，FIX001 任务清单见 x.progress.md，版本 V0.4.7.4）

### 零、回归复核清单

> 无上轮 A 编号报告（首轮），改为对照最近 4 个版本提交复核改动引入问题。

| 来源提交 | 结论（现状/证据） |
| --- | --- |
| 2638a09（V0.4.7.0 安全加固） | ⚠️ 引入回归 1 项：gaierror 逃逸降级契约（P1-1）；write_json 越界校验对全部调用方成立（settings.py:67 + tests 3 处），无漏改 |
| 6f29d47（V0.4.7.1） | ⚠️ 漏改：T001.1 仅落地 GUI 侧动态周期，CLI sleep 硬编码残留（P2-8）；死 import 与失实注释各 1（P3-25/26） |
| 05273bd（V0.4.7.2 监控） | ✅ 装配时机、crash 日志清理、句柄防 GC 均正确；边界项 `--version` 日志副作用（P3-9） |
| bd0c5e4（V0.4.7.3） | ✅ 快捷键/预设信号链/动画/tooltip 实现正确无双发；⚠️ 子进程测试写真实用户配置无还原（P2-5，实施当日实际发生并手工还原，确证） |

### 一、P0-P3 修复清单

#### P0（启动崩溃 / 不可逆数据丢失，确定性复现）

| 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- |
| utils/file_utils.py:36 | 1 正确性 | `read_json` 异常捕获漏 `UnicodeDecodeError`（ValueError 子类，两者均不捕获）：配置含非 UTF-8 字节（GBK 记事本保存）时异常穿透 import 链，启动即崩溃 | except 补 `UnicodeDecodeError` | 新增 | 配置体系/跨模块 |
| config/settings.py:57-70 + utils/file_utils.py:59-61 | 13+2 | 损坏配置静默重置为默认值 + 任一保存即以默认值覆盖旧文件（闹钟/倒计时/城市记忆不可恢复丢失）；`write_text` 非原子写入加剧闭环 | 损坏时先转存 `.bak` 再兜底；写入改临时文件+rename 原子替换（同步改 test_corrupted_json 断言） | 新增 | 配置体系 |

#### P1（功能确定性失效）

| 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- |
| modules/weather_service.py:76, 111-117, 133 | 13 错误策略 | V0.4.7.0 引入回归：`socket.getaddrinfo` 的 `gaierror` 不在 retry_call 白名单也不在降级 except → 断网/DNS 故障 0 重试且抛异常而非承诺的 `None`（加固前经 urlopen 包装为 URLError 可重试可降级） | retry 与降级 except 均补 `socket.gaierror` | 新增（回归） | 天气服务 |
| modules/chinese_calendar.py:60 | 1 正确性 | `HOLIDAY_TRANSLATION` 仅映射 7 个 Holiday 英文名中的 2 个：春节/清明/端午/中秋/抗战胜利日泄漏英文（实测 2025-01-30 显示 "Spring Festival"） | 按库常量补全 7 项映射 | 新增 | 农历日历 |
| ui/panels/weather_panel.py:74-79, 143-149 | 1 正确性 | 默认城市启动时不触发天气查询：`setCurrentText` 在信号连接前执行、恢复 last_city 文本未变化不发射信号 → 天气区悬挂"获取天气中..."最长 30 分钟 | `__init__` 尾部或 `set_city` 未变化分支显式调 `update_weather()` | 新增 | GUI 面板 |
| ui/alarm_dialog.py:29, 56-74, 104-133 | 1 正确性 | `sound_combo` 无 `currentIndexChanged` 监听：自定义铃声闹钟选任何预设铃声被静默忽略（自定义→预设方向完全不可用）；自定义闹钟打开时按钮不回填文件名 | combo 切换回调复位 `sound_type="preset"`；编辑模式回填按钮文案 | 新增 | 闹钟服务/GUI 面板 |
| modules/time_dilation.py:53-64 | 2 防御 | 构造只校验 `rate_min` 不校验 `rate_max` 且失败 raise 不回退：脏配置（rate<1.0、非数值）经 ui/main_window.py:64 原样传入 → 启动即崩溃；超大值另致 1000Hz 轮询 | 补上限校验或超界回退 default_rate 并记日志 | 新增 | 时间膨胀/主流程编排(main) |

#### P2

| 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- |
| modules/alarm_service.py:116-120 + ui/panels/alarm_panel.py:154-160 | 2 防御 | `created_at=None`（脏配置可构造成功）时 `fromisoformat` 抛 `TypeError` 穿透 QTimer 槽（实测 TypeError）→ PyQt6 槽内未捕获异常将中止应用；与 S10 A2 修复模式不一致（同场景漏改） | except 改 `(ValueError, TypeError)` | 新增 | 闹钟服务 |
| utils/dataclass_utils.py:15-24 + config/settings.py:27-54 + modules/alarm_service.py:130-133 | 2 防御 | dataclass 反序列化无运行时类型校验（同根因合并）：`theme:null`、`time_dilation_rate:"abc"` 静默载入回写；`repeat_days:["1"]` 构造成功但星期匹配恒 False（实测，闹钟永不重复响） | from_dict 增加逐字段类型校验，非法回退默认 | 新增 | 配置体系/闹钟服务/跨模块 |
| ui/main_window.py:104-106, 257 + ui/panels/countdown_panel.py:167-171 | 1 正确性 | 恢复的倒计时目标未填 `countdown_target_date`（None）→ `get_target_text` 返回空 → 下一次正常退出即把持久化值清空（跨会话数据丢失） | 恢复时同步解析填充 date（不启动计时），或改"有文本即返回" | 新增 | GUI 面板 |
| ui/main_window.py:117, 251-258 | 1+3 | 主题选择完全不持久化：`is_dark_theme=False` 硬编码、save_settings 不写 theme；`base.json default_theme` 与 `UserConfig.theme` 成互喂死链路（全仓 grep 证实无其他读写点） | init 读配置、切换时持久化 | 新增 | GUI 面板/配置体系 |
| tests/test_rate_presets.py:20-92 | 12 测试卫生 | 子进程测试写真实 user_config.json 且无还原（conftest 隔离对子进程无效），每次跑 pytest 都把用户倍率改成 10.0 | 子进程内重定向配置路径（环境变量注入临时目录），与 conftest 共用机制 | 新增 | 测试 |
| ui/themes.py:89, 180 + ui/system_tray.py:41 | 3 硬编码 | QSS 按钮文本 `color: white` 与托盘指针 `QColor("white")` 硬编码（"该进配置未进"，不可豁免） | ui.json 增白色系键并接入 `_apply_colors` | 新增 | GUI 面板/托盘与音频 |
| modules/weather_service.py:119-127 | 2 防御 | Open-Meteo 响应字段缺失时默认 0：接口变更时显示"晴 0.0°C"假数据而非失败 | 字段缺失视为失败返回 None | 新增 | 天气服务 |
| modules/time_dilation.py:173, 176 | 3 硬编码 | CLI 循环硬编码 `sleep(1.0)`/`sleep(0.01)`，`clock_tick_ms` 与 `tick_interval_ms` 被 CLI 旁路（T001.1 计划口径含 CLI，漏改） | `run_live_clock` 消费 `tick_interval_ms` | 新增（漏改） | CLI/时间膨胀 |
| modules/weather_service.py:88, 115-116 | 3 硬编码 | `timeout=10`/`retries=3`/`delay=1.0` 网络参数硬编码 | 入 base.json | 新增 | 天气服务/配置体系 |
| utils/file_utils.py:42 vs 62 | 2+11 | 缓存键不一致：读用未 resolve 路径、写清缓存用 resolve 后路径（V0.4.7.0 改动残留）；当前调用方恰好全部规范化故未触发 | 三处统一 `Path(path).resolve()` 为键 | 新增 | utils 公共契约 |
| utils/logger.py:33-40 | 2 防御 | 跨天重开日志文件无异常防护：重开失败后文件日志通道静默永久失效直至重启 | 重开段包 `try/except OSError` 降级 | 新增 | 日志 |
| config/settings.py:65-70 + ui/main_window.py:170 | 10+13 | 保存失败静默：磁盘满/只读时 `save_config` 返回 False，`_save_alarms`/`save_settings` 等调用方全部忽略返回值，用户无感知且内存磁盘持续分叉 | 失败上浮 UI 提示 | 新增 | 配置体系/GUI 面板 |
| modules/alarm_service.py:114-121 + ui/alarm_dialog.py:116-117 | 1/2 语义 | 一次性闹钟"仅创建当天触发"（S8.4 定案）副作用：创建时设定时间已过今日则永不触发且 UI 持续显示启用；对话框无日期选择无法表达"明天的一次性" | **已定案修复（2026-09-11 用户确认）：创建时已过今日时间自动顺延次日触发（语义变更，需回归 S8.4 相关用例）** | 新增 | 闹钟服务 |

#### P3（低）

| # | 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | modules/alarm_service.py:78 + ui/panels/alarm_panel.py:219 | 2 | 越界 `repeat_days=[9]` 可致启动 IndexError【需验证】 | `__post_init__` 过滤 0-6 | 新增 | 闹钟服务 |
| 2 | modules/weather_service.py:66, 88 | 8 | `_OPENER` 全局复用理论性线程安全【需验证】 | 每任务构建或注释声明单线程假设 | 新增 | 天气服务 |
| 3 | main.py:44-50, 71-76 | 1 | CLI `--rate 2.05` 等非 0.1 步进值被银行家舍入静默吸附且方向不一致，帮助文本未说明粒度（实机验证） | 帮助文本注明 0.1 步进或显式提示 | 新增 | CLI |
| 4 | ui/panels/clock_panel.py:104-106 | 6 | 提示"必须大于1.0"与闭区间校验矛盾，且预设"工作 1.0x"即取该值 | 文案改"必须不小于" | 新增 | GUI 面板 |
| 5 | ui/system_tray.py:64 | 3 | 托盘菜单初始倍率硬编码 "2.0x" | 取 `base["default_rate"]` | 新增 | 托盘与音频 |
| 6 | ui/panels/weather_panel.py:143-149 | 1 | 列表外 `--city` 时下拉框显示旧城市而查询按实际城市 | combo 只读展示实际城市 | 新增 | GUI 面板 |
| 7 | ui/panels/clock_panel.py:190-215 | 9 | 滑杆拖动每格重建实例+写盘（2.0→10.0 约 80 次写盘）；`apply_acceleration` 显式 emit + setValue 再 emit 双发（幂等但冗余 IO） | sliderReleased 再发/写盘去抖；删显式 emit 统一走 `set_rate` | 新增 | GUI 面板 |
| 8 | ui/panels/countdown_panel.py:194, 243 | 9 | 选择器对话框重复打开累积未释放子 QDialog | `exec()` 后 `deleteLater()` | 新增 | GUI 面板 |
| 9 | main.py:19-22, 68 | 2 | `--version/--help` 仍创建当日日志文件（setup_logging 先于 parse_args） | setup_logging 移至 parse_args 后 | 新增 | 主流程编排(main) |
| 10 | ui/panels/world_clock_panel.py:82-84 | 13 | `logger.error` 无堆栈，与他处同类回调 `logger.exception` 风格不一 | 统一 `logger.exception` | 新增 | GUI 面板 |
| 11 | modules/time_dilation.py:47-52 | 6 | 类属性用 docstring 当注释（S10 P5 清理残留） | 改 `#` 注释 | 新增 | 时间膨胀 |
| 12 | modules/chinese_calendar.py:5 | 5 | `Tuple` import 未使用 | 删除 | 新增 | 农历日历 |
| 13 | modules/chinese_calendar.py:11 | 5 | `SHI_CHEN` 条目 `(23, 1, "子时")` 恒 False 死数据（结果靠 else 兜底，正确但误导） | 改 `(23,24)`+`(0,1)` 或注释说明 | 新增 | 农历日历 |
| 14 | modules/weather_service.py:102-108 vs 42-43 | 4 | URL 字面量与 `_API_SCHEME/_API_HOST` 白名单双源维护 | URL 用常量拼接 | 新增 | 天气服务 |
| 15 | modules/alarm_service.py:174-176 | 2 | `_last_triggered` 仅内存，同分钟重启重复响（窗口极窄） | 持久化或豁免定案 | 新增 | 闹钟服务 |
| 16 | modules/alarm_service.py:250-259 | 10 | `from_dict_list` 丢弃损坏条目无日志 | logger.warning | 新增 | 闹钟服务 |
| 17 | modules/alarm_service.py:147-149 vs 59 | 12 | 声音兜底元组与 CLASSIC 条目字面量重复（且不可达） | 兜底改引用 CLASSIC | 新增 | 闹钟服务 |
| 18 | data/timezones.py:19 | 6 | 说明区写"供 ui/main_window.py 使用"，实际消费方是 world_clock_panel | 更新说明区 | 新增 | 文档 |
| 19 | utils/logger.py:46 vs utils/monitor.py:18 | 12 | crash 前缀双源（改一处即永不清理） | 常量单源化 | 新增 | 日志 |
| 20 | utils/logger.py:15, 64 | 12+3 | backup_days 代码默认值与 base.json 双处维护；日志级别未配置化 | level 入 base.json | 新增 | 日志/配置体系 |
| 21 | config/static/static_config.py:21-28 | 13+6 | config.json 缺失/损坏抛 KeyError/AttributeError，与注释承诺的 RuntimeError 不符 | 显式校验映射表 | 新增 | 配置体系 |
| 22 | config/settings.py:23 | 5 | `CONFIG_DIR` 死常量（仅测试 monkeypatch） | 删除或让 CONFIG_FILE 派生 | 新增 | 配置体系 |
| 23 | utils/monitor.py:54-55 | 2 | excepthook 直接覆盖不链式保留原钩子（当前无冲突方） | 保存并调用 `_previous` 钩子 | 新增 | 日志 |
| 24 | config/settings.py:99-112 | 5+6 | 旧 latin1 几何格式兼容层疑似死代码【需验证存量文件】 | 验证后按"不留废弃方案"清理 | 新增 | 配置体系 |
| 25 | ui/panels/countdown_panel.py:19 | 5 | `Qt` import 死代码（V0.4.7.1 重构遗留） | 移除 | 新增（漏改） | GUI 面板 |
| 26 | ui/main_window.py:132 | 6 | "100ms 定时器驱动"注释失实（周期已随倍率联动） | 更新注释 | 新增（漏改） | GUI 面板 |
| 27 | README.md:3, 13 + x.progress.md:4 | 6 | 文档版本滞后：README 徽章 0.4.7.2、x.progress"当前版本 0.4.7.0" vs base.json 0.4.7.3 | 同步 | 新增 | 文档 |
| 28 | tests/ | 10 | T004 新功能（动画/tooltip/快捷键）仅探针验证，无沉淀断言 | 探针断言子进程化沉淀 | 新增 | 测试 |

### 二、参考级观察项（记录不修；2026-09-11 用户复核：全部维持观察级，不提升）

| 文件:行号 | 描述 | 回落理由 |
| --- | --- | --- |
| utils/monitor.py:73-75 | 重复 install_crash_handler 窗口期旧 fd 失效 | 无可达触发路径【需验证】 |
| utils/monitor.py:72 | crash 文件日期安装时固定，跨天进程写昨日文件 | 桌面应用叠加概率极低 |
| utils/file_utils.py:18-21 | 临时目录整目录放行超出最小写入面 | 有注释依据（pytest 隔离豁免） |
| config/settings.py:118-120 | get_alarms 仅隔离外层，内层 dict 共享引用 | 无可证触发路径，需 alarm_panel 侧验证 |
| utils/monitor.py:11-12 | utils 层 import PyQt6，与分层先例有张力 | 代码有注释论证，无运行时影响 |
| modules/weather_service.py:58-66 | 重定向被拒后按网络错误重试 3 次（约 2s 浪费） | 官方接口正常不重定向 |
| modules/weather_service.py:71-72 | DNS rebinding TOCTOU | 注释显式声明接受（固定官方域名） |
| modules/alarm_service.py:99, 231 | enabled 双重检查 | 无害冗余防御 |
| modules/chinese_calendar.py:89-160 | lunar-python 异常直接上抛不包装 | 符合窄捕获上抛策略，上层有单轮兜底 |
| modules/weather_service.py:172 | humidity 无小数格式化与他项不一 | 极小展示瑕疵 |
| ui/panels/clock_panel.py:165-175 | QPropertyAnimation 每 tick stop/start | 单线程无竞态、收敛正确（本轮亲核） |
| ui/main_window.py:205-215 | 窗口隐藏托盘后快捷键失效 | Qt WindowShortcut 机制固有，托盘菜单有等效入口 |
| ui/panels/clock_panel.py:215 | `int(rate*10)` vs `int(round(...))` 写法不一致 | 实机验证 [1.0,20.0] 全步进当前无差值；防御性建议统一 |
| main_window.py:284 | 未设 QApplication.applicationName | 外观类 |
| ui/panels/alarm_panel.py:85-152 | refresh_list 在信号栈内 clear+重建 | Qt 延迟删除实践安全；排查 y.problems#6 时可复查 |

### 三、亮点

- 零硬编码兑现度高：base.json 21 键、ui.json 23 键逐一核对无死键、无读取不存在键；版本字符串零硬编码
- T001.1 农历缓存键设计正确（细于全部下游依赖，无脏命中）；T004 信号链无双发、动画生命周期管理正确
- SSRF 校验链健壮：userinfo/IPv6 绕过技巧被 urlsplit 化解，URL 由已校验 float 拼接无注入面
- 测试断言质量良好（56 用例无恒真断言）；pathlib/顶层 import/# 注释体系/文件尾说明区整体合规

---

## 附录 A002：全量代码审计报告（第 2 轮，2026-09-11）

> 范围：main.py + config/ data/ modules/ ui/ utils/ tests/ 全部 45 个 .py/.json 文件全文通读（三路并行子审计 + 主会话高严重度亲核；上轮修复本身为主要审查对象）
> 方式：只读审计，未修改任何代码；豁免定案清单当时为空
> 状态：📌 待修复（FIX002 任务清单见 x.progress.md）

### 零、上轮（A001）修复复核清单

复核方式：全局 grep 关键行 + `git show ec8403f` 逐 hunk 对比 + 子审计运行时探针（节日名 8400+ 天全扫描、闹钟顺延边界、天气异常层级实测）。

| A001 条目 | 现状 | 证据 |
| --- | --- | --- |
| FIX001.1/.2/.3/.4/.8/.9/.10/.12/.13/.17/.18/.19/.20/.21/.22/.24/.25/.26 等 | ✅ 在位且实现正确（探针实证） | grep + 运行时探针 |
| FIX001.7 启动脏倍率回退 | ❌ 修复被残留代码击穿：防护构造（main_window.py:46-52）落地，但 72-73 行旧无保护构造仍执行并覆盖——越界持久化倍率仍启动即崩，回退分支成死代码 | 主会话亲核 72-73 行（P1-1） |
| FIX001.14 天气缺字段判失败 | ⚠️ 修复不完整：只查键存在，null/错型值穿透（P2-2） | weather_service.py:140-144 |
| P3#5 托盘初始倍率取配置 | ⚠️ 修复不完整：取 default_rate 而非恢复的持久化倍率（P2-3） | system_tray.py:66 + main_window.py:133,161 |
| P3#18 timezones 说明区 | ❌ 已列未修（唯一真遗留）（P3-3） | data/timezones.py:19 |
| P3#20 backup_days 双源 / P3#21 static_config | ⚠️ 部分修复（P3-2 / P3-1） | logger.py:77、static_config.py:26-31 |
| FIX001.5 / FIX001.11 / FIX001.23 | ✅ 功能生效，⚠️ 各引入一个轻微副作用（P3-6 / P2-4 / P3-9） | 见修复清单 |

### 一、P0-P3 修复清单

#### P1（确定性启动崩溃）

| 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- |
| ui/main_window.py:72-73 | 1 正确性 | A001 修复残留（三组独立发现，主会话亲核）：FIX001.7 的受保护构造（46-52）落地后，旧无保护构造 `AcceleratedWorld(time_dilation_rate=saved_rate)`（72-73）仍执行并覆盖前者。持久化倍率越界（手改 user_config.json 为 0.5/50.0，类型校验只挡类型不挡取值域）→ 73 行重抛 ValueError 无人捕获 → 启动崩溃；回退分支实为死代码，正常路径也重复构造两次 | 删除 72-73（含注释），仅保留 46-52 防护版；补"越界持久化倍率启动不崩"子进程用例 | A001 修复残留 | GUI 面板/时间膨胀/配置体系 |
| utils/dataclass_utils.py:46-48 + config/settings.py:71-76 | 1+2 | 合法 JSON 但非 dict 结构穿透：`data.items()` 在 try 外，user_config.json 内容为 `[]`/"x"/123 时 AttributeError 穿透启动崩溃；且 `data is not None` 判定放行 → FIX001.2 的 .bak 转存对这类损坏完全旁路。元素级同型：`"alarms": ["字符串"]` 穿透 tolerant 容错 | `dataclass_from_dict` 入口加 isinstance(dict) 判定；`load_config` 对非 dict data 走损坏转存分支；补用例 | 新增 | 配置体系/跨模块 |

#### P2

| # | 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | utils/dataclass_utils.py:27-29 | 2 | NaN/Infinity 穿透类型校验：json.load 默认放行非标准字面量，`"time_dilation_rate": NaN` 载入后区间比较恒 False 不拒 → `int(1000/nan)` 启动崩溃 | float 分支加 `math.isfinite`，或 read_json 用 parse_constant 拒绝 | 新增 | 配置体系/时间膨胀 |
| 2 | modules/weather_service.py:140-144 | 2 | null/错型值穿透缺字段校验（FIX001.14 残留）：只查键存在，`temperature_2m: null` 穿透后在格式化层抛 TypeError（被 UI 兜底吞掉但日志误导） | 校验键存在 + 值非 None | A001 修复残留 | 天气服务 |
| 3 | modules/weather_service.py:104,159 | 13 | 读体阶段网络异常逃逸：`response.read()` 中途断开抛 ConnectionResetError/IncompleteRead，不在重试白名单也不在降级 except，穿透到 UI 兜底 | read 纳入重试范围或 except 元组补充（收窄取舍） | 新增 | 天气服务 |
| 4 | config/settings.py:25-35 + utils/file_utils.py:57-59 | 2+11 | env 注入配置路径与写入白名单未对齐：注入项目根/临时目录之外的合法路径时保存抛 ValueError 绕过 FIX001.19 降级链直达 Qt 槽；_backup_corrupted_config copyfile 同样无白名单 | save_config 捕获 ValueError 同 OSError 策略返回 False；或注入路径预检回退 | 新增 | 配置体系 |
| 5 | utils/file_utils.py:60 | 8 | 原子写 tmp 文件名固定：双开 GUI/多进程同时保存时同 tmp 文件交错写，原子替换失效 | tmp 名加 pid 后缀或 mkstemp | 新增 | 配置体系 |
| 6 | utils/file_utils.py:71 | 2 | except OSError 内 `tmp_path.unlink(missing_ok=True)` 自身可抛 PermissionError 逃逸出 write_json，违背"IO 失败返回 False"契约 | 清理再包一层 try/except OSError | 新增 | 配置体系 |
| 7 | tests/test_gui_features.py:203-210 | 12 | 动画断言时间依赖，约 02:10 后整套 GUI 用例必失败：前置 check 把倍率设为 6.0，进度条已被 tick 驱动到实时加速小时（=当日秒数/600），断言 `start < 13` 仅当地时刻早于约 02:00 成立——当前全绿纯属提交时间巧合 | 断言改方向性收敛或 check 前把倍率置 1.0 | 新增 | 测试 |
| 8 | ui/system_tray.py:66 + ui/main_window.py:133,161 | 1+6 | 托盘菜单初始倍率显示错值：rate_action 取 default_rate，而主窗口加载的持久化倍率从不传给托盘——持久化 10.0 时菜单显示 2.0x 与悬停文本自相矛盾 | __init__ 托盘创建后补 `tray.update_rate(...)` | A001 修复残留 | 托盘与音频 |
| 9 | ui/main_window.py:307-311 | 1 | `--theme light` 在持久化深色下被静默忽略：apply_startup_args 只有 dark 分支（FIX001.11 持久化后语义缺口显形） | 补 `elif theme == "light":` 分支 | 新增（FIX001.11 副作用） | GUI 面板/主流程编排(main) |
| 10 | tests/test_rate_presets.py:38 | 12 | 预设子进程每次运行发起真实天气请求：未打桩 get_weather_by_city，联网时引入最长约 33s 尾延迟与网络依赖 | 照抄 gui_features 的天气打桩 | 新增（FIX001.5 副作用未适配） | 测试 |
| 11 | tests/test_gui_features.py:146,167-168 | 12 | 打桩泄漏无还原：tray.show_notification 实例替换、mw.set_setting 模块替换、匿名 lambda 信号连接三处无 try/finally（同脚本 save_config 桩有还原，标准不一） | 三处补 try/finally 还原 | 新增 | 测试 |

#### P3（低）

| # | 文件:行号 | 类型 | 描述 | 建议 | 性质 |
| --- | --- | --- | --- | --- | --- |
| 1 | config/static/static_config.py:26-31 | 13+6 | A001 P3-21 修复不完整：映射表缺 base/ui 键仍 KeyError、值为非串抛 TypeError、分类内容非 dict 放行 | 循环后显式校验键集与 dict 类型 | A001 修复残留 |
| 2 | utils/logger.py:77 + main.py:18 | 12 | A001 P3-20 部分修复：backup_days=7 与 base.json 仍双源；log_level 兜底 "INFO" 与 _DEFAULT_LEVEL 同值双写 | 默认值单源于 base.json 并注释声明 | A001 修复残留 |
| 3 | data/timezones.py:19 | 6 | A001 P3#18 已列未修：说明区仍写"供 ui/main_window.py 使用"，实际消费方 world_clock_panel | 更新 | 遗留 |
| 4 | utils/logger.py:31-34,47 | 4 | 日志文件名模式两处重复拼装（emit 未复用带副作用的 _today_path） | 拆无副作用 _path_for(day) 共用 | 新增 |
| 5 | config/settings.py:98 | 6 | get_setting 注释写"AppConfig"，类名实为 UserConfig | 改正 | 新增 |
| 6 | config/settings.py:71-86 | 13+9 | 损坏文件未修复期间每次 load_config 重复转存+告警（启动约 6-7 次）无节流 | 模块级标志首次后置位 | 新增 |
| 7 | modules/alarm_service.py:86-93 | 2 | repeat_days 规范化口径不一：[True]→[1] 布尔穿透、[1.7]→[1] 截断而 ["1.5"]→[] 剔除（闹钟静默翻转为一次性） | 元素层排除 bool、小数统一剔除 | 新增 |
| 8 | modules/alarm_service.py:67-69 | 11 | SUPPORTED_AUDIO_FORMATS 是 Qt 文件对话框过滤器串（;; 语法），UI 展示配置置于业务层 | 迁 ui 层或注明 | 新增 |
| 9 | ui/panels/weather_panel.py:153-156 | 9 | 列表外城市 addItem 后当次会话永久残留下拉框 | 可接受或标记临时项 | 新增（FIX001.23 副作用） |
| 10 | ui/main_window.py:87,107 + weather_panel | 9 | 启动期天气双请求（FIX001.5 副作用）：init 首查 default_city + set_city 联动查询，非默认 last_city 时第一次结果被丢弃，多打一次真实 API | set_city 先设 current_city 再首查/构造参数注入 | 新增 |
| 11 | tests/test_rate_presets.py:45 | 11 | 子进程脚本调用私有方法 `window._flush_pending_rate()`（_ 前缀约定外部不调用） | 提供公开 flush 或等待事件循环 | 新增 |

### 二、参考级观察项（记录不修，含回落理由）

**A001 观察项携带复核**：get_alarms 浅拷贝、monitor 重复安装 fd 窗口、crash 文件日期固定、临时目录放行、DNS rebinding、enabled 双检、lunar 异常直抛、QPropertyAnimation 重启、快捷键 WindowShortcut、refresh_list 信号栈、QFont 样板、int(rate*10) 写法、applicationName——原样保留无变化，按 2026-09-11 定案继续维持观察级。

**本轮新增观察项**：

| 位置 | 描述 | 回落理由 |
| --- | --- | --- |
| utils/dataclass_utils.py:35-36 | 未知注解形态放行（未来新字段类型可能绕过过滤） | 当前两 dataclass 字段全覆盖，无可达路径【需验证新增字段】 |
| utils/monitor.py:41-66 | install_excepthook 二次安装成链式套娃 | main.py 单点装配，同 A001 crash-handler 豁免口径 |
| utils/logger.py:36-53 | 重开失败期每次 emit 重试的开销 | 注释声明"自动重试直至恢复"设计取向；INFO 低频 |
| utils/file_utils.py:64-66 | 无 fsync，掉电丢最近一次保存 | 桌面应用可接受 |
| alarm_service.py:193-196 | _last_triggered 仅内存，同分钟重启理论重复响 | 已定案永久豁免（2026-09-11 用户确认，见豁免清单①），后续轮次不再报告 |
| alarm_service.py:229-235 | replace_alarm 不清理同 id 去重键 | 仅同分钟内编辑场景，无害 |
| weather_service.py:32 | _weather_cache 无锁 | GIL 下原子，最坏重复请求一次 |
| time_dilation.py:79 | 1ms tick 下限仅当 rate_max>1000 可达 | 当前配置不可达（触发条件：静态配置变更） |
| countdown_panel.py:74 vs 159 | 占位符 4 段与运行态 3 段格式不一 | 外观细节 |
| ui/audio_player.py:35-36 | QAudioOutput 局部变量疑虑 | 本轮探针证伪（C++ 侧持引用），记录防复发 |

### 三、亮点

- 三组独立交叉验证锁定同一条 P1 残留（回归复核机制有效性的直接证明）；A001 的 22/27 子任务修复实证完全在位
- 节日映射经 8400+ 天全量扫描验证与库枚举精确匹配；gaierror/URLError 异常层级、SSRF ValueError 穿透路径、闹钟顺延全部边界经运行时探针实证
- 配置键卫生保持满分：base.json 26 键、ui.json 25 键零死键零缺失；文档版本五处（README/x.progress/m.milestone/AGENTS/base.json）全部一致
- 上轮两个疑点经探针证伪（QAudioOutput GC、--version 副作用），避免无效修复

---

## UI 2.0 迭代方案（PL001–PL003，2026-09-11 立项）

> 决策（用户拍板）：①路线 A——PyQt-Fluent-Widgets 整体重写；②跟随系统深浅色；③经典 QSS 皮肤不保留（新系统新皮肤，themes.py 管线退役）；④直接立项，不做打包验证前置
> 基线：`ui1-final` 标签（V0.4.7.5）为经典 UI 技术收官存档
> 执行清单：三个 PL 的细致 todo 见 x.progress.md 对应任务组
> 状态：PL001 架构解耦 ✅（2026-09-11：interface/ 契约落地、ui/ 后端 import 清零、无 Qt 场景接口测试独立通过、软件实测运行）；PL002 Fluent 重写 ✅（2026-09-11：PyQt-Fluent-Widgets 1.11.3 落地、三态主题 auto 跟随系统、QSS 管线退役、全量回归 131 用例绿，视觉走查待用户确认）；PL003 未开始

### 架构定案：后端 / 接口 / UI 三大块

```
AccelWorld/
├── modules/ config/ utils/ data/   【后端】业务核心（不 import ui；稳定层）
├── interface/                       【接口】前后端唯一契约（新建，无 PyQt6 依赖）
│   ├── app_interface.py             AppInterface：UI 唯一的后端访问入口
│   └── types.py                     DTO 转出（TimeInfo/WeatherData/Alarm/PresetSound/UiPreferences）
└── ui/                              【UI】Fluent 重写（可独立替换）
    ├── panels/                      面板 = 纯展示（渲染数据 + 发用户动作，零计算零后端 import）
    ├── tools/                       UI 层运算（展示格式化/解析/进度换算）
    └── ...                          dialogs/tray/audio_player/themes(PL002 退役)
```

### 四条铁律

1. 面板零逻辑：运算一律在 `ui/tools/`（展示格式化/解析/进度换算）；业务运算仍归后端 modules
2. UI 不 import 后端：`modules/config/data` 的直接 import 在 ui/ 下清零（含 tools）；DTO 经 `interface.types` 转出
3. 接口双向：UI→接口（查询/动作）；后端事件（时钟 tick/闹钟触发/天气回包）由 UI 定时器经接口**拉取**（接口保持同步、无 Qt 依赖，线程编排留在 UI 层）
4. 解耦验收（软化版）：后端 + 接口的 pytest 不依赖 PyQt6 可运行；UI 层可整体替换而不改接口与后端

### 契约清单（AppInterface 方法，PL001 落地）

| 域 | 方法 | 后端实现 |
| --- | --- | --- |
| 时钟 | get_time_info() / get_tick_interval_ms() / get_version() | AcceleratedWorld / base.json |
| 倍率 | get_rate() / get_rate_bounds() / get_rate_presets() / apply_rate(rate)（校验+重建+持久化内聚） | AcceleratedWorld + settings |
| 天气 | get_city_names() / fetch_weather(city) / get_weather_refresh_interval_ms() / format_weather_display(city, weather) | cities / weather_service |
| 闹钟 | AlarmManager 所有权迁入接口：load_alarm_dicts() / save_alarm_dicts(list) / check_alarms(now) / get_max_alarms() | settings + AlarmManager |
| 时区 | get_timezone_options() | data.timezones |
| 配置 | get_ui_preferences() -> UiPreferences(theme/last_city/last_timezone/countdown_target)；save_theme / save_last_city / save_last_timezone / save_countdown_target | settings |
| 几何 | load_window_geometry() / save_window_geometry(bytes) | settings |

### 迁移要点

- UI 现存 22 处后端 import（main.py 2 处 + 12 个 UI 文件）为 PL001 收编清单，逐文件清零
- AlarmManager 所有权从 alarm_panel（UI 层持业务状态）迁入接口
- `config/settings.py` 读写归后端；UI 需要的配置经 get_ui_preferences()/save_* 暴露，UI 不直接 import settings
- main.py 留根目录：装配 AppInterface → 装配 UI（注入）→ 进事件循环/CLI 分发，不含业务
- PL002 跟随系统深浅色：qfw Theme.AUTO；主题偏好取值扩为 auto/light/dark（旧 light/dark 兼容），经 save_theme 持久化
- PL002 退役清单：themes.py、LIGHT/DARK_THEME 常量、ui.json 颜色键转主题色映射（键保留换用途）

### 三阶段概要与版本策略

| 阶段 | 目标 | 出口标准 |
| --- | --- | --- |
| PL001 架构解耦 | interface/ 契约落地、面板纯展示化、ui/tools/ 抽取（视觉不动，仍为 ui1 外观） | ui/ 目录后端 import 清零；tests/test_interface.py 无 Qt 全绿；GUI 子进程用例全过；手动启动功能无回归 |
| PL002 UI 2.0 重写 | Fluent Widgets 重写全部面板/对话框/托盘 + 跟随系统深浅色 + QSS 管线退役 | 全量回归绿；GUI 子进程用例适配后全过；用户视觉验收（涉及截图时切多模态核对） |
| PL003 落地打磨 | 设计 tokens（间距/字号/圆角）入 ui.json、动效参数化、一致性清理、文档同步、1.0 收口 | 全量回归绿；手动验收走查清单通过；版本策略经用户定案 |

风险记录：PL002 打包（PyInstaller + qfw 资源）未做前置验证（用户决策跳过）——若 1.0 需要分发，届时在 PL003 或独立任务中补验证。
