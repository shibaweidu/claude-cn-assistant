# M3 Windows 安装协调开发与验收

日期：2026-11-27。实现代码已接入，真实官方 MSIX 正向安装仍待验收。

## 实现

1. 只有下载管理器记录的完整文件可进入安装检查；非 Windows MSIX 拒绝进入安装。
2. 检查前和确认后重验 SHA256；锁定文件期间禁止其他进程写入或删除，再核对哈希与 Windows 签名。
3. 读取 MSIX AppxManifest.xml，不解压安装包到用户路径。限制清单大小、禁止 DTD 和外部 XML 解析。
4. 名称固定为 Claude，Publisher 固定为本机已观察到的 Anthropic, PBC 官方发布身份；签名要求 Windows Get-AuthenticodeSignature 返回 Valid，签名者 X500 身份与清单一致。身份/签名不明时停止，不以哈希替代签名信任。
5. 核对清单版本与下载版本、本机原生架构；拒绝更低版本。运行中的 Claude 要求用户正常退出，不强杀。普通安装保留；不自动卸载或迁移数据。
6. 系统安装器通过 MSIX 文件关联打开。命令与路径分开传递，不默认提权。打开系统窗口不视为安装完成。
7. 以 Get-AppxPackage 实际注册版本、固定发布身份、包家族 Claude_pzs8sxrjxfjjc、架构及健康状态 Ok 确认目标版本或更高版本已存在。
8. 等待期间每 15 秒检查，10 分钟后仍未达到目标则为未确认；检测失败提供重试。结束等待不会取消系统安装，不把用户关闭安装器推断成已取消或失败。
9. 安装状态保存到 install-state.json，重启后进行中的流程显示 interrupted，不凭旧记录继续安装。手动重新检测可确认结果。
10. 安装检查、安装启动与新下载通过协调锁互斥。安装成功后刷新本机快照；已有 managed 目录时标记需要重建，但不执行尚未实现的汉化。
11. Windows PowerShell 子进程仅使用系统模块目录，避免继承 PowerShell 7 的 PSModulePath 后错误加载签名模块。

## 检查与证据

- npm run check、npm run build：通过。
- cargo test -p assistant-core --offline：21 项通过。
- cargo clippy -p assistant-core -p claude-cn-assistant --offline -- -D warnings：通过。
- cargo fmt --all --check：通过。
- npm run desktop:build -- --debug --no-bundle：通过，内嵌前端资源。
- 只读 Windows 安装环境探测：本机 x64、Claude 正在运行、注册版本 6.6395.0.0、包家族及发布者与固定身份一致。
- 合成未签名 MSIX：真实 Zip/XML/Windows 签名探测返回非 Valid，安装策略拒绝；使用中文与空格临时路径测试通过。未调用系统安装。
- 单元用例覆盖错误身份、签名不可信、签名者不一致、架构错误、版本不一致、运行中、降级、注册版本不足、包状态不健康、无已验证下载、重启中断恢复及重复流程拒绝。

## 当前限制

- 上阶段真实大文件下载在本网络超时；当前没有完整官方 MSIX，未完成签名 Valid 正向验证、真实打开系统安装器、用户执行安装及安装后成功确认的端到端实测。
- 未更改或安装本机 Claude。未关闭正在运行的 Claude，不以开发任务为由重装用户应用。
- 系统外部安装器无法可靠提供用户取消/失败事件，因此明确显示结果未确认，交由用户查看系统错误；不伪造精确结果。
- 系统签名校验受本机证书信任、网络、Windows SIP 支持影响。超时或 UnknownError 均停止流程，需要可达网络与真实官方包验证；不绕过校验。
- 固定发布者身份若随官方证书更新发生变化，需要审查后更新兼容配置，不能自动接受新身份。
- 仅按本机原生架构提供安装支持，ARM64 上 x64 仿真安装不在当前支持范围。
- 尚未接入受管汉化副本；needsRebuild 为未来模块预留状态。

只读联调：cargo run -p assistant-core --example install_probe，可追加 MSIX 文件路径以检查包，不会打开安装器。
