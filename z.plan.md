# AccelWorld 重构方案与进度（z.plan.md）

> 依据：2026-08-08 项目审计（对照 AGENTS.md 规范，参考 DeepTransHub 分层结构）
> 状态：**Rust 重构进行中**——Python 时代 S1-S10（ver 0.46）与 UI 2.0/3.0（0.5.x–0.6.0.0）已定版留档；Rust 重构 PL011–PL015 完成（core 全量 parity + egui 外壳 + 五页 + 系统集成），当前阶段为第 7 轮审计修复（FIX007 已完成，V0.6.0.7）
> 更新（2026-09-10）：接入 DeepTransHub 工作流体系；本文件新增「审计观察项豁免定案清单」与「附录（审计报告归档区）」，旧内容原样保留
> 更新（2026-10-16）：新增「Rust 重构总纲：AccelWorld RS」节（见下文），Python 版全量归档 `.archived/`
> 进度明细见 `x.progress.md`，本文件保留执行要点与历史方案记录

---

## 已完成 ✅（Python 时代 S1-S10，原样留档）

| 阶段               | 完成内容                                                                                                                                                                                                |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1 结构骨架        | 建立 `utils/ → config/ → modules/ → ui/ → data/` 五层包结构；`main.py` 收编入口与 VERSION 单一来源                                                                                                      |
| S2 数据规范化      | 引入 `TimeInfo`/`LunarInfo`/`WeatherData`/`UserConfig` 四个 dataclass；天气表合并为 `WEATHER_CODE_INFO` 单表；配置走 file_utils 缓存单例 + base64 窗口几何                                              |
| S3 Bug 修复        | B1（启动参数生效）、B2（托盘退出保存设置）、B3（裸 except）；D1/D2/D9 及死代码清理                                                                                                                      |
| S4 GUI 面板化      | 6 个面板（ui/panels/）+ `SystemTray` 独立类 + `AlarmEditDialog` 完善；signal/slot 解耦（rate_changed/theme_toggled/alarm_saved/alarm_triggered）                                                        |
| S5 后台化          | 天气查询移入 QThreadPool + 30 分钟缓存 + 网络重试；闹钟预设铃声异步播放（`play_alarm_sound_async`）                                                                                                     |
| S6 规范补齐        | 全文件函数 `#` 注释 + 末尾 `# =====` 说明区；类型注解补齐；行宽 ≤100；函数内 import 清零                                                                                                                |
| S8 审计修正        | 跨天闹钟去重键、配置反序列化容错、天气首次加载、一次性闹钟语义、低优先级 7 项、规范性 4 项、全量回归                                                                                                    |
| S9 代码质量优化    | 异常上抛与窄捕获、死代码清理、效率优化（秒级缓存/轮询降频）、抽象合并、应用静态配置层（零硬编码）、pytest 41 用例测试引入                                                                               |
| 收尾               | workingboard M08 勾选、README/AGENTS.md 更新、版本升级 ver 0.45                                                                                                                                         |
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

- 抽象合并 5 项（dataclass 序列化通用化/\_trigger_key/TimeInfo 属性/gui_args 推导式/PresetSound 辅助）
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

## 审计观察项豁免定案清单（2026-09-10 随工作流体系接入初始化；2026-09-13 A001-A004 四轮观察项归拢定案；2026-10-18 补录 A005 七条；2026-10-18 再补录 A006 十八条＝15 永久 + 3 条件；2026-10-18 再补录 A007 九条＝8 永久 + 1 条件；2026-10-23 补录 A008 十四条＝14 永久，同轮清理指向已删文件的陈旧条目；2026-10-24 补录 A009 一条＝1 永久；A010 轮观察项 12 条经复核全部维持观察级、无新增豁免）

> 豁免唯一权威源：已定案项审计时不再重复报告。新定案条目由归档环节（`.agents/skills/audit-report`）经用户确认后追加。
> 分级规则：①**永久豁免**——设计定案/用户保证/容错体系覆盖，后续轮次不再报告不再讨论；②**条件豁免**——当前条件下不可达或可接受，**触发条件变化时重新评估**（每项标注触发条件）。
> 归拢说明（2026-09-13）：A001-A004 四轮报告的参考级观察项已全部归拢至本清单（多轮重复项去重 + 单轮项逐一裁决），各报告观察项节已删除；3 条真缺陷当场热修复（chips 非法日期防护/from_index 越界回退/滑杆 int(round)，见各文件注释"热修复 2026-09-13"）；2 条已被后续修复自然消解（占位符格式、悬浮 rail 高度与 GL tint 硬编码）；chinese-calendar 数据年末维护项登记于 y.problems#8。
> 归拢说明（2026-10-18，A005 / Rust 侧首轮）：参考级观察项 13 条经用户复核——**6 条提升为 P3** 并入 FIX005（隐藏期事件时延 / 退出保存 / 边界 panic / auto 主题 GL / file_io 注释 / hourly 取数），余 **7 条**永久豁免（下表 2026-10-18 行）；Python 时代条目（A001-A004）保留原样，其对象代码已全量归档 `.archived/`（本轮不审计归档代码）。
> 归拢说明（2026-10-18，A006 / Rust 侧第 2 轮）：参考级观察项 22 条经用户复核——**4 条提升为 P3** 并入 FIX006（夹具计数断言 / clock 除零理论缺口 / 非 Windows 构建告警与桩缺对等导出 / lunar_compare 冗余列），余 **18 条**定案（下表 2026-10-18 A006 行：15 永久 + 3 条件）；A005 与本轮均已闭环的 Python 时代条目保留原样不复核。
> 归拢说明（2026-10-18，A007 / Rust 侧第 3 轮）：参考级观察项 11 条经用户复核——**2 条提升为 P3** 并入 FIX007（日志与配置路径基准不一致 / weather_code 非标量宽松容错），余 **9 条**定案（下表 2026-10-18 A007 行：8 永久 + 1 条件＝file_io 符号链接 TOCTOU）；「任何非 ASCII 上下文」类历史条目与已闭环项不复核。
> 归拢说明（2026-10-23，A008 / Rust 侧第 4 轮）：参考级观察项 18 条经用户复核——**2 条提升为 P3** 并入 FIX008（bus.rs Tick 回退字面量改读 token／Cargo.toml 版本注释同步），**2 条由归档动作即时消解**（z.plan 补 UIR 衔接纪要／本清单清理指向已删文件的陈旧条目：nav.rs/glass 族/render_probe/acrylic/ring 及 A005-A006 重叠行），余 **14 条**定案（下表 2026-10-23 行：全部永久）；UIR 重塑后部分旧行号漂移，同源条目按性质适用、行号以现行代码为准。
> 归拢说明（2026-10-24，A009 / Rust 侧第 5 轮）：参考级观察项 13 条经用户复核**全部维持观察级**（回落理由成立：无累计误差/既有定案/毫秒竞窗/静态常量可达性等）；另将「ThemePref 三态契约悬空＝仅存档定案的协议位保留」登记入永久豁免表（下表 2026-10-24 行），防止后续轮次误报。
> 归拢说明（2026-10-24，A010 / Rust 侧第 6 轮）：参考级观察项 12 条经用户复核**全部维持观察级**（回落理由成立：防御域常量/静态表 μs 级查表/两条件叠加才可达/token 可信等）；无提升、无新增豁免。ReplaceAlarm 零生产者已立 P3（FIX010 #7）不重复计；AGENTS 连带清单条目放宽已并入 FIX010 #3 随修复落地。

### ① 永久豁免（不再讨论）

| 文件:行号                                                                | 描述                                                                                  | 定案理由                                                                                                        | 定案日期   |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ---------- |
| modules/alarm_service.py:193-196                                         | 闹钟触发去重键 \_last_triggered 仅内存不持久化（同分钟内重启理论可重复响一次）        | 单实例桌面应用；去重键含日期维度，触发窗口极窄（仅"精确触发分钟内重启"）；代码注释已声明接受（FIX001.21 P3#15） | 2026-09-11 |
| utils/monitor.py                                                         | crash 日志文件名取启动日，跨天运行归档日期有差                                        | 纯日志归属，无可达危害（四轮复核零变化）                                                                        | 2026-09-13 |
| modules/weather_service.py                                               | DNS rebinding TOCTOU                                                                  | 固定官方域名，代码注释显式声明接受（四轮复核零变化）                                                            | 2026-09-13 |
| modules/weather_service.py:32                                            | \_weather_cache 无锁                                                                  | GIL 下原子，最坏重复请求一次（三轮复核零变化）                                                                  | 2026-09-13 |
| modules/chinese_calendar.py                                              | lunar-python 异常直接上抛不包装                                                       | 符合窄捕获上抛策略，上层有单轮兜底（三轮复核零变化）                                                            | 2026-09-13 |
| modules/alarm_service.py                                                 | enabled 双重检查                                                                      | 无害冗余防御（四轮复核零变化）                                                                                  | 2026-09-13 |
| utils/monitor.py                                                         | 重复 install_crash_handler 窗口期旧 fd 失效                                           | main.py 单点装配，无可达触发路径                                                                                | 2026-09-13 |
| utils/monitor.py:71-74                                                   | Qt 消息回调 \_handler 无 try/except                                                   | 回调体仅安全属性访问 + logger.log（logging 内部自吞异常）                                                       | 2026-09-13 |
| utils/monitor.py:11-12                                                   | utils 层 import PyQt6（与分层先例有张力）                                             | 代码有注释论证，无运行时影响（三轮复核零变化）                                                                  | 2026-09-13 |
| modules/weather_service.py                                               | 重定向被拒后按网络错误重试（约 2s 浪费）                                              | 官方接口正常不重定向（三轮复核零变化）                                                                          | 2026-09-13 |
| utils/file_utils.py                                                      | 临时目录整目录放行超出最小写入面                                                      | 有注释依据（pytest 隔离豁免，三轮复核零变化）                                                                   | 2026-09-13 |
| ui/panels/clock_panel.py                                                 | QPropertyAnimation 每 tick stop/start                                                 | T004.3 既有设计取向，单线程无竞态收敛正确（四轮复核零变化）                                                     | 2026-09-13 |
| ui/glass_card.py                                                         | 卡片 resize 全量重渲染纹理                                                            | T005 定案取向，缩放非高频（三轮复核零变化）                                                                     | 2026-09-13 |
| ui/glass_card.py                                                         | \_posToValue 值映射域与视觉行程域 2px 偏差                                            | 绘制几何细节，用户定案视觉优先（三轮复核零变化）                                                                | 2026-09-13 |
| ui/main_window.py                                                        | 窗口隐藏托盘后快捷键失效                                                              | Qt WindowShortcut 机制固有，托盘菜单有等效入口（三轮复核零变化）                                                | 2026-09-13 |
| main.py                                                                  | 未设 QApplication.applicationName                                                     | 外观类（四轮复核零变化）                                                                                        | 2026-09-13 |
| utils/logger.py                                                          | 重开失败期每次 emit 重试的开销                                                        | 注释声明"自动重试直至恢复"设计取向；INFO 低频（三轮复核零变化）                                                 | 2026-09-13 |
| utils/file_utils.py                                                      | 原子写无 fsync，掉电丢最近一次保存                                                    | 桌面应用可接受（三轮复核零变化）                                                                                | 2026-09-13 |
| modules/alarm_service.py                                                 | replace_alarm 不清理同 id 去重键                                                      | 编辑保留原 id，去重键仍有效，本就无需清理（2026-09-13 调查定案）                                                | 2026-09-13 |
| ui/audio_player.py                                                       | QMediaPlayer/QAudioOutput 引用滞留与 GC 疑虑                                          | A002 探针证伪（C++ 侧持引用），记录防复发                                                                       | 2026-09-13 |
| ui/panels/alarm_panel.py                                                 | refresh_list 在信号栈内 clear+重建                                                    | FIX004.1 后清空前显式注销场景面，较原行为更安全；Qt 延迟删除实践安全                                            | 2026-09-13 |
| modules/chinese_calendar.py                                              | 干支按立春分界                                                                        | 库默认口径，民俗流派选择（两轮复核零变化）                                                                      | 2026-09-13 |
| modules/alarm_service.py                                                 | 哔声 duration=200 硬编码                                                              | 单次使用常量，有注释依据（两轮复核零变化）                                                                      | 2026-09-13 |
| 各级 \_\_init\_\_.py                                                     | 空文件无说明区                                                                        | 空包标记，历轮未列                                                                                              | 2026-09-13 |
| ui/gl/glass_canvas.py:53-55                                              | 场景脏轮询 QTimer 启动后无 stop（隐藏期空转）                                         | 空闲期仅布尔比较，开销可忽略                                                                                    | 2026-09-13 |
| ui/gl/glass_canvas.py:134                                                | monotonic % 3600 每小时呼吸相位跳变一次                                               | hover 呼吸可见性极低                                                                                            | 2026-09-13 |
| ui/gl/shaders.py                                                         | 材质微调常量（云雾/rim/bezel/spec/投影/色散错位/veil/alpha 档）                       | GLSL 材质参数，有注释依据，非业务配色                                                                           | 2026-09-13 |
| accel-core/src/alarm.rs:188                                              | Rust：obj["sound_type"].as_str().unwrap()                                             | 前置匹配已保证该键为字符串（可证明安全），仅风格问题                                                            | 2026-10-18 |
| rs/fixtures/manifest.json:12、z.plan.md 工程树/字段名                    | Rust：manifest generated_by 与计划文本路径/字段名漂移                                 | 属历史方案 spec 与已登记执行偏移（z.plan.md:899、910）                                                          | 2026-10-18 |
| accel-core/tests/parity.rs 静态计数 155                                  | Rust：与 m.milestone 快照计数 145（PL014 时点）不冲突                                 | 时点快照口径差异，非缺陷                                                                                        | 2026-10-18 |
| rs/Cargo.lock 多版本传递依赖                                             | Rust：chrono-tz 0.8.6/0.10.4、thiserror 1/2、windows 0.61/0.62 并存                   | 直接依赖版本单一、清单一致，属第三方传递依赖                                                                    | 2026-10-18 |
| accel-core/src/alarm.rs:63-71,170                                        | Rust：PRESET_SOUND_CONFIG 枚举参数表；obj["sound_type"].as_str().unwrap()             | 有注释依据的固定枚举表；前置 match 已保证该键为字符串（可证明安全）                                             | 2026-10-18 |
| accel-core/src/interface.rs:24                                           | Rust：BUS_CAPACITY=64 通道容量                                                        | 单次使用无调参场景常量（满载对策已由 FIX005.22 落地）                                                           | 2026-10-18 |
| accel-core/src/weather.rs:307-308                                        | Rust：LAT_RANGE/LON_RANGE 地理定义域常量                                              | 地理常量、有注释依据（SSRF 校验用）                                                                             | 2026-10-18 |
| accel-core/src/data/world_cities.rs:18                                   | Rust：world_cities.json 置 config/static/ 而其余表在 rs/data/                         | 说明区已登记 z.plan 执行偏移，非缺陷                                                                            | 2026-10-18 |
| accel-core/src/settings.rs:216、alarm.rs:212                             | Rust：to_json_value 用 .expect("不可能失败")                                          | 全字段 serde 可序列化，不可达                                                                                   | 2026-10-18 |
| accel-app/src/platform/notify.rs:5、tray.rs 菜单文案                     | Rust：AUMID ／ 中文菜单文案（DWMSBT_TRANSIENTWINDOW／acrylic.rs 引用已随 UIR 删除）   | 平台常量与 UI 文案，单次使用无调参场景                                                                          | 2026-10-18 |
| accel-app/src/pages/ime_sim.rs                                           | Rust：整文件 #![cfg(test)]                                                            | 设计定案（真机 IME 实测取消）                                                                                   | 2026-10-18 |
| accel-app/src/window.rs:692-696                                          | Rust：字体搜索目录固定 C:/Windows/Fonts 等                                            | 平台集成路径常量（族名已 token 化）                                                                             | 2026-10-18 |
| accel-app/src/theme.rs:211,238,277                                       | Rust：accent/tint 键损坏兜底色值                                                      | 纯防御路径，键齐备时不可达                                                                                      | 2026-10-18 |
| accel-core/src/alarm.rs:560-570                                          | Rust：quick_target 注入 now 而落库 created_at 取真实钟                                | 生产路径两者同源，仅测试语义缝隙                                                                                | 2026-10-18 |
| y.problems.md:18-19、z.plan.md:830,910                                   | Rust：y.problems 行内自引用；总纲修订二仍列 egui_extras                               | 表述瑕疵与历史定案 spec（漂移已登记，A005 已豁免同源）                                                          | 2026-10-18 |

| 文件:行号                                                                                                      | 描述                                                                                               | 定案理由                                                            | 定案日期   |
| -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ---------- |
| accel-app 各 UI 文件（window/pages/widgets/theme）                        | Rust：派生局部几何/间距字面量（含设置浮窗锚点；nav/ring 等已删文件引用随之失效，UIR 新 UI 同源适用） | 绘制几何细节参数，无调参诉求（A005-A008 同源延续；应 token 化项已落地） | 2026-10-18 |
| accel-core/src/audio.rs:20-31,52,59-60                                                                         | Rust：amplify(0.2)／静音段 44.1kHz 单声道／懒初始化持锁探测设备                                    | 有注释依据的音频参数；单次点火无可达性能危害                        | 2026-10-18 |
| accel-core/src/weather.rs:338-339、alarm.rs:563-571、accel-app/src/pages/countdown.rs:55                       | Rust：固定 API base（SSRF 防线组成部分）／quick_target 中文标签／24h 校验常量                      | 构造期固定 base 有注释依据；UI 文案与 UX 规则无调参诉求             | 2026-10-18 |
| w.study.md:46、y.problems.md:18-19,21-24、z.plan.md:948,1028                                                   | Rust：三表表述瑕疵／行内自引用／总纲仍列 egui_extras                                               | 表述瑕疵 + 历史 spec 漂移（A005 已豁免）                            | 2026-10-18 |
| accel-core/src/alarm.rs:395-399                                                                                | Rust：gen_id 8 位十六进制域的跨进程/跨重启理论碰撞（计数器随进程重置）                             | uuid4[:8] 形态 parity 硬约束＋概率极低＋说明区已登记语义            | 2026-10-23 |
| accel-core/src/alarm.rs:118,139-147                                                                            | Rust：Alarm/AlarmDraft 的 serde derive 未使用（序列化走 to_json_value）                            | 总线为进程内枚举，零成本 API 面，删除收益趋零                       | 2026-10-23 |
| accel-core/src/interface.rs:21、alarm.rs:580、settings.rs:199、chinese_calendar.rs:212                         | Rust：THEME_CHOICES／「快捷 N 分钟」／「我的目标」迁移文案／365 天 chips 窗口                      | UI 文案与有注释算法参数，非「配置承诺存在而未进」形态               | 2026-10-23 |
| accel-core/src/interface.rs:399-412                                                                            | Rust：闹钟一次性自动禁用仅内存，延迟到下次保存落盘                                                 | 装载期 is_expired_at 自愈禁用，终点状态一致                         | 2026-10-23 |
| accel-core/src/weather.rs:150-158                                                                              | Rust：错型元素复用 SeriesMissing 变体                                                              | 仅诊断文案归类不精确                                                | 2026-10-23 |
| accel-app/src/window.rs:353,614,236,385、bus.rs:207                                                            | Rust：托盘 1s 节流／退出存档 300ms／平台轮询 200ms／PENDING_CAP=256／探针 1200ms                   | 工程节奏常量，单点使用，无调参场景                                  | 2026-10-23 |
| accel-app/src/window.rs:79-93                                                                                  | Rust：每帧 DWM 属性写入                                                                            | 幂等且受 repaint 节流约束                                           | 2026-10-23 |
| accel-app/src/window.rs:926                                                                                    | Rust：字体兜底表含 msyhbd.ttc（粗体）                                                              | 极罕见命中（常规 msyh 缺失才轮到），留观察                          | 2026-10-23 |
| accel-app/src/window.rs:421                                                                                    | Rust：探针目录不存在无 mkdir，capture 恒 FAIL 但流程走完                                           | 开发期专用路径，日志可辨                                            | 2026-10-23 |
| accel-app/src/pages/alarm.rs:48,116 ↔ theme.rs:38                                                              | Rust：faint_bg 兜底灰 10 与 38 两处不同                                                            | 键恒存在兜底不可达                                                  | 2026-10-23 |
| accel-app/src/widgets/digits.rs                                                                                | Rust：cell_width 每帧多次 layout "0"（hero 一帧 4 次）                                             | 单字符量宽开销极小                                                  | 2026-10-23 |
| accel-core/src/types.rs:114                                                                                    | Rust：UiPreferences.countdown_target app 侧零消费                                                  | settings.rs:62 已登记 legacy 读兼容                                 | 2026-10-23 |
| rs/crates/accel-app/src/bin/                                                                                   | Rust：空目录本地残留                                                                               | git 不跟踪空目录，clone 者不可见，工作区卫生                        | 2026-10-23 |
| y.problems.md:55、x.progress/milestone 历史注记                                                                | 玻璃/render_probe 历史表述                                                                         | 历史执行记录当时真实，历史豁免                                      | 2026-10-23 |
| types.rs:153、interface.rs:21、settings.rs theme 字段、base.json default_theme                                 | Rust：ThemePref 三态契约悬空（app 仅深色零消费）——「仅存档定案」的协议位保留，SetTheme 命令位不动  | FIX008.2 完成注记明示的有意保留；退役须连带 parity 域，另立产品决策 | 2026-10-24 |

### ② 条件豁免（触发条件变化时重新评估）

| 文件:行号                            | 描述                                                                        | 触发条件                                           | 定案日期   |
| ------------------------------------ | --------------------------------------------------------------------------- | -------------------------------------------------- | ---------- |
| utils/dataclass_utils.py:35-36       | 未知注解形态放行（未来新字段类型可能绕过过滤）                              | 新增非基础类型字段（date/嵌套 dataclass 等）时重评 | 2026-09-13 |
| utils/dataclass_utils.py             | get_type_hints 每次 from_dict 重解析                                        | 字段类型多样化致实测卡顿时重评                     | 2026-09-13 |
| utils/file_utils.py:61               | tmp 文件名仅 pid，同进程双线程写同路径竞态                                  | 引入多进程/多线程写同一配置文件时重评              | 2026-09-13 |
| config/settings.py:102-106           | save 的 json.dumps TypeError 穿透（仅捕 ValueError/OSError）                | 配置值允许出现不可序列化对象时重评                 | 2026-09-13 |
| config/static/static_config.py:27-33 | config.json 额外分类静默丢弃                                                | 新增配置分类需被消费时重评                         | 2026-09-13 |
| modules/time_dilation.py:79          | tick 下限钳 1ms 仅当 rate>1000 可达                                         | rate_max 配置 >1000 时重评                         | 2026-09-13 |
| config/settings.py                   | get_alarms 浅拷贝仅隔离外层，内层 dict 共享引用                             | 出现对内层 dict 的写路径时重评                     | 2026-09-13 |
| ui/gl/shaders.py:78                  | r_cap 量纲混合（y 基准半径 vs x 基准宽度）                                  | 玻璃面半径 > 半窗宽时可达（当前胶囊 r=h/2 不可达） | 2026-09-13 |
| accel-core/src/interface.rs:549-551  | Rust：每调用 settings::load（无配置缓存，磁盘 IO）                          | 调用点扩展到逐帧/高频路径时重评                    | 2026-10-18 |
| accel-core/tests/parity.rs:571       | Rust：夹具 parity 未含 world_cities 表断言（该表由内联单测覆盖）            | 为 world_cities 建 golden 夹具时重评               | 2026-10-18 |
| rs/fixtures/README.md:13-14          | Rust：「双跑 sha256 可再生性检验」依赖已归档导出脚本（库内不可复现）        | 夹具再生脚本重新入库时重评                         | 2026-10-18 |
| accel-core/src/util/file_io.rs:37-41 | Rust：不存在路径仅词法规范化，不解析符号链接（理论可经 symlink 越白名单写） | 引入用户可控的外部写入路径／可配置写入根时重评     | 2026-10-18 |

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

| 来源提交                     | 结论（现状/证据）                                                                                                             |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| 2638a09（V0.4.7.0 安全加固） | ⚠️ 引入回归 1 项：gaierror 逃逸降级契约（P1-1）；write_json 越界校验对全部调用方成立（settings.py:67 + tests 3 处），无漏改   |
| 6f29d47（V0.4.7.1）          | ⚠️ 漏改：T001.1 仅落地 GUI 侧动态周期，CLI sleep 硬编码残留（P2-8）；死 import 与失实注释各 1（P3-25/26）                     |
| 05273bd（V0.4.7.2 监控）     | ✅ 装配时机、crash 日志清理、句柄防 GC 均正确；边界项 `--version` 日志副作用（P3-9）                                          |
| bd0c5e4（V0.4.7.3）          | ✅ 快捷键/预设信号链/动画/tooltip 实现正确无双发；⚠️ 子进程测试写真实用户配置无还原（P2-5，实施当日实际发生并手工还原，确证） |

### 一、P0-P3 修复清单

#### P0（启动崩溃 / 不可逆数据丢失，确定性复现）

| 文件:行号                                            | 类型     | 描述                                                                                                                                                | 建议                                                                                          | 性质 | 影响面          |
| ---------------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ---- | --------------- |
| utils/file_utils.py:36                               | 1 正确性 | `read_json` 异常捕获漏 `UnicodeDecodeError`（ValueError 子类，两者均不捕获）：配置含非 UTF-8 字节（GBK 记事本保存）时异常穿透 import 链，启动即崩溃 | except 补 `UnicodeDecodeError`                                                                | 新增 | 配置体系/跨模块 |
| config/settings.py:57-70 + utils/file_utils.py:59-61 | 13+2     | 损坏配置静默重置为默认值 + 任一保存即以默认值覆盖旧文件（闹钟/倒计时/城市记忆不可恢复丢失）；`write_text` 非原子写入加剧闭环                        | 损坏时先转存 `.bak` 再兜底；写入改临时文件+rename 原子替换（同步改 test_corrupted_json 断言） | 新增 | 配置体系        |

#### P1（功能确定性失效）

| 文件:行号                                   | 类型        | 描述                                                                                                                                                                                           | 建议                                                             | 性质         | 影响面                    |
| ------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ------------ | ------------------------- |
| modules/weather_service.py:76, 111-117, 133 | 13 错误策略 | V0.4.7.0 引入回归：`socket.getaddrinfo` 的 `gaierror` 不在 retry_call 白名单也不在降级 except → 断网/DNS 故障 0 重试且抛异常而非承诺的 `None`（加固前经 urlopen 包装为 URLError 可重试可降级） | retry 与降级 except 均补 `socket.gaierror`                       | 新增（回归） | 天气服务                  |
| modules/chinese_calendar.py:60              | 1 正确性    | `HOLIDAY_TRANSLATION` 仅映射 7 个 Holiday 英文名中的 2 个：春节/清明/端午/中秋/抗战胜利日泄漏英文（实测 2025-01-30 显示 "Spring Festival"）                                                    | 按库常量补全 7 项映射                                            | 新增         | 农历日历                  |
| ui/panels/weather_panel.py:74-79, 143-149   | 1 正确性    | 默认城市启动时不触发天气查询：`setCurrentText` 在信号连接前执行、恢复 last_city 文本未变化不发射信号 → 天气区悬挂"获取天气中..."最长 30 分钟                                                   | `__init__` 尾部或 `set_city` 未变化分支显式调 `update_weather()` | 新增         | GUI 面板                  |
| ui/alarm_dialog.py:29, 56-74, 104-133       | 1 正确性    | `sound_combo` 无 `currentIndexChanged` 监听：自定义铃声闹钟选任何预设铃声被静默忽略（自定义→预设方向完全不可用）；自定义闹钟打开时按钮不回填文件名                                             | combo 切换回调复位 `sound_type="preset"`；编辑模式回填按钮文案   | 新增         | 闹钟服务/GUI 面板         |
| modules/time_dilation.py:53-64              | 2 防御      | 构造只校验 `rate_min` 不校验 `rate_max` 且失败 raise 不回退：脏配置（rate<1.0、非数值）经 ui/main_window.py:64 原样传入 → 启动即崩溃；超大值另致 1000Hz 轮询                                   | 补上限校验或超界回退 default_rate 并记日志                       | 新增         | 时间膨胀/主流程编排(main) |

#### P2

| 文件:行号                                                                                    | 类型        | 描述                                                                                                                                                                            | 建议                                                                                                        | 性质         | 影响面                   |
| -------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------ | ------------------------ |
| modules/alarm_service.py:116-120 + ui/panels/alarm_panel.py:154-160                          | 2 防御      | `created_at=None`（脏配置可构造成功）时 `fromisoformat` 抛 `TypeError` 穿透 QTimer 槽（实测 TypeError）→ PyQt6 槽内未捕获异常将中止应用；与 S10 A2 修复模式不一致（同场景漏改） | except 改 `(ValueError, TypeError)`                                                                         | 新增         | 闹钟服务                 |
| utils/dataclass_utils.py:15-24 + config/settings.py:27-54 + modules/alarm_service.py:130-133 | 2 防御      | dataclass 反序列化无运行时类型校验（同根因合并）：`theme:null`、`time_dilation_rate:"abc"` 静默载入回写；`repeat_days:["1"]` 构造成功但星期匹配恒 False（实测，闹钟永不重复响） | from_dict 增加逐字段类型校验，非法回退默认                                                                  | 新增         | 配置体系/闹钟服务/跨模块 |
| ui/main_window.py:104-106, 257 + ui/panels/countdown_panel.py:167-171                        | 1 正确性    | 恢复的倒计时目标未填 `countdown_target_date`（None）→ `get_target_text` 返回空 → 下一次正常退出即把持久化值清空（跨会话数据丢失）                                               | 恢复时同步解析填充 date（不启动计时），或改"有文本即返回"                                                   | 新增         | GUI 面板                 |
| ui/main_window.py:117, 251-258                                                               | 1+3         | 主题选择完全不持久化：`is_dark_theme=False` 硬编码、save_settings 不写 theme；`base.json default_theme` 与 `UserConfig.theme` 成互喂死链路（全仓 grep 证实无其他读写点）        | init 读配置、切换时持久化                                                                                   | 新增         | GUI 面板/配置体系        |
| tests/test_rate_presets.py:20-92                                                             | 12 测试卫生 | 子进程测试写真实 user_config.json 且无还原（conftest 隔离对子进程无效），每次跑 pytest 都把用户倍率改成 10.0                                                                    | 子进程内重定向配置路径（环境变量注入临时目录），与 conftest 共用机制                                        | 新增         | 测试                     |
| ui/themes.py:89, 180 + ui/system_tray.py:41                                                  | 3 硬编码    | QSS 按钮文本 `color: white` 与托盘指针 `QColor("white")` 硬编码（"该进配置未进"，不可豁免）                                                                                     | ui.json 增白色系键并接入 `_apply_colors`                                                                    | 新增         | GUI 面板/托盘与音频      |
| modules/weather_service.py:119-127                                                           | 2 防御      | Open-Meteo 响应字段缺失时默认 0：接口变更时显示"晴 0.0°C"假数据而非失败                                                                                                         | 字段缺失视为失败返回 None                                                                                   | 新增         | 天气服务                 |
| modules/time_dilation.py:173, 176                                                            | 3 硬编码    | CLI 循环硬编码 `sleep(1.0)`/`sleep(0.01)`，`clock_tick_ms` 与 `tick_interval_ms` 被 CLI 旁路（T001.1 计划口径含 CLI，漏改）                                                     | `run_live_clock` 消费 `tick_interval_ms`                                                                    | 新增（漏改） | CLI/时间膨胀             |
| modules/weather_service.py:88, 115-116                                                       | 3 硬编码    | `timeout=10`/`retries=3`/`delay=1.0` 网络参数硬编码                                                                                                                             | 入 base.json                                                                                                | 新增         | 天气服务/配置体系        |
| utils/file_utils.py:42 vs 62                                                                 | 2+11        | 缓存键不一致：读用未 resolve 路径、写清缓存用 resolve 后路径（V0.4.7.0 改动残留）；当前调用方恰好全部规范化故未触发                                                             | 三处统一 `Path(path).resolve()` 为键                                                                        | 新增         | utils 公共契约           |
| utils/logger.py:33-40                                                                        | 2 防御      | 跨天重开日志文件无异常防护：重开失败后文件日志通道静默永久失效直至重启                                                                                                          | 重开段包 `try/except OSError` 降级                                                                          | 新增         | 日志                     |
| config/settings.py:65-70 + ui/main_window.py:170                                             | 10+13       | 保存失败静默：磁盘满/只读时 `save_config` 返回 False，`_save_alarms`/`save_settings` 等调用方全部忽略返回值，用户无感知且内存磁盘持续分叉                                       | 失败上浮 UI 提示                                                                                            | 新增         | 配置体系/GUI 面板        |
| modules/alarm_service.py:114-121 + ui/alarm_dialog.py:116-117                                | 1/2 语义    | 一次性闹钟"仅创建当天触发"（S8.4 定案）副作用：创建时设定时间已过今日则永不触发且 UI 持续显示启用；对话框无日期选择无法表达"明天的一次性"                                       | **已定案修复（2026-09-11 用户确认）：创建时已过今日时间自动顺延次日触发（语义变更，需回归 S8.4 相关用例）** | 新增         | 闹钟服务                 |

#### P3（低）

