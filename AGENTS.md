# AccelWorld 项目说明

单包 Python 桌面应用：基于加速倍率的时间膨胀时钟（PyQt6，中文界面为主）。无 lint/格式脚本、无构建步骤；单元测试用 pytest（tests/，131 用例）。2026-09-10 起接入 DeepTransHub 工作流体系（文档四件套 + Commit 提交规范 + `.agents/skills/` 项目级技能）。

## 运行与验证

- 入口 `main.py`：GUI 为默认模式，CLI 用 `--cli`；版本号单一来源在 `config/static/base.json`（`base["version"]`，当前 `0.5.3.2`），各模块（main.py --version/窗口标题/托盘 toolTip）一律从配置读取，代码中不得出现版本字符串
- **版本体系**（2026-09-10 切换）：自 `0.4.7.0` 起启用四段式纯数字 `X.Y.Z.P`（无 `ver ` 前缀）；历史存量 `ver 0.4x` 为旧三段式带前缀格式，仅存留于历史文档与提交记录，不回溯改写
- 没有测试/lint 命令。改动后验证：`.\.venv\Scripts\python.exe -c "import main, modules.time_dilation, modules.chinese_calendar, modules.weather_service, modules.alarm_service, config.settings, config.static.static_config, ui.main_window, ui.alarm_dialog, ui.audio_player, ui.system_tray, data.cities, data.timezones, data.weather_codes, utils.logger, utils.file_utils, utils.retry, interface, ui.tools.countdown_tools, ui.tools.clock_tools, ui.tools.alarm_text"`。不要直接跑 GUI 验证（会弹窗阻塞）
- GUI 无头初始化验证（不弹窗）：`$env:QT_QPA_PLATFORM="offscreen"; .\.venv\Scripts\python.exe -c "from PyQt6.QtWidgets import QApplication; from interface import AppInterface; from ui.main_window import AcceleratedWorldGUI; app = QApplication([]); w = AcceleratedWorldGUI(AppInterface()); print('GUI init OK')"`（进程退出码可能为已知退出期崩溃所污染，以 stdout 输出为准）
- `pyproject.toml` 仅有 basedpyright 配置，且绝大多数检查被显式放宽为 `"none"` —— 不要引入严格类型修复，也不要改动这些配置
- CLI 冒烟测试：`.\.venv\Scripts\python.exe main.py --version`、`main.py --cli --rate 2.0`
- 单元测试：`.\.venv\Scripts\python.exe -m pytest tests/ -v`（131 用例覆盖 time_dilation/chinese_calendar/settings/alarm_service/weather_service/file_utils/rate_presets/logger/monitor/static_config/gui_features/interface/countdown_tools/ui_tools；依赖 `tests/requirements-dev.txt` 的 pytest；GUI 断言类测试放子进程执行、以 stdout 标记断言——本机 GUI 进程退出期硬崩溃见 y.problems#6，勿在 pytest 主进程内联创建 QApplication；子进程配置隔离统一走 ACCELWORLD_CONFIG_FILE 环境变量，FIX001.12）

## 环境陷阱

- `.venv` 是机器绑定的：`pyvenv.cfg` 的 `home` 指向创建时机器的 Python 路径。换机器/换用户后解释器损坏，症状是 VSCode Python 扩展报 `write EPIPE / Shutting down server`（Jedi 语言服务器无法启动）。重建：`Remove-Item -Recurse -Force .venv; py -3.14 -m venv .venv; .\.venv\Scripts\python.exe -m pip install -r requirements.txt`，然后在 VSCode 重选解释器
- 依赖（`requirements.txt`）：PyQt6、lunar-python、chinese-calendar、pytz

## 结构与约定

