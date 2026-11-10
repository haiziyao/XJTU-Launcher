# 第三方代码与素材说明

本项目以 MIT 许可证开源（见 `LICENSE`）。以下文件包含来自其他开源项目的派生代码或素材，按各自许可证分发；其余素材见文末。

## XJTU-PPT-Capture-Extension

- 来源：<https://github.com/MeowSaint/XJTU-PPT-Capture-Extension>
- 版本：v3.1.0（初版按 v2.5.2 移植，2026-11-06 对齐到 v3.1.0）
- 许可：MIT License，Copyright (c) 2026 MeowSaint and contributors
- 引入日期：2026-11-05

原项目是 Chrome 扩展。本项目只移植其截取与导出算法，用 TypeScript 重写以适配本项目的插件页面、组件库与 Tauri IPC；Chrome 专有部分（`permissions`、`tabs`、`webRequest`、`declarativeNetRequest`、`getDisplayMedia`、请求嗅探）没有移植。

派生文件：

| 文件                                                                       | 对应能力                                                                     | 修改说明                                                                                                      |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `src/features/plugins/capture/frames.ts`                                   | 192×108 采样、逐通道去偏、灰度签名、翻页 / 相似 / 空白判定                   | 重写为纯函数；阈值改为具名常量；保留上游“空白帧不覆盖上一张”的规则                                            |
| `src/features/plugins/capture/scanner.ts`                                  | 三种扫描方式、0.5 秒补查、0.25 秒确认、疑似过渡段合并、去重顺序、断点续扫    | 从上游扫描循环提取为带 hooks 的纯逻辑，视频读写由调用方注入；新增 `checkpoint` 回调用于落盘                   |
| `src/features/plugins/capture/seek.ts`                                     | 跳转等待、超时与帧就绪                                                       | 新增的类型化封装（`SeekTimeout`、`requestVideoFrameCallback` 回退），供扫描循环使用                           |
| `src/features/plugins/capture/naming.ts`                                   | 课程名、时间戳与文件命名                                                     | 按上游 `课程-YYYY-MM-DD-HH-mm-ss.pdf`、`001-HH-mm-ss.jpg` 规则重写；时间固定为东八区                          |
| `src/features/plugins/capture/export.ts`                                   | 手写 PDF（JPEG `/DCTDecode`，原始版与 A4 讲义版）与 ZIP（store 方式、CRC32） | 重写为无依赖的 ByteWriter；去除上游的下载与文件系统调用，只返回字节；ZIP 时间戳固定为 1980-01-01 以保证可复现 |
| `src/features/plugins/capture/store.ts` + `src-tauri/src/plugins/capture.rs` | 会话、页码、进度与图片的本地保存                                             | 本项目自行实现（按账号隔离的缓存目录），仅沿用上游的会话字段与续扫语义                                        |

`src/features/plugins/PptCaptureView.vue` 是本项目按现有界面组件新写的页面，不含上游代码；后端解析接口同样为本项目实现。

### MIT License

```
MIT License

Copyright (c) 2026 MeowSaint and contributors

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

## 其他素材

- 内嵌 Noto Sans SC 字体：SIL Open Font License 1.1，许可证文本见 `src/assets/fonts/OFL.txt`。
- 桌面宠物精灵图：来源与条款见 `src/assets/pets/NOTICE.md`，不适用 MIT，仅限非商业使用。
- 校园协议参考：[XJTUToolBox](https://github.com/yan-xiaoo/XJTUToolBox)（GPL-3.0）。本项目按其公开行为独立用 Rust 重新实现，未复制其源码。
