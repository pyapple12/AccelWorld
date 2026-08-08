# 世界时钟时区表（GUI 时区下拉框使用）

# 常用时区列表（显示名, IANA 时区标识）
# 夏令时标注修正（E12）：巴黎/伦敦/纽约/悉尼含 DST，标两档偏移（标准/夏令时）
TIMEZONES = [
    ("北京 (UTC+8)", "Asia/Shanghai"),
    ("东京 (UTC+9)", "Asia/Tokyo"),
    ("首尔 (UTC+9)", "Asia/Seoul"),
    ("伦敦 (UTC+0/+1)", "Europe/London"),
    ("巴黎 (UTC+1/+2)", "Europe/Paris"),
    ("纽约 (UTC-5/-4)", "America/New_York"),
    ("洛杉矶 (UTC-8/-7)", "America/Los_Angeles"),
    ("悉尼 (UTC+10/+11)", "Australia/Sydney"),
]

# ===== data/timezones.py 函数/常量说明 =====
# TIMEZONES: list[tuple[str, str]]，世界时钟时区表（显示名, IANA 时区）
#   设计理由：时区表原为 GUI 内局部变量，外置后 GUI 与未来 CLI/配置层可复用
#   关联配置：无外部依赖，供 ui/main_window.py 使用
