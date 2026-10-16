# golden 夹具目录（PL011.04）

Python 0.6.0.0 行为快照，作为 Rust 侧 parity 断言的数据源（git 入库）：

- `time_vectors.json`：倍率/时间向量（rate 正常/边界/越界/脏值各口径）
- `config_behavior.json`：配置读写往返 / 损坏转存 / 脏值回退 / world_pins 脏值
- `weather_samples.json`：Open-Meteo 解析样本（正常 + 全部拒绝分支）
- `lunar_samples.json`：2026 全年农历逐日快照（PL012 消费）

再生物：`.temp/export_fixtures.py`（PL011.14 归档后随 Python 侧移入 `.archived/`），
任意机器可重复导出，双跑 sha256 必须一致（可再生性检验）。
