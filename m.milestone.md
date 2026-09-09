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

## 版本 0.4.7.1 的 MileStone（实测问题修复，任务组 T001/T002）

1. 加速时间刷新频率随倍率变化（1/rate 秒节奏，P1）
2. 倒计时日期选择器实时反馈（快捷按钮/日历点击即勾选，P2）
3. 时间选择框显示不全修复（分钟可见，P3）
4. 运行监控体系三项落地（excepthook / qInstallMessageHandler / faulthandler，可选）

## 后续版本的 MileStone（方向待定稿，任务组 T003/T004，暂不定版本号）

1. UI 美化方向决策（PyQt-Fluent-Widgets / 低成本方案 / 维持现状）
2. 短期功能项：快捷键、倍率预设、进度条动画、托盘 toolTip 实时化
3. 中大型项候选（需 OpenSpec 提案流程）：多语言界面、PyInstaller 打包发布、自定义城市
4. 长期愿景（数据统计可视化、主题商店、云端配置同步等）：暂缓，等待版本 1.0 规划