| #   | 文件:行号                                                  | 类型 | 描述                                                                                                                        | 建议                                                        | 性质         | 影响面           |
| --- | ---------------------------------------------------------- | ---- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------ | ---------------- |
| 1   | modules/alarm_service.py:78 + ui/panels/alarm_panel.py:219 | 2    | 越界 `repeat_days=[9]` 可致启动 IndexError【需验证】                                                                        | `__post_init__` 过滤 0-6                                    | 新增         | 闹钟服务         |
| 2   | modules/weather_service.py:66, 88                          | 8    | `_OPENER` 全局复用理论性线程安全【需验证】                                                                                  | 每任务构建或注释声明单线程假设                              | 新增         | 天气服务         |
| 3   | main.py:44-50, 71-76                                       | 1    | CLI `--rate 2.05` 等非 0.1 步进值被银行家舍入静默吸附且方向不一致，帮助文本未说明粒度（实机验证）                           | 帮助文本注明 0.1 步进或显式提示                             | 新增         | CLI              |
| 4   | ui/panels/clock_panel.py:104-106                           | 6    | 提示"必须大于1.0"与闭区间校验矛盾，且预设"工作 1.0x"即取该值                                                                | 文案改"必须不小于"                                          | 新增         | GUI 面板         |
| 5   | ui/system_tray.py:64                                       | 3    | 托盘菜单初始倍率硬编码 "2.0x"                                                                                               | 取 `base["default_rate"]`                                   | 新增         | 托盘与音频       |
| 6   | ui/panels/weather_panel.py:143-149                         | 1    | 列表外 `--city` 时下拉框显示旧城市而查询按实际城市                                                                          | combo 只读展示实际城市                                      | 新增         | GUI 面板         |
| 7   | ui/panels/clock_panel.py:190-215                           | 9    | 滑杆拖动每格重建实例+写盘（2.0→10.0 约 80 次写盘）；`apply_acceleration` 显式 emit + setValue 再 emit 双发（幂等但冗余 IO） | sliderReleased 再发/写盘去抖；删显式 emit 统一走 `set_rate` | 新增         | GUI 面板         |
| 8   | ui/panels/countdown_panel.py:194, 243                      | 9    | 选择器对话框重复打开累积未释放子 QDialog                                                                                    | `exec()` 后 `deleteLater()`                                 | 新增         | GUI 面板         |
| 9   | main.py:19-22, 68                                          | 2    | `--version/--help` 仍创建当日日志文件（setup_logging 先于 parse_args）                                                      | setup_logging 移至 parse_args 后                            | 新增         | 主流程编排(main) |
| 10  | ui/panels/world_clock_panel.py:82-84                       | 13   | `logger.error` 无堆栈，与他处同类回调 `logger.exception` 风格不一                                                           | 统一 `logger.exception`                                     | 新增         | GUI 面板         |
| 11  | modules/time_dilation.py:47-52                             | 6    | 类属性用 docstring 当注释（S10 P5 清理残留）                                                                                | 改 `#` 注释                                                 | 新增         | 时间膨胀         |
| 12  | modules/chinese_calendar.py:5                              | 5    | `Tuple` import 未使用                                                                                                       | 删除                                                        | 新增         | 农历日历         |
| 13  | modules/chinese_calendar.py:11                             | 5    | `SHI_CHEN` 条目 `(23, 1, "子时")` 恒 False 死数据（结果靠 else 兜底，正确但误导）                                           | 改 `(23,24)`+`(0,1)` 或注释说明                             | 新增         | 农历日历         |
| 14  | modules/weather_service.py:102-108 vs 42-43                | 4    | URL 字面量与 `_API_SCHEME/_API_HOST` 白名单双源维护                                                                         | URL 用常量拼接                                              | 新增         | 天气服务         |
| 15  | modules/alarm_service.py:174-176                           | 2    | `_last_triggered` 仅内存，同分钟重启重复响（窗口极窄）                                                                      | 持久化或豁免定案                                            | 新增         | 闹钟服务         |
| 16  | modules/alarm_service.py:250-259                           | 10   | `from_dict_list` 丢弃损坏条目无日志                                                                                         | logger.warning                                              | 新增         | 闹钟服务         |
| 17  | modules/alarm_service.py:147-149 vs 59                     | 12   | 声音兜底元组与 CLASSIC 条目字面量重复（且不可达）                                                                           | 兜底改引用 CLASSIC                                          | 新增         | 闹钟服务         |
| 18  | data/timezones.py:19                                       | 6    | 说明区写"供 ui/main_window.py 使用"，实际消费方是 world_clock_panel                                                         | 更新说明区                                                  | 新增         | 文档             |
| 19  | utils/logger.py:46 vs utils/monitor.py:18                  | 12   | crash 前缀双源（改一处即永不清理）                                                                                          | 常量单源化                                                  | 新增         | 日志             |
| 20  | utils/logger.py:15, 64                                     | 12+3 | backup_days 代码默认值与 base.json 双处维护；日志级别未配置化                                                               | level 入 base.json                                          | 新增         | 日志/配置体系    |
| 21  | config/static/static_config.py:21-28                       | 13+6 | config.json 缺失/损坏抛 KeyError/AttributeError，与注释承诺的 RuntimeError 不符                                             | 显式校验映射表                                              | 新增         | 配置体系         |
| 22  | config/settings.py:23                                      | 5    | `CONFIG_DIR` 死常量（仅测试 monkeypatch）                                                                                   | 删除或让 CONFIG_FILE 派生                                   | 新增         | 配置体系         |
| 23  | utils/monitor.py:54-55                                     | 2    | excepthook 直接覆盖不链式保留原钩子（当前无冲突方）                                                                         | 保存并调用 `_previous` 钩子                                 | 新增         | 日志             |
| 24  | config/settings.py:99-112                                  | 5+6  | 旧 latin1 几何格式兼容层疑似死代码【需验证存量文件】                                                                        | 验证后按"不留废弃方案"清理                                  | 新增         | 配置体系         |
| 25  | ui/panels/countdown_panel.py:19                            | 5    | `Qt` import 死代码（V0.4.7.1 重构遗留）                                                                                     | 移除                                                        | 新增（漏改） | GUI 面板         |
| 26  | ui/main_window.py:132                                      | 6    | "100ms 定时器驱动"注释失实（周期已随倍率联动）                                                                              | 更新注释                                                    | 新增（漏改） | GUI 面板         |
| 27  | README.md:3, 13 + x.progress.md:4                          | 6    | 文档版本滞后：README 徽章 0.4.7.2、x.progress"当前版本 0.4.7.0" vs base.json 0.4.7.3                                        | 同步                                                        | 新增         | 文档             |
| 28  | tests/                                                     | 10   | T004 新功能（动画/tooltip/快捷键）仅探针验证，无沉淀断言                                                                    | 探针断言子进程化沉淀                                        | 新增         | 测试             |

### 二、参考级观察项

> 已归拢（2026-09-13）：本轮 15 条观察项经逐一裁决——3 条热修复（int(rate\*10) 写法）、7 条永久豁免、2 条条件豁免、3 条被后续重构消解/并入其他条目，全部条目见「审计观察项豁免定案清单」，本节原文删除。

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

| A001 条目                                                                  | 现状                                                                                                                                         | 证据                                       |
| -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| FIX001.1/.2/.3/.4/.8/.9/.10/.12/.13/.17/.18/.19/.20/.21/.22/.24/.25/.26 等 | ✅ 在位且实现正确（探针实证）                                                                                                                | grep + 运行时探针                          |
| FIX001.7 启动脏倍率回退                                                    | ❌ 修复被残留代码击穿：防护构造（main_window.py:46-52）落地，但 72-73 行旧无保护构造仍执行并覆盖——越界持久化倍率仍启动即崩，回退分支成死代码 | 主会话亲核 72-73 行（P1-1）                |
| FIX001.14 天气缺字段判失败                                                 | ⚠️ 修复不完整：只查键存在，null/错型值穿透（P2-2）                                                                                           | weather_service.py:140-144                 |
| P3#5 托盘初始倍率取配置                                                    | ⚠️ 修复不完整：取 default_rate 而非恢复的持久化倍率（P2-3）                                                                                  | system_tray.py:66 + main_window.py:133,161 |
| P3#18 timezones 说明区                                                     | ❌ 已列未修（唯一真遗留）（P3-3）                                                                                                            | data/timezones.py:19                       |
| P3#20 backup_days 双源 / P3#21 static_config                               | ⚠️ 部分修复（P3-2 / P3-1）                                                                                                                   | logger.py:77、static_config.py:26-31       |
| FIX001.5 / FIX001.11 / FIX001.23                                           | ✅ 功能生效，⚠️ 各引入一个轻微副作用（P3-6 / P2-4 / P3-9）                                                                                   | 见修复清单                                 |

### 一、P0-P3 修复清单

#### P1（确定性启动崩溃）

| 文件:行号                                                 | 类型     | 描述                                                                                                                                                                                                                                                                                                                                            | 建议                                                                                                    | 性质          | 影响面                     |
| --------------------------------------------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | ------------- | -------------------------- |
| ui/main_window.py:72-73                                   | 1 正确性 | A001 修复残留（三组独立发现，主会话亲核）：FIX001.7 的受保护构造（46-52）落地后，旧无保护构造 `AcceleratedWorld(time_dilation_rate=saved_rate)`（72-73）仍执行并覆盖前者。持久化倍率越界（手改 user_config.json 为 0.5/50.0，类型校验只挡类型不挡取值域）→ 73 行重抛 ValueError 无人捕获 → 启动崩溃；回退分支实为死代码，正常路径也重复构造两次 | 删除 72-73（含注释），仅保留 46-52 防护版；补"越界持久化倍率启动不崩"子进程用例                         | A001 修复残留 | GUI 面板/时间膨胀/配置体系 |
| utils/dataclass_utils.py:46-48 + config/settings.py:71-76 | 1+2      | 合法 JSON 但非 dict 结构穿透：`data.items()` 在 try 外，user_config.json 内容为 `[]`/"x"/123 时 AttributeError 穿透启动崩溃；且 `data is not None` 判定放行 → FIX001.2 的 .bak 转存对这类损坏完全旁路。元素级同型：`"alarms": ["字符串"]` 穿透 tolerant 容错                                                                                    | `dataclass_from_dict` 入口加 isinstance(dict) 判定；`load_config` 对非 dict data 走损坏转存分支；补用例 | 新增          | 配置体系/跨模块            |

#### P2

| #   | 文件:行号                                            | 类型 | 描述                                                                                                                                                                                                      | 建议                                                                      | 性质                          | 影响面                    |
| --- | ---------------------------------------------------- | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------------------- | ------------------------- |
| 1   | utils/dataclass_utils.py:27-29                       | 2    | NaN/Infinity 穿透类型校验：json.load 默认放行非标准字面量，`"time_dilation_rate": NaN` 载入后区间比较恒 False 不拒 → `int(1000/nan)` 启动崩溃                                                             | float 分支加 `math.isfinite`，或 read_json 用 parse_constant 拒绝         | 新增                          | 配置体系/时间膨胀         |
| 2   | modules/weather_service.py:140-144                   | 2    | null/错型值穿透缺字段校验（FIX001.14 残留）：只查键存在，`temperature_2m: null` 穿透后在格式化层抛 TypeError（被 UI 兜底吞掉但日志误导）                                                                  | 校验键存在 + 值非 None                                                    | A001 修复残留                 | 天气服务                  |
| 3   | modules/weather_service.py:104,159                   | 13   | 读体阶段网络异常逃逸：`response.read()` 中途断开抛 ConnectionResetError/IncompleteRead，不在重试白名单也不在降级 except，穿透到 UI 兜底                                                                   | read 纳入重试范围或 except 元组补充（收窄取舍）                           | 新增                          | 天气服务                  |
| 4   | config/settings.py:25-35 + utils/file_utils.py:57-59 | 2+11 | env 注入配置路径与写入白名单未对齐：注入项目根/临时目录之外的合法路径时保存抛 ValueError 绕过 FIX001.19 降级链直达 Qt 槽；\_backup_corrupted_config copyfile 同样无白名单                                 | save_config 捕获 ValueError 同 OSError 策略返回 False；或注入路径预检回退 | 新增                          | 配置体系                  |
| 5   | utils/file_utils.py:60                               | 8    | 原子写 tmp 文件名固定：双开 GUI/多进程同时保存时同 tmp 文件交错写，原子替换失效                                                                                                                           | tmp 名加 pid 后缀或 mkstemp                                               | 新增                          | 配置体系                  |
| 6   | utils/file_utils.py:71                               | 2    | except OSError 内 `tmp_path.unlink(missing_ok=True)` 自身可抛 PermissionError 逃逸出 write_json，违背"IO 失败返回 False"契约                                                                              | 清理再包一层 try/except OSError                                           | 新增                          | 配置体系                  |
| 7   | tests/test_gui_features.py:203-210                   | 12   | 动画断言时间依赖，约 02:10 后整套 GUI 用例必失败：前置 check 把倍率设为 6.0，进度条已被 tick 驱动到实时加速小时（=当日秒数/600），断言 `start < 13` 仅当地时刻早于约 02:00 成立——当前全绿纯属提交时间巧合 | 断言改方向性收敛或 check 前把倍率置 1.0                                   | 新增                          | 测试                      |
| 8   | ui/system_tray.py:66 + ui/main_window.py:133,161     | 1+6  | 托盘菜单初始倍率显示错值：rate_action 取 default_rate，而主窗口加载的持久化倍率从不传给托盘——持久化 10.0 时菜单显示 2.0x 与悬停文本自相矛盾                                                               | **init** 托盘创建后补 `tray.update_rate(...)`                             | A001 修复残留                 | 托盘与音频                |
| 9   | ui/main_window.py:307-311                            | 1    | `--theme light` 在持久化深色下被静默忽略：apply_startup_args 只有 dark 分支（FIX001.11 持久化后语义缺口显形）                                                                                             | 补 `elif theme == "light":` 分支                                          | 新增（FIX001.11 副作用）      | GUI 面板/主流程编排(main) |
| 10  | tests/test_rate_presets.py:38                        | 12   | 预设子进程每次运行发起真实天气请求：未打桩 get_weather_by_city，联网时引入最长约 33s 尾延迟与网络依赖                                                                                                     | 照抄 gui_features 的天气打桩                                              | 新增（FIX001.5 副作用未适配） | 测试                      |
| 11  | tests/test_gui_features.py:146,167-168               | 12   | 打桩泄漏无还原：tray.show_notification 实例替换、mw.set_setting 模块替换、匿名 lambda 信号连接三处无 try/finally（同脚本 save_config 桩有还原，标准不一）                                                 | 三处补 try/finally 还原                                                   | 新增                          | 测试                      |

#### P3（低）

| #   | 文件:行号                                | 类型 | 描述                                                                                                                                   | 建议                                           | 性质                     |
| --- | ---------------------------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- | ------------------------ |
| 1   | config/static/static_config.py:26-31     | 13+6 | A001 P3-21 修复不完整：映射表缺 base/ui 键仍 KeyError、值为非串抛 TypeError、分类内容非 dict 放行                                      | 循环后显式校验键集与 dict 类型                 | A001 修复残留            |
| 2   | utils/logger.py:77 + main.py:18          | 12   | A001 P3-20 部分修复：backup_days=7 与 base.json 仍双源；log_level 兜底 "INFO" 与 \_DEFAULT_LEVEL 同值双写                              | 默认值单源于 base.json 并注释声明              | A001 修复残留            |
| 3   | data/timezones.py:19                     | 6    | A001 P3#18 已列未修：说明区仍写"供 ui/main_window.py 使用"，实际消费方 world_clock_panel                                               | 更新                                           | 遗留                     |
| 4   | utils/logger.py:31-34,47                 | 4    | 日志文件名模式两处重复拼装（emit 未复用带副作用的 \_today_path）                                                                       | 拆无副作用 \_path_for(day) 共用                | 新增                     |
| 5   | config/settings.py:98                    | 6    | get_setting 注释写"AppConfig"，类名实为 UserConfig                                                                                     | 改正                                           | 新增                     |
| 6   | config/settings.py:71-86                 | 13+9 | 损坏文件未修复期间每次 load_config 重复转存+告警（启动约 6-7 次）无节流                                                                | 模块级标志首次后置位                           | 新增                     |
| 7   | modules/alarm_service.py:86-93           | 2    | repeat_days 规范化口径不一：[True]→[1] 布尔穿透、[1.7]→[1] 截断而 ["1.5"]→[] 剔除（闹钟静默翻转为一次性）                              | 元素层排除 bool、小数统一剔除                  | 新增                     |
| 8   | modules/alarm_service.py:67-69           | 11   | SUPPORTED_AUDIO_FORMATS 是 Qt 文件对话框过滤器串（;; 语法），UI 展示配置置于业务层                                                     | 迁 ui 层或注明                                 | 新增                     |
| 9   | ui/panels/weather_panel.py:153-156       | 9    | 列表外城市 addItem 后当次会话永久残留下拉框                                                                                            | 可接受或标记临时项                             | 新增（FIX001.23 副作用） |
| 10  | ui/main_window.py:87,107 + weather_panel | 9    | 启动期天气双请求（FIX001.5 副作用）：init 首查 default_city + set_city 联动查询，非默认 last_city 时第一次结果被丢弃，多打一次真实 API | set_city 先设 current_city 再首查/构造参数注入 | 新增                     |
| 11  | tests/test_rate_presets.py:45            | 11   | 子进程脚本调用私有方法 `window._flush_pending_rate()`（\_ 前缀约定外部不调用）                                                         | 提供公开 flush 或等待事件循环                  | 新增                     |

### 二、参考级观察项

> 已归拢（2026-09-13）：本轮 10 条观察项经逐一裁决——1 条探针证伪（QAudioOutput GC）、1 条已被后续重构消解（占位符格式）、5 条永久豁免、3 条条件豁免，全部条目见「审计观察项豁免定案清单」，本节原文删除。

### 三、亮点

- 三组独立交叉验证锁定同一条 P1 残留（回归复核机制有效性的直接证明）；A001 的 22/27 子任务修复实证完全在位
- 节日映射经 8400+ 天全量扫描验证与库枚举精确匹配；gaierror/URLError 异常层级、SSRF ValueError 穿透路径、闹钟顺延全部边界经运行时探针实证
- 配置键卫生保持满分：base.json 26 键、ui.json 25 键零死键零缺失；文档版本五处（README/x.progress/m.milestone/AGENTS/base.json）全部一致
- 上轮两个疑点经探针证伪（QAudioOutput GC、--version 副作用），避免无效修复

---

## UI 2.0 迭代方案（PL001–PL004，2026-09-11 立项）

> 决策（用户拍板）：①路线 A——PyQt-Fluent-Widgets 整体重写；②跟随系统深浅色；③经典 QSS 皮肤不保留（新系统新皮肤，themes.py 管线退役）；④直接立项，不做打包验证前置
> 基线：`ui1-final` 标签（V0.4.7.5）为经典 UI 技术收官存档
> 执行清单：三个 PL 的细致 todo 见 x.progress.md 对应任务组
> 状态：PL001 架构解耦 ✅（2026-09-11：interface/ 契约落地、ui/ 后端 import 清零、无 Qt 场景接口测试独立通过、软件实测运行）；PL002 Fluent 重写 ✅（2026-09-11：PyQt-Fluent-Widgets 1.11.3 落地、三态主题 auto 跟随系统、QSS 管线退役、全量回归 131 用例绿，视觉走查定案）；PL003 视觉迭代 ✅（2026-09-11：六导航页拆分、设置页主题选择器、Acrylic 背板 DWM 读回=3、英雄区重构、全量回归 131 用例绿，材质肉眼验收待用户确认）；PL004 收口 ✅（2026-09-11：设计 tokens 收编、动效审计、退出崩溃 os.\_exit 定案规避、文档同步、版本定版 **0.5.0.0**，终验走查待用户确认）

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

| 域   | 方法                                                                                                                                                            | 后端实现                     |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| 时钟 | get_time_info() / get_tick_interval_ms() / get_version()                                                                                                        | AcceleratedWorld / base.json |
| 倍率 | get_rate() / get_rate_bounds() / get_rate_presets() / apply_rate(rate)（校验+重建+持久化内聚）                                                                  | AcceleratedWorld + settings  |
| 天气 | get_city_names() / fetch_weather(city) / get_weather_refresh_interval_ms() / format_weather_display(city, weather)                                              | cities / weather_service     |
| 闹钟 | AlarmManager 所有权迁入接口：load_alarm_dicts() / save_alarm_dicts(list) / check_alarms(now) / get_max_alarms()                                                 | settings + AlarmManager      |
| 时区 | get_timezone_options()                                                                                                                                          | data.timezones               |
| 配置 | get_ui_preferences() -> UiPreferences(theme/last_city/last_timezone/countdown_target)；save_theme / save_last_city / save_last_timezone / save_countdown_target | settings                     |
| 几何 | load_window_geometry() / save_window_geometry(bytes)                                                                                                            | settings                     |

### 迁移要点

- UI 现存 22 处后端 import（main.py 2 处 + 12 个 UI 文件）为 PL001 收编清单，逐文件清零
- AlarmManager 所有权从 alarm_panel（UI 层持业务状态）迁入接口
- `config/settings.py` 读写归后端；UI 需要的配置经 get*ui_preferences()/save*\* 暴露，UI 不直接 import settings
- main.py 留根目录：装配 AppInterface → 装配 UI（注入）→ 进事件循环/CLI 分发，不含业务
- PL002 跟随系统深浅色：qfw Theme.AUTO；主题偏好取值扩为 auto/light/dark（旧 light/dark 兼容），经 save_theme 持久化
- PL002 退役清单：themes.py、LIGHT/DARK_THEME 常量、ui.json 颜色键转主题色映射（键保留换用途）

### 视觉迭代定案（PL003，2026-09-11 拍板）

> 用户四点拍板：①侧栏拆页取方案 A（五页 + 设置置底）；②毛玻璃只做 Acrylic 完全版（不做 Mica）；③主题控制搬入设置页（三段选择器）；④原 PL003 收口六项顺延改编为 PL004，视觉迭代即 PL003 先行

设计原则：**时间即主角**，系统材质做背景，卡片只承载内容，控制在手边；把大胆留给时钟英雄区，其余克制。

```
侧栏          页面内容
🏠 时钟      → 加速时间（绝对主角，等宽数字防走字抖动）+ 标准时间对照（次要小字）
               + 膨胀百分比大数字 + 日期/农历 + 倍率滑杆/预设紧凑组/应用
⏱ 倒计时    → 目标设置 + 大字剩余
🌐 世界时钟  → 时区下拉 + 世界时间
☁️ 天气      → 城市下拉 + 当前天气
⏰ 闹钟      → 闹钟列表 + 添加
⚙️ 设置      → 主题三段选择器（跟随系统/浅色/深色）+ 关于（版本号）   [侧栏置底]
```

Acrylic 技术要点：

- 底座：qfw FluentWindow 透明机制 + ctypes DwmSetWindowAttribute（DWMWA_SYSTEMBACKDROP_TYPE=38，值 3=TRANSIENTWINDOW），HWND 取 window.winId()，调用置于 show 后
- 只做 Acrylic（用户定案，不做 Mica）；不支持环境窄捕获静默降级纯色
- 已知特性：DWM 材质不进截图/录屏，验收以真机肉眼为准；失焦时系统可能短暂收材质（Windows 可读性保护）

现代化清单（随本组落地，动效不加新）：英雄区强化（加速时间绝对主角，等宽数字缺失则固定宽度右对齐）/ 预设按钮收身（去全宽拉伸）/ 字体层级规整（字号数值归 PL004 tokens）/ 图标全 FluentIcon（天气 emoji 保留个性）/ 主题 emoji 按钮随搬家退役

### 四阶段概要与版本策略

| 阶段                            | 目标                                                                                      | 出口标准                                                                                             |
| ------------------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| PL001 架构解耦                  | interface/ 契约落地、面板纯展示化、ui/tools/ 抽取（视觉不动，仍为 ui1 外观）              | ui/ 目录后端 import 清零；tests/test_interface.py 无 Qt 全绿；GUI 子进程用例全过；手动启动功能无回归 |
| PL002 UI 2.0 重写               | Fluent Widgets 重写全部面板/对话框/托盘 + 跟随系统深浅色 + QSS 管线退役                   | 全量回归绿；GUI 子进程用例适配后全过；用户视觉验收（涉及截图时切多模态核对）                         |
| PL003 视觉迭代                  | 侧栏多页拆分（方案 A）+ Acrylic 背板 + 主题控制入设置页 + 英雄区强化与组件现代化          | ui/ 后端 import 与样式表持续清零；全量回归绿；DWM 材质与多页观感经用户真机验收                       |
| PL004 落地打磨（原 PL003 顺延） | 设计 tokens、动效一致性、崩溃复查、文档同步、版本收口（0.5.0.0 目标，在多页新形态上执行） | 全量回归绿；手动验收走查清单通过；版本策略经用户定案                                                 |

风险记录：PL002 打包（PyInstaller + qfw 资源）未做前置验证（用户决策跳过）——若需要分发，届时在 PL004 或独立任务中补验证。

---

## 「时之砂」UI 打磨方案（PL005–PL007，2026-09-12 立项）

> 依据：《UI 打磨意见书》（`.temp/design/UI打磨意见书.md`，gitignored 仅本地；六页现状截图 `design/current/`、六张设计图与 HTML 源 `design/mockups/`，可改 token 重渲染）
> 决策（用户拍板，2026-09-12）：①accent 定鎏金 `#E4B36A`（弃 Material 绿，膨胀时间=金色、标准时间=天青 `#7FB4E8`）；②仪表数字字体 Bahnschrift（Win10/11 自带零依赖，雅黑兜底）；③世界时钟矩阵化（多城常驻，新增 world_pins 配置键）；④进度改环形表盘（qfw ProgressRing，零新依赖）；⑤PL006 材质落地按"闹钟+设置试点→推广全六页"两步走
> 传承约束：Acrylic 完全版只做一个（backdrop.py 定案不动）；y.problems#6 的 os.\_exit 缓解不动；零硬编码原则只扩 token 不破坏；动效克制（延续单源配置约定）
> 执行清单：三个 PL 的细致 todo 见 x.progress.md 对应任务组
> 状态：PL005 视觉快赢 ✅（2026-09-12：回显缺陷修复、hint 移除、scale 六级尺度、六页顶锚、导航常开 160（singleShot show 后触发，pre-show 调用污染 qfw 状态机已探针定案）、反向验收 5/5、全量回归 131 用例绿、版本 **0.5.0.1**）；PL006 玻璃材质 ✅（2026-09-12：token 三节+鎏金配色、GlassCard 双路径组件（offscreen 栅格器缺陷探针定案）、窗内光场、胶囊化、对比度 16.1/20.3:1、Acrylic 类型 3 双保险保持、全量回归 131 用例绿、版本 **0.5.1.0**）；PL007 仪表化 ✅（2026-09-12：环形表盘+农历 chips、世界时钟 4×2 矩阵（world_pins 配置链）、倒计时居中仪表+常用目标、天气卡结构化直填；节气 chips 与小时级预报依赖后端扩展顺延；反向验收+毒化两批全过、全量回归 131 用例绿、版本 **0.5.2.0**，「时之砂」三期收官）；T005 真机观感返工 ✅（2026-09-12 用户走查反馈：光晕恢复真渐变/光场降不透明度恢复磨砂/去投影效果与 DPR 渲染根除圆角毛刺；定案"视觉不得向测试妥协"入 AGENTS；版本 **0.5.2.1**）

### 设计理念：Liquid Glass 转译（苹果 WWDC25 材质体系）

- 概念「时之砂」：**鎏金 = 被拉长的时间**（加速时间/主操作），**天青 = 标准时间**（对照系/世界时钟）；夜空光场（深 `#0D1322→#090D16`）与晨光光场（浅 `#E7EEF7→#F2ECDC`）双主题自适应；农历/节气/月相/财神为表盘刻度（chips 化）
- 三层材质模型：L0 光场（DWM Acrylic 真玻璃打底 + 窗内低饱和渐变与双晕）→ L1 内容玻璃卡（tint 对角渐变 + 顶部 1px 高光描边 + 柔投影，纯 QPainter 光学近似）→ L2 胶囊控件浮于其上
- 通透三级来源（重要边界）：渲染图级（blur 需背后有丰富内容可采样）/ 真机窗级（Acrylic 实时模糊壁纸，已启用但被实底卡片遮挡——真机做薄卡片后会比渲染图更透；DWM 材质永不进截图）/ 卡片级真模糊（抓快照+缓存成本高，仅 hero 可选、默认不做顺延）
- 可读性地板：正文对比度 ≥7:1、辅助 ≥4.5:1（iOS 26 透明过量遭回调的前车之鉴），PL006.06 程序化断言，不达标回调 tint 密度

### 三阶段概要与版本策略

| 阶段           | 目标                                                                                                                    | 版本     | 出口标准                                                           |
| -------------- | ----------------------------------------------------------------------------------------------------------------------- | -------- | ------------------------------------------------------------------ |
| PL005 视觉快赢 | 倍率回显缺陷修复（意见书 G4）、常驻校验文案移除（G5）、类型尺度六级收编、六页构图统一顶锚、导航展开（API 可行性先行）   | V0.5.0.1 | 全量回归绿；offscreen 六页抓图对照存档；rg 硬编码字号清零          |
| PL006 玻璃材质 | ui.json 扩 field/glass/radius 三节 + font_family_digits、GlassCard 纯 QPainter 组件、窗内光场层、控件胶囊化、对比度验收 | V0.5.1.0 | 试点→推广完成；对比度达标记录数值；毒化探针复跑过；DWM 读回=3 保持 |
| PL007 仪表化   | 环形进度 + 农历 chips 行、世界时钟 4×2 矩阵（world_pins）、倒计时居中大仪表 + 常用目标、天气卡视觉化                    | V0.5.2.0 | 全量回归绿；拉起程序用户走查清单逐项确认                           |

风险与顺延项：天气小时级预报依赖后端 WeatherData 扩展（本轮不做，已标注）；卡片级真模糊默认不做；导航展开若 qfw 1.11.3 无对应 API 则降级保留现状并记录；hero 数字 label 禁挂模糊/阴影（100ms 重绘脏区红线）。

---

## 附录 A003：全量代码审计报告（第 3 轮，2026-09-13）

> 范围：main.py + config/ data/ modules/ ui/ utils/ interface/ tests/ 全部 55 个 .py/.json 全文通读（三路并行子审计 + 主会话亲核裁决；**interface/ 层为 2026-09-11 新增，本轮首次纳入审计范围**）
> 方式：只读审计，未修改任何代码；上轮修复提交 `ca6bfbb`，审计时 HEAD `0961883`（V0.5.3.1）
> 状态：📌 待修复（FIX003 任务清单见 x.progress.md）

### 零、上轮（A002）修复复核清单

复核方式：主会话全局 grep 关键行 + 三路子审计 `git show ca6bfbb` 逐 hunk 对比 + 探针实证（OverflowError 实测、春节 chip 解包实测、PyQt6 excepthook 行为对比、qfw 1.11.3 源码逐项核对）。

| A002 条目                                                                                             | 现状                                                                                         | 证据                                                                                                         |
| ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| P1-1 旧构造残留 / P1-2 非 dict 穿透                                                                   | ✅ 在位                                                                                      | grep 零命中；dataclass_utils.py:51-56                                                                        |
| P2-1 NaN isfinite                                                                                     | ⚠️ 本体在位但引入回归：超大 int 触发 OverflowError 穿透 try 块与 tolerant 容错（→ 一、P0-2） | dataclass_utils.py:34,58-63（实机验证）                                                                      |
| P2-2 天气 null 校验 / P2-3 read 异常 / P3-7 repeat_days / P3-8 音频格式迁移                           | ✅ 在位；data/ modules/ 自修复提交后零改动                                                   | weather_service.py:159-171、:65-71；alarm_service.py:87-90                                                   |
| P2-4 env 路径对齐                                                                                     | ⚠️ save 侧在位，转存路径漏改绕过白名单（→ 一、P1-7）                                         | settings.py:104 vs :89-96                                                                                    |
| P2-5 tmp pid / P2-6 unlink 嵌套 try                                                                   | ✅ 在位                                                                                      | file_utils.py:61,72-76                                                                                       |
| P2-7 动画断言 / P2-8 托盘倍率 / P2-9 theme light / P2-10 预设打桩 / P2-11 打桩泄漏                    | ✅ 全部在位（P2-9 以 `_THEME_SWITCH` 三态映射新形态落地）                                    | test_gui_features.py:345-378；main_window.py:44-48,182,299,432-434                                           |
| P3-1 键集校验 / P3-2 默认值单源 / P3-3 timezones 说明 / P3-4 日志路径复用 / P3-5 注释 / P3-6 转存节流 | ✅ 在位；P3-2 有一处漏改：main.py:21 兜底字面量仍双源（→ 一、P2-3）                          | static_config.py:26-36；main.py:21；logger.py:19                                                             |
| P3-7 ~ P3-11（repeat_days / 音频格式 / 天气下拉 / 天气双请求 / 私有调用）                             | ✅ 在位                                                                                      | alarm_service.py:87；alarm_dialog.py:25；weather_panel.py:82；main_window.py:100；test_rate_presets.py:48-53 |

**结论：A002 24 项修复 23 项在位，1 项引入回归（isfinite OverflowError）+ 1 项部分漏改（转存白名单）+ 1 处漏改（log_level 双源）**，均入本轮清单。

### 一、P 级修复清单（按严重度）

#### P0（确定性崩溃/功能失效，均有实证）

| 文件:行号                                                               | 类型 | 描述                                                                                                                                                                                    | 建议                                                                          | 性质                           | 影响面            |
| ----------------------------------------------------------------------- | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------ | ----------------- |
| ui/panels/countdown_panel.py:204 + config/static/base.json:22           | 1    | 常用目标"春节"chip 配置全日期 `2027-02-06`，解析器只支持 `MM-DD` → 三段解包 ValueError，点击春节确定性失效（探针实证；PyQt6 默认钩子下为 qFatal 硬崩）                                  | 解析器支持 3 段格式（含年份，已过顺延次年）或装载时校验拒载；两端契约同步收敛 | 新增（V0.5.2.0 引入）          | GUI 面板/配置体系 |
| utils/dataclass_utils.py:34（配合 :58-63）                              | 1+2  | `math.isfinite` 对超 float 范围的 int（如 400 位整数字面量）抛 OverflowError，且在字典推导内位于 try 之外、tolerant 模式也不兜底 → 手改配置超大数启动即崩（实机验证）                   | int/float 分流校验或推导纳入 try 补捕 OverflowError；补测试                   | 新增（ca6bfbb 修复引入回归）   | 配置体系/时间膨胀 |
| config/settings.py（UserConfig）+ ui/panels/world_clock_panel.py:35,101 | 2    | `world_pins` 元素零校验（UserConfig 无 `__post_init__`）：手改 `[1]` 穿透 → 世界时钟 `_city_name(1)` 调 `iana.rsplit` AttributeError 构造期崩溃（主会话亲核，裁决子审计"已有兜底"误判） | UserConfig 加 `__post_init__` 剔除非串元素（对齐 repeat_days 模式）           | 新增（A002 P1-2 同族未覆盖面） | 配置体系/GUI 面板 |

