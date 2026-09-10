# 闹钟展示文案工具（PL001.07 自 alarm_panel/alarm_dialog 迁入，plan#UI2.0 运算出面板）
# 重复规则文案与铃声文件名文案的单一来源

import os

# 星期字符表（重复闹钟显示用，模块级常量避免每次调用重建，E6 语义保留）
_WEEKDAY_CHARS = ["一", "二", "三", "四", "五", "六", "日"]


def format_repeat_display(repeat_days: list) -> str:
    # 空列表显示"一次"，否则按星期数字映射拼接（如 [0,1] → "周一二"）
    if not repeat_days:
        return "一次"
    return "周" + "".join(_WEEKDAY_CHARS[d] for d in repeat_days)


def format_sound_button_name(path: str) -> str:
    # 铃声文件名文案：📁 前缀 + basename 截断 15 字符（列表声音标签与对话框按钮共用）
    return f"📁 {os.path.basename(path)[:15]}"


# ===== ui/tools/alarm_text.py 函数/常量说明 =====
# _WEEKDAY_CHARS: 星期字符表（0-6 → 一~日），模块级常量（原 alarm_panel 私有常量迁入）
# format_repeat_display(repeat_days) -> str
#   输入：重复星期数字列表（0-6，空=一次性）；输出：展示文案
#   设计理由：文案规则单一来源，列表行与未来其他展示点共用（plan#UI2.0 铁律 1）
#   异常处理：无（索引由 Alarm.__post_init__ 规范化保证 0-6 范围）
# format_sound_button_name(path) -> str
#   输入：铃声文件完整路径；输出：📁 前缀截断文案（长文件名保留前 15 字符）
#   设计理由：alarm_panel 声音标签与 AlarmEditDialog 按钮原文案完全一致，合并去重（DRY）
#   异常处理：无（os.path.basename 对空串/非法路径不抛）
#   关联配置：无（纯函数）
