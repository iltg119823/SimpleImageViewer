# JPEG / RAW 选片版（预发布）

基于 z16166/SimpleImageViewer 3.1.6，保留 GPL-3.0 许可。

- **Mac M 系列芯片**：下载 `SimpleImageViewer-aarch64-apple-darwin.zip`，解压后将 `.app` 拖到「应用程序」。目标 macOS 11 或更新版本；实际最低可运行版本仍需真机验证。
- **Windows 10 x64**：下载 `SimpleImageViewer-x86_64-pc-windows-msvc.zip`，完整解压后运行 `SimpleImageViewer.exe`，保留同目录中的运行库。
- `SHA256SUMS.txt` 提供文件校验值。

默认只看 JPEG，数字键 0–5 评分；同目录同名 RAW 共用评分。支持 ≥、等于和任意星级组合筛选，后台联动复制到 `jpgHHMM` / `rawHHMM`，不覆盖源图和已有导出。

**评分只在当前会话保存，退出后清除。** 导出完成前请保持程序运行。详细规则见包内 `CULLING_CN.md`。

本版没有 Apple 开发者证书公证，也没有 Windows Authenticode 签名；系统可能显示来源提示。macOS 包仅做本地签名（ad-hoc）。两个平台通过 CI 构建及核心测试后才发布，但这不替代 Windows 10 和 Apple Silicon 真机验收。
