# 断点续传与启动修复

2026-11-28：加入部分文件保留、暂停与续传。同版本/文件名/大小/SHA256 复用任务 ID，根据部分文件实际大小请求 Range；存在强 ETag 时加 If-Range。206 响应严格核对 Content-Range 起点、终点和总大小；ETag 冲突拒绝拼接。200 响应安全截断重下。最终完整 SHA256 不匹配丢弃损坏文件。续传由用户点击继续下载触发，尚未自动无限重试。

旧程序主动删除的部分文件无法恢复。服务不支持 Range 时仍需重下。没有 ETag 时仍请求 Range，但最终完整 SHA256 是文件是否可用的必要条件。

Claude 6.6395.0 真实启动日志确认 ReferenceError: require is not defined。改用 process.getBuiltinModule 及 module.createRequire 加载汉化 hook，真实测试返回页面注入确认 1，受管 Claude 进程正常运行。此结果验证启动注入成功，不等于所有页面文本已逐项验证。

现有副本在启动前校验核心文件收据，再更新内置启动脚本，不重新复制客户端。完整副本路线来自上游补丁 Desktop：上游也是复制应用，改副本 Electron fuse、本地资源包与启动注入。另一条完整功能路线依赖 ClaudeMom，不是简单词表替换。

验证：37 项测试通过、1 项原有本机 macOS 测试忽略；TypeScript 检查、Rust clippy 和内嵌桌面构建通过。新增真实本地 HTTP 续传、If-Range、200 重置与错误 Content-Range 用例。真实公益镜像的完整大文件断点恢复仍需网络环境下验收。
