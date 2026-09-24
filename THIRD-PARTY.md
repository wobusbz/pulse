# 第三方组件 / Third-party components

Pulse 自身以 [MIT](LICENSE) 许可发布。仓库与发布产物中**捆绑**了以下第三方组件，
它们遵循各自的许可，再分发时请一并遵守。

本软件在运行时会加载以下二进制；发布包（安装包 / 便携包）中包含它们的副本。

## LibreHardwareMonitor

- 文件：`lib/LhmNative.dll`（内含 LibreHardwareMonitorLib，编译为 .NET NativeAOT）
- 用途：读取 CPU / GPU / 内存 / 硬盘 等传感器
- 许可：**MPL-2.0**
- 上游：<https://github.com/LibreHardwareMonitor/LibreHardwareMonitor>

> MPL-2.0 是文件级 copyleft：对本文件（及其修改）需继续以 MPL-2.0 提供源码。
> 若你修改了 `LhmNative.dll` 的源码，请一并公开对应源码。

## PawnIO

- 文件：`lib/PawnIO_setup.exe`（namazso 官方签名安装器）
- 用途：内核驱动，用于读取 CPU 封装温度（MSR）与内存 SPD 温度（SMBus）
- 许可：见上游（第三方签名驱动，**再分发前请确认其许可条款**）
- 上游：<https://pawnio.eu>

> LHM 0.9.6 通过 PawnIO 做 ring-0 访问；安装包仅在系统未安装时以管理员权限静默安装它。

## 其它 Rust 依赖

Rust crate 依赖的许可见 `Cargo.lock`；如需完整清单，可用
[`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) 或
[`cargo-about`](https://github.com/EmbarkStudios/cargo-about) 生成。
