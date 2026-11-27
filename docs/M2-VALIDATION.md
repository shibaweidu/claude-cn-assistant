# M2 下载与校验开发记录

日期：2026-11-27。

## 实现

- GitHub 最新发布页重定向解析固定 tag，避免 API 匿名限流。只访问 Wangnov/claude-app-mirror 仓库及 GitHub 资产域名。
- 清单支持已观察到的 schemaVersion 2：version、sources、assetName、contentLength、sha256，releaseTag 可选但存在时必须一致。
- 同一 tag 获取清单、SHA256SUMS.txt 与安装包；先核对清单 SHA256，再核对每个安装包声明与校验和一致。
- 支持 Windows x64/arm64 MSIX、macOS Universal DMG、Linux x64/arm64 DEB。
- 元数据缓存 6 小时；手动最小间隔 60 秒；403/429 冷却。自动查询仅在程序运行且偏好开启时执行。
- 下载创建独立 UUID .part，校验大小和 SHA256 后重命名为完成文件。失败/取消清理该任务部分文件。连接失败最多额外重试两次，等待 2 秒、4 秒；不自动重试 403/429，不实现断点续传。
- 进度事件由真实字节产生，支持取消和跨页任务提示；无 HTTP Content-Length 也可使用已验证清单大小。
- 防重复任务、重启中断识别；已完成文件重启后为待验证状态，重新 SHA256 验证后才能定位文件。
- 下载缓存路径和资产名验证；不允许前端指定任意下载 URL 或目标目录。提交阶段与取消互斥。
- 仅下载，未执行安装。Windows 包签名/身份校验与系统安装器协调属于 M3，不将 SHA256 等同于发布者签名。

## 验证证据

- npm run check / npm run build：通过。
- cargo test -p assistant-core --offline：15 项通过（原 5 项与下载模块 10 项）。
- cargo clippy -p assistant-core -p claude-cn-assistant --offline -- -D warnings：通过。
- 桌面内嵌资源构建成功并启动，实际下载页显示镜像版本、平台、包大小、SHA256 与检查时间。
- 真实网络查询：成功解析 claude-app-v6.28899.6，清单及 SHA256SUMS 校验通过，5 项资产。
- 真实 Windows x64 包 312231751 字节：桌面首次尝试连接被远端重置；加入有限连接重试后命令层联调仍超时。无数据完成文件，明确记录 failed，界面显示错误而非成功。此环境下没有验证真实大安装包下载完成，不能宣称已通过。
- 成功传输/校验/重启复验在本地 HTTP 测试服务使用非可执行合成文件验证，避免反复消耗公益镜像带宽。
- 本地故障测试覆盖错误哈希、截断响应、取消清理、无 Content-Length、重复下载、429 冷却、缓存篡改、损坏已下载文件。

## 尚待

- 可达 GitHub 大文件的网络下验证完整真实安装包以及 UI 进度终态。
- 托管加速端点需真实公开仓库与实名 User-Agent；当前不伪造该标识，不访问其托管服务。
- 断点续传、HTTP-date 形式 Retry-After 精确处理、安装包签名校验与安装协调尚未实现。未提供数字秒数时采用默认冷却。
- 当前使用 Rust HTTPS 客户端；网络失败可打开镜像发布页手动获取，不绕过服务限制。

来源样本：tests/fixtures/mirror-manifest-v2.json 与 SHA256SUMS.txt 来自该固定 GitHub Release。示例联调命令 cargo run -p assistant-core --example mirror，添加 -- --download 才请求实际 Windows x64 安装包。
