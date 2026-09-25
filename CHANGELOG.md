# Changelog

本项目的所有重要变更都记录在此。
格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### Fixed

- **拖动详情面板导致进程静默崩溃**。拖动标题时用 `SendMessageW` 同步启动了 Windows 的模态移动循环，
  而它是在 GPUI 事件回调内执行的（此时 `App` 仍被可变借用）。模态循环抽消息时跑到了面板的 1 秒刷新定时器，
  其 `Entity::update` 去 `AppCell::borrow_mut()` 撞上未释放的借用，panic `RefCell already borrowed`；
  release 是 `panic = "abort"`，因此表现为无提示崩溃（事件日志 `0xc0000409` / `FAST_FAIL_FATAL_APP_EXIT`）。
  改用 `PostMessageW`，把模态循环推迟到回调返回、借用释放之后进入。

### Added

- panic 会写入 `%LOCALAPPDATA%\Pulse\panic.log`（位置 + 消息）。此前 release 同时是 `panic = "abort"`
  和 GUI 子系统（无控制台），panic 信息完全丢失，崩溃无从回报。

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