#### P1（确定性显示错值/防御缺口）

| #   | 文件:行号                                                           | 类型 | 描述                                                                                                                                                                                    | 建议                                                                             | 性质           | 影响面                |
| --- | ------------------------------------------------------------------- | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------- | --------------------- |
| 1   | modules/time_dilation.py:66,122 vs :133,137-138                     | 1    | 一天总长双口径：钟面回绕用 `int(24*rate)`、剩余/日长用浮点 `24.0*rate` → rate=1.1 时钟面 26 点回绕而"一天小时数"显示 26.40、环形进度 96%→0% 跳变永不填满（确定性复现）                  | 三处统一口径                                                                     | 新增           | 时间膨胀/GUI 面板/CLI |
| 2   | modules/chinese_calendar.py:188-201 + ui/panels/date_panel.py:80-87 | 1+11 | 农历串拼接顺序把"节气/公历节日"插在"月相"与"财神"标记之间，date_panel 反解析被污染 → 每年 30+ 天（如 09-23 秋分）月相 chip 确定性显示"望月 节气：白露…拜"                               | 根治：农历字段走 interface.types 结构化 DTO；最小改：节气/节日段移到"月相："之前 | 新增           | 农历日历/GUI 面板     |
| 3   | modules/weather_service.py:192-206 + interface/app_interface.py     | 11+5 | 手动刷新与自动刷新共用 30 分钟缓存且全仓无缓存失效入口：TTL 内点刷新为确定性 no-op；`clear_weather_cache` 生产零调用                                                                    | interface 暴露 `fetch_weather(city, force=True)` 或删除该函数                    | 新增           | 天气服务              |
| 4   | modules/alarm_service.py:232-238                                    | 2+11 | replace_alarm 绕过 add_alarm 的同时间同标签去重与上限校验，可制造 add 路径必然拒绝的重复条目                                                                                            | replace 复用去重检查（排除自身 id）                                              | 新增           | 闹钟服务              |
| 5   | modules/alarm_service.py:80                                         | 2    | created_at 类型非法被过滤后由 default_factory 重建为载入时刻 → 一次性闹钟"复活"                                                                                                         | 语义必填字段非法时该条目整体拒绝                                                 | 新增           | 闹钟服务/配置体系     |
| 6   | requirements.txt（chinese-calendar）                                | 12   | 数据实际覆盖至 2026-12-31（子审计"截止 2026-10-07"系把最后一条节假日误读为数据边界，已探针实测纠正）；**2026 年末需升级依赖获取 2027 年数据**，到期后法定节日名优雅降级停止显示（不崩） | 升级 chinese-calendar 并回归节日表                                               | 新增           | 农历日历              |
| 7   | config/settings.py:89-96                                            | 2+7  | 损坏转存 copyfile 绕过写白名单：env 注入项目外路径时在白名单外落盘 .bak                                                                                                                 | 注入路径非项目内时跳过转存仅告警                                                 | A002 P2-4 遗留 | 配置体系              |

#### P2/P3（配置卫生/清理/规范）

| #   | 文件:行号                                                                                                                                               | 级别 | 类型 | 描述                                                                                                                                                                                                                                                                                      | 性质                       |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| 1   | config/static/ui.json + base.json                                                                                                                       | P2   | 3+12 | ui.json 死键约 18 处（scale.body/caption；layout.rate*entry_width/confirm_button_size/preset_button_size/world_combo_width/dialog_form_spacing/dialog_repeat_spacing；colors 四键；glass 双主题 fill_to/highlight/border；dialog*\* 两键应接线保留）；base.json `rate_presets` 成间接死键 | 新增（0961883 减法轮回归） |
| 2   | main.py:21                                                                                                                                              | P2   | 12   | `base.get("log_level", "INFO")` 兜底字面量与 base.json 双源（FIX002.14 漏改）                                                                                                                                                                                                             | A002 P3-2 遗留             |
| 3   | interface/app_interface.py:88-93,124-126,201-212                                                                                                        | P3   | 5    | 三个契约方法生产零调用：get_rate_presets / format_weather_display（连带 format_weather_info import）/ get_system_theme_hint（连带 win import 与注册表常量）                                                                                                                               | 新增                       |
| 4   | ui/glass_card.py:11,53 + ui/panels/clock_panel.py:18                                                                                                    | P3   | 6    | QRadialGradient 同语句重复 import；`_noise_tile` 函数内 `import random`（延迟 import 明令不豁免）；clock_panel 跨模块 import 私有名 `_with_alpha`                                                                                                                                         | 遗留+新增                  |
| 5   | ui/panels/alarm_panel.py:223,29-30 + weather_panel.py:213 + main_window.py:471 + tests/test_gui_features.py:69                                          | P3   | 6    | 四处说明区/注释失实（token 已收编未更新 / format_weather_display 已弃未改 / \_make_page 签名不符 / "2 窗"实为 3 窗——后者承载防硬崩上限语境）；alarm_panel 类定义前仅一空行                                                                                                                | 新增                       |
| 6   | README.md:3 + x.progress.md:4                                                                                                                           | P3   | 6    | 版本徽章/"当前版本"仍 0.5.3.0，滞后 base.json 0.5.3.1（A001 P3#27 同类复发）                                                                                                                                                                                                              | 新增（回归）               |
| 7   | modules/time_dilation.py:43-46 + modules/chinese_calendar.py:127-130 + data/cities.py:28 + data/weather_codes.py:10 + ui/panels/world_clock_panel.py:39 | P3   | 5+6  | 后端清理批次：custom_second 零调用、getCurrentJieQi 无效回落、cities 说明区漏 interface 消费方、weather_codes english 字段零读取、`dict[str, Any]` 未导入 Any                                                                                                                             | 新增                       |
| 8   | tests/test_gui_features.py:80,100,370,410-441 + tests/test_rate_presets.py:62                                                                           | P3   | 11   | 子进程脚本多处访问 `_` 前缀私有成员（与 A002 P3-11 同性质）                                                                                                                                                                                                                               | 新增                       |

### 二、参考级观察项

> 已归拢（2026-09-13）：本轮 18 条观察项经逐一裁决——1 条已消除（glass_card origin 提前定义）、3 条热修复/登记（chips 防护、from_index 越界回退、春节数据时效入 y.problems#8）、8 条永久豁免、5 条条件豁免、1 条由 GL 返工定案（\_posToValue 保留），全部条目见「审计观察项豁免定案清单」，本节原文删除。

### 三、亮点

- A002 24 项修复经三路独立复核全部在位，并抓获 1 处修复引入回归（isfinite OverflowError）——回归复核机制再次证明有效
- 三组独立审计对同一 P0（春节 chip）形成"代码推理 + 探针实证 + excepthook 行为对比"的完整证据链
- 胶囊滑杆整轮重写经 qfw 1.11.3 源码逐项核对（信号重接/属性转发/覆写派发）成立，无残留无悬挂
- UI 2.0 四铁律 rg 扫描零违反；配置键反查"代码读取的键全部存在"
- HOLIDAY_TRANSLATION 与 chinese-calendar 库枚举全量匹配；依赖数据时效已实测（1.11.0 覆盖至 2026-12-31，年末升级 2027 数据）

---

## 附录 A004：全量代码审计报告（第 4 轮，2026-09-13）

> 范围：63 个 .py + JSON 全文（三路并行子审计 + 主会话亲核裁决）；审计基线 HEAD `3bf9603`（V0.5.6.0），重点为 A003 后新增的 GL 渲染链路约 1000 行（提交 e8c3ced/3bf9603，首次被审计）
> 方式：只读审计，未修改任何代码
> 状态：📌 待修复（FIX004 任务清单见 x.progress.md）

### 零、上轮（A003 / FIX003）修复复核清单

主会话全局 grep + 三路子审计 `git show dd16497` 逐 hunk 对比。

| A003 条目                                                                               | 现状                                                                                                                                           | 证据                                                                                           |
| --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| P0-1 春节三段解析 / P0-3 world_pins                                                     | ✅ 在位                                                                                                                                        | countdown_panel.py:205-213；settings.py:62-65                                                  |
| P0-2 超大 int isfinite                                                                  | ⚠️ dataclass 层在位但**崩溃链位移**：app_interface.py:43 `float(saved_rate)` 抛 OverflowError 穿透 except(TypeError, ValueError)（→ 一、P1-2） | dataclass_utils.py:34-36；app_interface.py:43-44                                               |
| P1-1 日长口径 / P1-3 天气 force / P1-4 replace 去重 / P1-5 created_at / P1-7 转存白名单 | ✅ 在位                                                                                                                                        | time_dilation.py:62,128；app_interface.py:110-115；alarm_service.py:239,156；settings.py:97-99 |
| P2-1 死键 / P2-3 log_level / P3 各批次                                                  | ✅ 在位；P2-1 有残留死键 rate_presets（→ P2-6）、interface 说明区漏改（→ P3-6）                                                                | main.py:22；base.json:6；app_interface.py:234,238-239                                          |
| A003 观察项 glass_card origin 条件绑定                                                  | ✅ 已消除                                                                                                                                      | glass_card.py:350 提前定义                                                                     |

**结论：十八项修复 17 项完好在位，1 项位移未消除（升格本轮 P1-2）。**

### 一、P 级修复清单（按严重度）

#### P1（确定性缺陷，均有代码实证）

| #   | 文件:行号                                                                             | 类型  | 描述                                                                                                                                                                                                                                                                                                                                       | 建议                                                                                 | 性质                           | 影响面               |
| --- | ------------------------------------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------ | ------------------------------ | -------------------- |
| 1   | ui/glass_card.py（全类）+ ui/gl/glass_scene.py:33-35 + ui/panels/alarm_panel.py:78-85 | 1+5+9 | GL 模式 GlassCard 销毁不注销场景 → 幽灵玻璃面累积：全仓零调用 SCENE.unregister；闹钟页每次 refresh_list（开关/增删/编辑）clear() 销毁行卡重建，旧 GlassSurface 以旧 rect、visible=True 残留每帧绘制；闹钟上限 10 + 药丸 6 = 16 恰满 \_MAX_SURFACES，一次刷新即超限截断——幽灵面保留、新行卡反被剔除渲染（gl_enabled 默认 true，确定性复现） | GlassCard 接销毁生命周期调 SCENE.unregister（destroyed 信号或宿主清理钩子）          | 新增（PL009 引入注册未接销毁） | GL 模式闹钟页/动态卡 |
| 2   | interface/app_interface.py:43（except :44）                                           | 1+2   | 超大 int 崩溃链位移未消除：float(saved_rate) 抛 OverflowError 穿透 except (TypeError, ValueError) → 手改 user_config 即 GUI 启动确定性崩溃（子审计实证）                                                                                                                                                                                   | except 补 OverflowError + 更正 tests/test_settings.py:200-204 失实断言并补接口层用例 | 新增（dd16497 修复不完整）     | GUI 启动/配置体系    |
| 3   | modules/chinese_calendar.py:194 + ui/panels/date_panel.py:15-16,80-87                 | 1+11  | 月相 chip 全年显示"蛾眉新月 拜"：modules 拼接"月相：X 拜财神：Y"，date_panel 以"财神："拆分命中"拜财神："内部，月相值捕获尾字"拜"（探针实证每天可见）                                                                                                                                                                                      | modules 侧去"拜"字（最小改）或根治走 LunarInfo 结构化 DTO                            | 遗留（A003 P1-2 修复不彻底）   | 日期页农历 chip      |

#### P2（确定性显示不一致/功能失效）

| #   | 文件:行号                                                | 类型 | 描述                                                                                                                | 建议                                                | 性质             | 影响面        |
| --- | -------------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ---------------- | ------------- |
| 1   | ui/glass_card.py:291-296                                 | 1    | 世界时钟选中金描边 GL 失效：set_selected 只失效栅格 \_pix；GL 模式 paintEvent 直接 return，场景面 selected 恒 0     | set_selected 在 gl_mode 下写场景面 selected + touch | 新增             | GL 世界时钟页 |
| 2   | ui/panels/glass_nav.py:178-183 + ui/main_window.py:238   | 1    | 药丸初始选中态丢失：装配期 set_current 时药丸未注册，强度无处落；首次注册不回填 \_selected → 启动时时钟药丸无金描边 | \_sync_geometry 注册时回填 selected                 | 新增             | GL 导航       |
| 3   | config/static/base.json:22 + countdown_panel.py:205-213  | 1    | 春节 chip 2027-02-06 过期后确定性错值：顺延逻辑使 2027 年末点击设 2028-02-06，实测 2028 春节为 01-26                | 登记年末数据维护；根治 lunar 库派生                 | 新增（时间门控） | 倒计时面板    |
| 4   | ui/gl/capability.py:39                                   | 1+2  | makeCurrent(...) is not None 恒真（PyQt6 返回 bool）→ 探测层误报可用，靠画布降级兜底                                | bool(ctx.makeCurrent(surface))                      | 新增             | GL 能力探测   |
| 5   | ui/panels/glass_nav.py:18,111 + ui/gl/glass_canvas.py:29 | 1+9  | hover 呼吸实际 ~6.7fps：药丸 33ms touch 但画布 150ms 轮询重绘，帧率错配                                             | 呼吸期缩短轮询或 touch 直连 update                  | 新增             | GL hover 观感 |
| 6   | config/static/base.json:6                                | 12   | rate_presets 死键残留（get_rate_presets 已删）                                                                      | 删除                                                | A003 P2-1 遗留   | 配置体系      |

#### P3（卫生/规范/防御）

| #   | 文件:行号                                                          | 类型  | 描述                                                                                                                   | 性质                       |
| --- | ------------------------------------------------------------------ | ----- | ---------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| 1   | modules/alarm_service.py:138-145                                   | 1     | 过期一次性闹钟（关机跨过触发时刻）永久死条目不清理                                                                     | 新增                       |
| 2   | modules/alarm_service.py:287-299                                   | 2     | from_dict_list 装载不校验 max_alarms 上限（与 add 路径不一致）                                                         | 新增                       |
| 3   | ui/main_window.py:282-289                                          | 2+13  | 窗口 paintEvent 无 try/except（防护矩阵应一致；\_field_pix 赋值 :95 早于首帧，触发路径当前不可达）                     | 新增                       |
| 4   | ui/glass_card.py + ui/panels/glass_nav.py                          | 2+13  | 回调防护矩阵不一致（仅 resize/paint 有 try/except，hide/show/move/enter/leave 无），与 AGENTS 约定口径需统一或定案豁免 | 新增                       |
| 5   | ui/gl/glass_canvas.py:161-165 + shaders.py:50 vs glass_card.py:372 | 1     | 光晕 y 语义双径镜像：栅格 Qt 顶原点 / GL 底原点直传，同一配置键两径观感颠倒                                            | 新增（返工只对齐 GL 单径） |
| 6   | interface/app_interface.py:234,238-239                             | 6     | 说明区引用已删方法、fetch_weather 签名漏 force                                                                         | 新增（dd16497 残留）       |
| 7   | ui/gl/glass_canvas.py:211,216-218 + glass_nav.py:201-202           | 6     | 说明区失实三处（"长度 8"实为 16 / init 签名漏 glass_tokens / \_rail_navigate 已不存在）                                | 新增                       |
| 8   | ui/gl/glass_scene.py:33 + glass_nav.py:183-190                     | 5     | 死代码：unregister 零调用（P1-1 修复后转正）、pill_for_page 零调用                                                     | 新增                       |
| 9   | ui/gl/capability.py:34-41                                          | 2     | 探测未按 PL008 定案校验 GL 版本 ≥2.1                                                                                   | 新增                       |
| 10  | tests/test_gui_features.py:17-67                                   | 10+12 | GL 链路零测试覆盖；子进程未固定 ACCELWORLD_FORCE_NO_GL，回归路径随机器漂移                                             | 新增                       |
| 11  | modules/alarm_service.py:106,117-118                               | 4     | HH:MM 归一化两处重复实现                                                                                               | 新增                       |
| 12  | modules/alarm_service.py:10 + weather_service.py:15                | 6     | 旧式 List/Dict/Optional 注解与全仓风格不一致                                                                           | 遗留（FIX003 未覆盖）      |
| 13  | modules/chinese_calendar.py:144,177-179,209                        | 6     | 注释/说明区失实（数据边界硬编码、"空字段跳过"过时）                                                                    | 新增                       |
| 14  | ui/glass_card.py:13 + glass_nav.py:9-10                            | 6     | import 样式（单行 13 名/连续两行 QtCore）                                                                              | 新增                       |

### 二、参考级观察项

> 已归拢（2026-09-13）：本轮 10 条观察项经逐一裁决——2 条已提升修复（FIX004.9/.10）、2 条热修复（from_index 越界回退、chips 非法日期防护）、1 条登记 y.problems#8（chinese-calendar 年末维护）、4 条永久豁免、1 条条件豁免（r_cap），全部条目见「审计观察项豁免定案清单」，本节原文删除。

### 三、亮点

- ui.json 双向核对零死键零缺键（含 PL009 新增 7 键全部被消费）——零硬编码原则执行良好
- A003 十八项修复 17 项完好，dd16497 修复质量整体过硬
- GlassCanvas uniform 与 GLSL 声明逐一匹配、paintGL/initializeGL 防护完整、GL 装配降级分支对称性全覆盖
- 版本四源一致（base.json = README = x.progress = AGENTS）

---

## UI3.0 进化总纲：GLSL 液态玻璃（2026-09-13 立项，接替「时之砂」）

> 愿景（用户拍板）：整窗液态玻璃——窗口本体是一块大玻璃面板，左侧栏为玻璃药丸按钮组，内容卡片为嵌套玻璃面板（参考 Liquid Glass UI Kit 形态图）；现状整窗仅磨砂 Acrylic + 实底背景，达不到该观感
> 范围界定（换引擎，不换体系）：GLSL 只接管"玻璃光学"渲染层（QPainter 光学近似 → QOpenGLWidget + 片段着色器）；qfw 控件体系、interface 七域分层、六页业务逻辑、ui.json token、鎏金/天青视觉语言全部不动；栅格路径保留为无 GL 环境永久降级（能力探测自动切换）
> 排期：PL008（V0.5.5.0 GL 地基，GL 默认隐藏）→ PL009（V0.5.6.0 铺开）→ PL010（V0.6.0.0 定版）；探针数据仅作建议，**不设止损点，最终以用户真机人工目测为准**（T005 定案延续）
> 打包：不做、不顺延、移出 UI3.0 范围，等全部定性后另行评估
> 调优复盘：滑杆几何与自绘圆弧留边的经验见 t.hint.md 与 AGENTS.md「自绘圆弧留边」

### 渲染架构定案（技术实现细节）

**1. 单画布合成（核心决策）**：整窗只建一个 QOpenGLWidget（`ui/gl/glass_canvas.py` GlassCanvas），lower() 压在所有页面控件底层；六页的 qfw 控件保持现有父子结构、透明底叠在其上。GL 画布负责画全部"玻璃视觉"（一次全屏 pass 画完所有玻璃面），Qt 负责画文字与鼠标命中——规避多 GL 上下文的性能与层序灾难

**2. 两 pass 着色管线**：

- 背景 pass（FBO 离屏）：光场渲染成纹理（夜空 `#0D1322→#090D16` / 晨光 `#E7EEF7→#F2ECDC` 渐变 + 双晕 + 云雾 fbm + 颗粒抖动，配色取 ui.json field 节），尺寸 = 画布 × DPR
- 玻璃 pass（全屏）：片元着色器对场景内每块玻璃面执行四步——①SDF 圆角盒 `sdRoundBox(p, halfSize, r)` 定形 ②折射 = 面板内像素沿 SDF 梯度方向偏移采样**模糊纹理**（偏移 ∝ (1−smoothstep(0, 软边, −d))²，软边宽由 blur 参数映射）③色散 = RGB 三通道各用 1.3/1.0/0.7 缩放的偏移分别采样 ④fresnel rim（|d|∈[0,1.5px] 亮线）+ 顶部镜面高光带 + 颗粒抖动去色带

**3. 玻璃面描述模型**：glass_scene 维护 GlassSurface 列表（rect: QRectF、radius、tint、refraction、blur、elevation），面板在 resizeEvent/moveEvent/showEvent 时把自身几何同步进场景；z 序按 elevation 与注册顺序

**4. 文字与交互**：qfw/Qt 控件保持现有父子结构与布局，透明底叠在画布上方（raise\_），Qt 负责全部文字渲染与鼠标命中；GL 画布鼠标事件穿透

**5. Acrylic 共存**：GL 画布 QSurfaceFormat 申请 alpha 8bit，顶层 WA_TranslucentBackground；探针第一项验证合成正确性——失败走 B 方案（GL 画布自绘背景，放弃桌面透出）

**6. 能力探测与降级**：ui/gl/capability.py 探测 QOpenGLContext create + 版本 ≥2.1 + 非远程桌面会话（GetSystemMetrics SM_REMOTESESSION）→ gl_available；ui.json 增 `gl_enabled` 开关（默认 false，PL009 才开放）；两者与运算决定路径，栅格路径（现 GlassCard `_render_texture` 与 `render_field_pixmap`）原样保留为降级

**7. 光场升级**：夜空/晨光渐变 + 云雾 fbm + 双晕 + 颗粒着色器化（配色继承 ui.json field 节），后续可扩展用户选图模式——折射需要背后有可辨内容，这是观感成立的前提

### 系统边界与实现度（用户提供 4 张参考图）

桌面壁纸无法逐卡弯折（DWM Acrylic 只能整窗模糊），逐卡折射对象 = **窗内光场层**；"贴壁纸"观感 = 整窗 Acrylic 透壁纸 + 卡面 GLSL 高光/轻折射。实现度评估：天气卡贴壁纸 ≈90%（最接近现有路线）/ Pink Sky 折射卡 ≈80%（光场做丰富后窗内全可做）/ Liquid Glass 套件 ≈70%（即目标形态）/ 厚玻璃按钮 ≈60%（色散高光神似，3D 体积感属离线渲染属性）

### PL008：GL 地基——探针、画布、材质管线（V0.5.5.0）

> 交付物（全部新建，不动现有业务代码）：探针脚本 `.temp/probe_pl008_gl.py`（已产出待走查）+ `ui/gl/` 包四文件 + ui.json 增 `gl_enabled` 开关（默认 false）
> 技术要点：

- GlassCanvas(QOpenGLWidget)：initializeGL 编译背景/玻璃两套着色器程序；paintGL 单 pass；静态帧缓存（glass_scene 无 dirty 直接 return）；QTimer 仅动画期启动
- 玻璃着色器核心数学：折射偏移 = SDF 梯度方向 × (1−smoothstep(0, 软边, −d))² × lensStrength（曲率从 0 渐入，无折角）；色散 = 三通道偏移 1.3/1.0/0.7；rim = abs(d)∈[0,1.5px]
- 路径判定矩阵：gl_enabled=False → 栅格（默认）；True 且 gl_available → GL；True 且不可用 → 栅格 + 日志告警
  > 出口标准：①探针样例真机走查用户认可 ②栅格模式全量回归 134 绿 ③GL 模式帧时间 <8ms 记录入档

#### Qt5→PyQt6 迁移与回调防护（一次性根治，2026-09-13 定案）

> 诊断定案：GL 探针的 127 硬崩 = **Qt 回调（paintGL/initializeGL）内抛出未捕获 Python 异常时，PyQt6 立即 fail-fast 终止进程**（0xC0000409，零输出）——与 NVIDIA 驱动、透明窗、Acrylic 无关（二分实测：透明窗/QOpenGLWidget 显示/离屏着色器编译全部正常）
> 已锁定并修正的 API 错误：`context().functions()`（PyQt6 不存在 → `QOpenGLVersionFunctionsFactory.get(vp, ctx)`）；`setAttributeArray(0, tuple, 2)` 三参数重载不存在 → `setAttributeBuffer + VBO`
> 三层根治方案：①全仓排查 Qt5 遗留 API 逐个替换为 PyQt6 正确写法 ②所有 Qt 回调（paintGL/paintEvent/initializeGL/resizeEvent 等）一律 try/except 包裹，异常降级为纯色/跳过绘制 + logger.error，绝不外抛 ③每条降级都落日志，问题可查
> 验收：回调防护探针（故意触发异常 → 进程存活 + 降级生效）+ 栅格回归 134 绿 + GL 模式真机走查

### PL008 完成记录（2026-09-13）

- 探针三验证：GL 上下文/文字叠层/玻璃样例全部实现（.temp/probe_pl008_gl.py）；**发现本机 glDrawArrays 驱动层 GPF（进程 127 硬崩，零输出）**，二分定位：透明窗/QOpenGLWidget 显示/离屏着色器编译均正常，崩溃在首次绘制调用原生层——与 y.problems#6 同家族环境问题
- 处置（按用户定案"探针仅建议、不设止损"）：gl_enabled 默认 false 保持关闭，栅格路径不受影响（回归 134 绿）；GL 代码全量保留，待驱动/环境复查后重测（probe_pl008_gl.py 一键复测）
- 版本 bump 0.5.5.0 五处同步 + m.milestone 条目

### PL009：UI3.0 铺开——侧栏药丸 + 六页迁移（V0.5.6.0）

> 交付物：ui/panels/glass_nav.py（GlassNavRail）+ 六页 GlassCard 切 GL 模式 + main_window 装配分支
> 技术要点：

- GlassNavRail(QWidget)：竖排玻璃药丸（icon + 文字 Qt 叠层、SDF 玻璃面 GL 绘制），选中金描边 #E4B36A，hover/选中呼吸着色器化；点击发 navigate(page_id) 信号，main_window 现有 switchTo 逻辑接收——只换皮不换交互
- 六页迁移 = GlassCard 构造加 gl_mode 参数逐页切换（时钟→倒计时→世界时钟→天气→闹钟→设置），每页迁移后真机走查再进下一页
- 主题联动：设置页切深浅色 → glass_canvas.set_light_field(theme) 重上传纹理（配色仍取 ui.json field 双套）；降级模式自动回 qfw 导航
  > 出口标准：六页真机逐页走查确认 + 全量回归绿 + 关 GL 降级模式六页功能无损

### PL009 返工：玻璃渲染修复——预乘合成、量纲归一与立体感补齐（2026-09-13 定案，并入 V0.5.6.0）

> 背景：真机走查判定玻璃效果整体失效，诊断报告定案九项缺陷。根因一：PL008 探针
> 着色器（uv 归一化空间）移植主程序时改写为设备像素空间，效果参数量纲未随迁——
> rim 带宽/折射偏移缩小数百倍，玻璃面"画了但视觉不存在"（FBO 实测面内外色差仅
> 2/255、rim 增量 6/255）。根因二：shader 输出直通色而 Qt 合成假定预乘 alpha，
> Acrylic 被 100% 盖死，透壁纸全链失效。另：光晕 y 语义颠倒、主 shader 缺立体感
> 三要素、多页卡面叠画、面数截断（18 面 > 8 上限）。
> 返工要点（任务组 PL009.09-.14）：

- 预乘 alpha：gl_FragColor 输出 col×alpha（Qt 合成假定预乘；普通合成模式不变、
  画布保持子控件下层）——背景≈0.55 / 玻璃面≈0.88 双档半透明，壁纸模糊按比例透出，
  厚玻璃微透；文字在画布上层清晰度天然不受影响（否决 onTop 路线：玻璃面会雾化盖字）
- 量纲归一：着色器整体回 uv 归一化空间（u_rect/radius 注入前除以分辨率），rim/
  折射/色散参数按 PL008 探针验证量纲回迁
- 立体感三要素：顶部镜面高光带（探针 spec 迁回）+ 双带边缘（外亮线+内侧收暗）+
  SDF 偏移投影（厚玻璃体积感，对照 Liquid Glass 参考图）
- 光晕 y 语义：ui.json 为 Qt 坐标（0=顶），注入前 1-y 翻转对齐 GL
- 场景可见性：GlassSurface.visible + GlassCard show/hideEvent 同步，隐藏页不参与
  绘制；\_MAX_SURFACES 8→16
- 质量门：真机渲染断言探针（FBO 分区采样量化阈值）全过才进走查——PL009 首轮跳过
  渲染断言致量纲缺陷漏网，本环节升级为强制门
  > 返工出口：渲染断言探针全 PASS + 回归绿 + 冒烟 + 真机逐页走查（PL009.01-.08 随
  > 返工一并确认）；并入 V0.5.6.0 一次提交

### PL009 完成记录（2026-09-13）

- 交付：玻璃药丸导航（悬浮左缘/hover 呼吸/选中金描边）+ 六页 GL 迁移 + 画布铺底
  装配 + 主题联动 + 微黑透光背景定档（field alpha 0x23，壁纸透出 ~86%）
- 走查返工两轮定案三项根因（全部 FBO 取证）：①cov 面内外颠倒（`-d` 判向写反，
  玻璃混合自首轮整体失效）②Qt 合成假定预乘 alpha（直通色满亮覆盖，Acrylic 全灭）
  ③SDF 无纵横比校正（药丸圆角椭圆化）。修复：uv 量纲回迁 + 预乘输出 + aspect
  校正 + 亮度模型反转（白 veil gl_lift 26/34 + 高透 0.80）+ rim 细化方向性 +
  bezel 内亮带 + SDF 投影 + 隐藏页僵尸面消灭（注册延迟到首 show）
- 质量门：真机渲染断言探针 9/9（色差 26.3/rim 200/高光 28/预乘 0.14 档/光晕
  定位/可见性）+ 功能探针 15/15 + 回归 134 绿 + 降级对照 PASS + 用户走查确认
- 遗留：药丸 hover 呼吸等观感微调项后续走查持续收集（不阻塞收口）

### PL010：UI3.0 收口定版（V0.6.0.0）

> 收口三笔 + 文档：

1. 性能：paintGL 内 QElapsedTimer 计时（GL/栅格两模式各录 100 帧均值入档）；静态帧缓存断言（glass_scene 无 dirty 时跳过重绘）；tick/交互重绘 <8ms 实测
2. 崩溃：y.problems#6 带 GL 复测（GL 开关开/关 × 反复启停 20 次 × 多窗口），崩溃栈监控比对 logs/crash-\*.log
3. 回归：全量绿 + 降级模式六页功能断言
4. 文档：AGENTS 增"GL 渲染约定"节、t.hint 补 UI3.0 复盘、m.milestone 0.6.0.0 条目、版本五处同步
   > 出口标准：回归全绿 + 用户定版走查确认

## 附录 A005：全量代码审计报告（第 5 轮，Rust 侧首轮，2026-10-18）

> 范围：`rs/`（accel-core 18 源文件 + accel-app 33 源文件 + 集成测试与 example + 4 份静态配置 + 3 份 Cargo 清单 + 工具链/CI 脚本 + fixtures + 根文档 8 件）；**`.archived/`（Python 归档）按用户要求排除**，`.temp/`、`.history/`、`rs/target/`、`.mimosa/`、第三方依赖同样排除。
> 方式：核心域（interface/types/alarm/weather/settings/time_dilation/chinese_calendar/static_config/audio/data/util）主会话逐文件亲读；GUI 层（window/bus/nav/theme/logging/glass/widgets/pages/tools）、平台与基建配置、文档与夹具三路并行后由主会话逐条复核（高严重度项均代码级确证）。
> 状态：✅ 已修复（FIX005 全 33 条勾选，2026-10-18；四关卡 ALL_GREEN + 真窗探针 + render_probe 全绿）——**执行补录**：①`colors.*` 原为 `#RRGGBB` 而解析器只收 8 位 → 强调色/语义色全程走兜底（已修解析器 + 补 12 个语义色 token）；②原本表未列的 21 个死键（`glass.*.{fill_from,lift}`/`gl_enabled`/`scale.page_title`/`layout` 下 20 键之一部/`base.json` 五键）按"不长期保留废弃方案"删除；③`.14` 首版 `block_on(pop_error_scope)` 在 eframe 主线程**死锁**（真窗探针实证挂起）→ 改 `on_uncaptured_error` 处理器；④观察项 6 条提升为 P3 并已随 FIX005 修复（详见 x.progress.md FIX005 完成记录）

### 零、上轮修复复核清单

**无上轮（Rust 侧首轮）**。Python 时代 A001–A004 的修复对象已全量归档 `.archived/`，不构成回归复核对象；其豁免清单中「视觉设计不得向测试环境妥协」等条目已平移至 `AGENTS.md`。

### 一、P0–P3 修复清单（按严重度；P0 无）

> 性质：**遗留** = Python 时代同源缺陷随 parity 移植带入；**新增** = Rust 重构期间引入。★ = 本轮观察项经用户复核提升。