- 包结构按依赖单向分层（UI 2.0 三大块，plan#UI2.0）：`interface/` 接口层（AppInterface 七域契约 + types 类型转出，无 Qt 依赖；UI 访问后端的唯一入口）→ 后端 `utils/` 通用工具（logger/file_utils/retry/dataclass_utils/monitor 运行监控，无业务依赖）+ `config/` 配置（settings 用户配置 + static/ 应用静态配置层，用户配置存项目内 `config/user_config.json`，日志存项目内 `logs/app-YYYY-MM-DD.log` 每日独立文件、崩溃栈 `logs/crash-YYYY-MM-DD.log`）+ `modules/` 业务核心（time_dilation 时间膨胀、chinese_calendar 农历/干支/节气、weather_service Open-Meteo 天气、alarm_service 闹钟）+ `data/` 静态数据（cities/timezones/weather_codes）→ `ui/` 界面（Fluent Widgets：main_window 主窗口装配器 FluentWindow 六导航页、panels/ 7 面板含设置页、tools/ 展示运算纯函数、backdrop.py Acrylic 背板、system_tray 托盘、alarm_dialog 闹钟对话框、audio_player 音频；**ui/ 零后端 import，一切经 interface**）
- 代码零硬编码原则：业务参数（倍率范围/默认值/定时器周期/窗口几何/字体颜色/日志路径等）全部从 `config/static/` 的 json 读取（`get_static_config()` 单例，映射表 config.json 由 static_config.py 的 `__file__` 自定位——唯一结构约定）；用户配置默认值经 `default_factory` 从 base.json 现取
- **自绘圆弧留边**（2026-09-12 定案）：自绘圆/圆弧与控件裁剪边界相切时，抗锯齿会把切点处的圆弧量化成平边（125% 缩放等非整数 DPR 下更明显）。自绘圆形/圆弧元素的控件须比可见图形四周各大 ≥1px 透明边距，定位偏移同步计入——参照 `ui/glass_card.py` CapsuleSlider 的 `TRACK_PAD` 模式（杆子/橙槽/旋钮三层同心几何）与 `ui/system_tray.py` 托盘图标的 2px 边距；玻璃卡自身圆角为容器轮廓、属固有贴边，不适用
- `main.py` 收编 CLI/GUI 分发与版本读取；模块间顶层 import，不要使用函数内延迟 import
- 提交信息规范见下文「Commit 提交规范」节；功能开发先走 OpenSpec 提案流程
- 工作流文档四件套（2026-09-10 接入 DeepTransHub 工作流体系）：`w.study.md` 项目分析报告 / `x.progress.md` 任务清单（已完成在前、未完成在后；审计修复组 `FIX{NNN}`）/ `y.problems.md` 已知问题 / `z.plan.md` 方案记录与审计附录（含豁免定案清单，附录 `A{NNN}` 递增）；另有 `m.milestone.md` 版本里程碑清单
- 项目级 Agent 技能在 `.agents/skills/`（audit-project 全量审计 / audit-report 审计归档 / progress-task 按任务组执行 / skillforge 技能创建），已 gitignore 仅本地使用
- 历史规划文档 `workingboard/` 已归档至 `archived/workingboard`（gitignore）

## 工程原则（设计哲学，所有项目通用）

> 设计哲学总纲（18 条 / 5 大类，与用户级 instructions.md 同源）；下文"代码规范"为项目细则，两者冲突时以本项目边界为准。

### 核心思想

- 以第一性原理思考问题：理解需求背后的真实目标，而非直接套用已有模式或技术方案。
- 优先解决本质问题，避免为假设中的未来需求提前设计复杂系统。
- 在保证长期可维护性的前提下，选择当前最简单、可靠、清晰的实现方案。

### 简洁与设计

- 遵循 KISS：优先选择简单直接的实现，避免不必要的复杂度。
- 遵循 DRY：避免重复逻辑，但不要为了消除少量重复而创建过度抽象。
- 遵循 SOLID 思想：职责清晰、降低模块耦合，提高可维护性和扩展能力。

### 架构

- 不长期保留废弃方案：优先删除过时代码，而不是增加兼容层、fallback 或临时迁移逻辑。
- 不进行未经验证的架构设计：避免提前引入抽象、配置和间接层。
- 从最小可工作的版本开始逐步演进，每次修改建立在已有可运行系统之上。
- 永远不要用未来可能需要的复杂性，牺牲当前产品的可用性。

### 代码质量

- 保持模块职责明确，避免一个模块承担过多职责。
- 优先使用成熟、稳定、维护良好的第三方库，而不是重复造轮子。
- 使用项目已有依赖解决问题之前，不要随意新增依赖。
- 在引入新方案前，先检查已有代码、依赖、文档和能力。
- 避免为了"看起来更优雅"而增加实际复杂度。

### 工程决策

- 优先选择长期可维护的方案，而不是只能临时运行的解决方案。
- 代码应该服务于业务目标，而不是为了展示技术复杂度。
- 如果简单方案已经满足需求，不要主动升级为复杂方案。

## 代码规范

### 函数注释规则

- 每个函数定义下方紧跟 `#` 注释，说明该函数的用途和核心逻辑（1-3 行）
- **禁止使用 docstring（三引号字符串）替代 `#` 注释**——函数/类/模块文档统一走 `#` 注释体系，docstring 不承担注释职责；单行 docstring 当注释用属于违规（`.temp/verify_s11.py` 自动检测）
- 每个 `.py` 文件末尾必须有完整的函数逻辑说明区，用 `# =====` 分隔，涵盖文件中所有函数/模块级常量：
  - 输入、输出、逻辑步骤
  - 设计理由（为什么这样做）
  - 异常处理说明
  - 关联的配置或外部依赖

### 代码约定

