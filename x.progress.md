# 重构进度追踪（x.progress.md）

> 依据：`z.plan.md`（AccelWorld 审计与重构方案报告）
> 当前版本：ver 0.44（M08 架构优化已发布）
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

### S7 测试引入（对应 z.plan.md 第八章 S7 与 M08c）

> 目标：pytest 覆盖核心模块（modules/config 不依赖 PyQt，可直接测试）

#### S7.1 测试框架

- [ ] S7.1.1 添加 pytest 依赖 + `tests/` 目录
- 状态：⏳ 待开发｜优先级：中

#### S7.2 测试用例

- [ ] S7.2.1 `time_dilation`：倍率校验、自定义时间计算、24h 边界、`TimeInfo` 字段
- [ ] S7.2.2 `chinese_calendar`：`LunarInfo` 各字段（干支/生肖/时辰/节气/节日）、已知日期断言
- [ ] S7.2.3 `config.settings`：默认配置、读写往返、损坏 JSON 容错、缓存
- [ ] S7.2.4 `alarm_service`：重复规则、一次性闹钟、触发去重、max_alarms 上限
- [ ] S7.2.5 `weather_service`：格式化为空容错、缓存命中
- 状态：⏳ 待开发｜优先级：中

#### S7.3 验证

- [ ] S7.3.1 `pytest` 全部通过
- 状态：⏳ 待开发｜优先级：高
