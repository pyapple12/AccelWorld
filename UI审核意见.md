# AccelWorld UI 审核意见

> **2026-10-28 复审更新**：本文件为 2026-10-27 的上一轮评审记录；其“**不建议完全重新设计**”的结论已被 `UI审核意见_20261028.md` 撤回（本轮基于六页实拍与三档窗口实测，判定需要 PL017 结构性重排）。本文件其余证据与条目保留作历史对照，不再作为当前设计依据。

> 生成：2026-10-27 · 评审基线：HEAD `7083456`（V0.6.1.8），文中行号以该版本为准
> 方式：**只读静态评审**——读代码与设计令牌，未构建、未运行；本文件是本次评审对仓库的唯一写入
> 评审标准：Anthropic frontend-design 技能 + taste-skill 应用界面量规（`app-screens.md`，源自 23 个真实产品屏实测）+ gamedev `game-ui-ux` / `game-feel` 跨引擎纪律技能 + kinetics 弹簧参数 + icon assets 选型决策树
> 局限：taste-skill `critique.md` 要求"看截图评、不看代码评"——本文是代码层可判的部分；正式三轮截图评审的流程见 §五

---

## 一、总体结论

UI 骨架**已达到设计技能的核心要求，多项超出**：星图身份独特（不落任何"AI 模板脸"——非奶油底衬线、非近黑荧光绿、非 SaaS 卡片流）、令牌体系完整（ui.json 五段：colors/scale/spacing/radius/layout）、对比度有实测矩阵兜底（文本 ≥4.5:1、图形 ≥3:1）、"含义唯一原则"是"结构即信息"的更严格版、`icons.rs` 矢量描线早已解决字符图标跨设备不一致问题。

**不建议完全重新设计。** 星图身份就是该花大胆的地方（frontend-design："把大胆花在一处，其余安静克制"）；重设计大概率落回更平庸的方案，且要重写六页自绘控件。建议做一次**定向精修**：修 1 个 P0 显示缺陷 + 补 4 个缺口（字体默认性、动效层、图标扩编、状态覆盖/文案），可在一个 V0.7 主题内消化。

---

## 二、已达标项（复核确认，保持现状）

| 量规要求                                   | 现状证据                                                   |
| ------------------------------------------ | ---------------------------------------------------------- |
| 颜色只上数据不上 chrome（accent 份额极低） | accent 唯一用色纪律（`theme.rs:62`）；语义色五层按角色命名 |
| 表面扁平、1px 发丝线分离、零阴影           | 全实色无透明无 GL（`theme.rs` 说明区）                     |
| 数字等宽 / tabular figures                 | `digits.rs` 槽宽制，强于 tabular-nums                      |
| 产品签名元素（only this product has）      | 星盘主仪单环双针（`pages/clock.rs`，UIR4.6 定案）          |
| HUD 事件驱动、禁止轮询                     | 命令/事件总线推送 + Tick 快照，无每帧读状态                |
| 导航退后（active 非彩色强调）              | 侧栏比窗底深一档（sidebar_bg `#0B0F19`）                   |
| 图标零字体依赖                             | `widgets/icons.rs` 16 个矢量描线，线宽随尺寸缩放           |
| 字号地板                                   | meta 12px ≥ 11px house floor                               |
| 布局随容器自适应（非绝对像素）             | `dial_diameter` 钳 [200,440] + `hero_fit_scale` 收号       |

---

## 三、修改意见（按优先级）

### P0（立即）

1. **星盘金针角度计算错误**——显示级，hero 元素在 rate>1 时大部分时间失效。
   - 位置：`pages/clock.rs:24-38` `day_fraction` 按 86400 秒（24h）归一并 `clamp(0,1)`；而 `types.rs:15` 明确 `custom_time` 小时可超 24（rate=2.0 时日长 48h）。小时 ≥24 后分数恒 1.0 → **金针死钉表盘正上方**；rate=20 时加速日约 96% 的时间如此，"金针甩开灰针"的核心卖点不成立。
   - 修法（二选一，属设计决策）：按 `(custom_hour×3600+分×60+秒)/(day_hours×3600)` 归一（加速日进度语义，金针每现实日 rate 圈）；或小时对 24 取模（保持 24h 表盘语义）。现有构件够用：`types.rs:42` `custom_hour()` + `TimeInfo.day_hours`。
   - 必须补测：>24h 与三位数小时用例——现有 `day_fraction_cardinals` 单测只锚定 0..24h，正是缺陷漏出处。

### 高（小改动，建议本版本内）

2. **补状态覆盖四态**【taste-skill §2.6/§11 硬要求：每块 empty/zero/loading/error】
   - countdown / alarm / world 空列表补一句行业话术空态（行动邀请，如"还没有闹钟——右上角 + 新建"），不留白；
   - 天气加拉取中指示；
   - 错误态改为**保留旧值 + "X 前缓存 · 刷新失败"角标**——当前 `WeatherFailed` 清 UI 旧数据（`bus.rs:115-119`）+ 缓存仅内存，断网刷新一次即整页空白且重启不恢复。
3. **破坏性操作加确认/撤销**【taste-skill house rule】：删除闹钟现为 Trash 直删；至少点名确认或提供 undo。
4. **纯键盘走查六页**【game-ui-ux 焦点导航】：拔鼠标走完 clock/countdown/world/weather/alarm/设置——每屏打开是否有初始焦点、Tab 序是否合理、自绘控件（slider/icon_button）是否注册 egui 焦点、焦点高亮是否可见；修补缺口。
5. **设最小窗口尺寸**【game-ui-ux 缩放策略】：表盘下限 200px，更小的窗口会挤压失控；设 eframe `min_window_size` 或做窄窗降级。