| #    | 级别 | 文件:行号                                                                                                                                                                                                                                                                                                                  | 类型            | 描述                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | 建议                                                                                                        | 性质      | 影响面                       |
| ---- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | --------- | ---------------------------- |
| 1    | P1   | accel-core/src/alarm.rs:529-546                                                                                                                                                                                                                                                                                            | 1 正确性        | 启动装载"过期一次性禁用"判据为 `created.date() < now.date()`：对"昨天 23:50 建、目标次日 08:00"（PL014.14 跨午夜快捷闹钟）在当日 08:00 前重启应用即被判过期并静默禁用 → 闹钟永不响（Python alarm_service.py:321-330 同逻辑，parity 平移缺陷）                                                                                                                                                                                                                                                                                                               | 判据改为"计划触发日 < 今天"（由 created_at + 时分推导 scheduled_date 后比较）                               | 遗留      | 闹钟服务                     |
| 2    | P1   | accel-app/src/window.rs:470-476 ＋ bus.rs:209-213,223                                                                                                                                                                                                                                                                      | 1 正确性        | 启动期不拉初始快照：`create_app` 以 `..UiState::default()` 起手且 core spawn 首帧不推事件，`drain` 仅在 ConfigSaved 时重拉 → 持久化闹钟、世界时钟城市卡、基准时区、倒计时目标启动全空至首次保存；`Bus::initial_prefs` 定义后零调用                                                                                                                                                                                                                                                                                                                          | create_app 内显式拉 get_alarms/get_world_pins/get_countdown_targets/last_timezone，或 core spawn 首帧推全量 | 新增      | 闹钟服务 / GUI 面板 / 跨模块 |
| 3    | P1   | accel-app/src/nav.rs:136                                                                                                                                                                                                                                                                                                   | 1 正确性        | `scene.set_selected(SURFACE_NAV, …)` 写在 `for page in PAGE_ORDER` 循环体内，被逐轮覆写为末位（Alarm）结果 → 仅 Alarm 页 GL 选中呼吸/金描边非零，其余四页 shader `u_sel` 恒 0                                                                                                                                                                                                                                                                                                                                                                               | `set_selected` 移出循环，循环后按 `state.current_page` 写一次                                               | 新增      | GUI 面板                     |
| 4    | P1   | accel-core/src/types.rs:139-140 ＋ interface.rs:494-497 ＋ accel-app/src/bus.rs:62,108                                                                                                                                                                                                                                     | 1/5 功能缺口    | 天气多城市未接线：`AddWeatherCity/RemoveWeatherCity` 零发送零处理（落入 `Some(other) => warn!`），`UiState.weather_cities` 无任何赋值路径 → 多城 chips/增删持久化不可达（PL014.13 声称已落地）                                                                                                                                                                                                                                                                                                                                                              | 补 UI 增删入口 + core 分发 + 变更后重拉；或降级单城并更正文档                                               | 新增      | 天气服务 / GUI 面板          |
| 5    | P2   | accel-app/src/window.rs:281（↔ config/static/ui.json:38-50）                                                                                                                                                                                                                                                               | 12 配置         | `pointer("/glass/gl_lift")` 路径不存在（实为 `glass.{dark,light}.gl_lift`）→ 恒走 `unwrap_or(0.10)`：token 从不生效、深浅不分；`glass.*.{gl_tint,fill_from,lift}` 亦零消费                                                                                                                                                                                                                                                                                                                                                                                  | 按生效主题读 `/glass/{mode}/gl_lift`，tint 注入 GlassSurface；或键提升顶层并补 token 断言                   | 新增      | 配置体系 / GUI 面板          |
| 6    | P2   | accel-app/src/logging.rs:11,25,30                                                                                                                                                                                                                                                                                          | 1/10 正确性     | `LOG_GUARD` 单体 `OnceLock<WorkerGuard>`：第二次 `set(console_guard)` 必失败且被 `let _ =` 吞掉 → console guard 立即 drop，init 后控制台日志实际丢失（文件日志正常）                                                                                                                                                                                                                                                                                                                                                                                        | 改 `OnceLock<(WorkerGuard, WorkerGuard)>` 或两个独立静态                                                    | 新增      | 日志                         |
| 7    | P2   | accel-app/src/window.rs:153-170 ＋ 484-497                                                                                                                                                                                                                                                                                 | 2 防御性        | 托盘与热键同时构造失败时二者皆 None，唯一可置 `quit_requested` 的通道全部缺失：关闭按钮走隐藏驻留后进程既无法显示也无法正常退出                                                                                                                                                                                                                                                                                                                                                                                                                             | tray/hotkeys 均缺失时关闭按钮直接走退出序列，或禁用隐藏驻留                                                 | 新增      | 主流程编排 / 托盘            |
| 8    | P2   | config/static/ui.json（约 39 键）＋ base.json（6 键）                                                                                                                                                                                                                                                                      | 3/12 配置       | 配置死键：ui.json 的 font_family/font_family_digits/gl_enabled/scale.page_title/scale.city_time/glass._.{fill_from,lift,gl_tint,gl_lift}/radius._/colors.{danger,tray_accent,tray_hand}/layout.*（20 键）零消费（值写死在代码）；base.json 的 quick_countdown_targets/rate_save_debounce_ms/progress_anim_ms/user_config 零消费                                                                                                                                                                                                                             | 逐一接线或删除；"该进配置却写死"须接线（不可豁免）                                                          | 遗留      | 配置体系                     |
| 9    | P2   | 多文件：pages/_.rs、pages/mod.rs:72-76、widgets/_.rs、window.rs:412,537、theme.rs                                                                                                                                                                                                                                          | 3 硬编码        | 视觉/业务字面量未 token 化：页偏移 (180,20)/(204,40)+add_space(156.0)（5 页重复）、颜色 from_rgb(228,117,106)（= colors.danger）、字号 12/14/15/16/18/24/34、圆角 16/12、环径 96（token 88）、滑轨 16（token 24）、快捷分钟 [15,30,60,240]、闹钟默认 sound_type/value、CHIP_SIZE 120×64、字体目录/文件名（install_fonts 忽略 font_family* token）                                                                                                                                                                                                           | 值收归 ui.json/base.json；页偏移抽公共 content_page_rect()                                                  | 遗留+新增 | GUI 面板 / 配置体系          |
| 10   | P2   | rs/fixtures/manifest.json:13-19                                                                                                                                                                                                                                                                                            | 夹具完整性      | manifest 的 sha256 清单缺 PL014 新增的 `festival_derive.json`（被 parity.rs 消费），README 声称覆盖"各夹具"（已登记 5 个哈希实测全部一致，无 P1）                                                                                                                                                                                                                                                                                                                                                                                                           | 补该夹具 sha256；generated_by 路径同步为 `.archived/export_fixtures.py`                                     | 新增      | 测试 / 文档                  |
| 11   | P2   | accel-app/src/pages/countdown.rs:152-179                                                                                                                                                                                                                                                                                   | 4/11 重复实现   | 页面自行 `parse::<NaiveDate>()`（仅 YYYY-MM-DD，强填 23:59:59），与声明为单一来源的 `tools::countdown::parse_target_text`（19/16/10 位）不一致 → 无法输入带时分目标，且绕过 Bus::send 直接用 cmd.try_send                                                                                                                                                                                                                                                                                                                                                   | 统一调用 parse_target_text，命令走 bus.send                                                                 | 新增      | GUI 面板 / 时间膨胀          |
| 12   | P2   | 多文件（app 侧 15 个文件）                                                                                                                                                                                                                                                                                                 | 11/6 契约       | app 直接消费 `static_config::Static`、`alarm::{PresetSound,AlarmDraft,Alarm}`、`audio::{play_preset,play_file}`、`settings::CountdownTarget`、`data::world_cities::{tz_of,all_names}`，与 AGENTS/z.plan「app 一切后端访问经 interface（唯一入口）」不符                                                                                                                                                                                                                                                                                                     | 收敛进 interface，或修订文档明确 token/DTO/纯常量可直取                                                     | 新增      | 跨模块 / 文档                |
| 13   | P2   | accel-app/src/bus.rs:124-126 ＋ pages/weather.rs:49-55                                                                                                                                                                                                                                                                     | 1 正确性        | `WeatherFailed` 只写 weather_errors，不清除同城旧成功数据 → 刷新失败时"红字失败 + 完整数据"并存，UI 自相矛盾                                                                                                                                                                                                                                                                                                                                                                                                                                                | 失败分支同时 weather.remove(&city)，页面渲染二选一                                                          | 遗留      | 天气服务 / GUI 面板          |
| 14   | P3   | accel-app/src/glass/painter.rs:63-69,148                                                                                                                                                                                                                                                                                   | 2/13 防御性     | `attach_pipeline` 文档承诺"失败返回 Err → 降级纯色背板"，函数体无任何 `return Err` 恒返 Ok → window.rs:259-268 降级分支不可达，真实错误走 wgpu uncaptured（默认 panic）                                                                                                                                                                                                                                                                                                                                                                                     | 用 device.push/pop_error_scope 捕获转 Err，或改述说明区                                                     | 新增      | GUI 面板                     |
| 15   | P3   | interface.rs:91,96,405、settings.rs:265、static_config.rs:78,96,112、data/world_cities.rs:45、data/timezones.rs:27,32、alarm.rs:52,66；app：widgets/glass_card.rs、pages/mod.rs:97-106、window.rs:21(SURFACE_SETTINGS)、glass/surface.rs(unregister/clear_dirty/dirty)、pages/mod.rs:131、tools/clock.rs:23,28、bus.rs:223 | 5 死代码        | 生产零调用死代码集合（app_static/ui_static/clear_cache/defaults_json/base_bool/ui_i64/ui_nested_str/coords_of/by_iana/iana_list/PresetSound::{display_name,from_index}/GlassCard 整件/SettingsPop 空实现/SURFACE_SETTINGS（只 set_visible 从不 register）/unregister/clear_dirty/dirty/next_page/progress_bounds/format_hms/initial_prefs）                                                                                                                                                                                                                 | 删除或接线；SURFACE_SETTINGS 要么注册要么删常量                                                             | 新增      | 跨模块                       |
| 16   | P3   | 多文件                                                                                                                                                                                                                                                                                                                     | 6/13 规范       | 说明区/注释失实：painter.rs:63-64（Err 承诺）、surface.rs:14（gl_tint 注入未发生）、window.rs:239-240（dirty 直连重绘：dirty 生产只写不读）、data/world_cities.rs:50（"48 城"实 50）、util/file_io.rs:88（"缩进 4"实 2）、settings.rs:282（"PL012 落地 Alarm 强类型"仍为 Vec<Value>）、data/mod.rs 说明区漏 world_cities.rs、logging.rs:126（guard 保活仅文件侧生效）                                                                                                                                                                                       | 逐条修正注释或补实现                                                                                        | 新增      | 文档 / 跨模块                |
| 17   | P3   | 多文件                                                                                                                                                                                                                                                                                                                     | 6 文档一致性    | ①README.md:115、AGENTS.md:25、w.study.md:14,56、rs/README.md:12 称"六页（…settings）"实为五页 + 设置浮窗（与 README.md:32 自相矛盾）；②README.md:58、rs/README.md:18 产物名 accelworld.exe 实为 accel-app.exe；③AGENTS.md:112 称 rs/config/user_config.json"已纳入版本控制"实不存在；④z.plan.md:4 头部仍写 PL015 进行中；⑤x.progress PL013.04 引"z.plan L 节"实为 K 节；⑥t.hint.md:48 引 AGENTS"TRACK_PAD"节已不存在；⑦源码悬空引用 pages/ime_sim.rs:1,46（PL015.08）、platform/hotkey.rs:129（PL015.06）；⑧pages/mod.rs 缺文件尾说明区（54 个 .rs 中唯一） | 逐条更正；"已完成"断言须与产物一致                                                                          | 新增      | 文档                         |
| 18   | P3   | accel-core/src/interface.rs:51,60；weather.rs:133-153,64-69                                                                                                                                                                                                                                                                | 4/2/11          | ①with_config_path 对同一路径 settings::load 调两遍；②series_numbers 以字符串 "HourlyNull" 运行期分派变体（stringly-typed）；③extract_hourly_daily 在 hourly 键缺失/非对象时直接返回空，即使 daily 合法也丢弃                                                                                                                                                                                                                                                                                                                                                | ①合并一次 load；②改枚举参数；③两者独立判定                                                                  | 新增      | 接口层 / 天气服务            |
| 19   | P3   | accel-core/src/interface.rs:395-406                                                                                                                                                                                                                                                                                        | 13 错误策略     | SetRate/ApplyRate 在倍率越界时 set_rate 返 false 后不推任何事件 → UI 乐观更新与后端状态静默偏离                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 失败路径回推失败事件                                                                                        | 新增      | 时间膨胀 / GUI 面板          |
| 20   | P3   | accel-core/src/settings.rs:122-124                                                                                                                                                                                                                                                                                         | 2 防御性        | `from_json_value` 用 `serde_json::from_value::<UserConfigRaw>` + `expect("不可能失败")`，对非对象输入该断言不成立会 panic（当前 load 已前置对象判定故不可达，但函数为 pub）                                                                                                                                                                                                                                                                                                                                                                                 | 入口加对象判定返默认值，或改 Result                                                                         | 新增      | 配置体系（需验证触发面）     |
| 21   | P3   | accel-app/src/window.rs:97-104                                                                                                                                                                                                                                                                                             | 2 防御性        | `acrylic_applied` 在句柄失败/非 Win32/apply_acrylic 返 false 时一律置 true；首帧句柄未就绪即永久放弃 Acrylic                                                                                                                                                                                                                                                                                                                                                                                                                                                | 仅在真正调用 apply_acrylic 后置 true，或允许有限重试                                                        | 新增      | GUI 面板                     |
| 22   | P3   | accel-app/src/bus.rs:177-181,192-197；glass/surface.rs:82-92                                                                                                                                                                                                                                                               | 8/2/9           | ①send 用 try_send（容量 64），滑杆拖动期每帧 SetRate 可能满通道被静默丢弃；②drain 在 UI 帧线程同步执行 notify_alarm/play_alarm_sound；③MAX_SURFACES=16 未在 register 强制，仅打包时按 elevation 升序 take(16) 截断                                                                                                                                                                                                                                                                                                                                          | ①关键命令改 send().await 或去抖；②通知/音频移后台任务；③register 处拒绝/淘汰并记日志                        | 新增/遗留 | GUI 面板 / 性能              |
| 23   | P3   | 多文件（app）                                                                                                                                                                                                                                                                                                              | 9 性能          | 每帧分配热点：sync_page_visibility/register_page_surface 每帧 format!、weather_cities_view 每帧 Vec clone、world 页每帧 clone pins、countdown 页每帧 clone+sort、draw_list 每帧 Vec+sort、uniforms.to_bytes 每帧 1.2KB、accent_rgba 每次调用重做 pointer 解析                                                                                                                                                                                                                                                                                               | 缓存 id 串/几何、复用缓冲、accent 每帧算一次                                                                | 新增      | 性能                         |
| 24   | P3   | accel-app/src/bus.rs:231-237                                                                                                                                                                                                                                                                                               | 7 安全          | 自定义闹钟铃声 sound_value（用户配置）直接 play_file(Path::new(...))，无扩展名白名单/路径规范化                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 校验扩展名（wav/mp3）+ 规范化，非法回退 preset                                                              | 遗留      | 闹钟服务 / 安全              |
| 25   | P3   | data/world_cities.rs:50；accel-core/Cargo.toml；accel-app/Cargo.toml:14,23；base.json:44                                                                                                                                                                                                                                   | 12 依赖卫生     | ①world_cities.rs 说明区"48 城"实 50；②egui_extras 声明零引用；③egui-wgpu 直写版本号违"一律走 workspace.dependencies"；④base.json 的 user_config token 未接线而 default_config_path 硬编码同值（双源）                                                                                                                                                                                                                                                                                                                                                       | 删未用依赖、迁 workspace 版本表、token 接线或删除                                                           | 新增/遗留 | 依赖与配置卫生               |
| 26   | P3   | tests/parity.rs（config_parity）；m.milestone.md:156→163                                                                                                                                                                                                                                                                   | 测试卫生 / 文档 | ①parity 用例在 temp_dir 建目录/文件后不清理，反复运行残留；②m.milestone 版本条目 0.5.3.2 直接跳 0.5.5.0，0.5.4.x 段无说明（需验证）                                                                                                                                                                                                                                                                                                                                                                                                                         | 临时目录清理或纳入沙箱；补一行说明                                                                          | 遗留      | 测试 / 文档                  |
| 27 ★ | P3   | accel-app/src/window.rs:66-82,125-170                                                                                                                                                                                                                                                                                      | 8 并发/时延     | 隐藏期托盘/热键事件仅靠 logic 帧驱动轮询（依 request_repaint_after ≤1s），事件最坏延迟约 1s；是否即时唤醒依赖 winit 消息泵                                                                                                                                                                                                                                                                                                                                                                                                                                  | 改事件驱动唤醒（hook/回调 request_repaint），至少补隐藏期响应时序探针                                       | 新增      | 托盘与音频                   |
| 28 ★ | P3   | accel-app/src/window.rs:153-170                                                                                                                                                                                                                                                                                            | 13 错误策略     | 退出序列 `SaveNow` 后立即 `Close`，`drop(runtime)` 可能取消尚未处理的命令（各变更路径已即时落盘，SaveNow 属冗余兜底）                                                                                                                                                                                                                                                                                                                                                                                                                                       | 关窗前等待后端确认（回执事件）或在说明区明确冗余语义                                                        | 新增      | 主流程编排                   |
| 29 ★ | P3   | accel-core/src/time_dilation.rs:119；chinese_calendar.rs:253                                                                                                                                                                                                                                                               | 2 防御性        | `rem_euclid(day_hours)`（rate<0.042 时 day_hours=0）与 `CAI_SHEN_DIRECTION[(month-1)]`（month∉1..=12）会 panic（当前无可达路径，属域外入参）                                                                                                                                                                                                                                                                                                                                                                                                                | 前置钳制/取模保护或 debug_assert 域校验                                                                     | 新增      | 时间膨胀 / 农历日历          |
| 30 ★ | P3   | accel-app/src/theme.rs:84-94；window.rs:244-248                                                                                                                                                                                                                                                                            | 1 正确性        | `auto` 主题下 GL token/纯色背板恒按深色，egui 侧用 ThemePreference::System → 系统浅色时二者不一致                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 补系统深浅探测（windows 注册表/egui 内建），使 GL 与 egui 同源                                              | 新增      | GUI 面板                     |
| 31 ★ | P3   | accel-core/src/util/file_io.rs:32                                                                                                                                                                                                                                                                                          | 6 文档          | 说明区称 starts_with 有"Windows 大小写语义容忍"，实为大小写敏感（两侧均 canonicalize 故通常一致）                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 更正注释措辞                                                                                                | 新增      | 文档                         |
| 32 ★ | P3   | accel-core/src/weather.rs:376-382                                                                                                                                                                                                                                                                                          | 3 配置/资源     | 请求参数 `forecast_days=7` 对 hourly 亦返回 7×24 点，UI 仅展示 24 点（多余数据白拉）                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | hourly 侧限定小时数（如 &forecast_hours=24）或文档说明                                                      | 新增      | 天气服务                     |

### 二、参考级观察项（记录不修；含回落理由）

> 本节仅列**经用户复核后仍维持观察级**的 7 条；另 6 条已提升为 P3（见上表 #27–#32），已同步定案豁免清单。

| 文件:行号                                                                                | 描述                                                                                 | 回落理由                                                  |
| ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| accel-app/src/nav.rs:13-46、widgets/ring.rs:16-34、widgets/slider.rs、widgets/chip.rs:9  | 派生的局部几何常量（rail_x/pill_w、ring inset 与段数、TRACK_PAD、窗口锚点 [16,-72]） | 绘制几何细节参数；除已并入 #9 的应 token 化项外无调参诉求 |
| accel-app/src/glass/shader.rs:96-131 等                                                  | shader 内算法常量（投影/衰减/rim/bezel/spec/色散）                                   | 有注释依据的算法参数（HINT002 逐条勾销），非缺陷          |
| accel-app/src/glass/shader.rs:221-225                                                    | to_bytes() 内每帧 assert_eq!(len, UNIFORMS_SIZE)                                     | 布局不变量、单测已锚定，改动布局时立即暴露属期望          |
| accel-core/src/alarm.rs:188                                                              | obj["sound_type"].as_str().unwrap()                                                  | 前置匹配已保证该键为字符串（可证明安全），仅风格问题      |
| rs/fixtures/manifest.json:12、z.plan.md 工程树与字段名                                   | manifest generated_by 与计划文本的路径/字段名漂移                                    | 属历史方案 spec 与已登记偏移（z.plan.md:899、910）        |
| accel-core/tests/parity.rs 静态计数 155                                                  | 与 m.milestone 快照计数 145（PL014 时点）不冲突                                      | 时点快照口径差异，非缺陷                                  |
| rs/Cargo.lock 多版本传递依赖（chrono-tz 0.8.6/0.10.4、thiserror 1/2、windows 0.61/0.62） | 直接依赖版本单一、清单一致                                                           | 第三方传递依赖，不构成漂移                                |

### 三、亮点

- 注释房规执行度高：54 个 `.rs` 中 53 个具备模块头 + 函数 `///` + 文件尾说明区（仅 `pages/mod.rs` 缺失，见 #17⑧），说明区普遍如实登记已知偏移与降级理由。
- 零硬编码主线成立：base.json 关键业务参数与 ui.json 的 field._/nav__/scale 三档/colors.primary 均接线消费；配置缺失/错型 panic 带键名。
- 可测性扎实：Clock trait（VirtualClock 拨钟）使闹钟调度全链无真睡眠；RetrySpec/IsRetryable 白名单分级；core 87 单测 + 18 项夹具 parity 集成覆盖五类 golden 快照。
- 安全与降级：URL 由构造期固定 base + 定义域校验坐标派生（SSRF 结构性消除）、禁跟随重定向、写白名单 + 原子写、转存/失败一律"日志 + 降级不崩"；平台能力失败均降级且非 Windows 编译期桩化同形。
- 架构不变量机器化：CI 第 4 关卡以 cargo tree 强制 core 依赖树零 egui/eframe；四关卡本地与 Actions 同集。

---

## 附录 A006：全量代码审计报告（第 6 轮，Rust 侧第 2 轮，2026-10-18）

> 范围：`rs/`（accel-core 18 源文件 + accel-app 33 源文件 + 集成测试与 example + 4 份静态配置 + 3 份 Cargo 清单 + 工具链与 CI（`rs/ci.ps1`、`.github/workflows/rs-ci.yml`）+ fixtures 8 份 + 根文档 9 件）；**`.archived/`（Python 归档）按用户要求排除**，`rs/target/`、`rs/logs/`、`.temp/`、`.mimosa/`、第三方依赖同样排除。
> 方式：核心域（interface/alarm/settings/static_config/time_dilation/chinese_calendar/weather/types/data/util）主会话逐文件亲读；GUI 层、平台与基建配置、文档与夹具三路并行后由主会话**逐条确证**（P1/P2 全部代码级复核；1 项天气疑点经 Open-Meteo 真机 API 外部校验**否证**）。
> 维度覆盖：13 类逐类对照；**第 7 类（安全）本轮无新增**——URL 构造期固定 base + 坐标定义域校验派生（SSRF 结构性消除）、禁跟随重定向、写白名单 + 原子写、铃声扩展名白名单（`accel-app/src/bus.rs:364-382`）均闭环在位。
> 状态：📌 待修复（FIX006 任务清单见 x.progress.md）

### 零、上轮（A005 / FIX005，提交 a17cdd2）修复复核清单

| 上轮条目                                                                                           | 现状                              | 证据（文件:行号）                                                                                                                                                                                                                                                                                                                                                                                           |
| -------------------------------------------------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #1 #2 #3 #4 #5 #6 #7 #8 #10 #11 #12 #13 #18 #19 #20 #21 #22② #22③ #23 #24 #27 #29 #30 #31（24 项） | ✅ 仍在                           | `alarm.rs:220-245,536`（scheduled_at 判据）· `window.rs:474-479`（启动快照）· `nav.rs:136-138`（选中移出循环）· `bus.rs:117`（失败清旧数据）· `logging.rs:12-13`（双 guard）· `bus.rs:169-236`（CmdQueue + 副作用线程）· `surface.rs`（register 强制 MAX_SURFACES）· `theme.rs:118,129`（accent OnceLock）· `bus.rs:364-382`（铃声白名单）· `time_dilation.rs:58`+`chinese_calendar.rs:254-257`（越界兜底） |
| #9 视觉/业务字面量 token 化                                                                        | ⚠️ 修不彻底                       | 字号 11–34 仍散落 30+ 处、字面色/chip 尺寸/闹钟默认铃声未 token 化 → 本报告 #5                                                                                                                                                                                                                                                                                                                              |
| #14 attach_pipeline 错误降级                                                                       | ⚠️ 修不彻底                       | `glass/painter.rs:148-159` 恒返 Ok → `window.rs:291-293` Err 分支死代码；`painter.rs:240` 说明区仍称「编译失败返 Err 由调用方降级」→ #7                                                                                                                                                                                                                                                                     |
| #15 死代码清理                                                                                     | ⚠️ 部分（上轮判定保留，理由在案） | 清单内已删项确认清零；`surface.rs` 的 unregister/update_geometry/draw_list/total_count/dirty/clear_dirty、`SettingsPop`、`format_sound_button_name` 仍生产零调用 → #14                                                                                                                                                                                                                                      |
| #16 说明区/注释失实                                                                                | ⚠️ 修不彻底                       | `settings.rs:272`（悬空 `///`）、`settings.rs:283-287`（编号重复且自相矛盾）、`data/timezones.rs:51`（仍列已删 `by_iana/iana_list`）、`data/mod.rs:2,10`（「三表」实四表）、`interface.rs:983`（「48 城」实 50）→ #13                                                                                                                                                                                       |
| #17 文档一致性                                                                                     | ⚠️ 修不彻底                       | `pages/mod.rs` 仍缺文件尾说明区（**全仓 53 个 `.rs` 中唯一**）；`README.md:58` 产物名仍 `accelworld.exe`；`README.md:130` 仍列 `egui_extras`；`accel-app/Cargo.toml:7`、`pages/mod.rs:2`、RS 期文档仍写「六页」→ #12/#13                                                                                                                                                                                    |
| #22① 命令满载零丢失                                                                                | ⚠️ 修不彻底                       | CmdQueue 仅覆盖 `bus.send`；页面经 `bus.cmd_tx()`（`bus.rs:354-356`）拿原始 `Sender` 用 `try_send`（全仓 6 文件命中，含滑杆 SetRate 高频路径）→ #2                                                                                                                                                                                                                                                          |
| #25 依赖与配置卫生                                                                                 | ⚠️ 部分                           | 依赖表已收敛（版本归 workspace、`egui_extras` 已移除）；`README.md:130` 文档漏删、`ui.json` 死键 `scale.city_time`/`radius.xl` 未清 → #11/#13                                                                                                                                                                                                                                                               |
| #26 测试卫生（parity 临时目录）                                                                    | ⚠️ 部分                           | `parity.rs:538-555` 为「首尾 remove_dir_all」，非 RAII：中间断言 panic 则后置清理被跳过 → #20                                                                                                                                                                                                                                                                                                               |
| #28 退出等待保存回执                                                                               | ⚠️ 修不彻底                       | `bus.rs:64,124-126` 的 `save_ack` 只置位**从不复位** → 会话内任一次保存成功即令 `exit_ready`（`window.rs:411-414`）立即为真，等待被历史回执短路 → #6                                                                                                                                                                                                                                                        |
| #10 夹具完整性                                                                                     | ✅ 实测通过                       | `manifest.json` sha256 **6/6 一致**；`counts` **10/10 与实测一致**（`config_dirty_cases=22`、`n_probe=24` 等逐项吻合）                                                                                                                                                                                                                                                                                      |
| #32 hourly 取数限制                                                                                | ✅ 真机校验通过（非缺陷）         | 实测同 URL（`forecast_days=7&forecast_hours=24`）：`daily` 仍返回 **7 日**、`hourly` **24 点**；A 组「daily 退化」疑点经外部证据否证                                                                                                                                                                                                                                                                        |

> 结论：无行为性回归；24 项完整在位，8 项为**覆盖面缺口（修不彻底）**，均并入下方 P 级（不重复计数）。

### 一、P0–P3 修复清单（按严重度；P0 无）

> 性质：**遗留** = A005 已列未修/修不彻底；**新增** = 本轮新发现。★ = 本轮观察项经用户复核提升。

