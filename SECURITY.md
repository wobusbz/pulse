# 安全策略

## 支持的版本

只对**最新发布版本**提供安全修复。

## 上报漏洞

请**不要**开公开 issue。请使用 GitHub 的私密漏洞上报：

> 仓库 → **Security** → **Report a vulnerability**

请尽量附上：受影响版本、复现步骤、影响面，以及（如有）修复建议。

## 范围说明

- 本程序在系统**未安装** PawnIO 驱动时，会以**管理员权限**运行 `PawnIO_setup.exe -install -silent`
  来安装内核驱动。该驱动本身的安全问题请上报上游 <https://pawnio.eu>。
- `lib/LhmNative.dll` 由 LibreHardwareMonitor 构建（MPL-2.0），其问题请上报
  <https://github.com/LibreHardwareMonitor/LibreHardwareMonitor>。
- 本程序会读取系统传感器、注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
  与安装目录，不收集、不上传任何数据。
