# AccelWorld 项目说明

单包 Python 桌面应用：基于加速倍率的时间膨胀时钟（PyQt6，中文界面为主）。无测试套件、无 lint/格式脚本、无构建步骤。

## 运行与验证

- 入口 `main.py`：GUI 为默认模式，CLI 用 `--cli`；版本常量 `VERSION` 单一来源在 `main.py`（当前 `ver 0.45`），其他模块用 `from main import VERSION` 引用
- 没有测试/lint 命令。改动后验证：`.\.venv\Scripts\python.exe -c "import main, modules.time_dilation, modules.chinese_calendar, modules.weather_service, modules.alarm_service, config.settings, config.static.static_config, ui.main_window, ui.alarm_dialog, ui.themes, data.cities, data.timezones, data.weather_codes, utils.logger, utils.file_utils, utils.retry"`。不要直接跑 GUI 验证（会弹窗阻塞）
- GUI 无头初始化验证（不弹窗）：`$env:QT_QPA_PLATFORM="offscreen"; .\.venv\Scripts\python.exe -c "from PyQt6.QtWidgets import QApplication; from ui.main_window import AcceleratedWorldGUI; app = QApplication([]); w = AcceleratedWorldGUI(); print('GUI init OK')"`
- `pyproject.toml` 仅有 basedpyright 配置，且绝大多数检查被显式放宽为 `"none"` —— 不要引入严格类型修复，也不要改动这些配置
- CLI 冒烟测试：`.\.venv\Scripts\python.exe main.py --version`、`main.py --cli --rate 2.0`
- 单元测试：`.\.venv\Scripts\python.exe -m pytest tests/ -v`（41 用例覆盖 time_dilation/chinese_calendar/settings/alarm_service/weather_service；依赖 `tests/requirements-dev.txt` 的 pytest）

## 环境陷阱

- `.venv` 是机器绑定的：`pyvenv.cfg` 的 `home` 指向创建时机器的 Python 路径。换机器/换用户后解释器损坏，症状是 VSCode Python 扩展报 `write EPIPE / Shutting down server`（Jedi 语言服务器无法启动）。重建：`Remove-Item -Recurse -Force .venv; py -3.14 -m venv .venv; .\.venv\Scripts\python.exe -m pip install -r requirements.txt`，然后在 VSCode 重选解释器
- 依赖（`requirements.txt`）：PyQt6、lunar-python、chinese-calendar、pytz

## 结构与约定

- 包结构按依赖单向分层（参考 DeepTransHub）：`utils/` 通用工具（logger/file_utils/retry/dataclass_utils，无业务依赖）→ `config/` 配置（settings 用户配置 + static/ 应用静态配置层，用户配置存项目内 `config/user_config.json`，日志存项目内 `logs/app-YYYY-MM-DD.log` 每日独立文件）→ `modules/` 业务核心（time_dilation 时间膨胀、chinese_calendar 农历/干支/节气、weather_service Open-Meteo 天气、alarm_service 闹钟）→ `ui/` 界面（main_window 主窗口、alarm_dialog 闹钟对话框、themes 主题 QSS）→ `data/` 静态数据（cities/timezones/weather_codes）
- 代码零硬编码原则：业务参数（倍率范围/默认值/定时器周期/窗口几何/字体颜色/日志路径等）全部从 `config/static/` 的 json 读取（`get_static_config()` 单例，映射表 config.json 由 static_config.py 的 `__file__` 自定位——唯一结构约定）；用户配置默认值经 `default_factory` 从 base.json 现取
- `main.py` 收编 CLI/GUI 分发与 `VERSION`；模块间顶层 import，不要使用函数内延迟 import
- 提交信息用中文 conventional 风格并带版本号，如 `feat: V0.43，M07完成，添加日期时间选择器...`；功能开发先走 OpenSpec 提案流程
- 项目规划文档在 `workingboard/` 目录，重构进度在 `x.progress.md`，重构方案在 `z.plan.md`

## 代码规范

### 函数注释规则

- 每个函数定义下方紧跟 `#` 注释，说明该函数的用途和核心逻辑（1-3 行）
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

## Git 注意

- `.gitignore` 忽略 `CLAUDE.md`、`openspec/`、`.vscode/`、`.venv`、`archived/` —— 对这些文件的修改不会出现在 `git status` 中；`AGENTS.md` 已纳入版本控制

## 操作注意

- 执行命令前先检测当前 shell（Windows 下为 pwsh）：使用 PowerShell 兼容命令（`Select-String` 替代 `grep`，`Get-ChildItem` 替代 `ls` 等），避免 Linux-only 工具
- pwsh 会话带 `-NoProfile` 不加载 `$PROFILE`，输出中文前必须先设置编码：`[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;`
- 未经用户明确要求，不得擅自执行 `git add`、`git commit` 或任何其他 Git 写操作
