# Claude 中文助手

开源仓库：[shibaweidu/claude-cn-assistant](https://github.com/shibaweidu/claude-cn-assistant)。免费、非商业的 Claude 桌面汉化与镜像下载助手。

下载默认使用 Wangnov/claude-app-mirror 公益加速入口，携带指向本仓库的实名 User-Agent；下载页可手动切换 GitHub Releases 备用渠道。渠道切换会清空版本缓存，需要重新检查版本；不会中途切换正在下载的任务。加速入口仅提供最新版，清单/校验和或安装包哈希不一致时停止，不安装混合版本文件。实际速度取决于镜像路由和本地网络，不保证加速幅度。

苹果桌面应用风格的 Claude 下载、更新、汉化与恢复助手。暖白、陶土橙色系。已接入 Windows 桌面工程、安装检测、镜像下载、汉化与副本恢复，完整路线见 DEVELOPMENT_PLAN.md。

## 启动

需要 Node.js 20.19+、Rust、Visual Studio C++ Build Tools、Windows SDK 和 WebView2。

```sh
npm install
npm run desktop:dev
```

浏览器演示：`npm run web:dev`，访问 http://127.0.0.1:4173 。桌面开发会自动启动此服务，不要同时在同一端口运行两个服务。

检查：`npm run check`、`npm run build`、`cargo test -p assistant-core`。桌面生产构建：`npm run desktop:build`（当前 bundle 关闭，仅生成可执行文件）。

本机诊断：`cargo run -p assistant-core --example probe`，仅检测安装和读取本工具设置，不更改 Claude。

## 当前范围

- 概览、下载与更新、界面汉化、备份与恢复、偏好设置五个页面。
- 平台与架构选择、外观切换、偏好持久化、确认弹窗、汉化进度演示、演示备份恢复。
- 浏览器模式仍是演示状态。桌面模式使用 Rust 命令检测真实安装、产品版本、PE 架构、进程和资源布局，支持选择真实 Claude.exe、启动原版应用、打开数据目录。
- 桌面偏好原子写入 `%LOCALAPPDATA%/ClaudeCnAssistant/settings.json`。损坏或未来版本的设置报错并保留原文件，不覆盖。
- 桌面模式不运行模拟汉化或模拟备份；汉化页已接入 ASAR 副本制作与启动，备份恢复已接入：副本校验、恢复、中断处理及可恢复归档。重建偏好尚未自动执行；更新查询已按设定周期运行。
- 桌面版已支持 GitHub Release 固定版本查询、5 种平台/架构安装包、真实下载字节进度、取消、连接有限重试、SHA256 校验及失败后重新下载。下载完成后可打开所在目录；重启后先重新验证文件。
- 清单与校验和来自同一 Release；校验清单本身和安装包哈希。成功元数据缓存 6 小时，手动查询至少间隔 60 秒，403/429 冷却后再试。
- 自动检查仅在助手运行时生效。托管加速端点尚未启用，当前使用你指定的镜像项目 GitHub Release。不会自动安装下载的文件。
- Windows MSIX 已接入「检查并安装」：重验 SHA256、Windows 签名、固定包身份和本机架构，阻止降级及运行中安装；确认后交给系统应用安装程序，由用户在系统窗口完成安装。每 15 秒检测注册结果，10 分钟仍未确认则保留未确认状态。macOS/Linux 文件只保存。
- 「结束等待」不会取消 Windows 安装。助手不会自动提权、强杀进程或卸载已有普通安装。当前没有真实完整 MSIX 可用于安装正向验收，不能宣称已完成系统安装实测。

## 下载来源

安装包镜像来自免费开源社区项目 [Wangnov/claude-app-mirror](https://github.com/Wangnov/claude-app-mirror)，非 Anthropic 官方运营。该项目原样镜像官方安装包；汉化在本地副本单独进行。

后续直接接入托管端点前，按其 [Open Client Policy](https://github.com/Wangnov/claude-app-mirror/blob/main/TERMS.md#open-client-policy) 配置真实公开仓库及实名 User-Agent、元数据缓存与至少 30 分钟自动检查间隔、SHA256 校验。当前未虚构公开仓库标识。默认自动检查间隔为 6 小时，仅在助手运行时生效。

## 下一步

M2 与 M3 的核心及界面已实现，真实发布清单和本机包环境已验证；当前网络下完整真实安装包与系统安装正向流程仍待验证。细节见 docs/M2-VALIDATION.md 和 docs/M3-VALIDATION.md。M4 已接入 ASAR 汉化副本制作和启动注入，真实资源副本验证通过；完整启动待验收，见 docs/M4-VALIDATION.md。M5 备份恢复与中断处理已接入，见 docs/M5-VALIDATION.md。下一阶段为打包与完整端到端验收。当前不应当作为完整汉化器分发。
