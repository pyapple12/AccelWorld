# 问题记录（y.problems.md）

> 记录待处理问题；修复后在对应项标注解决方式与版本
> 创建时间：2026-08-09（ver 0.46，S10 完成后用户实测反馈）；2026-09-10 随工作流体系接入由 y.problem.md 改名并纳入版本控制

---

## 1. 加速时间刷新频率未随倍率变化（P1）

- **现象**：加速时间的刷新与标准时间一样固定 1 秒刷新一次，无论倍率是多少
- **预期**：加速时间的刷新应按加速时钟的"秒"节奏刷新——加速时间每走 1 秒，现实间隔为 1/rate 秒（如倍率 2.0 时约 0.5 秒刷新一次，倍率 10.0 时约 0.1 秒）
- **根因分析**：`get_custom_time()` 秒级缓存按**标准时间秒**做键（time_dilation.py），GUI `update_clock` 100ms tick 但同秒内直接返回缓存 → 界面每秒只变一次，与倍率无关；CLI `run_live_clock` 同样按标准秒检测
- **关联**：`clock_tick_ms`（base.json，100ms）、秒级缓存 `_time_cache`、`TimeInfo.custom_second` 秒变化检测
- **状态**：已修复（V0.4.7.1：农历/中文日期改标准秒级缓存，标准/加速时间每次现算，新增 `tick_interval_ms` 属性并联动 GUI 定时器与 CLI 显示检测）

## 2. 倒计时日期选择器交互反馈缺失（P2）

- **现象与预期**：
  - 点击"今天/明天/一周后"快捷按钮，日历应立即勾选对应日期并反馈；当前点击后无可见反馈，需按 OK 才生效
  - 点击日历上的日期数字，应勾选对应日期并反馈（输入框同步显示）；当前点击后无反馈，需按 OK 后输入框才更新
- **现状**：`countdown_panel.show_date_picker()` 中快捷按钮已 `setSelectedDate`，日历点击已选但仅在 `dialog.exec()` 返回 Accepted 后才写回 `countdown_target` 输入框——实时反馈缺失
- **状态**：已修复（V0.4.7.1：日历 `clicked` 与今天/明天/一周后快捷按钮均即时写回输入框日期部分并保留时间部分）

## 3. 时间选择器显示不全（P3）

- **现象**：`show_time_picker()` 的时间选择框只能看到小时，分钟部分显示不出来（显示不全）
- **根因分析**：`QTimeEdit.setFixedSize(120, 40)` 宽度不足，`setDisplayFormat("HH:mm:ss")` 三位时间在 120px 内放不下
- **状态**：已修复（V0.4.7.1：移除弹窗与 QTimeEdit 固定尺寸，构建器 `_build_time_dialog` 交由 Qt sizeHint 自适应，sizeHint 198px 完整容纳时分秒）

---

## 待办汇总

| #   | 问题                                                | 优先级 | 状态   |
| --- | --------------------------------------------------- | ------ | ------ |
| 1   | 加速时间刷新频率随倍率变化（1/rate 秒节奏）         | P1     | 已修复（V0.4.7.1） |
| 2   | 倒计时日期选择器实时反馈（快捷按钮/日历点击即勾选） | P2     | 已修复（V0.4.7.1） |
| 3   | 时间选择框显示不全（分钟看不见）                    | P3     | 已修复（V0.4.7.1） |

---

## 4. 程序运行监控体系建议（P2，2026-08-09 记录，待实施）

- **背景**：部分错误控制台反应不出来（GUI 下 stderr 不可见、Qt 原生警告不进 Python 日志、后台线程异常静默、原生崩溃无栈）
- **现状**：已有 `utils/logger.py`（控制台 + logs/ 每日文件），实时跟踪：
  `Get-Content logs\app-YYYY-MM-DD.log -Wait -Tail 50`