- 注释必须用 `#`，禁止 `//` 或其他语言注释符号；所有注释使用中文
- 命名风格：函数/变量用 `snake_case`，类用 `CamelCase`，常量用 `UPPER_CASE`
- `_` 前缀：函数名前加 `_` 表示模块内部私用，如 `_format_tokens()`，外部模块不应直接调用
- `def main()`：每个可独立运行的脚本都有 `main()` + `if __name__ == "__main__": main()`
- 类型注解：优先使用 Python 类型注解，包括 `typing` 模块和 `| None` 语法
- dataclass：配置聚合优先用 `@dataclass`
- import 顺序：标准库 → 第三方库 → 本地模块，每组之间空行分隔
- f-string：字符串格式化优先用 f-string，避免 `.format()` 或 `%`
- 推导式：优先用列表/字典推导式而非手写 for 循环构建集合
- 布尔值判断：用 `if x:` / `if not x:` 而非 `if x == True:` / `if x is False:`
- 空值判断：用 `if x is None:` / `if x is not None:` 而非 `if x == None:`
- 异常捕获：避免裸 `except:`，至少用 `except Exception:`，指定具体异常类型更好；捕获多个异常类型可用 `except (Exc1, Exc2):`
- 行长度：每行尽量不超过 100 字符（超过时在运算符或逗号后换行）
- 空格约定：逗号后加空格、冒号前不加空格（切片冒号两侧不加）、赋值/比较运算符两侧加空格、函数定义前后各空两行、类定义前后各空两行、方法之间空一行
- 字符串引号：普通字符串用双引号，文档字符串用 `"""` 三引号；f-string 内含大量双引号时允许外层使用单引号
- 路径处理：强制使用 `pathlib` 代替 `os.path`
- 临时文件：所有临时生成的脚本/文件必须写入项目根目录下的 `.temp/` 文件夹（已 gitignore）
- **文件修改必须用 edit 工具**：修改既有文件（.py/.md/.json）一律用 edit 的精确 oldString/newString 替换，禁止用 python -c 或 PowerShell 脚本做内容替换（易踩引号/缩进坑）；新建 .temp 探针脚本不受限
- **版本单一来源**：版本号只存 `config/static/base.json` 的 `version` 字段，其他处一律 `get_static_config().base["version"]` 引用，禁止第二处硬编码；版本格式见「运行与验证」节（四段式 X.Y.Z.P）
- **全量回归输出**：`.\.venv\Scripts\python.exe -m pytest tests/ -v` 的结果行全量显示，禁止 `tail`/`Select-Object -First/-Last` 截断——截断会掩盖前段 FAIL（2026-09-10 约定）

## Commit 提交规范

- **标题行**：`<type>: V<版本>，<摘要>`——版本号与 `config/static/base.json` 的 `version` 一致（四段式全写，如 `V0.4.7.0`）；摘要一句话概括核心
- **版本递增**：`feat`/`fix`/`refactor`/`perf` 提交前先 bump version（patch 位 +1）；`docs`/`test`/`style`/`chore` 不强制
- **type 全集**（conventional 风格）：`feat` 新功能 / `fix` 修复 / `refactor` 重构（行为不变）/ `perf` 性能 / `docs` 文档 / `test` 测试 / `style` 格式 / `build` 构建依赖 / `ci` CI / `chore` 杂项 / `revert` 回滚
- **正文**（改动大时可选）：`- ` 列表，按主题分组、分组下子条目缩进，每个功能块一行自然中文描述，每行 ≤ 100 字符
- **禁止项**：内部编号（A0.1/B1.5/FIX1.1/T001.2 等任务编号）、验证/回归数字（如"全量回归 44 项通过"）、英文混排描述
- **提交范围**：一个版本的所有连带改动一次提交（源码 + 配置 + 文档同步）
- **流程**：由 AI 根据 `git status`/`git diff` 核对清单并草拟 commit 内容（git add 清单 + message）→ 用户审阅后自行执行 `git add`/`git commit`/`git push`（AI 不执行 git 写操作）

## Git 注意

- `.gitignore` 忽略 `CLAUDE.md`、`openspec/`、`.vscode/`、`.venv`、`archived/`、`.opencode/`、`.agents/`、`.temp/`、`.history/`、`.mimosa/` —— 对这些文件的修改不会出现在 `git status` 中；`AGENTS.md` 与工作流文档四件套（w/x/y/z/m）已纳入版本控制
- **`config/user_config.json` 每次提交必须入库**（2026-09-11 用户定案）：该文件虽为运行时用户配置，但已纳入版本控制且未 ignore，历次提交均须 `git add` 一并入库，不得以"运行时状态"为由排除

## 操作注意

- 执行命令前先检测当前 shell（Windows 下为 pwsh）：**搜索统一用 `rg`**（ripgrep 已安装，自动遵循 .gitignore 排除 .venv/.history，中文 UTF-8 正常；如 `rg -n "pattern" --type py`，统计匹配数用 `rg -c`），无 rg 环境时回退 `Select-String`；目录操作使用 PowerShell 兼容命令（`Get-ChildItem` 替代 `ls` 等），避免 Linux-only 工具
- pwsh 会话带 `-NoProfile` 不加载 `$PROFILE`，输出中文前必须先设置编码：`[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;`
- 未经用户明确要求，不得擅自执行 `git add`、`git commit` 或任何其他 Git 写操作
- **视觉设计不得向测试环境妥协**（2026-09-12 定案）：offscreen/无头探针仅作功能与结构参考；不得因 offscreen 渲染缺陷（如渐变栅格化随机崩溃）降级、简化或改动桌面端设计——桌面端一律按原设计实现；视觉与材质的最终实测以用户桌面走查为准
