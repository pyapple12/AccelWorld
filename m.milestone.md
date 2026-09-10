# m.milestone.md —— 版本里程碑清单（格式参考 y.problems.md；命名沿用用户指定）

> 用途：记录后续版本的里程碑任务路线。格式：编号标题：内容（markdown 列表风格）。
> 本文件为规划清单，不参与运行；执行进度仍以 x.progress.md 为准。
> 版本号自 0.4.7.0 起启用四段式纯数字（见 AGENTS.md「版本体系」）。

---

## 版本 0.4.7.0 的 MileStone（工作流体系接入，2026-09-10 已发布）

1. 接入 DeepTransHub 工作流体系：文档四件套（w/x/y/z）+ m.milestone.md
2. AGENTS.md 引入工程原则与 Commit 提交规范
3. .agents/skills/ 项目级技能四件（audit-project / audit-report / progress-task / skillforge）
4. 版本号切换四段式（ver 0.46 → 0.4.7.0）

## 版本 0.4.7.1 的 MileStone（实测问题修复，任务组 T001，2026-09-10 发布）

1. 加速时间刷新频率随倍率变化（1/rate 秒节奏，P1）✅
2. 倒计时日期选择器实时反馈（快捷按钮/日历点击即勾选，P2）✅
3. 时间选择框显示不全修复（分钟可见，P3）✅
4. 运行监控体系三项落地（excepthook / qInstallMessageHandler / faulthandler，可选）→ 于 V0.4.7.2 发布

## 版本 0.4.7.2 的 MileStone（运行监控体系，任务组 T002，2026-09-10 发布）

1. 全局异常钩子：主线程/子线程未捕获异常带堆栈写入 logs/ 每日日志
2. Qt 原生警告转发：五级消息按级别接入日志体系（QSS 解析失败等可见）
3. faulthandler 原生崩溃栈落盘：logs/crash-YYYY-MM-DD.log，并入同保留期清理

## 版本 0.4.7.3 的 MileStone（短期功能项，任务组 T004，2026-09-10 发布）

1. 快捷键支持：Ctrl+S 保存 / Ctrl+Q 退出 / Ctrl+T 主题切换，键位常量入 base.json ✅
2. 倍率预设：工作/专注/睡眠一键切换（预设定义入 base.json，与滑杆/手输共用信号链）✅
3. 进度条动画：QPropertyAnimation 平滑过渡替代 setValue 跳变（时长入 base.json）✅
4. 托盘 toolTip 实时化：悬停显示当前加速时间/倍率，随 tick 更新（文本未变跳过重绘）✅
5. 附带：UI 美化方向定案归档（T003，维持现状）；已知问题 #6（GUI 退出期硬崩溃，规避方案固化）记录
6. 测试扩充：54 → 56 用例（新增倍率预设数据合法性 + 子进程隔离的配置生效断言）

## 版本 0.4.7.4 的 MileStone（第 1 轮审计修复，任务组 FIX001，2026-09-11 发布）

1. P0 配置链路加固：GBK 编码容错、损坏配置转存 .bak、JSON 原子写入（防半截文件与不可恢复丢失）✅
2. P1 功能失效修复：天气断网降级+重试回归、法定节假日中文名、默认城市启动即查询、铃声自定义→预设切换、倍率上限校验+启动脏值回退 ✅
3. P2 防御与接线：dataclass 反序列化类型校验、闹钟 created_at 防崩、倒计时恢复跨会话保留、主题持久化、子进程测试配置隔离（ACCELWORLD_CONFIG_FILE）、白色系颜色入 ui.json、天气缺字段判失败、CLI 周期同源、网络参数入配置、缓存键统一 resolve、日志跨天重开防护、保存失败上浮提示 ✅
4. P2 语义定案：一次性闹钟创建时已过时间自动顺延次日触发（替代 S8.4"仅创建当天"）✅
5. P3 批次清理 28 项：写盘去抖、对话框释放、excepthook 链式、--version 零副作用、规范与死代码清理等 ✅
6. 测试扩充：56 → 78 用例（新增 logger/monitor/gui_features 测试文件，GUI 断言子进程化沉淀）✅
7. 审计报告归档：z.plan.md 附录 A001（第 1 轮全量审计，2026-09-10）✅

## 版本 0.4.7.5 的 MileStone（第 2 轮审计修复，任务组 FIX002，2026-09-11 发布）

1. P1 回归与穿透修复：启动脏倍率回退被残留构造击穿（A001 FIX001.7 残留）、合法 JSON 非 dict 结构穿透类型校验并旁路 .bak 转存 ✅
2. P2 防御补全：NaN/Infinity 拒绝、天气响应 null/错型值判失败、读体阶段网络异常入重试白名单、白名单外保存降级、原子写 tmp 唯一化 ✅
3. P2 修复副作用治理：GUI 动画断言去时间依赖、托盘初始倍率同步持久化值、--theme light 生效、预设子进程天气打桩、GUI 测试打桩还原、启动双请求消除 ✅
4. P3 残留清理：static_config RuntimeError 承诺补全（A001 残留）、日志默认值单源化（A001 残留）、timezones 说明区（A001 遗留）、闹钟 repeat_days 口径统一、SUPPORTED_AUDIO_FORMATS 分层迁移、损坏转存节流等 ✅
5. 审计闭环：A002 报告归档（z.plan.md 附录）；_last_triggered 定案永久豁免（豁免清单①首条）✅
6. 测试扩充：78 → 90 用例（新增 tests/test_static_config.py，GUI 子进程用例扩至 12 项检查）✅

## 后续版本的 MileStone（暂不定版本号）

1. 已知问题排查：GUI 进程退出期硬崩溃（y.problems#6，P3，不阻塞功能）
2. 中大型项候选（需 OpenSpec 提案流程）：多语言界面、PyInstaller 打包发布、自定义城市
3. 1.0 大版本候选：UI 2.0（Fluent Widgets 重写 UI 层，呼应主题商店 F03c03）；长期愿景（数据统计可视化、云端配置同步等）随 1.0 规划评估