| #    | 级别 | 文件:行号                                                                                                                                                                                                                                                                                                                 | 类型             | 描述                                                                                                                                                                                                                                                                                                                                                                                                                                                         | 建议                                                                                                                      | 性质      | 影响面              |
| ---- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- | --------- | ------------------- |
| 1    | P1   | accel-app/src/pages/world.rs:51-54,56,66-72 ＋ accel-core/src/interface.rs:263-274,288-296 ＋ config/static/base.json:14-23 ＋ fixtures/config_behavior.json:13-22                                                                                                                                                        | 1/12 正确性+配置 | 世界时钟 `world_pins` 域不一致：权威域为 IANA 时区串（默认值、golden 夹具、`set_base_timezone` 校验、`base_timezone` 高亮比对均 IANA），而页面按**城市名**检索 `world_cities::tz_of(city)` → 默认 8 城全部显示「--:--:--」、昼夜指示恒 true；点卡发 `SetBaseTimezone("")` 被后端拒绝；`AddWorldCity` 又追加城市名（`all_names()`，测试断言 `contains("东京")`）→ 混合域，任一消费端都不可能同时正确                                                          | 统一为 IANA：页面直接 `now_in_tz(utc, city)` 解析；添加下拉按「城市名→IANA」映射（`AddWorldCity` 收名转 tz）或直接列 IANA | 新增      | GUI 面板 / 跨模块   |
| 2    | P2   | accel-app/src/bus.rs:354-356 ＋ pages/mod.rs:82 ＋ pages/{clock,countdown,world,weather,alarm}.rs（`try_send` 共 6 文件）                                                                                                                                                                                                 | 1/9 正确性+性能  | #22① 待发队列被绕过：页面经 `bus.cmd_tx()` 取原始 `Sender` 用 `try_send`（滑杆拖动 SetRate 高频路径在内），通道满时静默丢弃；提交注记「滑杆拖动不再丢命令」实际只覆盖 `bus.send`（全仓仅 1 处）                                                                                                                                                                                                                                                              | `cmd_tx` 改返回走 CmdQueue 的出口（或页面统一 `bus.send`，提供 `send_cmd`）                                               | 遗留      | GUI 面板 / 性能     |
| 3    | P2   | accel-core/src/chinese_calendar.rs:149,154,181 ＋ accel-app/src/pages/countdown.rs:110-111 ＋ widgets/chip.rs:62-63                                                                                                                                                                                                       | 5 死接线         | PL014.07 承诺未兑现：`next_spring_festival`/`year_chips` 生产零调用（仅 core 夹具测试消费），倒计时页无「默认派生目标（春节）」与「节日 chips」接线 → 空态显示「暂无目标」；页面与 chip 注释却已宣称派生存在；`x.progress.md:364` 该条标 [x]                                                                                                                                                                                                                 | 倒计时页装配期派生默认目标（不落盘）+ `year_chips` 渲染；或作废该承诺并同步注释与 x.progress                              | 新增      | 农历日历 / GUI 面板 |
| 4    | P2   | accel-app/src/pages/weather.rs:16-56 ＋ accel-core/src/interface.rs:399-531 ＋ window.rs:474-479                                                                                                                                                                                                                          | 2/1 防御性       | 冷启动天气整段空白：core spawn 首帧只推 Tick，装配期 `fill_startup_snapshot` 拉闹钟/城市/倒计时/时区但不含天气，页面也无首帧拉取 → 天气页在用户点击「⟳ 刷新」/换城前一直空白（A005 #2 同源未覆盖天气域）                                                                                                                                                                                                                                                     | 装配期对 `last_city` 拉一次（或 core spawn 首帧推 WeatherUpdated）                                                        | 新增      | GUI 面板 / 跨模块   |
| 5    | P2   | pages/weather.rs:68,76,82,88,92,97,107,111,118,127、pages/world.rs:60、pages/alarm.rs:29,57-58,77,86、pages/clock.rs:35,67、pages/countdown.rs:92,105,120,123、widgets/{chip.rs:9,40,43,47,50; ring.rs:37,44,69; slider.rs:95,122,126}、nav.rs:202,207,216、theme.rs:160,166,180、accel-core/src/alarm.rs:101-102,362-368 | 3 硬编码         | #9「视觉/业务字面量收归 token」未落实：字号 11/12/14/15/16/18/24/34 散落 30+ 处（`weather.rs:68` 34.0 与 `scale.std_time=34` 双源；`world.rs:60` 18.0 对应死键 `scale.city_time`）；字面色 (60,60,60,60)/(70,70,80,90)/(60,60,72,120)、`white_alpha(6)`、`corner(8)`；`CHIP_SIZE 120×64`；闹钟默认 `sound_type/value` 双端硬编码（A005 #9 明列的「该进配置却没进」）                                                                                         | 按 ui.json `scale.*`/`colors.*` 补键并接线（含 chip 尺寸）；默认铃声入 base.json 并双端消费                               | 遗留      | GUI 面板 / 配置体系 |
| 6    | P3   | accel-app/src/bus.rs:64,124-126 ＋ window.rs:175-186,411-414                                                                                                                                                                                                                                                              | 1/13             | #28 修复不彻底：`save_ack` 只置位不复位 → 退出序列「等 SaveNow 回执」被会话内历史回执短路（效果等同原「SaveNow 后立即 Close」）；影响低（各变更路径已即时落盘，SaveNow 属冗余兜底）                                                                                                                                                                                                                                                                          | 进入退出序列先复位 `save_ack`，或按 SaveNow 序号匹配回执                                                                  | 遗留      | 主流程编排(main)    |
| 7    | P3   | glass/painter.rs:148-159,240 ＋ window.rs:291-293,838                                                                                                                                                                                                                                                                     | 5/6/13           | #14 修复不彻底：`attach_pipeline` 恒返 Ok → `window.rs:291-293` Err 降级分支不可达（死代码）；`painter.rs:240` 说明区仍称「编译失败返 Err 由调用方降级纯色」，与函数体注释「本函数恒 Ok」自相矛盾                                                                                                                                                                                                                                                            | 删死分支并统一说明区措辞（真实降级 = frame 无 `wgpu_render_state`）                                                       | 遗留      | GUI 面板 / 文档     |
| 8    | P3   | accel-app/src/pages/countdown.rs:165-168                                                                                                                                                                                                                                                                                  | 1/2              | 目标输入 DST 歧义静默改写：`Local.from_local_datetime(&at).single().unwrap_or_else(Local::now)` 在歧义/不存在时刻把用户所填目标改成「现在」（立即到点）；中国区无 DST 故当前不可达                                                                                                                                                                                                                                                                           | 歧义分支用 `earliest()/latest()` 或提示用户，不静默落 now                                                                 | 新增      | GUI 面板            |
| 9    | P3   | accel-core/src/interface.rs:427-437                                                                                                                                                                                                                                                                                       | 8/9              | `RefreshWeather` 每命令 `tokio::spawn` 且克隆 `evt_tx`：连续点刷新无上限并发请求（无 in-flight 去抖）；`Quit` 后任务仍持 `evt_tx` → `evt_rx` 闭合推迟至任务结束                                                                                                                                                                                                                                                                                              | 单航班去抖（in-flight 标志）或 JoinSet 统一取消；Quit 时 abort 在飞任务                                                   | 新增      | 天气服务 / 接口层   |
| 10   | P3   | accel-app/src/theme.rs:118-138,249-270 ＋ window.rs:89,311,317 ＋ pages/mod.rs:93-100                                                                                                                                                                                                                                     | 9                | #23 只缓存了 accent：`glass_lift`/`glass_tint`（背板每帧 2 次）、`ink`（每页每帧多次）、`layout_i64`/`radius_px`/`font_size`、`quick_alarm_minutes`（每帧数组解析 + Vec 分配）仍逐帧 pointer 解析                                                                                                                                                                                                                                                            | 缓存到 `OnceLock` 或每帧取一次向下传（`quick_alarm_minutes` 缺键 panic 属「嵌入配置快速失败」，仅性能项）                 | 遗留      | 性能                |
| 11   | P3   | config/static/ui.json:6,47 ＋ accel-app/src/theme.rs:196                                                                                                                                                                                                                                                                  | 12/6             | 配置死键清尾：`scale.city_time(20)`、`radius.xl(24)` 全仓零消费（A005 #8 已列，未清）；`theme.rs:196` 注释仍列已删键 `page_title`                                                                                                                                                                                                                                                                                                                            | 接线或删除两键；注释同步                                                                                                  | 遗留      | 配置体系 / 文档     |
| 12   | P3   | accel-app/src/pages/mod.rs（文件尾）＋ :2                                                                                                                                                                                                                                                                                 | 6                | #17⑧ 漏改：仍缺 `// ===== 说明区 =====`（全仓 53 个 `.rs` 中唯一）；且 `:2`「六页实体」与 `:32`「五页 struct」、`:115`「五页实体」自相矛盾                                                                                                                                                                                                                                                                                                                   | 补文件尾说明区；统一为「五页 + 设置浮窗」                                                                                 | 遗留      | 文档                |
| 13   | P3   | README.md:58,130＋accel-app/Cargo.toml:7＋x.progress.md:356／z.plan.md:839,1114／m.milestone.md:219＋x.progress.md:5／z.plan.md:4＋accel-core/src/interface.rs:983＋data/timezones.rs:51＋data/mod.rs:2,10＋settings.rs:272,283-287＋platform/tray.rs:133                                                                 | 6                | 文档/说明区批量残留：①产物名 `accelworld.exe`（实为 `accel-app.exe`，与 rs/README.md:18 冲突）；②依赖表仍列已移除的 `egui_extras`；③Cargo description 与 RS 期文档仍写「六页」；④x.progress:5／z.plan:4 状态表述仍含 FIX005 进行中语义；⑤测试注释「48 城」实 50；⑥说明区仍列已删 API；⑦「三表」实四表；⑧settings.rs:272 悬空 `///`（描述已删函数）、:283-287 编号重复且自相矛盾；⑨tray.rs:133 与 132 自相矛盾（token 已删 vs 留 base.json）——⑨为**修复引入** | 逐条更正（建议并入一次文档清扫）；「已完成」断言须与产物一致                                                              | 遗留+新增 | 文档                |
| 14   | P3   | glass/surface.rs:105,117,178,185,192-202 ＋ pages/mod.rs:102-113 ＋ tools/alarm_text.rs:23                                                                                                                                                                                                                                | 5                | 生产零调用余留（上轮判定保留，理由已登记）：`unregister`/`update_geometry`/`draw_list`/`total_count`/`dirty`/`clear_dirty`、`SettingsPop`（空 impl + `open` 零读写）、`format_sound_button_name`；`surface.rs:206,208` 说明区仍称「输出 draw_list（painter 消费）」而 painter 已改 `surfaces()` 栈数组                                                                                                                                                       | UI 雕刻阶段清理或接线；说明区同步                                                                                         | 遗留      | 跨模块 / 文档       |
| 15   | P3   | accel-app/src/pages/mod.rs:73 ＋ 五页各自 `register_page_surface`（clock.rs:35 等）                                                                                                                                                                                                                                       | 4                | 同一帧对当前页玻璃面注册两次（容器一次 + 页内一次），幂等但冗余（每帧多一次几何写入与脏标记）                                                                                                                                                                                                                                                                                                                                                                | 去掉页内重复注册，统一由容器负责                                                                                          | 新增      | GUI 面板            |
| 16   | P3   | accel-app/src/bin/render_probe.rs:252-253                                                                                                                                                                                                                                                                                 | 6                | 断言 3 注释称「卡片顶缘像素 y=290」，实际 `rect(170,190,300,100)` → y∈[190,290]，y=290 为底缘；采样距离描述（1.3px / 2px）需校准                                                                                                                                                                                                                                                                                                                             | 更正为「底缘」并校准措辞                                                                                                  | 新增      | 测试 / 文档         |
| 17   | P3   | accel-app/src/bin/render_probe.rs:199-205                                                                                                                                                                                                                                                                                 | 2                | `let _ = device.poll(PollType::Wait{…})` 丢弃结果；poll 出错则随后 `rx.recv()` 无超时 → 探针可能永久挂起                                                                                                                                                                                                                                                                                                                                                     | 判 poll 结果或改 `recv_timeout` 兜底                                                                                      | 新增      | 测试                |
| 18   | P3   | rs/ci.ps1:19-24 ＋ .github/workflows/rs-ci.yml:47-54                                                                                                                                                                                                                                                                      | 2/4              | 关卡 4 未校验退出码（1–3 关卡均检）：`cargo tree` 失败 → `$tree` 为空 → `-match` 假 → 静默通过                                                                                                                                                                                                                                                                                                                                                               | 关卡 4 后加 `if ($LASTEXITCODE -ne 0) { … exit 1 }`（两处同改）                                                           | 新增      | 测试                |
| 19   | P3   | rs/Cargo.toml:30                                                                                                                                                                                                                                                                                                          | 12               | `windows` 依赖声明 feature `Win32_UI_WindowsAndMessaging` 全仓零使用（仅用 Foundation/Graphics_Dwm/UI_Controls）                                                                                                                                                                                                                                                                                                                                             | 删除该 feature                                                                                                            | 新增      | 配置体系            |
| 20   | P3   | accel-core/tests/parity.rs:538-555                                                                                                                                                                                                                                                                                        | 2/10             | 临时目录清理非 RAII（见零节 #26）：中间 panic 则跳过后置清理，`%TEMP%/accel_cfgparity_*` 残留                                                                                                                                                                                                                                                                                                                                                                | 改 drop guard / TempDir                                                                                                   | 遗留      | 测试                |
| 21   | P3   | accel-core/src/weather.rs:356-361                                                                                                                                                                                                                                                                                         | 12               | `custom()` 构造内 `ttl: Duration::from_secs(1800)` 写死，与 base.json `weather_cache_ttl` 双源（token 改动后该构造不再同步）                                                                                                                                                                                                                                                                                                                                 | 默认 TTL 提为常量或由调用方注入                                                                                           | 新增      | 天气服务 / 配置体系 |
| 22   | P3   | accel-core/src/interface.rs:180-182 ↔ :269-271                                                                                                                                                                                                                                                                            | 11               | 同域「已存在」幂等语义相反：`set_weather_city` 同名返 true、`add_world_city` 同城返 false                                                                                                                                                                                                                                                                                                                                                                    | 统一并登记说明区                                                                                                          | 新增      | 接口层              |
| 23   | P3   | accel-core/src/data/world_cities.rs:1 ＋ interface.rs:150-154 ＋ data/cities.rs:36-38                                                                                                                                                                                                                                     | 12/6             | 城市数据源双声明：`world_cities.rs:1` 自称「世界时钟/天气多城市共用统一城市表」，而天气候选实取 `cities` 表（17 城，rs/data/cities.json）；两表并存（world_cities 50 城）                                                                                                                                                                                                                                                                                    | 统一数据源或更正注释与表职责                                                                                              | 遗留      | 跨模块 / 文档       |
| 24   | P3   | accel-app/src/pages/weather.rs:137 ＋ bus.rs:307-350                                                                                                                                                                                                                                                                      | 5/10             | 冗余语句 `let _ = cmd;`（cmd 上文已多次使用）；`drain` 返回消费条数被调用方丢弃，说明区称「帧计数/节奏探针用」失实                                                                                                                                                                                                                                                                                                                                           | 删除或使用                                                                                                                | 新增      | GUI 面板 / 文档     |
| 25   | P3   | widgets/ring.rs:81 ＋ widgets/mod.rs:10 ＋ widgets/chip.rs:62-63 ＋ pages/mod.rs:2 ＋ glass/surface.rs:206,214                                                                                                                                                                                                            | 6                | 说明区/注释失实批量：ring 称 `progress_ring_size`「PL015 观感轮接 token」实已接线（clock.rs:56）；chip 声称消费方含 countdown/world/weather 实仅 clock；surface 见 #14                                                                                                                                                                                                                                                                                       | 逐条更正                                                                                                                  | 遗留      | 文档                |
| 26 ★ | P3   | accel-core/tests/parity.rs（config_parity 断言面）＋ fixtures/manifest.json                                                                                                                                                                                                                                               | 10 可测性        | 夹具计数未被断言：manifest 的 `config_dirty_cases=22`/`n_probe=24` 等计数仅由主会话手工实测复核，测试仅遍历数组 → 计数漂移不被捕获                                                                                                                                                                                                                                                                                                                           | parity 补 `assert_eq!` 断言 manifest counts 全字段（或至少上述两项）                                                      | 新增      | 测试                |
| 27 ★ | P3   | accel-app/src/pages/clock.rs:47 ＋ accel-core/src/time_dilation.rs                                                                                                                                                                                                                                                        | 2 防御性         | `remaining_hours / day_hours` 理论除零：`day_hours` 由 rate 派生，`rate_min` 配置改至 <0.042 时可达（当前 rate_min=1.0 不可达）                                                                                                                                                                                                                                                                                                                              | 前置 `day_hours.max(1)` 钳制或 `debug_assert` 域校验                                                                      | 新增      | 时间膨胀 / GUI 面板 |
| 28 ★ | P3   | accel-app/src/platform/acrylic.rs:4 ＋ platform/{hotkey,tray}.rs 非 Windows 桩                                                                                                                                                                                                                                            | 12/6             | 非 Windows 构建面缺口：`#[cfg(windows)]` 块内的 `use tracing::error` 在非 Windows 构建产生 unused_imports 告警；非 Windows 桩缺 `hotkey::shortcut_specs/parse_binding`、`tray::ID_*/command_for` 对等导出（均为 pub，当前无外部调用）                                                                                                                                                                                                                        | 导入移入 cfg 块（或整体 `#[cfg]`）；桩补齐对等导出面                                                                      | 新增      | 托盘与音频          |
| 29 ★ | P3   | accel-core/examples/lunar_compare.rs:82-91                                                                                                                                                                                                                                                                                | 4 重复实现       | `cmp` 闭包两个 `{:>2}` 恒传字面量 `0,0`（月/日占位残迹），输出冗余列                                                                                                                                                                                                                                                                                                                                                                                         | 删除冗余占位或传真实月/日                                                                                                 | 新增      | 测试                |

### 二、参考级观察项（记录不修；含回落理由）

> 本节仅列**经用户复核后仍维持观察级**的 18 条；另 **4 条已提升为 P3**（见上表 #26–#29 ★），已同步定案豁免清单。

| 文件:行号                                                                | 描述                                                                                           | 回落理由                                                              |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| accel-app/src/nav.rs:38-45,88,110,143,234、widgets/{ring,slider,chip}.rs | 派生局部几何常量（rail_x/pill_w/pill_h、TRACK_PAD、环 inset 与段数、chip 内距）                | 绘制几何细节参数，无调参诉求（A005 豁免延续；应 token 化项已并入 #5） |
| accel-app/src/glass/shader.rs:61-72,95-131、surface.rs:31-34             | shader 材质算法常量与 GlassSurface 默认值                                                      | 有注释依据的材质参数（HINT002 逐条勾销，A005 复核零变化）             |
| accel-app/src/glass/shader.rs:221-225                                    | `to_bytes()` 内每帧 `assert_eq!(len, UNIFORMS_SIZE)`                                           | 布局不变量、单测已锚定，改动布局即暴露属期望                          |
| accel-core/src/alarm.rs:63-71,170                                        | `PRESET_SOUND_CONFIG` 枚举参数表；`obj["sound_type"].as_str().unwrap()`                        | 有注释依据的固定枚举表；前置 match 已保证该键为字符串（可证明安全）   |
| accel-core/src/interface.rs:24,549-551                                   | `BUS_CAPACITY=64`；每调用 `settings::load`（无缓存，磁盘 IO）                                  | 单次使用常量；说明区已登记为已知偏移，调用频率低（非逐帧）            |
| accel-core/src/weather.rs:307-308                                        | `LAT_RANGE/LON_RANGE` 地理定义域常量                                                           | 地理常量、有注释依据（SSRF 校验用）                                   |
| accel-core/src/data/world_cities.rs:18                                   | world_cities.json 置 config/static/ 而其余表在 rs/data/（include_str 跨约定）                  | 说明区已登记 z.plan 执行偏移，非缺陷                                  |
| accel-core/tests/parity.rs:571                                           | 「数据三表 parity」未含 world_cities 表断言（该表由内联单测覆盖规模/唯一性/tz）                | 覆盖方式差异，非缺失                                                  |
| accel-core/src/settings.rs:216、alarm.rs:212                             | `to_json_value` 用 `.expect("不可能失败")`                                                     | 全字段 serde 可序列化，不可达                                         |
| accel-app/src/platform/notify.rs:5、tray.rs 菜单文案、acrylic.rs:7       | AUMID / 中文菜单文案 / `DWMSBT_TRANSIENTWINDOW`                                                | 平台常量与 UI 文案，单次使用无调参场景                                |
| accel-app/src/bin/render_probe.rs:13-15                                  | 断言阈值（12 / 60 / BG_ALPHA_RANGE）                                                           | 有注释依据的探针基线（PL013.07 实测），非业务参数                     |
| rs/fixtures/README.md:13-14                                              | 「双跑 sha256 可再生性检验」依赖已归档的 `.archived/export_fixtures.py`（gitignore 不入库）    | 库内不可复现，登记不修                                                |
| accel-app/src/pages/ime_sim.rs（整文件 `#![cfg(test)]`）                 | 仅测试模块                                                                                     | 设计定案（真机 IME 实测取消）                                         |
| accel-app/src/window.rs:692-696                                          | 字体搜索目录固定 `C:/Windows/Fonts` 等                                                         | 平台集成路径常量（族名已 token 化）                                   |
| accel-app/src/theme.rs:211,238,277                                       | accent/tint 键损坏兜底色值                                                                     | 纯防御路径，键齐备时不可达                                            |
| accel-core/src/alarm.rs:560-570                                          | `quick_target(minutes, now)` 注入 `now` 而落库 `created_at` 取真实钟（测试注入与落库或不同源） | 生产路径两者同源，仅测试语义缝隙                                      |
| y.problems.md:18-19                                                      | 状态列「见 #6/#7」行内自引用                                                                   | 轻微表述瑕疵                                                          |
| z.plan.md:830,910                                                        | 「总纲修订二」仍列 `egui_extras`                                                               | 历史定案 spec（A005 已豁免历史 spec 漂移）                            |

### 三、亮点

- 注释房规执行度高：53 个 `.rs` 中 52 个具备模块头 + 函数 `///` + 文件尾说明区（唯一缺失即 #12），说明区普遍如实登记已知偏移与降级理由。
- 零硬编码主线成立：base.json 业务参数与 ui.json 的 `field.*`/`nav.*`/`scale`/`colors.primary` 均接线消费；配置缺失/错型 panic 带键名（快速失败可定位）。
- 可测性扎实：`Clock` trait（VirtualClock 拨钟）使闹钟调度全链无真睡眠；`RetrySpec`/`IsRetryable` 白名单分级；全量测试 163（app 54 + core 91 + parity 18）含新加的命令零丢失与副作用线程两条回归。
- 安全面闭环：URL 由构造期固定 base + 坐标定义域校验派生（SSRF 结构性消除）、禁跟随重定向、写白名单 + 原子写、铃声扩展名白名单；平台能力失败一律降级不崩，非 Windows 编译期桩化同形。
- 架构不变量机器化：CI 第 4 关卡以 cargo tree 强制 core 依赖树零 egui/eframe；四关卡本地与 Actions 同集。
- 夹具完整性实测达标：manifest 哈希 6/6、计数 10/10 与实测一致（A005 #10 已彻底闭环）。
- 上轮修复到位率高：32 项中 24 项完整在位，8 项缺口均为「覆盖面」而非「回退」；本轮 P1 的唯一根因（world_pins 域）为 A005 未触及的真实盲区。

---

## 附录 A007：全量代码审计报告（第 7 轮，Rust 侧第 3 轮，2026-10-18）

> 范围：`rs/`（accel-core 18 源文件 + accel-app 33 源文件 + 集成测试与 example + 4 份静态配置 + 3 份 Cargo 清单 + 工具链与 CI（`rs/ci.ps1`、`.github/workflows/rs-ci.yml`）+ fixtures 8 份 + 根文档 9 件）；**`.archived/`（Python 归档）按用户要求排除**，`rs/target/`、`rs/logs/`、`.temp/`、`.mimosa/`、第三方依赖同样排除。
> 方式：核心域主会话亲读 + GUI 层／平台与基建／文档与夹具三路并行后由主会话**逐条确证**；`gen_id` 缺陷以 **rustc 同语言实证探头**（`.temp/genid_probe.rs`）复现；token 完备性以**机器化对账**（`.temp/a007_token_crosscheck.ps1`：零缺键、零死键）；上轮 30 项以 `.temp/a007_regression.ps1` 机器化复核（35 OK / 1 MISS，MISS 为注释历史引用非回归）。
> 维度覆盖：13 类逐类对照；**第 7 类（安全）本轮无新增**——URL 构造期固定 base + 坐标定义域校验（SSRF 结构性消除）、禁跟随重定向、写白名单 + 原子写、铃声扩展名白名单仍闭环；符号链接 TOCTOU 为已登记豁免（条件豁免）。
> 状态：✅ 修复完成（2026-10-22，FIX007 全 17 条落地——四关卡 ALL_GREEN + 测试 186 通过 + render_probe 0 FAIL + 真窗探针 exit 0；反向验收 `.temp/verify_fix007_accept.md`；执行补录三处口径修正见 x.progress FIX007 完成注记：gen_id 低 32 位建议算术不可行改计数+椒盐／weather_code 字符串按 Python 实测保留未知码回退／夹具迁移以 from_json_value 透传域为回归锁）

### 零、上轮（A006 / FIX006，提交 c24d957）修复复核清单

机器化复核 35 项 OK；逐项判定（**16 项完整在位 / 14 项存在缺口**，无行为性回退）：

| 上轮条目                                                                                                                                                                                                                                                                     | 现状                | 证据                                                                                                                                                                                           |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #2 命令出口 · #4 冷启动天气 · #5 token 化 · #6 save_ack 复位 · #8 DST 歧义 · #11 配置死键 · #12 pages/mod.rs 说明区 · #17 探针兜底 · #18 CI 关卡 4 退出码 · #19 未用 feature · #20 parity RAII · #24 冗余语句 · #25 注释失实 · #27 day_hours 钳制 · #29 lunar_compare · 收尾 | ✅ 仍在（16 项）    | `rg` 各项符合；本轮实测夹具 **sha256 6/6 + counts 10/10** 与实测一致；配置 token 对账**零缺键零死键**                                                                                          |
| #1 世界时钟域统一                                                                                                                                                                                                                                                            | ⚠️ 引入新缺陷       | 按 `city.tz` 落盘到位（`interface.rs:276-280`），但**同 tz 多城身份丢失** → #2                                                                                                                 |
| #3 节日派生接线                                                                                                                                                                                                                                                              | ⚠️ 部分             | 已接线（`countdown.rs:119,170`），引入每帧农历重算（#7）与跨页 chip 面残留（#3）                                                                                                               |
| #7 管线死分支                                                                                                                                                                                                                                                                | ⚠️ 部分             | 代码已删（`window.rs:284-294`）；`window.rs:838` 说明区仍称「管线失败降级纯色背板」→ #11                                                                                                       |
| #9 天气去抖                                                                                                                                                                                                                                                                  | ⚠️ 部分             | 在飞表/去抖/Quit abort 到位；**任务自摘以城市键**（force 替换竞态）→ #5                                                                                                                        |
| #10 键缓存                                                                                                                                                                                                                                                                   | ⚠️ 部分             | theme 侧闭环；`window.rs:439-464 page_margins()` 未纳入 → #6                                                                                                                                   |
| #13 文档清扫 / #23 数据源职责                                                                                                                                                                                                                                                | ⚠️ 部分             | 代码注释已改；`README.md:107,159`、`w.study.md:35`、`interface.rs:261,263,295,1079`、`data/world_cities.rs:54`、`data/cities.rs:65,67`、`lib.rs:27`、`accel-app/Cargo.toml:45,48` 未同步 → #11 |
| #14 死代码裁决                                                                                                                                                                                                                                                               | ⚠️ 部分             | `static_config.rs:82,114`、`world_cities.rs:16`、`weather_codes.rs:58` 未裁决；`world_cities.rs:50-51` 悬空 `///` → #10                                                                        |
| #15 页卡重复注册                                                                                                                                                                                                                                                             | ⚠️ 漏改             | 页卡面正确；**chip 子面注册后从不隐藏** → #3                                                                                                                                                   |
| #16 探针注释                                                                                                                                                                                                                                                                 | ⚠️ 部分             | 断言段已改；同文件说明区 `render_probe.rs:320` 仍写「~1.3px」→ #11                                                                                                                             |
| #21 TTL 单源                                                                                                                                                                                                                                                                 | ⚠️ 未闭环           | `DEFAULT_CACHE_TTL` 与 `base.json weather_cache_ttl` 仍双处同值 → #9                                                                                                                           |
| #22 幂等语义                                                                                                                                                                                                                                                                 | ⚠️ 部分             | 幂等 true 已落地；重复 tz **静默不落盘无日志**、`remove_world_city` 入参域不对称 → #2                                                                                                          |
| #26 夹具计数断言                                                                                                                                                                                                                                                             | ⚠️ 部分             | 10 键中 7 键仅断言 `>0` → #12                                                                                                                                                                  |
| #28 非 Windows 编译期桩                                                                                                                                                                                                                                                      | ⚠️ 处置与表述不准确 | 文件级桩为**死代码**（`platform/mod.rs:7-12` 仅 `#[cfg(windows)]` 载入文件模块，`:14-76` 已有内联桩）；真缺口是 `acrylic.rs:5` 常量未 cfg → #4                                                 |

### 一、P0–P3 修复清单（按严重度；P0 无）

> 性质：**遗留** = A006 已列未修/修不彻底；**新增** = 本轮新发现。★ = 本轮观察项经用户复核提升。

| #    | 级别 | 文件:行号                                                                                                                                                                                                                                  | 类型      | 描述                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | 建议                                                                                                                                                 | 性质                    | 影响面                 |
| ---- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- | ---------------------- |
| 1    | P1   | accel-core/src/alarm.rs:379-387（影响面 :438-503）                                                                                                                                                                                         | 1 正确性  | **`gen_id` 生成的 8 位 id 会碰撞**：`mix = (pid as u128) ^ nanos.rotate_left(64)`，`nanos < 2^64` 故 `rotate_left(64)` 等价 `<<64`，pid 只落低 32 位；`format!("{mix:08x}")[..8]` 取最高 8 个十六进制位 = nanos 的 bit32..63。**rustc 同语言实证**（`.temp/genid_probe.rs`）：同刻 pid=1234 与 987654 同为 `186cc6ac`；同刻间隔 1ms 连发 5 次 → 5 个 id 全同；粒度 = `2^32 ns ≈ 4.295 s`。后果：约 4.3s 内新建的多条闹钟共 id → `remove_alarm`/`toggle_alarm`/`get_alarm`（:444-482）只命中首条（删/开关错对象）；`check_alarms` 去重键 `last_triggered[id]`（:493-499）令「同时间不同标签」的两条闹钟只响一条；`AddAlarm` 的 (time,label) 去重不拦此情形。Python 对照 `uuid4()[:8]` 随机无碰撞。说明区 :590「pid×纳秒混合」亦失实 | 取 `mix` 低 32 位（`& 0xFFFF_FFFF`）或改为「pid 混入 + 时间戳 + 自增计数」/uuid；同步说明区                                                          | 新增                    | 闹钟服务               |
| 2    | P1   | config/static/world_cities.json ＋ accel-core/src/data/world_cities.rs:46-48 ＋ interface.rs:270-293 ＋ accel-app/src/pages/world.rs:51-53,72,82-89                                                                                        | 1/11      | **`world_pins` 在 tz 层丢失城市身份**：50 城表 IANA **有重复**（`Asia/Shanghai`×10、`Asia/Tokyo`×2、`Asia/Kolkata`×2、`America/Los_Angeles`×2）。`name_of_tz` 返首匹配名、`add_world_city` 按 `city.tz` 去重 → ①加入「大阪」落盘 `Asia/Tokyo`、卡片显示名反查为**「东京」**（错值，确定性）；②加入「上海/广州/…」时 tz 已常驻（默认含 `Asia/Shanghai`）→ **返回 true 但静默不落盘**（点击无反馈无日志）；③`remove_world_city(name: &str)` 实比较 IANA 串，与添加侧入参域不对称，误传城市名静默返 false                                                                                                                                                                                                                             | 统一常驻集**城市身份**（存城市名或 `name+tz` 复合键），`name_of_tz` 退化/删除；`remove_world_city` 参数改名并统一域；静默分支补日志                  | 新增（FIX006.1 副作用） | 世界时钟页 / 接口层    |
| 3    | P1   | accel-app/src/pages/clock.rs:105-109 ＋ pages/countdown.rs:178-189 ＋ window.rs:318-325 ＋ glass/surface.rs:84-102 ＋ glass/painter.rs:172-186                                                                                             | 1/9       | **chip 子玻璃面切页后永不隐藏**：clock 页 `lunar_main/phase/god` 与 countdown 页 `festival_{date}`（6 枚）经 `chip::show` 注册（默认 `visible=true`），而 `sync_page_visibility` 只对 `PAGE_ORDER` 五页卡面 `set_visible`、`set_tint_all` 只改 tint → 切页后 chip 矩形仍被 painter 每帧合成（CentralPanel 透明不遮盖）→ 游离玻璃矩形，违反 `window.rs:318` 自述。复合后果：chip id 以日期为键且从不 `unregister`，长期运行将逼近 `MAX_SURFACES=16`（页卡 5 + rail 1 + chip 9 = 15），此后新页卡面被 register 拒收 → 页卡玻璃整体消失                                                                                                                                                                                               | 切页时对非当前页 chip 子面统一 `set_visible(false)`（或 chip 注册携带页归属）；补断言「切页后非当前页可见面数=0」；`festival_*` 改固定 id 集或加注销 | 新增（FIX006.15 漏改）  | GUI 面板               |
| 4    | P2   | platform/tray.rs:129-144、hotkey.rs:98-111、notify.rs:42-46 ＋ platform/mod.rs:7-12,14-76 ＋ platform/acrylic.rs:5 ＋ x.progress.md:472、m.milestone.md:251                                                                                | 5/6/12    | **非 Windows 面处置与表述不准确**：`mod.rs:7-12` 仅 `#[cfg(windows)]` 载入三文件模块，非 Windows 走 `mod.rs:14-76` 内联桩 → 上轮在三个 `.rs` 内新增的 `#[cfg(not(windows))]` 块**永不被编译（死代码 + 与内联桩重复定义）**；`x.progress.md:472`「原无任何 cfg 桩→无法编译」与 `m.milestone.md:251` 同义句**可证失实**（`git show a17cdd2:…/platform/mod.rs` 已含内联桩）。真缺口：`acrylic.rs:5` 常量未 cfg 化 → 非 Windows `dead_code` 告警，房规「非 Windows 零告警」仍不成立（**需验证**：本机交叉检查被 `aws-lc-sys` 缺 Linux C 工具链阻断）                                                                                                                                                                                   | 删三处文件级死桩（保留内联桩）；该常量加 `#[cfg(windows)]`；订正两处文档措辞                                                                         | 新增                    | 跨模块 / 文档 / 构建   |
| 5    | P3   | accel-core/src/interface.rs:471-475（配合 :434-478、:554-561）                                                                                                                                                                             | 8 并发    | **在飞表自摘以城市键**：任务尾 `inflight_map.remove(&city_task)` 用城市名而非任务身份；force 分支先 `remove+abort` 旧任务再 `insert` 新任务，若旧任务已越过最后一个 `await`（abort 不生效）则删掉**新任务条目** → 去抖短暂失效、`Quit` 统一 abort 漏掉在飞任务（`evt_tx` 克隆推迟通道闭合）。窗口窄、后果有界                                                                                                                                                                                                                                                                                                                                                                                                                      | 自摘改按任务身份（`JoinHandle::id()`）或改 `JoinSet`                                                                                                 | 新增                    | 天气服务 / 接口层      |
| 6    | P3   | accel-app/src/window.rs:439-464（调用点 :431,457,462）                                                                                                                                                                                     | 9 性能    | **`page_margins()` 每帧多次解析**（`content_page_rect`/`content_left_inset`/`page_inner_margin` 各一次）——A006 #10 缓存未覆盖此路径                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 纳入 `theme::cached`（键 `page_margin`）                                                                                                             | 遗留（#10）             | 性能                   |
| 7    | P3   | accel-app/src/pages/countdown.rs:119,170                                                                                                                                                                                                   | 9 性能    | **每帧重算农历派生**（`next_spring_festival` 转换循环 + `year_chips` 重建 BTreeMap/遍历两年规则/节气取表），输入为日粒度                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 以 `now.date()` 为键缓存或仅日期变化时重算                                                                                                           | 新增                    | GUI 面板 / 性能        |
| 8    | P3   | accel-app/src/theme.rs:218-233,265-284                                                                                                                                                                                                     | 2/9       | **`color32_at` 缓存键不含兜底色**：当前 23 处调用各 key 的 `fallback_gray` 唯一（已全量核对）→ 当前无误；将来同 key 不同兜底则首值固化 → 静默错色                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | 键改 `format!("{key}#{fallback_gray}")` 或加调试断言                                                                                                 | 新增                    | 防御性                 |
| 9    | P3   | accel-core/src/weather.rs:348-350 ↔ config/static/base.json:32                                                                                                                                                                             | 12 配置   | **TTL 双源未闭环**：`DEFAULT_CACHE_TTL=1800` 与 token `weather_cache_ttl=1800` 同值两处，token 变更后 `custom()` 漂移                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 移除该常量，由调用方注入 ttl                                                                                                                         | 遗留（#21）             | 天气服务 / 配置体系    |
| 10   | P3   | accel-core/src/static_config.rs:82,114 ＋ data/world_cities.rs:16,50-51 ＋ data/weather_codes.rs:58 ＋ settings.rs:26-48                                                                                                                   | 5/4/6     | **死代码/死字段/重复实现/悬空注释**：`ui_f64`、`ui_nested_f64` 全仓零调用；`WorldCity.region` 仅反序列化无读取；`weather_codes::unknown()` 生产零调用（仅 `parity.rs:703` 与自身单测）；`CountdownTarget::parse_at` 与 `parse_date_text` 函数体逐行相同；`world_cities.rs:50-51` 悬空 `///`（原 `coords_of` 描述）挂到下方测试模块                                                                                                                                                                                                                                                                                                                                                                                                 | 逐项删除/接线；`parse_at` 委托 `parse_date_text`；删悬空注释                                                                                         | 新增                    | 跨模块                 |
| 11   | P3   | README.md:107,159 ＋ w.study.md:35 ＋ accel-core/src/interface.rs:261,263,295,1079 ＋ data/world_cities.rs:54 ＋ data/cities.rs:65,67 ＋ lib.rs:27 ＋ bin/render_probe.rs:320 ＋ window.rs:838 ＋ bus.rs:306 ＋ accel-app/Cargo.toml:45,48 | 6 文档    | **FIX006 改动未同步的下游文档/注释族**：①`README.md:159` 称天气覆盖「国内外 **50** 个主要城市（`world_cities.json`）」，实为 **17 城**（`interface.rs:150-154` → `data::cities`）；②`README.md:107`/`w.study.md:35`/`interface.rs:261,263,295,1079`/`world_cities.rs:54`/`cities.rs:65,67` 仍称「统一城市表/共用同一城市源/单源不双轨」，与 FIX006.23 的职责分离自相矛盾；③`lib.rs:27`「三静态表」与 `data/mod.rs:2`「四表」不一致；④`render_probe.rs:320` 仍写「峰在面内侧 ~1.3px」；⑤`window.rs:838` 仍称「管线失败降级纯色背板」；⑥`bus.rs:306` 称 `drain` 返回值「帧计数/节奏探针用」而唯一调用方 `window.rs:72` 直接丢弃；⑦`accel-app/Cargo.toml:45,48` 仍写「PL011 占位：仅依赖 core」且列 `rodio`（实属 accel-core）        | 逐条更正（并入一次文档清扫）；「已完成」断言须与产物一致                                                                                             | 遗留（#13/#16/#23/#7）  | 文档                   |
| 12   | P3   | accel-core/tests/parity.rs:595-630                                                                                                                                                                                                         | 10 可测性 | **夹具计数断言仅半程**：10 键中 `n_cities/n_tz/n_codes` 与真实表 `assert_eq!`，其余 7 键仅断言 `>0` → 数值漂移不被捕获                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 补与夹具数组长度/实测值的 `assert_eq!` 对账                                                                                                          | 遗留（#26）             | 测试                   |
| 13   | P3   | accel-app/src/bus.rs:173-180                                                                                                                                                                                                               | 10/13     | **副作用线程创建失败被 `.ok()` 吞掉**且不记日志（线程缺席时仅靠 send 失败侧记 error，信息不足）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | `if let Err(e) = …spawn(…) { tracing::error!(…) }`                                                                                                   | 新增                    | 托盘与音频             |
| 14   | P3   | accel-app/src/main.rs:35-38                                                                                                                                                                                                                | 2 防御性  | `1000 * secs / tick`（`tick = token clock_tick_ms`）在 token=0 时**除零 panic**；当前 token=100 不可达（**需验证**）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | `tick.max(1)` 钳制                                                                                                                                   | 新增                    | 探针 / 配置体系        |
| 15 ★ | P3   | accel-app/src/logging.rs:22 ↔ accel-core/src/interface.rs:88-93（AGENTS「现役用户配置在 rs/config/user_config.json」表述）                                                                                                                 | 9/6       | **日志与配置路径基准不一致**：日志目录为 **CWD 相对**（`logs`），配置路径为 **exe 相对**（`exe_dir/config/user_config.json`）；`cargo run`（CWD=`rs/`）下配置落 `rs/target/debug/config/` 而非 `rs/config/`，与 AGENTS 表述存在张力                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 统一基准（均取 exe 相对，或均取项目根相对）；AGENTS 表述同步                                                                                         | 新增                    | 日志 / 配置体系 / 文档 |
| 16 ★ | P3   | accel-core/src/weather.rs:207,231-249                                                                                                                                                                                                      | 2 防御性  | **`weather_code` 非标量宽松容错**：`weather_code` 为数组/对象时 Rust 返「未知」并成功，Python 对照 `dict.get(list)` 会抛 TypeError 走失败路径（口径不一致）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | 补类型校验：非整数码 → `WeatherErr::Parse`（与 Python 失败语义对齐）                                                                                 | 新增                    | 天气服务               |

