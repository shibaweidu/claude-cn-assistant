# 公益加速接入

2026-11-28 接入。客户端公开仓库为 https://github.com/shibaweidu/claude-cn-assistant ，源码和 AGPL-3.0-only 许可证已公开。

- 默认访问 https://claudeapp.agentsmirror.com/latest/manifest、/latest/checksums 和对应平台公开安装包入口。
- User-Agent：ClaudeCnAssistant/0.1.0 (+https://github.com/shibaweidu/claude-cn-assistant; open-source-noncommercial)。
- 跟随公开路由返回的 HTTPS 存储重定向，不硬编码底层存储地址。拒绝明文、IP literal、本地主机、带用户凭据和非 443 端口跳转。
- 自动检查六小时，手动操作有冷却；403/429 不伪造请求特征或更换子域继续请求。
- 加速仅提供 latest；清单 SHA256、安装包声明哈希与最终文件 SHA256 必须一致。发布切换导致不一致时停止，由用户重新检查版本。
- 下载页可切换 GitHub Releases 备用渠道；切换清空元数据，活动任务禁止切换。
- 接入验证：真实加速清单查询成功，返回 6.28899.6，五项资产，清单校验通过；Windows x64 实际数据传输成功，取样下载约 5 MB 后停止，未做完整 298 MB 完成校验实测。当地网络仍有波动，不声称已解决所有慢速问题。
- 34 项核心测试通过，1 项原有 macOS 本机测试忽略；类型检查、Rust clippy 和桌面构建通过。

断点续传与速度/预计时间显示尚未实现。前期文档中“未启用加速入口”的记录属于历史状态，以本文和当前代码为准。
