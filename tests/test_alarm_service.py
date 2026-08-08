# 闹钟模块测试（S9.7 测试引入）
# 覆盖：构造校验、重复/一次性触发、跨天去重、上限、容错、编辑保留 ID、预设铃声辅助

import datetime

from modules.alarm_service import Alarm, AlarmManager, PresetSound


def _alarm(label="测试", time="07:00", **kwargs):
    # 构造闹钟辅助（默认一次性，created_at 固定今天以便触发验证）
    alarm = Alarm(label=label, time=time, **kwargs)
    alarm.created_at = datetime.datetime(2026, 8, 8, 6, 0, 0).isoformat()
    return alarm


def test_invalid_time_raises():
    # 非法 time 构造抛 ValueError（__post_init__ 校验）
    for bad in ("25:99", "abc", "07:00:00:00"):
        try:
            Alarm(label="x", time=bad)
            raise AssertionError(f"非法时间 {bad} 未拒绝")
        except ValueError:
            pass


def test_repeat_weekday():
    # 重复闹钟按星期匹配：周一触发、周日不触发
    m = AlarmManager()
    alarm = _alarm(repeat_days=[0, 1, 2, 3, 4])  # 工作日
    m.add_alarm(alarm)
    assert len(m.check_alarms(datetime.datetime(2026, 8, 3, 7, 0))) == 1  # 周一
    assert len(m.check_alarms(datetime.datetime(2026, 8, 8, 7, 0))) == 0  # 周六


def test_one_time_only_today():
    # 一次性闹钟仅创建当天触发（S8.4 回归）
    alarm = _alarm()  # created_at 2026-08-08
    assert alarm.should_trigger_on(datetime.datetime(2026, 8, 8, 7, 0)) is True
    assert alarm.should_trigger_on(datetime.datetime(2026, 8, 9, 7, 0)) is False


def test_cross_day_trigger():
    # 跨天去重：周一触发后周二仍触发；同分钟不重复（S8.1 回归）
    m = AlarmManager()
    m.add_alarm(_alarm(repeat_days=[0, 1, 2, 3, 4, 5, 6]))
    assert len(m.check_alarms(datetime.datetime(2026, 8, 3, 7, 0))) == 1
    assert len(m.check_alarms(datetime.datetime(2026, 8, 4, 7, 0))) == 1
    assert len(m.check_alarms(datetime.datetime(2026, 8, 4, 7, 0))) == 0


def test_max_alarms():
    # 闹钟上限来自静态配置（max_alarms=10，S9.5 回归）
    m = AlarmManager()
    assert m.max_alarms == 10
    for i in range(10):
        assert m.add_alarm(_alarm(label=f"a{i}", time=f"{i:02d}:00"))
    assert not m.add_alarm(_alarm(label="extra", time="23:00"))


def test_duplicate_rejected():
    # 同时间同标签拒绝
    m = AlarmManager()
    assert m.add_alarm(_alarm(label="起床", time="07:00"))
    assert not m.add_alarm(_alarm(label="起床", time="07:00"))
    assert m.add_alarm(_alarm(label="起床", time="08:00"))  # 不同时间可加


def test_from_dict_tolerant():
    # 反序列化容错：非法 time 返回 None、未知键过滤（S8.2 回归）
    assert Alarm.from_dict({"label": "x", "time": "99:99"}) is None
    good = Alarm.from_dict({"label": "x", "time": "07:00", "extra": 1})
    assert good is not None and good.label == "x"
    # 混合列表加载：非法条目跳过
    m = AlarmManager()
    m.from_dict_list(
        [
            {"label": "合法", "time": "07:00"},
            {"label": "非法", "time": "bad"},
            None,
        ]
    )
    assert len(m.alarms) == 1


def test_replace_keeps_id():
    # 编辑场景整体替换保留 ID（S4 回归）
    m = AlarmManager()
    original = _alarm(label="旧名", time="07:00")
    m.add_alarm(original)
    updated = _alarm(label="新名", time="08:00")
    updated.id = original.id
    assert m.replace_alarm(updated)
    assert m.get_alarm(original.id).label == "新名"
    assert len(m.alarms) == 1


def test_preset_helpers():
    # 预设铃声辅助方法（S9.6 回归）
    assert PresetSound.CLASSIC.display_name == "Classic"
    assert PresetSound.CHIME.display_name == "Chime"
    assert PresetSound.CLASSIC.index() == 0
    assert PresetSound.from_index(2) is PresetSound.BEEP
    assert PresetSound.from_value("CLASSIC") is PresetSound.CLASSIC
    assert PresetSound.from_value("不存在的") is PresetSound.CLASSIC  # 兜底