### 二、参考级观察项（记录不修；含回落理由）

> 本节仅列**经用户复核后仍维持观察级**的 9 条；另 **2 条已提升为 P3**（见上表 #15–#16 ★），已同步定案豁免清单。

| 文件:行号                                                                                                                            | 描述                                                                       | 回落理由                                                                            |
| ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| accel-app/src/nav.rs:31,38-45,88,119-130,202-247、widgets/{ring,slider,chip}.rs、各页 `add_space(4/8/12/16)`、window.rs:340 浮窗锚点 | 派生局部几何/间距字面量                                                    | 绘制几何细节参数，无调参诉求（历轮豁免延续；应 token 化项已落地）                   |
| accel-app/src/glass/shader.rs:61-72,95-131,221-225、surface.rs:31-34、theme.rs:211,238,277                                           | shader 材质算法常量 / `to_bytes` 断言 / GlassSurface 默认值 / 键损坏兜底色 | 有注释依据 + 单测锚定 + 键齐备时不可达（永久豁免）                                  |
| accel-app/src/glass/surface.rs:105,117,178,185,192-202                                                                               | 注册表六方法生产零调用                                                     | A006 #14 定案「测试即契约」保留并就地登记理由                                       |
| accel-core/src/audio.rs:20-31,52,59-60                                                                                               | `amplify(0.2)`／静音段 44.1kHz 单声道／懒初始化持锁探测设备                | 有注释依据的音频参数；单次点火无可达性能危害                                        |
| accel-core/src/util/file_io.rs:37-41                                                                                                 | 不存在路径仅词法规范化，不解析符号链接 → 理论可经 symlink 越白名单写       | 配置路径受控、无用户可控写入面（A005 已登记设计，**需验证**）——本轮定案**条件豁免** |
| accel-app/src/bin/render_probe.rs:13-15                                                                                              | 断言阈值 12／60／BG_ALPHA_RANGE                                            | 有注释依据的实测基线                                                                |
| accel-core/src/weather.rs:338-339、alarm.rs:563-571、accel-app/src/pages/countdown.rs:55                                             | 固定 API base（SSRF 防线组成部分）／`quick_target` 中文标签／24h 校验常量  | 构造期固定 base 有注释依据；UI 文案与 UX 规则无调参诉求                             |
| z.plan.md:170/177、171/178、172/179                                                                                                  | 豁免清单 A005 与 A006 同源条目重叠                                         | 归拢说明已声明「多轮去重 + 留印」口径，保留各轮日期留档，无实际危害                 |
| w.study.md:46、y.problems.md:18-19,21-24、z.plan.md:948,1028                                                                         | 「三数据表…+world_cities」表述瑕疵／行内自引用／总纲仍列 `egui_extras`     | 表述瑕疵 + 历史 spec 漂移（A005 已豁免）                                            |

### 三、亮点

- **配置对账机器化通过**：`.temp/a007_token_crosscheck.ps1` 全量核对 `scale/layout/radius/colors` 与 `base.json` 引用键 —— **零缺键**（唯一 `no_such_key` 为负例测试）、**零死键**（`nav_pill_*` 经 `nav.rs` 局部 `nav_token()` 消费、`quick_alarm_minutes` 走 JSON 直读、`primary`/`page_margin` 走 pointer）。
- **夹具完整性实测**：`manifest.json` **sha256 6/6** 与 **counts 10/10** 与实测一致（`config_dirty_cases=22`、`n_codes=28`、`n_samples=61` 等逐项吻合）。
- **CI 体系同源**：`rs/ci.ps1` 与 `.github/workflows/rs-ci.yml` 关卡 4 命令文本逐字一致且均判退出码；四关卡（fmt / clippy `-D warnings` / test / 依赖树零 egui）与提交时实测一致。
- **注释房规 53/53**：全仓 `.rs` 文件模块头 + 函数 `///` + 文件尾说明区齐备。
- **架构不变量机器化 + 依赖收敛**：core 依赖树零 egui 由 CI 强制；`windows` 无未用 feature；双 bin 经 `default-run` 消歧。
- **上轮增量落地率高**：30 项中 16 项完整在位、14 项为覆盖面/表述缺口，仅 2 项属真实新缺陷（P1×2）、1 项为处置表述不准（P2）；无一项回退到修复前状态。

---

## UIR 界面推倒重塑执行纪要（2026-10-23，V0.6.1.0；补录于 A008 归档时）

> 缘由：A007 修复收口后用户发起 UI 修复评估，定案「方案 C 推倒重来，不受原设计约束」；一阶段（玻璃框架内 Dock/表盘重塑）执行中被用户否决——「现在的界面就和屎一样……不要在意以前的设定，什么玻璃……我要的是一个人机交互看上去没问题的界面」→ 二阶段全盘去玻璃，改用实心桌面应用语法。
> 定案：「Calm Instrument」实心体系——暗底单色板（window #171A21/侧栏 #12141B/卡片 #1F232E/品牌金 #E4B36A 唯一强调色）+ 左侧栏导航 + 五页重排 + 零透明零 shader；eframe 渲染器 wgpu→glow。任务清单 UIR.1-8（x.progress.md，全勾选）。
> 拆除面：src/glass/{mod,painter,shader,surface}.rs、bin/render_probe.rs、platform/acrylic.rs、nav.rs、widgets/{dock,ring}.rs；wgpu/egui-wgpu 依赖移除（GPU 黑窗面与 exe 体积同步收敛）。
> 新增面：widgets/{digits,icons,icon_button,switch}（等宽数字位槽宽制根治 y.problems#12 Bahnschrift 走秒抖动；矢量图标库零字体依赖）、ACCELWORLD_UI_SHOT 真窗截图探针、FrontendCommand::SetWindowGeometry 窗口几何记忆契约、DWM 标题栏钉色。
> 验证：四关卡 ALL_GREEN + 测试 180 项 + 真窗探针 exit 0 + 截图六帧自检；提交 a60cc09（39 文件，+2502/-3527）。
> 遗留：本轮 A008 审计接续体检（浅色主题失效/Tick 饿死等 19 条）；PrintWindow 截图底部白条判定为采集伪影（DWM 阴影 alpha 丢失），待真机复核。

## 附录 A008：全量代码审计报告（第 8 轮，Rust 侧第 4 轮，2026-10-23）

> 范围：`rs/` 双 crate 全部 71 个源文件 + 4 份静态配置 + Cargo 清单×3 + 工具链与 CI（rs/ci.ps1、rs-ci.yml）+ fixtures + 根文档九件；`.archived/`、`rs/target/`、`.temp/` 排除。
> 方式：只读审计。主会话亲做 FIX007 回归复核（grep + git show 双口径）并对全部 P1/P2 条目逐一亲核确证；core 业务／app 呈现／工程与文档三路并行全文通读后主会话合并去重（跨组同根因合并：几何解析、UIR 注释清扫等）。本轮为 UIR 界面推倒重塑（a60cc09）后的首轮全量审计，重点为全新 UI 代码。
> 状态：✅ 修复完成（2026-10-24，FIX008 全 19 条 + 收尾落地——四关卡 ALL_GREEN + 测试 185 通过（app 57 + bin 1 + core 107 + parity 20）+ 真窗 fps 探针 55/6s + 截图六帧自检；反向验收 `.temp/verify_fix008_accept.md`；FIX008.2 经用户定案**仅深色**——浅色体系整体删除属产品定案非回退，ThemePref 配置域保留存档）

### 零、上轮（FIX007，提交 c914b74，经 UIR 重写后）修复复核清单

**16 项 P 级修复全部在位，零回退**（grep + git show 双口径；UIR 对修复语义的平移——退出存档/冷启动天气/命令待发队列/cfg 分档——逐项核对无丢失）：

| 上轮条目 | 现状 | 证据 |
| --- | --- | --- |
| gen_id 计数+椒盐 / world_pins 城市名权威域 / 在飞表替换标记协议 / TTL 单源 / weather_code 容器 Parse / 路径基准 exe_dir / 死代码五项 | ✅ 仍在 | alarm.rs:382-399（3 条反向验收单测）；settings.rs:260-269 + interface 名域 + 夹具同步；interface.rs:609-663；weather.rs:340-357；weather.rs:207-213；util/paths.rs + interface.rs:90 + logging.rs:24；core 全域 rg glass/acrylic/wgpu 零命中 |
| color32_at 键含兜底 / cached_by_date 按日缓存 / tick 钳制 / 线程失败留痕 / 夹具计数全 assert_eq! / 非 Windows 桩 / 文档清扫 | ✅ 仍在 | theme.rs:71（`{key}#{fallback_gray}`+单测）；countdown.rs:44-61,135,277；main.rs:52；bus.rs:175-184；parity.rs 72 处 assert_eq!；acrylic.rs 整文件已删；文档七处已清（残留见 #6） |
| ——（非修复项发现） | ⚠️ UIR 丢失既有能力 | 旧版浅色主题消费 panel_light 等四键，UIR 重写后 light_visuals 读深色键 → 本轮 #2（新增，非 FIX007 回退） |

### 一、P0–P3 修复清单（P0 无；共 19 条 = P1×2 + P2×3 + P3×14）

> 性质：遗留 = 历轮已列未修；新增 = 本轮新发现。★ = 本轮观察项经用户复核提升。

| # | 级别 | 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | P1 | accel-core/src/interface.rs:429-431 ＋ accel-app/src/widgets/slider.rs:60-71 ＋ pages/clock.rs:28,127 | 1/8 | **Tick 饿死：拖动倍率滑杆时时钟走秒冻结**。主循环 `loop { sleep(period); select! }` 的 sleep 每轮重建，命令分支每轮优先；滑杆拖动期每帧一发 SetRate（约 16ms < tick 100ms）→ Tick 持续缺位，时钟页 hero 时间纯 Tick 驱动。确定性复现：按住滑杆拖动 1 秒以上走秒冻结，松手恢复 | core 侧改 `tokio::time::interval`（或对既有 deadline sleep_until）；app 侧无需动 | 新增 | 时间膨胀显示/接口层 |
| 2 | P1 | accel-app/src/theme.rs:229-268 ＋ config/static/ui.json ＋ pages/alarm.rs:58 ＋ window.rs:92 | 1/12 | **浅色主题完全失效**：light_visuals 与 dark_visuals 读同一套深色键（仅兜底灰不同而键恒存在兜底不可达）→ 切「浅色」无任何变化（确定性）；ui.json colors 单板全深色，7 键（panel_dark/window_dark/selection_dark/panel_light/window_light/bg_light/text_light）全仓零消费（rg 实证）；alarm.rs:58 无条件用 bg_dark、window.rs:92 标题栏恒钉深色为同根因症状 | 定案二选一：①恢复双主题（light_visuals 改读浅色键 + ink()/color32_at 增模式维度 + 标题栏钉色随主题）；②定案仅深色（删 7 死键 + 删浅色分支 + 设置项收敛）。顺带归位 alarm.rs:58 为 on_accent 语义 | 新增（UIR 丢失既有能力） | GUI 面板/配置体系 |
| 3 | P2 | accel-core/src/interface.rs:626-640（消费方 window.rs:818-830） | 2 | **窗口几何存档解析接受 NaN/inf/非正宽高**：`f32::from_str` 接受 "NaN"/"inf"/"1e30"，无 is_finite、无 w/h>0 校验，set_window_geometry 将原文落盘；user_config.json 用户可手编 → 启动恢复把非法四元组直达 with_position/with_inner_size | 解析后四值 is_finite() 且 w/h>0（可加屏幕域钳制），存档规范化 | 新增（UIR.7 引入） | 配置体系/GUI |
| 4 | P2 | accel-core/src/interface.rs:502-533 | 11/13 | **闹钟四命令失败零回推**：AddAlarm/RemoveAlarm/ReplaceAlarm/ToggleAlarm 失败路径无任何事件；同文件 SetRate 失败有 RateRejected（A005 #19 定案）、SetWeatherCity（:534-536）恒回推——同文件两套口径，用户操作被静默吞掉 | 失败也回推 AlarmsChanged（现值）或引入拒绝事件，与 RateRejected 对齐 | 新增 | 闹钟服务/接口层 |
| 5 | P2 | accel-app/src/pages/weather.rs:78-84（:73 vs :231 自相矛盾） | 1/6 | **天气现象图标恒为云**：hero 图标硬编码 Icon::Cloud，WeatherData.icon 与 weather_code 在 app 侧零消费——雨/雪/晴显示完全相同（确定性）；:73 注释与 :231 说明区口径矛盾；icons.rs 无雨雪等图标 | 按 weather_code 映射矢量图标（扩 icons.rs）或恢复消费 icon 字段，统一两处口径 | 新增 | GUI 面板 |
| 6 | P3 | 多处（合并，同根因＝UIR 未同步周边文本） | 6 | **UIR 残留失实注释与文档成片清扫**。代码侧：lib.rs:2（已删的 render_probe）；window.rs:1118-1143 说明区成片失效（透明 viewport/wgpu alpha/Acrylic/玻璃面注册/glass_lift/glass.gl_lift/ui.json field.*，其中「几何恢复未做」与 UIR.7 已实现直接冲突）；pages/mod.rs:113（nav.rs 已删）；icons.rs:330；main.rs:21/74/77；slider.rs:1-2；icon_button.rs:2；chip.rs:3（称上限 200，:20 实为 240）；theme.rs:2；accel-app/Cargo.toml:7-10,41-43（description「玻璃渲染」/双 bin default-run 消歧/wgpu 依赖注释）；interface.rs:667-668＋types.rs:190-194（命令「16 变体」实为 17，「协议 v1 不再变动」未登记 SetWindowGeometry）。文档侧：**x.progress.md:45-46 现役速查节仍列 `--bin render_probe`（确定性跑不通）**；x.progress.md:5（「待提交」失实）与 :6（wgpu 未清）；README.md:70-71（代码块重复两份）；w.study.md:4（注记称 3.7 留档作废，:127 实为重写现役）；y.problems.md:22,94-95（#10 称「当前无几何恢复需求」失实） | 一次清扫收口；✅ 断言与产物一致 | 新增 | 文档/跨模块 |
| 7 | P3 | theme.rs:50-52 ＋ window.rs:31/477/561 ＋ tools/clock.rs:23 ＋ widgets/switch.rs:8-32 ＋ pages/alarm.rs:119-150 ＋ widgets/chip.rs:20 ＋ pages/countdown.rs:290 ＋ interface.rs:159-167 | 4/5 | **死代码与空耗（合并）**：Theme::mode() 零调用；AppHandle.theme 只写不读；fmt_percent 零生产调用（遗留）；switch::show 生产零调用而 alarm.rs 内联逐行复刻同一开关（注释自证）；chip.rs id 死参＋countdown.rs:290 每帧 format! 空耗；fetch_weather 七域契约遗留零调用（错误策略与总线双轨） | 逐项删除/接线：alarm 行卡改调 switch::show 或删 switch.rs；删 fetch_weather 与死参 | 新增（fmt_percent 遗留） | GUI 面板/接口层 |
| 8 | P3 | window.rs:822-831 ＋ pages/weather.rs:190、countdown.rs:188、world.rs:37 | 2 | **窗口无最小尺寸下限，页内减法布局可为负宽**：无 with_min_inner_size，缩到极小时 bar_w/card_w/tile_w 变负构造倒置 Rect（tessellator 行为需验证，预计退化绘制不 panic，布局破损） | min_inner_size 入 token 设下限；页内宽度 max(0) 钳制 | 新增 | GUI 面板 |
| 9 | P3 | accel-core/src/weather.rs:64-84 | 2/6 | hourly/daily「键存在但容器错型」被静默按缺失处理（`hourly:"x"` → 空序列无 Err 无日志），与 :64-65 注释承诺（键存在则全严校验）相反 | 区分 is_some 时错型返 SeriesMissing，或修正注释 | 新增 | 天气服务 |
| 10 | P3 | accel-core/src/weather.rs:383,391,399 | 8/13 | 同一缓存锁三种中毒策略并存：force 清键 expect（panic）/写缓存静默跳过/读缓存 expect | 统一为 expect（中毒属不可恢复编程错误） | 新增 | 天气服务 |
| 11 | P3 | accel-core/src/util/file_io.rs:17-19 ↔ util/paths.rs:9-14 | 4 | 写白名单 exe 基准双轨残余：allowed_roots 自行 current_exe 推导（失败丢根）未复用 FIX007.15 的 exe_dir（失败回退 "."），语义分叉 | allowed_roots 改调 exe_dir，回归写白名单单测 | 新增 | 配置体系 |
| 12 | P3 | accel-app/src/window.rs:753-777 | 13/10 | DWM 钉色失败完全静默（双 `let _ =`）：Win10 <22000 不支持该属性恒失败，标题栏不同色且无日志；与托盘/热键降级有 warn 的纪律不一 | 失败记一次（去抖）warn/debug | 新增 | GUI 面板/日志 |
| 13 | P3 | accel-app/src/pages/alarm.rs:82-94 | 10/13 | 闹钟自定义添加：非法时间点击即发 AddAlarm 并无条件清空草稿，UI 零反馈（core 侧仅 warn 日志）；对照 countdown.rs:340-342 同场景有内联红字校验——一处有反馈一处静默（并放大 #4 静默面） | client 侧校验 HH:MM 或失败不清草稿/显示错误 | 遗留 | GUI 面板 |
| 14 | P3 | accel-core/src/interface.rs:354-362 | 10 | remove_countdown_target 不存在时静默返 false；对照 remove_world_city（:286-295）有 info 日志 | 补同款日志 | 新增 | 接口层 |
| 15 | P3 | accel-core/src/interface.rs:697-702 ＋ settings.rs:306-311 | 10 | 单元测试临时目录无 RAII 清理（断言失败即残留 %TEMP%）；对照 parity.rs:490-510 SandboxDir 已确立 RAII 标准 | 照搬 SandboxDir 模式 | 新增 | 测试 |
| 16 | P3 | accel-app/src/pages/weather.rs:8 vs :27 | 1/6 | 温度曲线注释称「平线居中」，公式实际把等温序列画在底缘（ratio=0 → y=height）；单点分支才居中；测试未覆盖平线 | 改公式（平线取中）或改注释，以设计意图为准 | 新增 | GUI 面板 |
| 17 | P3 | accel-core/src/alarm.rs:77,375 | 6 | `Clock::now` 与 `default_true` 缺 `///` 文档注释（房规每函数必备） | 各补一行中文 /// | 新增 | 规范 |
| 18 | P3 | accel-app/src/platform/tray.rs:88 | 9 | 托盘倍率菜单每秒无条件 set_text（tooltip 有未变化跳过，菜单文本无同款镜像） | 镜像上次倍率文本跳过 | 遗留 | 托盘与音频 |
| 19 ★ | P3 | accel-app/src/bus.rs:142 | 12 | Tick 未到的重绘回退 `unwrap_or(100)` 字面量与 clock_tick_ms token 同值双源 | 改读 `Static::get().base_i64("clock_tick_ms")` | 新增（★本轮提升） | 配置体系 |

### 二、参考级观察项（18 条经用户复核处置：2 条提升为 #19 与 #6 内 Cargo.toml 版本注释；2 条由归档动作即时消解——z.plan 补 UIR 纪要、豁免清单清理陈旧条目；余 14 条定案永久豁免，见「审计观察项豁免定案清单」2026-10-23 行）

### 三、亮点

- **FIX007 零回退**：上轮 16 项修复经 UIR 大面积重写后 100% 在位（grep＋git show 双口径），语义平移核查无丢失。
- **配置对账全绿**：base.json 26 顶层键零死键、app 消费键零缺键；夹具 sha256 6/6＋counts 10/10 实测相符；world_cities 50 城名唯一、默认八城全存在；默认值无双处漂移。
- **UIR 删除面完整**：glass/acrylic/wgpu/render_probe/nav/ring 在代码层零残留（残留仅注释层，见 #6）；core 侧零 UIR 污染。
- **CI 同源有效**：ci.ps1 与 rs-ci.yml 四关卡逐字一致且全部可跑；AGENTS「运行与验证」命令除速查节一处残留外均有效。
- **UIR 新基建单测锚定**：等宽数字位/矢量图标/按日缓存/几何解析均有就近测试。

---

## 附录 A009：全量代码审计报告（第 9 轮，Rust 侧第 5 轮，2026-10-24）

> 范围：`rs/` 双 crate 49 个源文件 + 4 份静态配置 + Cargo 清单×3 + CI + fixtures + 根文档九件；`.archived/`、`rs/target/`、`.temp/` 排除。
> 方式：只读审计。主会话亲做 A008 回归复核（grep 19/19 + git show fd04683 双口径）并对全部 P2 逐一亲核确证（platform 桩枚举/from_draft 透传/README 五处均代码层实证）；core 业务／app 呈现／工程与文档三路并行全文通读后主会话合并去重（同根因合并：文档清扫漏网族、死键族、ThemePref 悬空族）。本轮为 FIX008（V0.6.1.1）之后的体检轮，重点为 FIX008 新增代码与其文档同步盲区。
> 修复实证补录（2026-10-24，FIX009 执行期）：#6「需验证」项落地为确定性结论——**winit 对 with_inner_size 创建期不施加 with_min_inner_size**（tiny-geo 真窗探针：旧 100×100 存档修复前原样创建 125px 窄窗），恢复侧新增 clamped_restore_geometry 钳制后 650×525（520×420 逻辑点 @1.25 DPI）；该行为已记入 window.rs 代码注释与单测。
> 状态：✅ 修复完成（2026-10-24，FIX009 全 20 条 + 收尾落地——四关卡 ALL_GREEN + 测试 187 通过（app 58 + bin 1 + core 108 + parity 20）+ 真窗 fps 探针 60/6s + 截图六帧 + 旧小存档钳制真窗实证（winit 创建期不施加 min_inner_size——恢复侧 clamped_restore_geometry 钳制，tiny-geo 探针前后 125x125→650x525）；反向验收 `.temp/verify_fix009_accept.md`）

### 零、上轮修复复核清单（FIX008，提交 fd04683）

**19 条 P 级修复全部在位，零行为回退**（主会话 grep 19/19 + 三路 agent git show 补盲区；FIX008 新增 tick 并发逻辑专项逐行审未发现确定性缺陷）：

| 上轮条目 | 现状 | 证据 |
| --- | --- | --- |
| Tick 绝对截止时间 / 仅深色定案 / 几何解析防御 / 闹钟恒回推 / 图标随码 / 死代码 / 最小窗口 / 容器错型 / 锁统一 / exe_dir / DWM 留痕 / valid_hhmm / 倒计时日志 / RAII / 平线取中 / 补 /// / 托盘镜像 / bus token | ✅ 全部在位 | interface.rs:420-433（tick_sleep+reset）；theme.rs 无 light_visuals/Theme、ui.json 无 _light/_dark；parse+canonical+单测；四分支无条件回推；icon_for_code+16 变体；switch.rs 已删等 rg 零命中；window_min_*+三页 clamp；BadType；三处 expect；file_io:19；WARNED 去抖；valid_hhmm；info 日志；SandboxDir×2；flat ratio；///×2；last_rate_text；bus token+测试 |
| UIR 残留清扫（#6） | ⚠️ 清扫面漏网（非行为回退） | 漏网处见下表 #1/#4/#20——FIX008 #2/#6/#15 任务文本未覆盖 platform/mod.rs 桩、window.rs 测试目录、README 特性/使用说明节、w.study 3.7 正文、m.milestone 规划节 |

### 一、P0–P3 修复清单（P0/P1 无；共 20 条 = P2×3 + P3×17）

> 性质：遗留 = 历轮已列未修；新增 = 本轮新发现（含 FIX008 清扫面漏网）。

| # | 级别 | 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | P2 | accel-app/src/platform/mod.rs:47-51（对照 window.rs:588-593） | 5/6/1 | **非 Windows 热键桩 HotkeyKind 仍含 Theme 变体**：桩枚举三变体而 hotkey_action 仅 match Save/Quit 两臂 → 非 Windows 目标编译 E0004 non-exhaustive（确定性）；桩头注「同接口零分叉」与 hotkey.rs 说明区前提失实 | 桩删 Theme 对齐两键；订正两处表述 | 新增（FIX008 漏网） | 跨模块/构建 |
| 2 | P2 | accel-core/src/alarm.rs:150-168（对照 :217） | 2/13/6 | **from_draft 不规范化 repeat_days**：注释自称「规范化」实未做（对照 from_json_value 有 normalize_repeat_days）——总线 AddAlarm 带 `[9]` 落盘成功后 `contains(weekday 0-6)` 永假，闹钟每周静默不响且持久保留（总线是协议边界，手编/上游缺陷即触发） | from_draft 内联 normalize（拆 normalize_i64_days 共用）；订正注释 | 新增（PL012 既有形态首轮发现） | 闹钟服务 |
| 3 | P2 | README.md:31,38,39,80,86 | 6 | **README 三态主题/Ctrl+T 失实组（5 处）**：「深浅两套实心主题」「三态主题…Ctrl+T 循环」「全局热键（…/Ctrl+T 主题）」「切换主题：设置浮窗选择 跟随系统/浅色/深色…实时切换」「Ctrl+T 主题循环」——与仅深色定案确定性矛盾，照文档操作 Ctrl+T 无效 | 五处改仅深色口径；将「README 特性/使用说明/项目结构三节 + w.study 3.7 + m.milestone 规划节」列为 UI 行为变更固定连带清单 | 新增（fd04683 文档同步盲区） | 文档 |
| 4 | P3 | 多处（合并） | 6 | **文档清扫漏网族**：w.study.md:130-132（深浅两套/含 switch/钉色随主题，:4 自称现役口径）；README.md:90-91,152（配置/日志路径旧基准——FIX007.15 后为 exe 同目录）、:114（含 switch）、:112（src 列表缺 lib.rs）；m.milestone.md:285（「后续版本」节仍列浅色校准，与定案矛盾）；x.progress.md:5（状态行滞后一轮）；icons.rs:6,403（Dock 字样）；window.rs:631（Acrylic 口径）；window.rs:869（Monospace 槽位序号失实）；bus.rs:48（SetTheme 乐观更新注释失实）+ :124（ConfigSaved(_ok) 命名噪音） | 并入一次清扫 | 新增 | 文档 |
| 5 | P3 | accel-core/src/interface.rs:506-539 | 13/11 | 闹钟四分支 `let _ = save_config_now()` 吞落盘失败且不回推 ConfigSaved(false)——同文件其余 12 个写命令均回推；落盘失败时 UI 看似成功、重启丢失 | ok && saved 一并回推 ConfigSaved | 新增 | 闹钟服务/接口层 |
| 6 | P3 | accel-core/src/interface.rs:650 | 2 | 几何存档无上限（1e38 有限值入档直达 with_inner_size）；存档值 < window_min_* 时 winit 是否创建即钳制**需真机验证**——若不钳，旧存档首帧仍可塌负宽 | core 补上限域或恢复侧 clamp；真机验证并存行为 | 新增（A008 #3 延伸） | 配置体系/GUI |
| 7 | P3 | accel-core/src/interface.rs:597-601（对照 :584-594） | 8/13 | tick 分支 evt_tx.send 失败 break 不置 sd_tx、不 abort 在飞任务（与 Quit 收尾不对称）→「事件通道先关」路径下闹钟调度永续轮询；产品内同生命周期影响小 | break 前复用同一收尾块 | 新增（FIX008 前同形态） | 接口层 |
| 8 | P3 | accel-core/src/interface.rs:338 ↔ settings.rs:45 | 4 | 倒计时落盘时间格式内联与 format_date_text 单源逐字符同逻辑（FIX007.10 已收敛 parse 侧） | 改调 format_date_text | 新增 | 跨模块 |
| 9 | P3 | accel-core/src/interface.rs:159,182,229,238,250 ＋ weather.rs:261,264-277 | 5 | 零调用契约 API 组：get_weather_refresh_interval_ms/get_timezone_options/get_alarm/get_max_alarms/get_world_cities 仅自测试消费；format_weather_info+WEATHER_FAIL_TEXT 仅 parity 消费（A005 #12 只读直取后的悬空面，fetch_weather 先例已删） | 删除或登记「契约形态保留」说明 | 新增 | 接口层 |
| 10 | P3 | accel-core/src/weather.rs:417-424 | 3 | URL `forecast_days=7`/`forecast_hours=24` 字面量业务参数，base.json weather_* 域缺席（该进配置未进） | token 化 | 新增 | 天气服务/配置体系 |
| 11 | P3 | accel-core/src/static_config.rs:109 | 6 | 说明区仍列「bool」访问器（A005 #15 已删），与正文自相矛盾 | 删该词 | 新增 | 文档（源码注释） |
| 12 | P3 | accel-core/src/alarm.rs:418 | 6 | AlarmManager.alarms 内部列表 pub 裸暴露（crate 外可直写绕过上限/去重） | pub(crate) 或补 list() 读取器 | 新增 | 闹钟服务 |
| 13 | P3 | rs/crates/accel-app/Cargo.toml:16（说明区 :40） | 12 | pollster 声明依赖全 crate 零源引用（UIR 拆 wgpu 残留） | 删依赖行+说明区同步 | 新增 | 配置体系 |
| 14 | P3 | rs/config/static/ui.json:17 + layout.chip_size | 12 | scale.tiny 死键（font_size 消费面 12 键实测，tiny 零命中——历轮死键承诺面仅 colors 域与 base.json 顶层）；chip_size w=116 槽半死（chip 宽自适应只取 h） | 删 tiny；chip_size 改单值或登记 | 新增 | 配置体系 |
| 15 | P3 | accel-app/src/theme.rs:84 + ui.json colors.info | 5 | Ink.info 字段装载后全 crate 零读取（原消费方「降水概率」展示随 UIR 移除；注释仍写降水概率） | 删字段+键，或接线天气要素行 | 新增 | GUI/配置体系 |
| 16 | P3 | accel-app/src/pages/world.rs:21 | 3 | 昼夜判定 `(6..18)` 业务阈值字面量未走配置 | token 化或注释登记「纯展示启发式」 | 新增（UIR 平移） | GUI/配置体系 |
| 17 | P3 | accel-app/src/widgets/icons.rs:223-231 | 1 | Storm 闪电末端 0.88r 加半线宽后 16px 图标底部留边约 0.24px，低于「四周 ≥1px」纪律（四新图标唯一越线）——需验证可感性 | 闪电收尾至 ≤0.80r | 新增 | GUI 面板 |
| 18 | P3 | accel-app/src/pages/alarm.rs:114-120 | 1 | 红字提示为 ScrollArea 内条件行插入：首敲非法字符列表一次性下移 ≈18px、修回回跳（功能正确，非持续抖动）——需真机走查 | 若可感改恒占位（Visibility::Invisible） | 新增 | GUI 面板 |
| 19 | P3 | accel-app/src/pages/weather.rs:211-225 | 1 | 距条 `(bar_w*hi).max(min+4.0)`：bar_w=0（钳 0 后）轨道不可见仍画 4px 孤立金点（极窄容器可达） | bar_w≤0 跳过绘制 | 新增 | GUI 面板 |
| 20 | P3 | accel-app/src/window.rs:996-1027 | 10 | 测试临时目录 accel_startup_{pid} 无清理（FIX008.15 RAII 化漏网；对照 logging.rs 样板） | 补 remove 或 RAII guard | 新增（FIX008 漏网） | 测试 |

### 二、参考级观察项（13 条经用户复核全部维持观察级；另「ThemePref 三态契约悬空＝仅存档定案协议位保留」1 条登记入永久豁免表 2026-10-24 行）

