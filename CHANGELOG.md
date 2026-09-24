# Changelog

本项目的所有重要变更都记录在此。
格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [0.1.0] - 2026-09-25

### Added

- 任务栏部件：CPU / 内存 / 硬盘 的**使用率**与**温度**（2 行 × 3 列）
- 详情面板：CPU、GPU（使用率 / 温度 / 显存）、内存（使用率 / 温度）、硬盘（活动率 / 温度 / 读写）
- 面板内置「开机自启动」开关（读写 `HKCU\...\Run`）
- 面板可拖拽标题移动；失去焦点自动关闭
- 传感器初始化期间部件显示「正在初始化…」，且不响应点击
- NSIS 安装包（按平台命名 `Pulse-<版本>-windows-x64-Setup.exe`）、便携包与单独 exe
- 安装包随包携带 PawnIO 驱动安装器，系统未安装时以管理员权限静默安装
- GitHub Actions：CI（fmt / check / clippy / test / build）与 Release（打 tag 触发打包发布）

[Unreleased]: https://github.com/wobusbz/pulse/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/wobusbz/pulse/releases/tag/v0.1.0
