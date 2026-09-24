# 贡献指南

感谢参与！下面是本项目的环境、构建、测试与提交约定。

## 环境

- Windows 10/11 x64
- Rust stable（≥ 1.88，edition 2024）
- 打包安装包需要 [NSIS](https://nsis.sourceforge.io/)（`makensis`）

## 构建

```bat
cargo build --release     :: 产物 target\release\Pulse.exe
installer\build.bat       :: 产物 installer\Pulse-<版本>-windows-x64-Setup.exe
```

`installer\build.bat` 会依次查找 `C:\Program Files (x86)\NSIS\makensis.exe` 与 PATH 中的 `makensis`。

## 测试与检查

提交 PR 前请在本地跑一遍：

```bat
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

硬件相关测试需要装有 PawnIO 驱动与真实传感器，默认被忽略：

```bat
cargo test test_lhm -- --ignored --nocapture
```

## 代码约定

- 展示层放 `src/ui/`；业务与平台相关代码放 `src/internal/`。
- 不要引入裸颜色 / 裸尺寸；UI 一律使用 `cx.theme()` 与 rem 尺度助手。
- 新增依赖请说明理由，并确认许可（见 [`THIRD-PARTY.md`](THIRD-PARTY.md)）。

## 提交 / PR

- 一个 PR 聚焦一件事，避免顺手重构无关代码。
- 提交信息说明**为什么**，而不只是「改了什么」。
- UI 变更请附截图（任务栏部件 + 详情面板），并说明浅色 / 深色主题下的表现。

## 许可

任何贡献均视为同意以本项目的 [MIT](LICENSE) 许可发布。