| 位置 | 描述 | 回落理由 |
| --- | --- | --- |
| interface.rs:427-432,602-605 | FIX008 tick 语义细节两则（倍率变化 reset 取消一个已到期未观察 tick；reset 在 send().await 后随背压后移） | 时间值恒由墙钟重算无累计误差；有命令爆发期验收单测 |
| interface.rs:129-135 | set_rate 每发重建 TimeDilation（拖动期丢 lunar 秒级缓存） | FIX001.23 轻量路径既有定案 |
| interface.rs:383-405 | 一次性闹钟触发与 UI 删除竞窗（两段锁间删除仍发 AlarmTriggered 一次） | 毫秒窗、需恰在该窗操作 |
| interface.rs:477-503 | 天气任务 panic（非 abort）不自摘，条目滞留在飞表 | fetch 链路无已知 panic 点；force 可解 |
| settings.rs:156-177 | countdown_targets 装载不按 name 去重（手编同名共存） | add 路径有去重；手编域 |
| chinese_calendar.rs:137-146 | lunar_ymd_to_solar 运行期 expect（倒计时页渲染路径） | 输入为静态常量规则表 |
| weather.rs:348 | weather_cache_ttl 负值 as u64 回绕成巨 TTL | token 编译期嵌入可信产物 |
| util/file_io.rs:14-22 | current_exe 失败回退 "." 使 CWD 入写白名单 | FIX008 #11 同源化收益副作用，注释已登记 |
| icons.rs:236-245 | Bell 顶弦不对称小尖（UIR 引入） | 用户真机走查未提出即定稿 |
| window.rs:28-30、pages/mod.rs:24 | AppHandle pub 字段/Pages.current 可收私有 | crate 内零外部读者，历史结构 |
| world.rs:30 | 每帧 to_vec() 克隆 pins | ≤几十串、10fps 可忽略 |
| icons.rs:54-86 | cloud_line 底边重复描线 | 视觉无差，UIR 同源豁免延续 |
| m.milestone.md:287 | 「fps 探针 55/6s」为历史运行快照现树不可复核 | 当时记录真实，主会话四关卡佐证 |

### 三、亮点

- **A008 十九条 19/19 在位零回退**：三路 agent git show 补盲区仅发现清扫面漏网（文档/桩/测试目录），无一项行为回退；FIX008 新增 tick 并发逻辑专项逐行审无确定性缺陷。
- **配置对账全绿**：base.json 28 键零死键（window_min_* 消费闭环、shortcuts.theme 删除有负断言测试锚定）；ui.json 缺键即 panic 策略下无「缺键走兜底」隐患；夹具 sha256 6/6 + counts 10/10 实测相符。
- **m.milestone 0.6.1.1 五节数字断言逐条实证成立**（8 死键恰 8、520×420、四枚图标、185 测试三组交叉核算一致、截图六帧、反向验收文件在位）。
- **CI/AGENTS 命令集全部有效**：四关卡与 rs-ci.yml 同源逐字一致，版本单源链路（base.json→build.rs→注入→parity 单测）完好。
- **本轮无 P0/P1**：产品路径无新增 unwrap/expect，总线/退出序列/平台降级纪律完好；两条确定性缺陷（#1 编译面、#2 闹钟域）边界清晰、修复成本低。

---

## 附录 A010：全量代码审计报告（第 10 轮，Rust 侧第 6 轮，2026-10-24）

> 范围：`rs/` 双 crate 49 个源文件 + 4 份静态配置 + Cargo 清单×3 + CI + fixtures + 根文档九件；`.archived/`、`rs/target/`、`.temp/` 排除。
> 方式：只读审计。主会话亲做 FIX009 回归复核（grep 20/20 + git show 4a48890 双口径）并亲核唯一 P2 的完整触发链（interface 回推 → bus drain → WinRT toast 文案）；core 业务／app 呈现／工程与文档三路并行全文通读后主会话合并去重（A009/FIX009 清扫残余注释族、w.study 收口残余族各并一条）。本轮为 FIX009（V0.6.1.2）之后的体检轮，重点为 FIX009 新增代码与其注释同步盲区。
> 修复实证补录（2026-10-24，FIX010 执行期口径）：#1 的拒绝/IO 失败分流落地时，ApplyRate 越界与 SetWeatherCity 未知城市的同款错位（遗留同根因）一并收口。
> 状态：✅ 已修复（FIX010 完成，V0.6.1.3——11 条全落地，任务清单见 x.progress.md）

### 零、上轮修复复核清单（FIX009，提交 4a48890）

**20 条 P 级修复全部在位，零行为回退**（主会话 grep 20/20 + 三路 agent git show 补盲区；FIX009 新增代码专项——normalize 双路径防御等价性、clamped_restore_geometry 咬合、恒占位/距条跳绘/Storm 收边——三审无功能缺陷）：

| 上轮条目 | 现状 | 证据 |
| --- | --- | --- |
| 桩 Theme 删 / normalize_i64_days / README 三态组 + AGENTS 连带清单 / 文档漏网族 / ConfigSaved 回推 / 几何上限 + clamped_restore_geometry / tick 收尾 / format_date_text / 四转发删+双登记 / forecast token / bool 订正 / pub(crate) / pollster / tiny+chip_height / Ink.info / 昼夜登记 / Storm 0.80 / 恒占位 / 零宽跳过 / RAII | ✅ 全部在位 | platform/mod.rs 桩两变体；alarm.rs:165,348,381；README rg 零命中 + AGENTS:120；interface.rs 四分支 saved 回推；上限 1e4 + window.rs 纯函数+单测；base.json:39-40 → weather.rs 字段化；ui.json chip_height 单值；其余逐条 rg 在位 |
| ——（注释层漏网，非行为回退） | ⚠️ 漏改面 | FIX009 #14/#15 只删字段/键本体、相邻注释未随 → 本轮 #2（theme.rs:72/chip.rs:96 等 6 处）；完成注记「rg chip_size 零命中」验证语句不实 → #5 |

### 一、P0–P3 修复清单（P0/P1 无；共 10 条 = P2×1 + P3×9）

> 性质：遗留 = 历轮已列未修；新增 = 本轮新发现（含 FIX009 连带/残留）。

| # | 级别 | 文件:行号 | 类型 | 描述 | 建议 | 性质 | 影响面 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | P2 | accel-core/src/interface.rs:493-524 ＋ accel-app/src/bus.rs:327-329,190 ＋ platform/notify.rs:19-21 | 13/11 | **闹钟拒绝路径触发「配置保存失败：设置未能写入磁盘」假 toast（FIX009 #5 跨层连带）**：core 四分支把「操作被拒」（同时间同标签重复/超上限/不存在 id）也回推 ConfigSaved(false)，app drain 对 ConfigSaved(false) 无差别弹 WinRT toast「设置未能写入磁盘」——拒绝场景磁盘从未被写，对用户陈述假事实。确定性复现：闹钟页相同「时间+标签」添加两次 → 第二次弹盘失败 toast。同根因遗留：ApplyRate 越界/SetWeatherCity 未知城市拒绝同款错位 | 拒绝与 IO 失败分流：core 拒绝路径不回推 ConfigSaved（仅落盘失败回推；拒绝事实已由 AlarmsChanged 现值承载）——四分支改 if ok { saved=save; send ConfigSaved(saved) }；ApplyRate/SetWeatherCity 同口径收口；同步调整 bus_alarm_rejection 测试断言 | 新增（FIX009 #5 连带；同根因拒绝错位为遗留） | 闹钟服务/托盘与音频/GUI |
| 2 | P3 | 多处（合并） | 6 | **A009/FIX009 清扫残余注释组（6 处）**：theme.rs:72-73（Ink 注释仍列「info=信息」）；chip.rs:96（测试注释仍写 layout.chip_size）；alarm.rs:444-447（max_alarms() 文档仍引已删 get_max_alarms 契约且生产零调用成孤儿访问器）；audio.rs:112（消费方写 interface，实为 app bus AlarmTriggered 副作用）；static_config.rs:108（「sha256 一致零改写」本轮配置变更后失效）；pages/mod.rs:53（「镜像自 bus.apply 侧」指向不存在通路） | 逐处订正；max_alarms() 删或改「单测锚定探针」注释 | 新增（FIX009 残留） | 文档（源码注释） |
| 3 | P3 | w.study.md:16,53,56,17,22 ＋ AGENTS.md:120 | 6 | **w.study 仅深色/结构收口残余（5 处）**：:16 仍列「三态主题」；:53 「Visuals 深浅两套」；:56 widgets 仍含 switch；:17 配置路径旧基准（rs/config）；:22 「天气 50 城市」实为 17 城。**连带清单自身盲区实证**：三处散布在 w.study 第 1/2 节而 AGENTS:120 只锁 3.7 节——清单条目放宽为「w.study 主题/控件/路径相关表述（1/2/3.7 节）」 | 五处订正 + AGENTS 清单条目放宽表述 | 新增（#3 同族漏网）+遗留（:17/:22） | 文档 |
| 4 | P3 | AGENTS.md:10 ＋ x.progress.md:46 | 6 | `ACCELWORLD_UI_SHOT=<目录> cargo run` POSIX env 前缀语法在 AGENTS 强制的 pwsh 会话不可执行（README $env: 形态正确） | 两处改 $env: 形态 | 遗留 | 文档 |
| 5 | P3 | x.progress.md:569 ＋ :5 | 6 | FIX009.14 完成注记「rg chip_size 零命中」与现树不符（验证语句失实）；:5 行尾「当前阶段为 A009」提交后未翻转 | 注记订正 + 状态行翻转 | 新增 | 文档 |
| 6 | P3 | rs/crates/accel-core/Cargo.toml:15 | 12 | chrono-tz 列为正式依赖但 core 生产零使用（仅 cfg(test)） | 移入 [dev-dependencies] | 遗留 | 配置体系 |
| 7 | P3 | accel-core/src/alarm.rs:483-497 | 2 | replace_alarm 不校验 time、不规范化 repeat_days（当前 ReplaceAlarm 零生产者不可达；接线即复刻 A009 #2 静默不响形态） | 接线前补 parse_alarm_hhmm + normalize_i64_days | 遗留 | 闹钟服务 |
| 8 | P3 | accel-core/src/settings.rs:160-165 | 10 | countdown_targets 装载对非对象/空名条目静默 continue 无日志（对照闹钟装载 warn 口径不一） | 补 tracing::warn | 遗留 | 配置体系 |
| 9 | P3 | accel-app/src/widgets/slider.rs:87,128 | 5 | draw() 的 rect 形参零使用 + 尾部 let _ 压警告（私有函数） | 删形参与占位行 | 遗留 | GUI 面板 |
| 10 | P3 | .gitignore（两块） | 12 | .vscode/ 条目重复两条 | 删一条 | 遗留 | 配置体系 |

### 二、参考级观察项（12 条经用户复核全部维持观察级；无新增豁免）

| 位置 | 描述 | 回落理由 |
| --- | --- | --- |
| interface.rs:644 | 1e4 上限为单次使用常量未进 token | 解析器防御域非业务参数，注释已含理由 |
| interface.rs:568-578 vs 586-592 | Quit 与 tick 失败收尾块 ~8 行重复可提取 helper | A009 #7 最小内联改动；语义当前一致 |
| types.rs:143 | FrontendCommand::ReplaceAlarm 全 workspace 零生产者 | 协议「演进须登记」定案；接线时触发 #7 |
| weather.rs:360-361 | forecast_days/hours 无域校验（负值→API 400） | token 编译期嵌入可信，与 ttl 负值豁免同类 |
| settings.rs:33、chinese_calendar.rs:118 | parse_date_text/holidays pub 可收窄 | 单源依赖文档化/数据面导出，收益低 |
| world.rs:30,52-56 | 每帧每城 tz 字符串查表 + Utc::now()（8 城×10Hz） | chrono-tz FromStr 静态表 μs 级；与已豁免「每帧 to_vec」同族 |
| clock.rs:59-78 | 加速小时破百 + 贴最小窗宽时 hero+徽标可能超宽被裁 | 两条件叠加；仅展示右缘裁切；需验证，视觉终审在用户 |
| countdown.rs:79-81 | validate_add 不 trim，纯空格名双层放行 | 与 core 校验同构；空格名合法性属产品口径未定案 |
| weather.rs:226-227 | bar_w∈(0,4px) 时金条可溢出弱轨 4px | min 窗 520 下实际域 ≥150px 不可达，纯防御 |
| main.rs:8-9、logging.rs:34 | 探针/RUST_LOG 环境变量面 | 探针/诊断通道设计使然 |
| m.milestone.md:272 | 连带清单第四项「ui.json 死键」vs AGENTS「static *.json」口径差一档 | 语义等价，表述级 |
| z.plan.md:1157 | 「base.json 28 键」本轮 +2 后实为 30 键 | A009 附录为审计时点快照，历史真实 |

### 三、亮点

- **FIX009 二十条 20/20 在位零行为回退**：normalize 双路径防御等价性逐分支验证通过；clamped_restore_geometry 与 core parse 咬合验证通过；恒占位/距条跳绘/Storm 收边三审无功能缺陷。
- **token 对账双绿**：ui.json 35 键、base.json 30 键（含新增 weather_forecast_* 两键闭环）全消费零死键；夹具 sha256 6/6、counts 抽测全对。
- **m.milestone 0.6.1.2 六节断言逐条实证成立**（含 650×525 = 520×420×1.25 数学吻合、187 测试静态计数三处口径一致、反向验收文件在位）。
- **README 复扫零残留零新增失实**；AGENTS 热键两键/技术栈 glow 表述准确；4a48890 提交范围与正文逐条相符。
- **本轮无 P0/P1**；唯一 P2 是 FIX009 #5 的跨层语义连带（修复引出的新可见性问题）——印证「审计→修复→再审计」循环对跨层改动语义的把关价值。

---

## 临时记录：后续候选事项（2026-09-12 随手记，未立项、未排期）

> 性质：临时清单，只记方向不拆任务；正式立项时移入对应 PL 预记并细化到 x.progress.md

1. **UI 减法清理**：通盘排查并删减无用的 UI 项目；倒计时的目标输入改为更现代的日期时间选择方式
2. **世界时钟增强**：新增显示功能（形态待定）；同步调研时区与主要城市的对照表或可用 API，评估替代/补充现有城市表的可行性
3. **天气强化**：天气 API 能力增强（含既有顺延项"小时级预报"）与天气卡 UI 显示同步升级
4. **闹钟现代化**：闹钟页面视觉与交互更现代、优雅，支持多个闹钟的设定与管理
5. **后端清理**：排查无调用的后端程序与模块，确认后删除

---

## Rust 重构总纲：AccelWorld RS（2026-10-16 立项，接替 Python 版）

> 决策（用户 2026-10-16 拍板）：①计划与五 PL 切分认可；②Python 内容与 UI 全部移入新建 `.archived/`（git 忽略），Rust 项目按标准结构新建于仓库根；③**版本策略：同一产品延续身份，换引擎不换版本号段——重构定版仍为 0.6.0.0 段**（版本单一来源 base.json，开发期按 patch 位递增如 0.6.0.3，PL015 定版发布）；④HTML 原型暂不定稿，以当前页面效果 + AI 决策的合宜呈现为 v1 基准，UI 雕刻为重构完成后独立阶段（不占五 PL 容量）。
> 技术栈（定案）：GUI = egui + eframe(wgpu) + egui_extras，玻璃 = PaintCallback 跑 WGSL（GLSL 资产移植）+ windows-rs DWM Acrylic；集成 = tray-icon/muda + global-hotkey + winrt-notification + rodio；后端 = tokio + reqwest(rustls) + chrono/chrono-tz + lunar-rust + serde/serde_json + tracing/tracing-appender。
> 架构：cargo workspace——`crates/core`（纯业务零 GUI，Python modules/config/data/utils 的等价物）+ `app`（egui 前端）；core 每帧被拉取渲染，tokio 后台线程管网络与闹钟调度经 channel 喂主循环；`config/static/*.json` token 体系原样平移。
> parity 度量衡：Python 版（0.6.0.0）冻结为 golden 基准——PL011 导出行为夹具（fixtures JSON），Rust 侧全程对夹具断言；农历 parity 为最高风险项（PL012 集中对决，兜底个别字段本地实现）。
> 旧版处置：夹具导出并复验可从归档再生后，Python 全量移入 `.archived/`（.gitignore 增补 `.archived/`）；归档保持可运行（夹具再生/对照查询用）。
> 排期（五 PL 直线依赖）：

- PL011 工程地基 + 核心域一期（workspace/夹具导出/config/time_dilation/weather/CI/归档）
- PL012 核心域二期（农历 parity/节假日数据化/alarm/数据表/rodio → core 全量 parity）
- PL013 egui 外壳与玻璃地基（窗口 Acrylic/字体/WGSL 玻璃/theme/胶囊导航/设置浮窗/日志）
- PL014 五页功能落地 + 设置浮窗（以 HTML 原型 v1 为基准：时钟/倒计时多目标/世界时钟城市管理/天气增强/闹钟快捷）
- PL015 系统集成与收口（托盘菜单树/热键/系统通知/版本单源 build.rs/文档迁移）

> 出口标准：PL015 = 四关卡全绿 + 自动化验收通过（不做人工测试与定版仪式）；UI 雕刻另立项。

### 总纲修订一：结构镜像与信号接口（2026-10-16 用户确认，随任务细化同步）

> **结构镜像原则**：Rust 工程以 Python 现结构为母版一比一镜像，PL011.02 建骨架时目录全量落位（含空模块占位），**禁止大文件后拆**。注释房规的 Rust 对等物：`///` 中文文档注释（1-3 行，对应函数注释规则）+ 模块头 `//!` 说明 + 文件尾 `// ===== 说明区` 保留。测试布局：core 各模块内联 `#[cfg(test)]` 就近单测 + `rs/tests/` 放夹具驱动集成测试。

> **镜像映射表**（Python → Rust，PL011.02 验收对照物）：
>
> | Python                                  | Rust                                                                    |
> | --------------------------------------- | ----------------------------------------------------------------------- |
> | main.py（装配/CLI）                     | app/src/main.rs                                                         |
> | ui/main_window.py（主窗装配器）         | app/src/window.rs                                                       |
> | interface/（AppInterface 七域 + types） | core/src/interface.rs + types.rs                                        |
> | modules/time_dilation.py 等             | core/src/time_dilation.rs / chinese_calendar.rs / weather.rs / alarm.rs |
> | config/settings.py                      | core/src/settings.rs                                                    |
> | config/static/（token 单例）            | core/src/static_config.rs（OnceLock）                                   |
> | data/（三数据表）                       | core/src/data/（include_str JSON + 强类型）                             |
> | utils/（logger/retry/file_utils）       | core/src/util/                                                          |
> | ui/panels/（七面板纯展示）              | app/src/pages/（clock/countdown/world/weather/alarm.rs）                |
> | ui/tools/（展示运算纯函数）             | app/src/tools/                                                          |
> | ui/gl/（画布/场景/着色器）              | app/src/glass/（shader.rs/surface.rs/painter.rs）                       |
> | glass_nav + 自绘控件                    | app/src/nav.rs + app/src/widgets/（slider/ring/chip/glass_card.rs）     |
> | backdrop.py + system_tray.py            | app/src/platform/（acrylic/tray/notify/hotkey.rs）                      |

> **前后端解耦（信号接口）**：core 不知 egui、app 不直碰业务，两端仅经 interface 的命令/事件总线通信——
>
> - `FrontendCommand`（枚举）：UI 意图（SetRate/AddAlarm/RemoveAlarm/AddCountdownTarget/SetCity/...）
> - `BackendEvent`（枚举）：后端事实（Tick(TimeInfo)/WeatherUpdated(WeatherData)/AlarmTriggered(Alarm)/...）
> - 物理通道：tokio mpsc 双向；`app/src/bus.rs` 每帧 drain 事件、按需发命令；core::interface 同时提供纯同步查询方法（拉）与事件流（推）——Python「UI 定时器拉取」习惯的 Rust 对等物
> - 硬保障：CI 校验 core 依赖树零 egui/eframe（单向分层从约定升级为机器强制）
> - 契约边界（A005 #12 明确，2026-10-18）：app 侧**写操作**一律经命令总线；**只读**的渲染 token（`static_config::Static`）、DTO/纯常量（`types`/`PresetSound`/`CountdownTarget`）、数据检索（`data::cities`/`world_cities`）与平台播放封装（`audio`）允许直取——原「唯一入口」表述把只读读取也纳入，属过度约束（每帧渲染取 token 经总线转一层无价值）

> **粒度原则**：一个任务 = 一个可独立验证的交付物（PL011-015 共 59 条）；执行偏移以「镜像映射表 + interface 七域契约」为锚回正。

### 总纲修订二：PL011–PL015 详细技术方案（2026-10-16 补齐「怎么做」）

> 本节是 59 条任务的实现依据：结构定义 / 接口签名 / 数据 schema / 状态模型 / 验证流程全部落死。执行中与 x.progress 条目冲突时以本节为准回正。

#### PL011 详细方案：工程地基 + 核心域一期

**A. 工程树（PL011.02 验收对照物，精确到文件）**

```
rs/
├── Cargo.toml                    # [workspace] members = ["crates/*"]，workspace.dependencies 统一版本
├── rust-toolchain.toml           # channel = "stable"
├── rustfmt.toml / clippy.toml
├── fixtures/                     # golden 夹具（git 入库）
│   ├── time_vectors.json / config_behavior.json / weather_samples.json / lunar_samples.json
├── tests/                        # 夹具驱动集成测试（parity.rs 等）
└── crates/
    ├── accel-core/               # 纯业务，零 egui（CI 强制）
    │   ├── Cargo.toml
    │   └── src/{lib.rs, types.rs, interface.rs, time_dilation.rs, chinese_calendar.rs,
    │            weather.rs, alarm.rs, settings.rs, static_config.rs,
    │            data/{mod.rs,cities.rs,timezones.rs,weather_codes.rs},
    │            util/{mod.rs,retry.rs,file_io.rs}}
    └── accel-app/                # egui 前端
        └── src/{main.rs, window.rs, bus.rs, theme.rs, nav.rs,
                 glass/{mod.rs,shader.rs,surface.rs,painter.rs},
                 pages/{mod.rs,clock.rs,countdown.rs,world.rs,weather.rs,alarm.rs},
                 widgets/{mod.rs,glass_card.rs,slider.rs,ring.rs,chip.rs},
                 tools/{mod.rs,clock.rs,countdown.rs,alarm_text.rs},
                 platform/{mod.rs,acrylic.rs,tray.rs,notify.rs,hotkey.rs}}
```

**B. 依赖清单（workspace.dependencies，版本锁最新稳定）**
core：serde(derive) / serde_json / tokio(rt-multi-thread,macros,time,sync) / reqwest(rustls-tls,json) / chrono(clock) / thiserror / tracing；单测：httpmock。
app：eframe(wgpu) / egui / egui_extras / pollster / image / windows(Win32_Graphics_Dwm,Win32_UI_WindowsAndMessaging) / tray-icon / global-hotkey / rodio / chrono-tz；dev：httpmock 经 core 转接。
（lunar-rust / chrono-tz / rodio / tray 系在 PL012/PL015 各自任务引入，PL011 只装 core 所需 + app 占位）

**C. 通信协议 v1（types.rs + interface.rs，全部变体一次定义）**

```rust
// UI → 后端（意图）
pub enum FrontendCommand {
    SetRate(f64), ApplyRate(f64),                    // 实时生效 / 内聚落盘（对齐 set_rate/apply_rate 双档）
    RefreshWeather { city: String, force: bool }, AddWeatherCity(String), RemoveWeatherCity(String),
    AddAlarm(AlarmDraft), RemoveAlarm(String), ReplaceAlarm(Alarm), ToggleAlarm(String),
    AddCountdownTarget { name: String, at: chrono::DateTime<Local> }, RemoveCountdownTarget(String),
    AddWorldCity(String), RemoveWorldCity(String), SetBaseTimezone(String),
    SetTheme(ThemePref), SaveNow, Quit,
}
// 后端 → UI（事实）
pub enum BackendEvent {
    Tick(TimeInfo),                                  // 按倍率节奏推送
    WeatherUpdated { city: String, data: WeatherData },
    WeatherFailed { city: String, reason: String },
    AlarmTriggered(Alarm),
    AlarmsChanged(Vec<Alarm>),                       // 列表变更后回推全量（UI 无状态重建）
    ConfigSaved(bool), RateChanged(f64),
}
```

**D. interface.rs 结构与所有权模型**

```rust
pub struct AppInterface {          // app 持有，内部全 Arc，可跨线程移入 tokio 任务
    world: Arc<Mutex<TimeDilation>>,
    weather: Arc<WeatherClient>,   // WeatherClient 内部缓存 Mutex<HashMap<..>>
    alarms: Arc<Mutex<AlarmManager>>,
    settings: Arc<Mutex<Settings>>,// settings.rs 装载/落盘/容错门面
}
impl AppInterface {
    pub fn sync_query 七域方法（get_time_info/get_rate/get_alarms/... 对齐 Python AppInterface 一一对应）;
    pub fn spawn(self: Arc<Self>) -> (Sender<FrontendCommand>, Receiver<BackendEvent>);
    // spawn 内部：tokio task 主循环 select! { cmd = rx.recv() => 分发, _ = tick 定时器 => 发 Tick, ... }
}
```

规则：**同步查询只读快照（锁内 clone 出 DTO）；一切写操作走命令**——UI 不直接拿锁，杜绝锁跨帧。

**E. 夹具 schema（PL011.04 导出物格式）**

- time_vectors.json：`[{ "rate": 1.1, "std": "2026-09-13T09:00:00+08:00", "expect": { "accel": "022:26:24", "day_hours": 26, "tick_ms": 909, "remaining_h": .. } }]`（rate 边界/脏值/越界各含）
- config_behavior.json：`{ "roundtrip": {in, out}, "corrupt_backup": {in_file, bak_exists, default_restored}, "dirty_fields": [{field, bad_value, expect_default}] }`
- weather_samples.json：`[{ "name": "null_temp", "raw": {...}, "expect": "Err(NullField:temperature_2m)" } | { "raw": {...}, "expect": WeatherData }]`
- lunar_samples.json：`[{ "date": "2026-09-23", "expect": { "lunar_year": "丙午年", "shengxiao": "马", "lunar_month": "八月", "lunar_day": "廿三", "shichen": "午时", "jieqi": "秋分", "yue_phase": "蛾眉残月", "cai_shen": "正北", "position": "坎位", "info_str": "丙午年（马年）八月初三午时 月相：… 财神：…" } }]`（全年 366 天逐日 + 节气日加密抽样）
  导出脚本：Python 侧 `.archived` 可再生，输出前 sha256 自登记；Rust 侧 serde 反序列化为强类型夹具结构。

**F. 各模块设计要点**

- static_config.rs：`pub struct Static { pub base: serde_json::Value, pub ui: serde_json::Value }` + `Static::get() -> &'static Static`（OnceLock，启动 main 装配期 init，未 init 使用 panic 带指引文案 = Python RuntimeError 等价）；访问器按域取键（`base_f64("rate_min")` 等），**键名常量集中在 impl 内**对应 Python 零硬编码读取层
- settings.rs：`UserConfig` serde 字段名与 Python 完全一致（time_dilation_rate/theme/last_city/last_timezone/countdown_target/alarms/world_pins/window_geometry），`#[serde(default)]` + 默认值**从 Static 单源取**（Default 实现读 base.json，对齐 default_factory 行为）；load：损坏 → 转存 .bak（util::file_io）→ 默认值；save：原子写 + 白名单
- time_dilation.rs：`pub struct TimeDilation { rate: f64, day_start: DateTime<Local>, day_hours: i64 /* int(24*rate) 整数口径 */ }`；`fn time_info(&self, now) -> TimeInfo`（accel 推进公式与 Python 逐行对齐：day_start 固定当日 00:00 本地，accel = std 起点差 × rate，回绕取 day_hours）；tick_ms = 1000/rate 钳 [16,∞]；测试注入 now 参数（无 Clock 依赖，PL011 层面时间全参数化）
- weather.rs 解析层：`fn parse_open_meteo(v: serde_json::Value) -> Result<WeatherData, WeatherErr>`，WeatherErr 枚举（NullField(name)/BadType(name)/MissingField(name)/BadCode(u8)）逐分支对齐 Python 拒绝路径
- weather.rs 传输层：`WeatherClient { base: Url, cache: Mutex<HashMap<String,(Instant,WeatherData)>>, ttl: Duration, client: reqwest::Client }`；`async fn fetch(&self, city, force)`：白名单校验（scheme==https && host==open-meteo 常量）→ force 则清该城键 → GET → 解析 → 重试经 util::retry（RetrySpec{max:3, backoff:1s, 白名单错误}）→ 写缓存；**URL 由常量 + 已校验城市坐标拼接**（对齐 Python SSRF 防线）
- util::retry：`pub async fn retry<F, Fut, T, E>(spec: RetrySpec, f: F) -> Result<T, E> where E: IsRetryable`（IsRetryable trait 对齐 Python 白名单异常判定）

**G. PL011 验证流程（命令级）**

1. `cargo build --workspace` 绿；2. 结构对照 A 节树逐项；3. 故意在 core 引 egui → CI 依赖检查红，撤销绿；4. Python 夹具导出两次 sha256 一致；5. `cargo test -p accel-core` 输出含四类夹具断言计数（time/config/weather 各 N 条全绿）；6. httpmock 桩跑传输层五路径（命中/过期/重试耗尽/force/白名单拒）；7. 归档后再导出夹具哈希一致。

**H. PL011 完成记录（2026-10-16，全 14 条收口）**

G 节七项验证全过：build 绿/树对照零差异/egui 违例红（exit 1）撤销绿（exit 0）/夹具三跑 sha256 一致/63 测试全绿（含 time 47 向量·config 22 脏值·weather 61 样本·data 三表夹具断言）/httpmock 五路径绿/归档再生哈希一致。夹具清单：time_vectors 47、config_behavior 22 脏值 + 往返/损坏/未知键、weather_samples 61（35 收 26 拒）、lunar_samples 365 天 + 24 时辰、data_tables（17 城/8 时区/28 码 + 兜底）；manifest 自登记 sha256。

**执行偏移登记（plan 文本 vs 实做，以 Python 行为/结构事实为准回正）**：

1. 集成测试位置：rs/tests/ → crates/accel-core/tests/parity.rs（虚拟 manifest 根级目录不参与编译，占位测试从未运行即暴露）
2. CI 脚本：scripts/ci.sh → rs/ci.ps1（pwsh 对齐仓房规）；房规样例入 rs/README.md（不新建 CONTRIBUTING.md）；GitHub Actions 为 .github/workflows/rs-ci.yml（windows-latest）
3. serde_json **float_roundtrip 特性强制启用**：默认浮点解析可差 1 ULP（110.00000000000001 解析为 110.0），夹具位级 parity 的硬前提——已写入 rs/Cargo.toml 注释
4. tick 口径：plan 文本 clamp(16,∞) 有误，Python 实现为 max(1, min(clock_tick_ms, int(1000/rate)))，以 Python 为准
5. retry 规格：固定 delay 无 factor 指数退避（对齐 Python retry_call 实现而非 plan 文本）
6. weather_code 归一 i64：Python 动态类型渗漏（"0"/True 原样入 DTO），Rust 以 scalar_code_key 复刻查表语义（bool→1、整值浮点→整键、其余→未知），展示字段（weather/description/icon）位级 parity；BadCode 变体取消（未知码 Python 本就接受并回退）
7. SSRF 防线结构化：Python 运行期 scheme/host 白名单 + DNS 私网阻断 → Rust 构造期固定 base（生产 https 常量）+ URL 全派生 + 禁重定向；「host 篡改」在 Rust 下不可达，对应测试改为输入侧拒绝 + URL 派生断言
8. force 语义：Python 清全缓存 → per-city 清键（本总纲修订二·C 定案，多城市语义正确）
9. 配置白名单根：Python「项目根（**file** 相对）」→ exe 目录 + ACCELWORLD_WRITE_ROOT env + temp（编译型语言无源码根概念）
10. 序列化字节级偏移（值级等价）：serde_json 缩进 2 vs Python indent=4；JSON 文本 NaN/Infinity——Python 容忍并按脏值过滤（不转存），serde_json 判损坏走转存，两路径终点同为默认值
11. static_config：get_or_init 自初始化取代显式 init + 未 init panic（语义超集）；config.json 映射表不迁移（include_str 路径即唯一结构约定）
12. 夹具导出参数按覆盖等效微调：rate 集 {1.0,1.1,2.0,5.0,10.0,15.5,20.0} + 越界 {0.9,0.0,−3.0,20.1,100.0}、std 六时刻（含 23:59:59.999999/微秒三种尾数）、2026 全年实为 365 天
13. Python 归档执行：AI 不执行 git 写操作，用文件系统 mv（main.py 副本与 git 历史逐字节核验一致）；git 呈删除态，提交由用户定夺；AGENTS.md 仍为 Python 时代内容，验证命令自 PL011.14 起以 rs/ci.ps1 为准，重写归 PL015.04
14. 依赖版本落定（2026-10-16 crates.io）：serde 1.0.229/serde_json 1.0.154(+float_roundtrip)/tokio 1.53.3/reqwest 0.13.5（特性 rustls 已更名，非 rustls-tls）/chrono 0.4.45/thiserror 2.0.21/tracing 0.1.44/httpmock 0.8.3（Mock 无生命周期句柄，calls() 取代 hits()）

#### PL012 详细方案：核心域二期 → core 全量 parity

**A. LunarInfo DTO（chinese_calendar.rs，字段对齐 Python dataclass）**
`pub struct LunarInfo { lunar_year: String, shengxiao: String, lunar_month: String, lunar_day: String, shichen: String, jieqi: String, yue_phase: String, cai_shen_dir: String, position: String, public_holiday: String }` + `pub fn lunar_info_str(&self) -> String`（拼装顺序与标记契约逐字符对齐：`{年}（{生肖}年）{月}{日}{时辰} 月相：{X} 财神：{Y}方向（{Z}）`，节气/节日不入串——FIX003.5 定案延续）
**B. 实现路径三分法（PL012.01 出入清单的判据）**：lunar-rust 字段可用→直调用；API 有数据无便捷方法→本地薄实现（如月相名：lunar-rust 取农历日数 → 本地 30 日名称表映射，表从 lunar-python 源码导出为常量）；缺失→纯本地（财神方位：12 月支 → 方位表，表从 Python Cai_SHEN_DIRECTION 数据导出）。每条出入在 z.plan 登记来源。

