# 字体资产与许可

## MiSans（内嵌子集）

- **来源**：小米 MiSans 官方字体包（<https://hyperos.mi.com/font/>，2026-10 下载）。
- **文件**：`MiSans-Regular-sub.ttf` / `MiSans-Medium-sub.ttf` / `MiSans-Semibold-sub.ttf`。
- **子集**：fontTools 4.68 生成，覆盖 CJK 基本区、扩展 A、标点、拉丁与全角符号；供应用内嵌使用，减少二进制体积。
- **许可**：MiSans 字体知识产权许可协议（小米科技有限责任公司）——全球免费商用；**须在软件中特别注明使用了 MiSans 字体**；不得单独再分发、租赁或售卖字体软件本身。本目录仅作为应用内嵌资源，不单独分发字体包。
- **署名位置**：设置 → 关于 → 字体；本文件。

## Bahnschrift

- Windows 系统自带可变字体，运行期从 `C:/Windows/Fonts/Bahnschrift.ttf` 加载；**不随仓库分发**。
- 缺失时回退 egui 内置等宽族（应用仍可运行）。

## 参考

- MiSans 官方 FAQ / 许可协议：<https://hyperos.mi.com/font/zh/faq>、<https://hyperos.mi.com/font-download/MiSans字体知识产权许可协议.pdf>
