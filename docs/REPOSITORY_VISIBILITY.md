# 仓库与搜索可见度维护

已采用产品名称「Claude Desktop 汉化助手」，保留仓库地址 `shibaweidu/claude-cn-assistant`，避免破坏镜像 User-Agent、历史链接和下载来源标识。

## 已落地

- GitHub About：中英双语描述、GitHub Pages 主页、12 个相关 Topics；不使用不符合功能的 Claude Code 标签。
- README：产品定位、预览版范围、截图、平台选择、三步使用、FAQ、数据目录、源码构建及许可证。
- 文档站：七个静态页面，独立标题与描述、canonical、Open Graph、结构化数据、内部导航、sitemap.xml 和 robots.txt。
- Pages 工作流：校验本地链接、元数据和 sitemap 后部署。无需前端 JS 才能读取正文，不依赖外部字体。
- 预览发布：Windows x64、Apple Silicon、Intel Mac；三平台测试与构建全部成功后生成 SHA256SUMS.txt 并发布 GitHub prerelease。
- 反馈入口、贡献指南、更新记录、AGPL 和来源声明。

## 后续维护

1. 通过具有网站管理权限的 Google Search Console / Bing Webmaster Tools 账号添加站点并完成验证。可用的具体验证方式以平台当前界面为准；若采用 HTML 验证文件或 meta 标签，将平台给出的精确值加入站点，不生成假的验证码。
2. 站点地址：`https://shibaweidu.github.io/claude-cn-assistant/`；sitemap：`https://shibaweidu.github.io/claude-cn-assistant/sitemap.xml`。
3. 在站长平台检查索引覆盖、抓取错误、搜索曝光、查询词与点击率。站点可访问和提交 sitemap 都不等于已收录，也不能保证搜索排名。
4. 发布新版本时更新 CHANGELOG、平台要求、兼容实测和已知限制。兼容性证据需要区分自动化测试、包结构检查和真实客户端界面验收。
5. 有真实使用反馈后，再按目标社区或 Awesome 列表规则提交教程。不要自动群发、刷 Star、购买外链或批量生成同质页面。

## 验证命令

```sh
npm run site:build
npm run site:check
npm run check
node --test tests/update-flow.test.cjs
```

## 参考

- [参考项目的展示与教程](https://github.com/javaht/claude-desktop-zh-cn)
- [GitHub 仓库搜索字段](https://docs.github.com/en/search-github/searching-on-github/searching-for-repositories)：默认搜索名称、描述和 Topics；README 可以通过 in:readme 搜索。
- [GitHub Topics](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/classifying-your-repository-with-topics)

搜索流量需要持续维护。以真实功能、清晰教程和有效下载入口满足用户需求，不承诺关键词排名。
