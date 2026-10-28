# 字体资产与许可

## MiSans（内嵌子集）

- **来源**：小米 MiSans 官方字体包（<https://hyperos.mi.com/font/>，2026-10 下载）。
- **文件**：`MiSans-Regular-sub.ttf` / `MiSans-Medium-sub.ttf` / `MiSans-Semibold-sub.ttf`。
- **子集**：fontTools 4.68 生成，覆盖 CJK 基本区、扩展 A、标点、拉丁与全角符号；供应用内嵌使用，减少二进制体积。
- **许可**：MiSans 字体知识产权许可协议（小米科技有限责任公司）——全球免费商用；**须在软件中特别注明使用了 MiSans 字体**；不得单独再分发、租赁或售卖字体软件本身。本目录仅作为应用内嵌资源，不单独分发字体包。
- **署名位置**：设置 → 关于 → 字体；本文件。

## Tabler Icons（内嵌子集）

- **来源**：npm `@tabler/icons-webfont@3.50.0`（<https://tabler.io/icons>），取 `dist/fonts/tabler-icons.ttf`。
- **文件**：`TablerIcons-sub.ttf`（29 个字形，约 9KB），码点表在 `rs/crates/accel-app/src/widgets/icons.rs`。
- **子集**：只保留导航/操作/天气/表单实际用到的字形：
  `pyftsubset tabler-icons.ttf --unicodes=U+ea53,U+ea5e,U+ea5f,... --no-hinting --desubroutinize --output-file=TablerIcons-sub.ttf`
- **许可**：MIT。以下许可文本随字体保留：

```text
MIT License

Copyright (c) 2020-2026 Paweł Kuna

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- **用途**：`widgets::icons::draw` 以字体绘制 24 网格图标；不单独分发图标库。

## Bahnschrift

- Windows 系统自带可变字体，运行期从 `C:/Windows/Fonts/Bahnschrift.ttf` 加载；**不随仓库分发**。
- 缺失时回退 egui 内置等宽族（应用仍可运行）。

## 参考

- MiSans 官方 FAQ / 许可协议：<https://hyperos.mi.com/font/zh/faq>、<https://hyperos.mi.com/font-download/MiSans字体知识产权许可协议.pdf>
