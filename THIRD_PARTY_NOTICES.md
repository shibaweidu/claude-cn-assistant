# 来源与依赖

本项目采用 AGPL-3.0-only，全文见 LICENSE。

## 设计与开发参考

CodexPlusPlus：https://github.com/BigPizzaV3/CodexPlusPlus ，AGPL-3.0-only。2026-11-27 从其 main 下载的源码快照中移植：

- crates/codex-plus-core/src/claude_ion_asar.rs → crates/assistant-core/src/ion_asar.rs，保留 ASAR 词表、语言协商、完整性计算及测试。来源文件 SHA256：4EA74D2C639BF18487E6D7D8D70B8B648DE56A362D7CF92DE2F135CE0D59B78B；移植后格式化。
- assets/claude-localization/catalogs/ 下简体中文词表与 overrides → assets/localization/catalogs/。
- assets/claude-localization/page-shim.js → assets/localization/page-shim.js。
- Electron fuse 处理参考 claude_localization.rs，独立实现副本检查与两处 fuse 修改，不引入完整宿主、语音、扩展、Computer Use 或网关。

本地源码通过压缩快照取得，未取得可验证的提交 ID，因此以获取日期和源文件哈希记录；不伪造来源提交。hook.cjs、launch.cjs 与副本任务协调为本项目编写，启动界面需要提示副本的签名和 fuse 变化。

## 下载来源

Wangnov/claude-app-mirror：https://github.com/Wangnov/claude-app-mirror 。免费开源社区镜像，非 Anthropic 官方运营。使用其 GitHub Releases，不访问尚未配置实名开源标识的托管加速端点。安装包及 Claude 商标属于各自权利人。

## 工程依赖

Tauri、Rust 依赖的精确版本见 Cargo.lock，前端依赖见 package-lock.json。Tauri API/CLI 采用 MIT 或 Apache-2.0；Vite、TypeScript、rfd、serde 等依赖许可证应在正式打包发布时随交付审计并保留对应声明。此文件不替代各依赖原始许可证。