### 中（建议作为 V0.7"动效与反馈"主题）

6. **ui.json 增设 motion 令牌段 + 设计哲学一行**【kinetics + awesome-design-md 结构】：现五段已对齐 DESIGN.md 结构，缺 motion 段；文件顶部加一行"为什么"（参考 Apple DESIGN.md 的 description 字段做法）。
7. **三级动效**【game-feel 事件重要性分层 + kinetics 参数】：
   - **large（唯一的编排时刻）**：闹钟触发——应用内表盘 accent 描边闪一次 + 金针短促脉冲（hit-stop 0.08s 量级）；现仅系统 toast + 铃声；
   - **medium**：抽屉开合 0.3–0.4s 过冲滑入（spring bouncy 220/14，或 `cubic-bezier(0.18, 1.25, 0.4, 1)`）；
   - **small**：chip / icon_button hover 0.15s ease-out（spring soft 170/26）、按压下沉 0.06s（Push Button 配方）。
   - 铁律：夸张瞬时并回静息态、不阻塞输入、绝不给常规动作加、提供"减少动效"开关。
   - 实现：egui 无 tween 引擎，手写 spring lerp；动画窗口期（约 0.5s）内局部 `request_repaint_after(16ms)` 提频，静止后回到现有 ≤200ms/1s 节奏——**零忙转纪律不破**。
8. **应用内 toast**（egui-toast / egui-notify，后者带出入场动画）：统一保存失败/天气失败等应用内反馈，与系统通知分层。
9. **组件引入（克制清单）**：`egui_plot` 替换手绘温度曲线（顺带满足"图表无 chrome：网格线 3–7 条淡线、单序列无图例"）；`egui_flex` 仅当抽屉页布局复杂化再引。引入前 `cargo add` 验证与 egui 0.37 兼容并过四道 CI 门。

### 中（图标与字体）

10. **图标扩编统一**【icon assets 决策树"非网页 → 抠 path 数据存静态 .svg"，正中 egui 情形】：
    - 短期：不引库，把现有 16 图标统一到光学网格（等效 24 网格 / 一致线宽 / 圆角端点；云系很讲究，Hourglass 相对拼凑），补编辑/确认/定位/风等缺口；
    - 中期：引入 Lucide 体系（2px 描线圆角端点，与现有手绘语言同源）——`icon assets\pqoqubbw-icons\icons\*.tsx` 内嵌即 Lucide path 数据（或 lucide-static），配 `egui_extras` 的 `svg` feature（resvg 栅格化，tint 走 currentColor 方案）。
11. **正文字体**：Microsoft YaHei 是"任何项目都会伸手去拿的默认字体"（frontend-design 明令避免）；换 MiSans / HarmonyOS Sans SC 子集嵌入（`window.rs:798-838` 已有字体注入设施）；**Bahnschrift 数字保留**——DIN 仪表气质与星盘契合，是有意的好选择。

### 低（记录性核查）

12. **黛蓝地面偏离存档**：23 屏深色应用实测地面全为中性黑（#000/#111/#171719，"No navy and no slate"）；`#0F1420` 黛蓝是有意偏离（星图身份 + 对比度矩阵兜底）——把这条辩护写进 ui.json 注释，成为正式设计决策而非讨论记录。
13. **开关 on 态用色核查**【taste-skill §8.1】：on 色应为语义绿或近黑而非品牌 accent——若现用雄黄金，则与"accent = 当前值/主操作"的语义纪律冲突（开关不是当前值）。
14. **状态不只靠颜色**【game-ui-ux / 色盲安全】：闹钟启用态、danger 等除色相外补形状/位置冗余。
15. **文案**【frontend-design writing】：错误态按"说清发生了什么 + 怎么办，不道歉不含糊"重写；空态写成行动邀请。

---

## 四、刻意不套用的量规（避免误读）

- app-screens"应用界面数字不超 48px"：针对 productivity dashboard；本应用 hero 72 是时钟本体（信息本身），远观可读性要求大——不适用。
- 安全区（刘海/overscan）、触摸目标 ≥9mm、手柄屏幕栈：桌面鼠标键盘应用——不适用。
- 图表/表格大节：本应用表格极少，仅温度曲线一条适用（见第 9 条）。

---

## 五、后续验证（建议流程）

1. 修 P0 金针（<10 行 + 单测）；
2. 允许构建后跑自带 `ACCELWORLD_UI_SHOT` 探针出 6 页截图；
3. 按 taste-skill `critique.md` 正式三轮：14 遍看图法 → 评分（design 40% / usability 30% / creativity 20% / content 10%，含 cap 规则）→ 每轮修 top3 → 复照；
4. tokens 微调（motion 段落地）→ 试点 weather 页（最复杂）→ 推广全页；
5. 每步过 4 道 CI 门（fmt / clippy -D warnings / test / cargo tree 分层）。

---

## 附：量规来源

- taste-skill `references/app-screens.md`（23 真实产品屏实测规则）/ `references/critique.md`（评审方法与评分）
- gamedev `game-ui-ux`（焦点导航/缩放/对比度 ≥4.5:1、Xbox 无障碍 101/102）/ `game-feel`（反馈分层/缓动即生命/克制铁律）
- kinetics（spring 预设：snappy 400/32、soft 170/26、bouncy 220/14；Push Button / Toast Overshoot 配方）
- Anthropic frontend-design（反模板校准、文案原则、质量底线）
- icon assets（非网页环境取图标决策树）
