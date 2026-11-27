# 参与贡献

欢迎提供可重现的问题、翻译修订、兼容性实测和代码改进。

提交问题时注明助手版本、Claude Desktop 版本、系统与架构，说明下载／安装／制作副本／启动中的哪一步失败。日志和截图需要先隐去个人路径、账号及令牌。

翻译修订必须保留 ICU 占位符、标签、复数规则和变量名。不要批量覆盖未知键；请提供原文、译文及对应界面。新增平台支持需要明确区分合成测试、自动化构建和真实客户端验收。

代码提交前运行：

```sh
npm ci
npm run check
node --test tests/update-flow.test.cjs
cargo fmt --all -- --check
cargo test -p assistant-core --locked
```

文档修改请运行 `npm run site:build` 与 `npm run site:check`。不要提交下载缓存、构建产物、凭据和用户数据。代码与移植内容必须遵循 AGPL-3.0-only，并保留原作者署名。