**B'. PL012.01 字段级出入清单（2026-10-16 实测，107 代表日 × 7 字段 = 749 条比对，探针 examples/lunar_compare.rs）**：

| LunarInfo 字段           | 归属         | 依据（API 名 / 数据表来源）                                                                                                                                                                                                                                                                                           |
| ------------------------ | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| lunar_year（干支年）     | **直调**     | `Lunar::get_year_in_gan_zhi()` + "年"（107/107 全等）                                                                                                                                                                                                                                                                 |
| shengxiao（生肖）        | **直调**     | `Lunar::get_year_sheng_xiao()`（全等）                                                                                                                                                                                                                                                                                |
| lunar_month（农历月）    | **直调**     | `Lunar::get_month_in_chinese()` + "月"（全等，闰月"闰"前缀形态一致）                                                                                                                                                                                                                                                  |
| lunar_day（农历日）      | **直调**     | `Lunar::get_day_in_chinese()`（全等）                                                                                                                                                                                                                                                                                 |
| yue_phase（月相）        | **直调**     | `Lunar::get_yue_xiang()` + "月"（全等）——**消解 B 节预判**（无需本地 30 日名称表，lunar-rust 自带）                                                                                                                                                                                                                   |
| jieqi（节气）            | **直调**     | `Lunar::get_jie_qi()`（非节气日空串语义一致，含边界 ±1）                                                                                                                                                                                                                                                              |
| shichen（时辰）          | **纯本地**   | SHI_CHEN 12 档双时刻表移植（Python 同表，24 小时辰探针夹具已备）                                                                                                                                                                                                                                                      |
| cai_shen_dir/position    | **纯本地**   | 12 公历月 → 方位表（Python Cai_SHEN_DIRECTION 导出常量）                                                                                                                                                                                                                                                              |
| public_holiday（节日）   | **三级兜底** | T1 直调 `Lunar::get_festivals().first()`（春节/除夕/端午/中秋等农历节日日全等）；T2 holidays.json（chinese-calendar 全量导出——假日跨度多日 + 英文名经 HOLIDAY_TRANSLATION 中文化，运行时不再需要翻译表）；T3 CUSTOM_HOLIDAYS 12 西式/公历节日表移植。实测 38 DIFF 全部为 T2 跨度日与 T3 节日（T1 空），三级合流后闭合 |
| chinese_date（中文日期） | **纯本地**   | 星期中文表 + `format("%Y年%m月%d日")`                                                                                                                                                                                                                                                                                 |

零未决字段。lunar-rust 经 SolarRefHelper/LunarRefHelper trait 调用（`solar::from_ymd(y,m,d).get_lunar()`）。
**C. holidays.json schema**：`{ "2026": { "01-01": "元旦", "2026-01-28": "除夕", ... } }`（chinese-calendar 全量导出 + HOLIDAY_TRANSLATION 中文名），加载器 `fn holidays(year) -> Option<&Map>`（None = 超范围优雅降级空表，对齐 NotImplementedError 捕获路径）
**D. Alarm serde 结构（字段名/默认值与 Python to_dict 完全一致——持久化兼容硬约束）**

```rust
#[derive(Serialize, Deserialize)]
pub struct Alarm {
    pub label: String, pub time: String,                    // "HH:MM"
    #[serde(default = "default_preset")] pub sound_type: String,   // "preset"|"custom"
    #[serde(default = "default_classic")] pub sound_value: String,
    #[serde(default)] pub repeat_days: Vec<i64>,             // 0-6，空 = 一次性
    #[serde(default = "true_")] pub enabled: bool,
    #[serde(default = "gen_id")] pub id: String,
    #[serde(default = "now_iso")] pub created_at: String,
}
```

规范化在 `Alarm::normalize()`（构造后必调）：repeat_days 字符串强转 i64 / bool 剔除 / 越界剔除 / 去重；时间 HH:MM 校验失败 → Err。
**E. AlarmManager 状态机（触发判定，顺序即实现顺序）**
`fn should_trigger(&self, a: &Alarm, now: DateTime<Local>, ) -> bool`：enabled? → HH:MM 等值? → repeat_days 空 ? 一次性路径（created_at 解析 → 计划日 = created 当日 or +1（创建时刻已过设定时分）/ 计划日 < 今日 → 已过期不触发） : 星期命中。触发去重键 `"YYYY-MM-DD HH:MM"`（HashMap<id, String>，仅内存——永久豁免条目延续）。过期禁用：装载时对一次性且计划日 < 今日者 enabled=false + warning。
**F. Clock 抽象（可测性核心）**：`pub trait Clock: Send + Sync { fn now(&self) -> DateTime<Local>; }`；`RealClock`（chrono Local）/ `VirtualClock(Arc<Mutex<DateTime<Local>>>)`（测试可拨动）。AlarmManager 调度任务与全部触发单测经 Clock 注入——**杜绝真睡眠测试**。
**G. rodio 音频**：`fn play_preset(p: PresetSound)`（频率/次数/间隔三元组表从 Python _PRESET_SOUND_CONFIG 导出为常量，rodio Source 生成正弦段拼接）；`fn play_file(path)`（rodio Decoder，mp3/wav）。
**H. 统一城市表 schema**：`config/static/world_cities.json`：`[{ "name": "北京", "tz": "Asia/Shanghai", "lat": 39.9042, "lon": 116.4074, "region": "east-asia" }]`（40-60 城，IANA 逐条经 chrono-tz 解析校验后方可入库）；检索 `fn by_name / fn all_names`。
**I. PL012 验证流程**：1. 出入清单落档后逐字段跑 lunar_samples 全量（366 天字符串全等）；2. 节气边界日 ±1 断言；3. 2035 年降级空表不崩；4. Alarm 用例全集 = Python test_alarm_service 18 用例行为逐条移植（跨天顺延/编辑重复拒/复活防护/装载截断）；5. VirtualClock 拨到触发点断言 AlarmTriggered 恰发一次；6. holidays 全年断言；7. 全部夹具一次全跑清单入档。

**J. PL012 完成记录（2026-10-16，全 11 条收口）**

I 节七项验证全过：**95 测试全绿**（81 单测 + 14 夹具集成）+ CI 四关卡 ALL_GREEN（clippy 零警告）。夹具断言计数清单：time 47 向量位级 / config 默认+往返+22 脏值+损坏+未知键 / weather 61 样本（35 收 26 拒）+ 展示文本三态 / **lunar 365 天 × 十字段逐字段全等 + info_str 逐字符全等 + chinese_date 全等 + 24 时辰探针** / data 三表全量 + 城市表 50 城 tz 全解析；闹钟域 Python 用例集移植：跨天顺延/编辑成重复拒（排除自身 id）/created_at 非法不复活（FIX003.8）/装载 11 截 10（FIX004.15）/过期一次性禁用（FIX004.14）/同分钟去重/星期命中；调度 VirtualClock 三阶段拨钟（前一分不触/触发恰一次+一次性禁用回推/同分钟重拨不重发）。

**执行偏移登记（PL012）**：

1. 时间解析实测基线：Python `time.fromisoformat` 对 "9:30"（单位数小时）实为**拒绝**——x.progress 任务文本「"9:30" 补 :00」与实测不符，Rust `parse_alarm_hhmm` 以 Python 为准（两位小时严格位宽；无冒号补 ":00" 后仍须合法，"0930" 拒绝）
2. repeat_days 布尔语义：Python 实码 `isinstance(day, bool): continue` 为**剔除**（任务文本「[true]→[1]」错误，那是穿透防御注释的本意），Rust 同为剔除
3. lunar-rust API：`solar::from_ymd`（模块级自由函数）+ `SolarRefHelper/LunarRefHelper` trait；chrono 0.4.45 `NaiveDate::year` 固有方法私有须经 `Datelike` trait
4. rodio 0.23 API 与 plan 预想（OutputStream/Sink）全变：`DeviceSinkBuilder::open_default_sink` + `mixer::Mixer`（Arc 克隆）+ `Player::connect_new/append/detach`；声道/采样率为 NonZero 类型；新增 `core/src/audio.rs` 挂 lib（plan 工程树未列，登记）
5. world_cities 落位 50 城（40-60 区间内），放 config/static/、加载器在 data/world_cities.rs（include_str 路径跨约定，登记）
6. holidays.json：门控对齐 Python `if holiday_detail[0]`（初版漏门控多导 19 个调休补班日，自校验发现后修正）；2024-2026 共 89 天（28/28/33）；导出器 `.archived/export_holidays.py`（年末维护重跑即 y.problems#8 的根治路径）
7. 农历缓存承接位置修正：PL011 注记「interface 层承接」改为 **TimeDilation 模型内 Mutex 秒级缓存**（与 Python `_lunar_cache` 同构）；TimeInfo 扩 chinese_date/lunar_info 两字段（#[serde(default)] 向后兼容）
8. 闹钟调度关停：Quit 仅杀主循环会令调度任务持 evt_tx 悬挂（recv 永不闭合——测试实证卡死）→ `tokio::sync::watch` 保态关停信号双任务同退
9. 闹钟随 AppInterface 构造自配置装载（Python 由 UI 显式 load_alarm_dicts，Rust 内聚；登记）；一次性触发后自动禁用+回推为 PL012.07 编排定案（Python 无此自动禁用——UI 编排差异，任务文本定案）
10. 财税合规类:gen_id 用 pid×纳秒 xor 8 位十六进制替代 uuid4[:8]（同形态无 uuid 依赖）；now_iso 微秒为 0 省略段对齐 isoformat

**豁免清单复核（PL012.11，按 Rust 实现现状逐条标注）**：

- **延续 9 条**：_last_triggered 仅内存（Rust 同 HashMap 内存态）；alarm enabled 双重检查（保持冗余防御）；weather 重定向拒后按网络错重试（reqwest Policy::none 同语义）；临时目录整目录放行（file_io 同依据）；原子写无 fsync（同）；replace 不清同 id 去重键（语义同）；干支按立春分界（lunar-rust 默认口径，夹具全年实证）；哔声 200ms 常量（PRESET_BEEP_DURATION_MS）；条件豁免 2 条延续——tmp 仅 pid 竞态（触发条件不变）、tick 钳 1ms 仅 rate>1000 可达（触发条件不变）
- **core 域消解 12 条**：DNS rebinding TOCTOU（SSRF 结构性防线）、_weather_cache 无锁（Mutex）、lunar 库异常上抛（lunar-rust 无异常面）、PyQt6 入 utils（core 零 GUI CI 强制）、audio GC 疑虑（rodio 显式持有）、dataclass 未知注解放行（Option<Value> 显式 match）、get_type_hints 重解析（无此机制）、save TypeError 穿透（serde 不可能失败）、config.json 分类丢弃（映射表不迁移）、get_alarms 内层共享（Vec<Value>/Alarm Clone 深拷贝）、空 **init**.py（形态不存在）、r_cap 量纲留待 PL013.05 WGSL 移植时按新管线评估（归 UI 侧重评）
- **UI/监控域 16 条随 PL013-015 落地重评**：monitor 三条（PL013.12 panic hook/tracing 落地）、Qt 回调/GL 七条（PL013 玻璃管线）、托盘/主窗四条（PL013/015/014）、clock_panel 动画一条（PL014）——Python 版定案理由随 UI 重写失效，届时按 egui/wgpu 实现重评
- 豁免清单本体（第 130-179 行）不改写——Python 版豁免对归档代码仍然有效，Rust 侧结论以本节为准

#### PL013 详细方案：egui 外壳与玻璃地基

**A. App 主结构与主循环（window.rs + bus.rs）**

```rust
pub struct AppHandle {
    iface: Arc<AppInterface>, cmd_tx: Sender<FrontendCommand>, evt_rx: Receiver<BackendEvent>,
    state: UiState,                    // 全部 UI 状态（当前页/主题态/hover 态/各页临时输入）
    glass: GlassScene,                 // Rc<RefCell>（egui 单线程，无需 Mutex）
    theme: Theme, pages: Pages, nav: NavRail, settings_pop: SettingsPop,
}
impl eframe::App for AppHandle {
    fn update(&mut self, ctx, frame) {
        while let Ok(evt) = self.evt_rx.try_recv() { self.state.apply(evt); }  // ① drain 事件
        self.nav.ui(ctx, &mut self.state);                                     // ② 导航（发命令经 cmd_tx）
        self.pages.current_mut().ui(ctx, &self.state, &self.cmd_tx);           // ③ 当前页
        ctx.request_repaint_after(self.state.next_deadline());                 // ④ 节奏：Tick 间隔与动画期取短者，空闲 1s 兜底
    }
}
```

**B. WGSL uniform 布局（与 Python GLSL 一一对照，painter.rs 组装）**
`uniforms { u_res: vec2<f32>, u_count: u32, u_rect: array<vec4<f32>,16>, u_radius/u_refr/u_sel: array<f32,16>, u_tint: array<vec3<f32>,16>, u_top/u_bottom/u_glow_a/u_glow_b: vec4<f32>, u_glow_a_pos/u_glow_b_pos: vec2<f32>, u_glow_a_rad/u_glow_b_rad: f32, u_accent: vec3<f32>, u_time: f32, u_lift: f32 }`——键名/语义/默认值与 ui/gl/shaders.py 对照表落档（PL013.05 交付物），SDF/覆盖判向/预乘逐行移植，**每移植一条对照 t.hint HINT002 勾销一条**。
**C. GlassScene（surface.rs）**：`struct GlassScene { surfaces: Vec<GlassSurface> }`（id 排序即绘制序，无 HashMap——Rust 侧 Vec 线性检索 16 面内足够且保序）；`register/unregister/set_visible/set_selected/update_geometry/touch` 契约与 Python 同名同语义；touch → `ctx.request_repaint()`（egui 版直连重绘，等价 FIX004.7 hook）。
**D. 渲染断言探针（PL013.07）**：offscreen 渲染 → 读回像素（wgpu buffer→cpu）→ 分区采样断言（面内外色差 ≥12、rim 增量 ≥60、金晕底/紫晕顶、背景 alpha 微黑档 0.08-0.22）——**阈值直接沿用 Python 版探针实测基线**，脚本 `rs/tools/render_probe.rs`（cargo bin），CI 可选跑。
**E. theme.rs 映射表**：ui.json 键 → egui 对象（colors.primary→selection/hover 色、radius.*→Visuals.window_rounding/menu_rounding、scale.*→FontId::new、glass.*→玻璃 uniform 默认、field.*→光场 uniform）；`Theme::apply(&mut ctx)`；三态偏好经 SetTheme 命令持久化。
**F. PL013 验证流程**：1. cargo run 空窗退出码 0；2. bus 单测（命令往返 + fake Tick 驱动 state 变化）；3. 字形截图（中文/数字等宽断言）；4. Acrylic 截图对照 + 失败注入降级；5. render_probe 全阈值 PASS；6. 改 token 值 → 截图 diff 断言跟随；7. 导航切换状态断言；8. 空闲期 repaint 计数 = 节奏值（无忙转）。

**K. PL013 完成记录（2026-10-17，全 13 条收口；当日审计复核修复后定稿）**

F 节八项验证全过：真窗 exit 0/bus 单测/字形 AI 读图（无 tofu）/Acrylic 截图透出 + 失败注入/render_probe 五断言全 PASS（rim 74≥60）+ 破坏注入即红（22<60）/导航切换截图/帧计数探针静置 5s=50 帧恰中预期（零忙转）。**119 测试全绿**（app 24 + core 81 + 集成 14）+ **clippy `-D warnings` 零错误**。

**审计复核修复（2026-10-17 二次收口）**：审计独立复核发现 9 项缺口已全部修复——①clippy `-D warnings` 实为 9 错（早前"零警告"系未带 -D 的不实证）全修；②等宽断言补实做（tools/clock.rs galley 宽度，**实发现 Bahnschrift 可变字体在 egui/ab_glyph 下数字 advance 非等宽**（83.25 vs 124.53px，差 50%，与 Qt 版行为不符）——等宽方案为观感决策点留 PL014.01 用户定夺，现阶段断言收敛为 layout 确定性 + 差值探针）；③设置浮窗"点窗外关闭"补实做；④nav 点击路径状态单测补（apply_nav_frame 抽取）；⑤**accent 硬编码金色清除**——统一消费 colors.primary token（theme::accent_rgba 单源，GL u_accent 与导航共用）；⑥**nav 几何 token 化**（NavGeom::load 消费 nav_rail_* 五键，硬编码常量清除）；⑦interface 双 tokio 任务补 join panic 监控（不静默）；⑧5 文件 `// ===== 说明区` 补齐；⑨render_probe clippy 修复（recv panic 语义/drop Copy）。

**执行偏移登记（PL013，API 大版本实测适配）**：

1. **eframe 0.37 App trait 重构**：`update(ctx, frame)` → `logic(ctx, frame)`（每帧先行、隐藏期也调用——drain 唯一入口）+ `ui(&mut Ui, frame)`（面板挂 Ui 而非 Context）；on_close_event 移除 → `input.viewport().close_requested()` 检测
2. **egui 0.37 样式体系重构**：Color32 移除 `from_rgba_unpremultiplied`（全预乘）——FieldTokens 改存非预乘 f32 数组（shader parity 语义直传）；Rounding→CornerRadius(u8)；Visuals 圆角字段更名 window_corner_radius/menu_corner_radius；双主题并存（set_theme(ThemePreference::System/Light/Dark)——**System 跟随内建，替代 Python 手工跟随**）
3. **wgpu 30 API**：PipelineLayoutDescriptor（bind_group_layouts 为 Option 数组 + immediate_size）/VertexState·FragmentState（entry_point Option + compilation_options）/RenderPipelineDescriptor（multiview_mask）/RenderPassDescriptor（multiview_mask + depth_slice）/PollType::Wait{submission_index,timeout}/get_mapped_range 返 Result/create_buffer_init 移 util::DeviceExt
4. **egui-wgpu 0.37**：Callback::new_paint_callback + CallbackTrait（prepare/paint）+ CallbackResources = TypeMap——**条目需显式 insert 后才可 get_mut**（真窗首帧 panic 实证）；PaintCallbackInfo 来自 epaint
5. **Acrylic 关键发现**：无 DwmExtendFrameIntoClientArea 底座直接覆写 SYSTEMBACKDROP → **窗口不可见**（DWM 合成异常，真窗实证；Python 版经 qfw setMicaEffectEnabled 隐式带底座）——补 1px 底边扩展后恢复；MARGINS 在 windows crate 的 Win32::UI::Controls（非 Dwm 模块）；句柄链路 Frame→HasWindowHandle→WindowHandle→From→RawWindowHandle（0.6.2 特殊设计：window_handle 递归返自身）
6. uniform 尺寸实算 1168（73×16 对齐）；tint 打包进 radius 数组 yzw 通道（省一个 array<vec4,16>）
7. render_probe 采样点：rim 峰在面内侧 ~1.3px（0.002uv×宽）——外侧采样曾误红，实测修正
8. 页面避让 rail：Margin 字段 i8（180 超界）→ CentralPanel margin 24 + horizontal add_space(156) 横向占位
9. 帧计数探针：ACCELWORLD_FPS_PROBE=<秒> 环境变量 → 静置 N 秒打印 REPAINT_COUNT 并自动关窗（run_with_counter 注入 probe_close_secs）
10. 待查项（登记 y.problems）：CloseMainWindow 偶发失效（强杀兜底）；with_position 定位实际落点与 token 偏移（winit 行为）；GPU 热释放后紧接启动的一次性黑窗竞态（不可复现，单实例复现正常）
11. 复用备注：nav Area id 用 Id::unique(source)（Id::new 已弃用）

#### PL014 详细方案：五页功能落地 + 设置浮窗

**A. Page trait 与页面状态**

```rust
pub trait Page { fn ui(&mut self, ctx: &egui::Context, state: &UiState, cmd: &Sender<FrontendCommand>); }
pub struct Pages { current: PageId, clock: ClockPage, countdown: CountdownPage, world: WorldPage, weather: WeatherPage, alarm: AlarmPage }
// 各页自持临时输入态（如倒计时弹窗草稿），持久态一律在后端（UiState 只是后端快照）
```

**B. 倒计时数据迁移（serde 兼容双格式）**

```rust
#[derive(Deserialize)] #[serde(untagged)]
enum CountdownValue { Legacy(String), List(Vec<CountdownTarget>) }   // 旧单值 → 迁移为列表首项
pub struct CountdownTarget { name: String, at: DateTime<Local> }
```

迁移在 settings load 后处理一步完成并回写；默认目标（列表空时）= `next_spring_festival(now)` 动态派生（lunar 农历正月初一 → 公历），UI 只读不落盘（根治 y.problems#8）。
**C. 节日派生算法**：`fn year_chips(now) -> Vec<Chip>` = holidays.json 法定节日（当年+次年各取未过项）+ lunar 传统节日计算（春节/元宵/清明[节气法]/端午/七夕/中元/中秋/重阳/腊八/除夕，逐个的 lunar 日期规则表落档）→ 合并去重 → 按日期排序 → 截未来一年内。
**D. 天气 DTO 扩展**：`WeatherData { current: Current, hourly: Vec<HourlyPoint>, daily: Vec<DailyPoint> }`；请求参数追加 `&hourly=temperature_2m,precipitation_probability&daily=temperature_2m_max,temperature_2m_min,sunrise,sunset&forecast_days=7`；解析对新增字段同样 null/缺字段拒绝纪律。
**E. 闹钟快捷换算**：`fn quick_target(minutes: u64, now) -> AlarmDraft`（now+minutes → HH:MM + created_at=now，一次性 repeat_days 空——跨午夜由既有顺延语义自然覆盖）。
**F. PL014 验证流程**：每页 = ①纯函数单测（对 Python 用例移植）②虚拟数据渲染截图断言 ③交互走查三件套；六页过完跑 135 用例行为映射表核对（PL014.16 交付物：Python 用例编号 → Rust 测试名对照清单）。

**L. PL014 完成记录（2026-10-17，全 16 条收口）**

F 节验证全过：**145 测试全绿**（app 40 + core 87 + parity 集成 18）+ clippy `-D warnings` 零错误。分域清单：

- tools 纯函数（.01）：split_lunar_info 三段拆分（FIX004.3 无"拜"回归）/progress_bounds/format_hms/fmt_percent/format_repeat_display/format_sound_button_name（Python test_ui_tools 用例逐条移植）/parse_target_text 三格式/format_remaining
- widgets（.02-.04）：GlassCard（注册幂等/选中两值化 FIX004.4 平移/减号响应区几何锚定）/Chip（elevation 5 契约）/ring（progress_to_arc 含 0/100 边界钳制 + 48 段折线逼近弧——Painter::arc 已移除）/slider（ratio_to_rate 0.1 步进/零进度整圆定案/旋钮浮点居中）
- 倒计时域（.06-.08）：CountdownTarget 模型 + Legacy 迁移矩阵六用例（19/16/10 位/空/非法/新格式优先）/year_chips 三年夹具 parity（lunar-python 同规则 39+10+10 条）+ next_spring_festival 五用例 + 清明漂移抽查；pages/countdown（hero/网格/减号/加号 validate_add 校验）
- 天气域（.11-.13）：HourlyPoint/DailyPoint DTO + 请求参数扩展 + 同纪律拒绝（null/错型/长度）/缺键→空序列向后兼容（Python 快照 parity 保持——偏移登记）；pages/weather（当前卡/逐时 24 横条/七日/城市切换 chips/强制刷新）
- 世界域（.09-.10）：now_in_tz 纯函数（chrono-tz DST 表驱动）+ 城市矩阵（昼夜指示/点卡选基准/减号 ≤1 禁用/候选下拉 = 全表−已添加）
- 闹钟域（.14-.15）：quick_target 换算（15min@23:50→次日 00:05 一次性/240min 跨日）+ 四快捷按钮 + 自定义添加 + 列表行卡（开关/删除/重复文案）；**IME 组合事件模拟**——ImeEvent::Commit 与 Event::Text 双通路 InputState 可读断言（run_ui 全渲染 pass 测试环境 textures delta 崩溃——收敛为 InputState 层，实证登记）

**执行偏移登记（PL014）**：

1. untaged 双格式改为显式双字段（countdown_target 旧串 + countdown_targets 列表各自 Option<Value> 装载）——语义等价且校验粒度更细，无 untagged 歧义
2. weather hourly/daily **键缺失→空序列**（非 Err）：Python 0.6.0.0 快照无此键，严格拒绝会破坏 61 样本 parity——键存在时 null/错型/长度不一致仍全严拒绝（任务文本"缺字段→Err"收窄为此，偏移登记）
3. weather series 提取的 SeriesMissing 键名参数需 'static——literal 常量传参
4. 节日夹具三年份（2026/2027/2028）：2027 年中窗口跨至 2028 春节（次轮正月初一），两年份夹具不完备
5. IME 模拟收敛为 InputState 层：run_ui 全渲染 pass 留未处理 textures delta（0xC0000409 崩溃）——TextEdit 消费链路由 egui 自带测试覆盖
6. 页面输入态（countdown_draft/alarm_draft）挂 UiState——各页自持临时输入态的 trait 定案延续
7. ring 弧线 48 段折线逼近（egui 0.37 移除 Painter::arc）；半径 40+ 弦差 <0.1px 观感真圆
8. 法定+传统合并定案：传统名覆盖法定同日（夹具实证：春节正月初一覆盖元旦跨度重合日——无观察差异）
9. 135 用例映射表（PL014.16 交付物）落 .archived/tests_migration.md（Python 用例名 → Rust 测试名逐条对照，无对应者标注原因）
10. **天气域收敛单城（A005 #4 修复，2026-10-18）**：PL014.13 规划的"多城市增删（AddWeatherCity/RemoveWeatherCity + 持久化 weather_cities 列表）"从未接线（命令零发送零处理、UiState.weather_cities 无写路径），且 Python 0.6.0.0 侧天气本就是单城市模型（配置仅 last_city，`user_config.json` 无多城字段）。按"不长期保留废弃方案"删除两个死协议变体与 UiState.weather_cities/weather_cities_view，改以 `SetWeatherCity(String)`（命令 match 改穷尽，新增变体不接线即编译失败）+ 天气页城市选择器提供换城并持久化 last_city——功能等价于 Python 单城语义且可换城，PL014.13 任务文本的多城承诺作废

#### PL015 详细方案：系统集成与收口

**A. 托盘**：tray-icon 菜单树（显示窗口/隐藏到托盘/分隔线/当前倍率 x.x[只读，随 Tick 节流更新]/退出）；菜单事件与左键双击 → TrayCommand（主线程每帧 drain 归一）；关闭按钮 → 隐藏驻留（CancelClose + Visible(false)，对齐 closeEvent 语义），托盘"显示窗口"恢复；悬停文本 = 加速时间 + 倍率（Tick 更新，节流 1s，文本未变跳过）。
**B. 热键与通知**：global-hotkey 注册三组（保存/退出/主题循环，键位 base.json shortcuts token 读入），事件 → HotkeyKind 归一后转命令；主题循环 Auto→Light→Dark→Auto。WinRT 通知封装 notify(title, body)，闹钟触发/配置保存失败两调用点经 bus.rs 旁路接线（静音避免与铃声双声）；平台能力失败一律降级日志不崩。
**C. 版本单源**：app 侧 `build.rs` 读 `rs/config/static/base.json` 的 version → `env!("ACCELWORLD_VERSION")` 编译期注入（工作区相对路径防穿越读取）；lib.rs 暴露 compiled_version()，窗口标题/关于页/托盘 tooltip 统一经运行期 Static 读取，代码零版本字符串；单测锚定编译期 env 与运行期 base.json 两路 parity。
**D. 文档迁移清单**：AGENTS.md 重写为 Rust 房规（镜像映射表/注释房规 ///+//!+说明区/零硬编码 token/CI 四关卡/渲染质量门）；README 重写（Rust 构建与运行）；w.study 重做项目分析（RS 版）；x.progress/y.problems/z.plan/m.milestone 原文件续写；t.hint.md 补 Rust 侧教训节。
**E. 收口口径（用户定案 2026-10-17）**：不做人工走查、不做定版仪式；PL015 以四关卡全绿（fmt/clippy/test/依赖树）+ 自动化验收（版本单源 rg、托盘隐藏→恢复→退出真窗探针、离线渲染断言探针）收口。已知遗留登记 y.problems，视觉观感类微调归后续 UI 雕刻阶段。

## 附录 UIR2：子夜星图视觉重塑方案（2026-10-24，用户定案全盘推倒重来）

> 触发：用户「现在的UI太烂了，不符合设计，可以全盘推倒重来」+ 新装 better-* 设计技能族组合执行。
> 方法：better-interface 六域编排（accessibility/writing 两域按 egui 无 DOM/中文界面边界记 Not reviewed）+ better-colors 对比度精算纪律 + better-typography 字阶纪律 + better-layout 间距纪律 + better-ui 打磨值 + frontend-design 反默认校准。

### 审查结论（代码/token 层证据）

| 严重度 | 域 | 发现 | 处置 |
| --- | --- | --- | --- |
| HIGH | colors | text_dim #626979 卡面 2.85:1 失效（日期/走时/刻度等正文级信息） | meta 层 #8A8774 重设（卡 4.79/窗 5.08） |
| MEDIUM | colors | text_soft 与 text_muted 仅差 1.28:1 同职重叠；primary 键名非角色化 | 五层文本重构 + accent/track 角色命名 |
| MEDIUM | typography | 字阶 12 键含重复值（16×2/12×2）、1px 级邻差无梯度 | 8 键收敛（72…12，无重复值） |
| MEDIUM | layout | 卡内边距 14/16/10 三种魔法数；间距 6-20 散值无阶 | spacing 域 + card_pad token |
| MEDIUM | ui | 删除钮（倒计时/世界/闹钟）悬停才显形——破坏性操作无可视线索 | 常显 meta 色、悬停朱砂 |
| LOW | design | hero=大数字+小标签+强调徽标；近黑底+单暖强调=生成式默认聚类 | 方向重定（见下） |

### 设计定案：「子夜星图」

- 概念：App 的内容本质是中式历法天文（农历/节气/月相/世界走时）——取敦煌星图夜色为底、雄黄金为星，仪器刻度为结构语言。
- 色板：黛蓝 ramp（窗 #0F1420/侧栏 #0B0F19/卡 #141A29/悬停 #1A2135/发丝 #232C44/选中 #2A3552/轨 #303C5E）+ 暖米白文本五层 + 雄黄金 #E8B544 + 朱砂 #EE7265；全矩阵 ≥4.5:1（.temp/contrast_check.awk 实测矩阵在案）。
- 签名元素：星轨刻度（时钟页 48 道刻度日进度标尺——整点高/半点矮，已过金/未至轨）——boldness 集中一处，余皆克制。
- 字阶：hero72/display40/time_lg28/time_md22/time_sm18/body_lg16/body14/meta12；间距阶 xs4…xxl32。
- chip 56px 双行 → 32px 单行安静徽标；闹钟行卡补发丝描边。

### 验证

四关卡 ALL_GREEN；测试 192（app 61+bin 1+core 110+parity 20，净增 3：tiers_distinct/spacing_monotonic/tick_height_alternates）；截图探针六帧 exit 0；像素采样实证（System.Drawing 采 PNG：侧栏 #0B0F19/内容 #0F1420 精确命中，标题栏 DWM 钉色 #0B0F19 一致）。人工走查遗留：观感终审按房规留用户桌面走查（本会话模型无视觉通道，未做截图目检——像素采样为渲染实证替代手段）。

### 附录 UIR2 补录：二轮精修（2026-10-24，V0.6.1.5）

按 better-* 复检二轮：①实际渲染对精算补测——文本对全绿，非文本三处不足（未过刻度 1.69:1 → 新键 tick_idle #55648A 达 3.13:1；发丝 1.33 → #33406A 1.82，登记设计选择；egui 默认 weak 占位 3.06 → 对齐 muted 层）；②better-layout 纪律修正——倒计时/世界组间距 ≥2× 组内距、倒计时加号行随可用宽分宽（最小窗 340px 内容区溢出修复）、闹钟行标签截断加省略号并以悬停 tooltip 给全名回路；③better-ui 高频交互纪律——侧栏/快捷 chips 补按压态（靛底即时反馈）；④星轨刻度数随宽度自适应（24..48 取偶）。验证：四关卡 ALL_GREEN、测试 194、截图六帧、像素复采精确命中。

### 附录 UIR2 补录三段：三轮精修（2026-10-24，V0.6.1.6）

better-layout「结构撑到破为止」窄窗断点：倒计时卡 2→1 列（>380 阈，卡内元素碰撞处）、世界磁贴 3→2→1 列（480/300 阈）；时钟 hero+徽标组合宽超限时 hero_fit_scale 按比例收号（下限 0.55）。交互可供性：全部自绘点击目标（导航/磁贴/开关/删除/chips/图标按钮）补 PointingHand 指针对。签名元素贯穿：天气曲线 3h 标注点改 2×6 金刻度（星轨语言回响）。验证：四关卡 ALL_GREEN、测试 197（+3 断点/收号纯函数）、截图六帧。

### 附录 UIR2 补录五段：开创性重构（2026-10-24，V0.6.1.7）

用户定案「不是修修补补，要更开创性」——结构本体动刀：①侧栏导航废除，时钟从五页之一升为恒驻主仪（住与查的分离：主仪是「住在里面」的仪表，倒计时/世界/天气/闹钟是「查阅」用的右侧抽屉，开合只缩让不退场）；②双环星轨仪——外环现实一日 96 刻度（白游标）、内环加速一日 48 刻度（金游标），时间膨胀画成两个转速不同的同心环（角差即膨胀读数），中心等宽 hero+×徽标；③底部仪器坞图标导航（egui Panel::bottom + icon_button 复用），设置浮窗重锚右下。几何全纯函数（day_fraction/day_angle/polar/dial_diameter）。验证：四关卡 ALL_GREEN、测试 198、截图六帧、像素实证（坞带白为已知 PrintWindow 底部伪影——内容停界+浮窗锚位佐证占位）。
