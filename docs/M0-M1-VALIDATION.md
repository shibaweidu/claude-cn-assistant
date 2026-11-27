# M0/M1 开发与验证记录

日期：2026-11-27。当前交付为 Windows 本地开发版本，尚不是完整汉化器或安装包。

## 已实现

- Tauri 2 桌面宿主，保留现有暖白/陶土橙及深色界面。
- TypeScript 命令桥，Vite 构建，Rust 核心库。
- 实际普通安装目录及 MSIX 枚举、版本与 PE 架构读取、运行进程和资源布局检测。
- 安装文件选择、执行别名和不完整目录拒绝、多安装时不自动选定。
- 设置原子替换写入，未知 schema 和损坏文件保留并报错。
- 启动官方 Claude、数据目录打开及白名单项目链接命令。
- 桌面界面使用真实状态，不执行模拟汉化或模拟恢复。
- 浏览器演示与桌面功能分开，桌面错误不回退到假成功。

## 已执行检查

| 检查 | 结果 |
| --- | --- |
| npm run check | 通过 |
| npm run build | 通过 |
| cargo fmt --all --check | 通过 |
| cargo test -p assistant-core --offline | 5 项通过 |
| cargo clippy -p assistant-core -p claude-cn-assistant --offline -- -D warnings | 通过 |
| cargo build -p claude-cn-assistant --offline | 通过 |
| npm run desktop:build -- --debug --no-bundle | 通过，生成内嵌前端的可执行文件 |
| 原生桌面窗口 | 实际打开，正确显示本机安装 |
| 关闭 Vite 后独立运行 | 通过，文档地址为 http://tauri.localhost/ |
| 外观持久化 | UI 切换后文件记录 dark=true，重启后以深色打开；验证后恢复 dark=false |

实际安装探测：Claude 6.6395.0、x64、MSIX、ASAR 资源布局，检测时已有 Claude 进程运行。探测未修改该安装或用户配置。未为了测试重复启动正在运行的官方 Claude。

## 未完成和未验证

- 自动发布信息获取、下载、取消、SHA256 校验和安装器协调未实现。
- 汉化资产提取、受管副本、语言恢复、事务备份未实现。
- 手动文件选择和取消命令已实现，尚未覆盖全部路径组合的 GUI 实测。
- 普通安装、多安装共存、权限拒绝、干净账户和其他架构仍需实测。
- 未制作 NSIS 安装包、未代码签名、未对外发布仓库或 Release。
- 可执行文件使用 debug 编译，已嵌入前端，可脱离开发服务运行；正式发布仍需 release 构建和打包验收。

运行文件：target/debug/claude-cn-assistant.exe。开发窗口保持打开，供本地查看。
