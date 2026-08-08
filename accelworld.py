#!/usr/bin/env python3
"""
加速世界 - 兼容转发壳

保持 `python accelworld.py` 启动方式不变，实际逻辑已迁移至 main.py。
"""

from main import main

if __name__ == "__main__":
    main()


# ===== accelworld.py 函数/说明 =====
# 兼容转发壳：仅转发到 main.main()
#   设计理由：保持旧启动方式 `python accelworld.py` 可用，逻辑单一来源在 main.py
#   关联配置：无（纯转发）
