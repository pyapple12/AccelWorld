# golden 夹具目录（PL011.04）

Python 0.6.0.0 行为快照，作为 Rust 侧 parity 断言的数据源（git 入库）：

- `time_vectors.json`：倍率/时间向量（rate 正常/边界/越界/脏值各口径）
- `config_behavior.json`：配置读写往返 / 损坏转存 / 脏值回退 / world_pins 脏值
- `weather_samples.json`：Open-Meteo 解析样本（正常 + 全部拒绝分支）
- `lunar_samples.json`：全年农历逐日快照（十字段 + 信息串 + 节气日 ±1）
- `data_tables.json`：城市/时区/天气码三表快照
- `festival_derive.json`：节日派生三年份快照（PL014 消费）
- `manifest.json`：各夹具 sha256 清单

再生物：`.archived/export_fixtures.py`（PL011.14 归档后随 Python 版移入 `.archived/`），
任意机器可重复导出、双跑 sha256 必须一致（可再生性检验，对照 manifest.json）。

> 消费纪律：core 行为断言一律对夹具（不依赖运行环境）；夹具变更须同步 manifest 与导出脚本。