- **建议落地三项**：
  1. `main.py` 加全局 `sys.excepthook` → `logger.critical(..., exc_info=True)`（Python 未捕获异常不再静默，Qt 槽内抛错可见）
  2. 加 `qInstallMessageHandler` → Qt 原生警告（QSS 解析失败等）转发进 logger
  3. 可选：`faulthandler.enable(file=...)` 崩溃栈落文件；崩溃码查 Windows 事件查看器（应用程序日志）
- **目标覆盖**：Python 异常 / Qt 警告 / 原生崩溃 三层全部进 `logs/`
- **状态**：已实施（V0.4.7.2：`utils/monitor.py` 三层监控，main.py 参数解析后装配；崩溃栈写 `logs/crash-YYYY-MM-DD.log` 并入同保留期清理）

---

## 5. UI 美化方案评估：PyQt-Fluent-Widgets（P3，2026-08-09 记录，待讨论）

- **背景**：用户询问 Qt 美化现成方案，GitHub 筛选（1 年内仍更新）后保留：PyQt-Fluent-Widgets（8058★，2026-08 更新）、PyQtDarkTheme（750★，2026-07）、QtTheme（158★，2026-01）；放弃：QDarkStyleSheet（2025-07 停更）、qt-material（2024）、GTRONICK/QSS（2023）
- **PyQt-Fluent-Widgets 评估结论**：
  - 优点：维护最活跃、Fluent 现代视觉上限最高、中文文档
  - 成本：是组件库非皮肤——需重写整个 UI 层（6 面板 + 对话框 + 托盘）；主题体系与现有 themes.py（QSS 模板 + ui.json 零硬编码管线）冲突；新增依赖与体积
  - 定位：属"换 UI 框架"级别决策，适合作为下一个大版本（UI 2.0）立项，可呼应未完成项"主题商店（F03c03）"；不适合在 P1-P4 修复清单中顺手做
- **建议路线**：短期不动；可先装官方 demo 看实际效果再决策；中期若走现代 UI 路线以它为基础做原型验证
- **备选低成本方案**：PyQtDarkTheme（轻量、跟随系统主题）或 QtTheme（纯 QSS 导出，配合现有 themes.py 管线）——效果提升小但成本极低
- **状态**：方向已定案（2026-09-10，T003）：**C. 维持现状**——已实际体验官方 gallery demo（PyQt6-Fluent-Widgets v1.11.3，独立临时 venv 验证：Python 3.14 + PyQt6 本机可运行，`[full]` 扩展开启亚克力特效；许可 GPLv3 非商业与本项目兼容；官方 demo 获取方式：仓库 PyQt6 分支 `examples/gallery/demo.py`）。**UI 2.0（Fluent Widgets）列为 1.0 大版本候选**，待功能项稳定后随 1.0 规划重新评估；低成本备选方案（PyQtDarkTheme）保留为可选尝鲜项，不排期

---

## 6. GUI 进程退出期硬崩溃（P3，2026-09-10 记录，待排查）

- **现象**：任何创建 `QApplication` 的 Python 进程（主程序、探针、pytest 内联 GUI 用例）在解释器退出阶段以 0xC0000409（Git Bash 回显 127）硬崩溃；崩溃点在事件循环结束后、进程退出前，**所有业务输出与日志在此之前均已正常完成**，功能不受影响
- **已排除**：非本次改动引入——T001 时用 `git stash` 对照实验证明基线代码同样崩溃；与运行监控（T002 faulthandler）无关（监控装配前即存在）
- **影响**：开发者体验问题（命令行 exit code 失真、无 flush 的 print 缓冲输出丢失）；pytest 主进程若内联创建 QApplication 会连带退出码污染
- **已用规避方案**（T004.2 落地）：GUI 断言类测试一律放子进程执行，以 stdout 标记断言结果（不依赖退出码）；探针脚本全部 `print(..., flush=True)`
- **排查方向（待定）**：PyQt6 版本与 Python 3.14 组合的退出析构顺序、QSystemTrayIcon/托盘图标析构、主题 QSS 残留对象
- **状态**：待排查（规避方案已固化，不阻塞功能与测试）
