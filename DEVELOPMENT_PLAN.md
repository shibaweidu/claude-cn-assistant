# Claude 中文助手开发计划

日期：2026-11-27。基于当前 UI 原型与本地保存的 CodexPlusPlus 上游源码。本文是实施计划，未将规划中的功能标记为已实现。

## 1. 首版目标与边界

交付可安装的 Windows x64 桌面应用，完成「检测安装 → 获取原版安装包 → 安装 → 制作中文副本 → 启动 → 检查更新 → 重建副本 → 恢复」流程。

汉化对象为 Claude Desktop，包括其本地界面及可适配页面；不包含终端 Claude Code CLI 汉化。延续现有苹果桌面应用风格和陶土橙色系。

首版支持 Windows 普通安装目录及 MSIX/商店安装检测。Windows ARM64 下载可支持，但在取得真机验证前，不声明完整汉化兼容。macOS、Linux 下载入口保留；macOS 汉化与 Linux 汉化不计入首版承诺。下载其他平台安装包不等于支持在当前系统安装。

首版不引入 API 网关、账号中转、插件市场、会话管理、语音或 Computer Use。工具自身自动安装更新放在第二期，首版提供新版本检查与发布页入口。

## 2. 已完成与环境事实

- 已有五个可交互页面：概览、下载与更新、界面汉化、备份与恢复、偏好设置。
- 当前使用 HTML/CSS/JavaScript；无前端框架依赖，演示状态保存在 localStorage。
- 当前 server.mjs 仅是浏览器预览服务器，不是桌面后台。
- 安装状态、汉化进度及备份均为演示；下载按钮打开镜像 GitHub 发布页。
- 已确认本机可调用 Node.js、npm、Rust、Cargo，检测到 VS Installer 与 WebView2 路径；C++ 编译组件、Windows SDK 仍需通过实际构建核实。
- 上游汉化模块依赖 settings、paths、诊断日志、扩展计划及 ClaudeMom 的 claude-local。不能整文件复制后直接编译。

## 3. 技术决策

采用 Tauri 2 + Rust 核心库，前端保留当前原生实现，逐步加入 TypeScript 和 Vite。保留现有 UI，不为接入桌面后台迁移 React。

前端只负责展示、输入和事件订阅；网络、文件系统、子进程和状态持久化归 Rust。浏览器预览使用显式 mock adapter，桌面使用 Tauri adapter，界面清楚区分两者。桌面连接失败必须报错，不回退到模拟成功。

不依赖整个 codex-plus-core 或 ClaudeMom。仅移植本项目所需的汉化资产和资源包处理逻辑；安装探测与设置层独立实现。

建议结构：

```text
claude-cn-assistant/
├─ src/                       # 页面、组件、类型与桥接适配器
│  ├─ bridge/                 # tauri / mock 实现同一接口
│  ├─ pages/
│  └─ state/                  # 快照、任务订阅及选择器
├─ src-tauri/                 # 桌面窗口、权限、命令注册
├─ crates/assistant-core/src/
│  ├─ detection/              # 路径、版本、架构、进程
│  ├─ releases/               # 发布信息、缓存、渠道适配
│  ├─ downloads/              # 流式下载、取消、SHA256
│  ├─ installation/           # 调用系统安装、结果复检
│  ├─ localization/           # 副本、词表、ASAR、启动
│  ├─ recovery/               # 收据、日志、备份与恢复
│  └─ settings/               # 原子写入、版本迁移
├─ assets/localization/
├─ tests/fixtures/            # 合成目录与合成 ASAR
└─ docs/                      # 接口契约、兼容记录与验收
```

应用数据放在独立的本地应用数据目录，细分 settings、downloads、managed、backups、transactions、logs。日志不采集对话、账户凭据或完整用户配置。

## 4. 实施阶段与退出标准

### M0：工程与依赖准备

任务：

1. 保存现有原型基线，创建独立 Git 仓库与开发说明，不默认推送远程。
2. 确定新项目包名、应用标识与数据目录；更新旧项目环境变量和路径前缀。
3. 加入 Tauri 2、Vite、TypeScript，建立 core crate 与最小命令桥。
4. 保留 web:dev 预览入口及显式演示模式；桌面默认连接真实命令。
5. 配置本地资源 CSP、最小 Tauri capabilities、外部链接域名白名单。
6. 增加窗口拖动及真实窗口控制，移除装饰性伪窗口按钮；关闭窗口时正确处理后台任务。
7. 建立上游归属记录。移植 AGPL 代码时沿用 AGPL-3.0-only，并保留版权和修改说明。

退出标准：npm 检查、前端生产构建、cargo check 通过；本机能打开独立桌面窗口，五个页面保持既有设计。实际验证 C++/SDK/WebView2 构建链。

### M1：真实本地状态与设置

任务：

1. 检测普通安装目录、MSIX/商店安装、用户选择的路径；区分应用执行别名和真实 Claude.exe。
2. 多个安装并存时列出候选，由用户明确选择；不悄悄切换来源。
3. 读取产品版本、架构、资源布局、运行进程与所需运行时状态。
4. 设置迁移到 Rust 持久化层，以临时文件加替换写入；单独保存版本号以支持后续迁移。
5. 前端移除桌面模式中的固定版本号和默认已安装状态，增加检测中、未安装、权限不足、检测超时状态。
6. 实现选择安装目录、打开数据目录、启动官方版、打开来源项目等基础命令。

退出标准：不存在安装时不显示已安装；MSIX 别名不会作为源目录复制；用户可选路径；重启助手后设置仍保留；探测超时可恢复。

### M2：发布信息与下载

任务：

1. 为 Wangnov/claude-app-mirror 实现独立 release provider，验证真实 release-manifest.json 结构，固定测试样本；不凭 README 示例推断所有字段。
2. GitHub Release 渠道获取精确 tag、平台文件及同一 Release 校验和。下载开始后固定版本，不能让安装包与校验和来自不同版本。
3. 预留托管镜像 provider；公开仓库与 User-Agent 配置满足来源条件后，使用其公开入口。开发阶段通过本地样本测试全部状态。
4. 流式下载到 .part 文件，展示真实字节数、总量和阶段。未知总量时使用不定进度，不制造百分比或速度。
5. 支持取消、超时和手动重试；有限指数退避；首版不要求断点续传。失败部分文件不作为可安装文件使用。
6. 同版本校验和匹配后原子转为完成文件，执行安装前再做必要校验。Windows 包另核对签名及包身份。
7. 元数据默认缓存 6 小时，展示最后检查时间；手动检查也设最短冷却，限流按 Retry-After 处理。后台检查只在助手运行时执行，不承诺关闭后调度。

退出标准：平台及架构匹配正确；哈希不符不可安装；取消后能重新下载；中断下载不显示完成；403/429 给出可操作错误；元数据过期或离线不误报最新版。

### M3：Windows 安装与更新协调

任务：

1. 将已校验 MSIX 交给系统安装程序；必要时提供显式安装命令，但不默认自行提权。
2. 下载其他系统安装包仅提供保存，不允许调用本机安装。
3. 安装前检查 Claude 运行状态，提示用户正常退出；不默认强杀进程。
4. 安装程序启动后进入等待安装状态，复检真实包版本；区分启动安装器、安装成功、用户取消和安装失败。
5. 安装成功后如存在旧汉化副本，标记需要重建，连接后续汉化任务。
6. 明确官方版更新由安装器完成；不能承诺商店版自动更新暂停，也不能保证系统允许官方包降级。

退出标准：关闭安装器不算成功；新版本检测成功才继续汉化；失败保留原汉化副本。系统策略阻止侧载时给出明确原因。

### M4：汉化核心提取与真实启动

任务：

1. 从上游提取词表、必要启动脚本与 claude_ion_asar.rs；移除扩展、听写、Computer Use、bash timeout 等需求外逻辑及相关注入。
2. 将安装路径、设置、目录、进程和日志适配为本项目接口，去掉 codex-plus-core 与 claude-local 依赖。
3. 支持目录形式的 ion-dist 与 ion-dist.asar。记录资源布局、来源版本、资产哈希及补丁版本；资源结构不认识时停止，不强行替换。
4. 从官方源复制到 staging，写入词表并应用必要补丁，验证成功后替换 managed，先保留旧副本。
5. 基于实际文件工作量报告阶段进度。补丁校验、替换提交阶段不能中途取消，其余长阶段可取消。
6. 实现必要的 Electron 启动注入：仅作用于受管副本，调试入口限制在本机且任务结束后关闭，超时清理子进程。
7. 决定运行时交付：优先随包携带经校验的私有 Node 运行时；首个联调版可先检测 Node 22+，生产安装包不得默默依赖开发机 Node。
8. 语言设置仅备份和修改相关字段；启动失败且进程已退出后再恢复，避免覆盖 Claude 正在写入的配置。
9. 启动中文版后检查真实进程及注入确认信息；不能以创建子进程成功替代汉化成功。真实窗口做人工中文覆盖验收。

退出标准：未修改官方安装目录；成功启动中文版；不支持的新布局明确失败且旧副本可用；失败清理 staging；关闭汉化后可按原语言启动。

### M5：备份、恢复与崩溃续接

任务：

1. 每次替换写入事务记录：planned、staged、verified、committed。程序重启检查中断事务并提供恢复。
2. 备份记录 source version、补丁版本、路径、时间、校验信息、相关语言字段；目录大小以实际读取为准。
3. 恢复前停止相关任务并检查进程，验证备份可用后切换，不使用复制失败的备份。
4. 语言恢复只修改工具拥有的字段，不覆盖其他新配置；用户在工具外修改过语言时提示冲突。
5. 默认保留最近两份成功副本备份，磁盘不足时提供用户可见的清理入口；不得自动清理官方目录和用户数据。
6. 区分「回退汉化副本」与「降级官方客户端」；首版只实现前者。旧副本与新版用户数据是否兼容需实测，不能保证任意跨版回退。

退出标准：在 staging、校验和替换阶段模拟退出后可恢复；源目录始终保留；恢复不回滚对话；损坏备份不可使用。

### M6：打包与首版验收

任务：

1. 生成 Windows x64 安装包，验证干净账户首次启动与卸载行为；无签名构建如实标明签名状态。
2. 在普通版、MSIX/商店版、多安装共存、中文与空格路径环境验证流程。
3. 测试离线、连接中断、哈希错误、磁盘不足、文件占用、权限不足、配置损坏与资源布局不兼容。
4. UI 验证深浅外观、键盘访问、125%/150% 显示缩放、弹窗焦点、任务跨页持续显示、错误及恢复入口。
5. 加入助手版本检查和发布页入口；自动安装助手更新、签名 manifest 与发布流水线留待第二期。
6. 写安装说明、支持范围、已验证版本、来源说明、第三方许可和日志导出方式。

退出标准：交付实际运行的 Windows 程序、安装包、验收记录和已知限制。只通过构建不能代替实机启动与流程验收。

## 5. 前后端接口与任务模型

前端入口建议：

| 命令 | 行为 |
| --- | --- |
| get_app_snapshot | 读取安装、汉化、任务及缓存状态 |
| update_settings | 校验并保存偏好 |
| select_source_installation | 保存用户选择的真实源安装 |
| check_release | 获取/刷新发布信息 |
| start_download / cancel_task | 启动下载 / 请求任务取消 |
| install_downloaded_package | 校验完成文件并打开安装流程 |
| prepare_localized_copy | 创建汉化任务 |
| launch_claude | 按 official / managed 启动 |
| list_backups / restore_backup | 查询 / 恢复指定备份 |
| restore_language | 恢复工具保存的原语言字段 |
| export_diagnostics | 导出脱敏诊断日志 |

长任务立即返回 taskId，推送 progress、completed、failed 事件。事件携带 taskId、阶段、实际字节数、可选 total、可取消标志和错误码。前端切页不取消任务，重新订阅后从快照恢复。冲突任务由后台互斥，前端禁用按钮只作为反馈，不能承担互斥正确性。

状态机：

```text
安装：unknown → detecting → notInstalled | installed | error
下载：idle → resolving → downloading → verifying → ready | cancelled | failed
安装任务：ready → awaitingInstaller → verifyingInstallation → installed | cancelled | failed
汉化：notPrepared | outdated → copying → patching → verifying → committing → prepared | failed
启动：idle → starting → running | failed
恢复：idle → validating → restoring → completed | failed
```

progress 不保证每个阶段有百分比。所有错误包含稳定 code、简短中文 message、可选 detail 和可用恢复动作。

## 6. 测试策略

- Rust 核心：版本比较、manifest/校验和解析、路径边界、ASAR 修改、语言字段恢复、事务状态与回滚。
- 下载集成：本地 HTTP 测试服务模拟重定向、未知长度、网络断开、取消、错误 hash、限流及超时，不反复消耗公益镜像带宽。
- 文件操作：临时目录与合成资源包；测试不自动碰用户真实 Claude 配置或安装。
- 前端：关键状态和命令失败处理；浏览器检查导航、焦点、切页保持任务、恢复弹窗和真实/演示模式区分。
- 桌面：每阶段完成后构建并启动本地实例；涉及安装、汉化与恢复时使用明确测试目录或专用账户。
- 验证命令将随工程落地为 npm run check、npm run build、cargo fmt --check、cargo clippy、cargo test，避免预先声称不存在的脚本已通过。

## 7. 来源与待落地信息

上游汉化：
- https://github.com/BigPizzaV3/CodexPlusPlus
- 核心文件：crates/codex-plus-core/src/claude_localization.rs、claude_ion_asar.rs、assets/claude-localization/

下载来源：
- https://github.com/Wangnov/claude-app-mirror
- 发布信息与完整性校验固定在同一 Release。
- 托管入口接入条件参考 https://github.com/Wangnov/claude-app-mirror/blob/main/TERMS.md#open-client-policy
- 本地保存的条款说明要求：完整公开开源、非商业、显著来源标识、指向真实仓库的 User-Agent、自动元数据检查至少间隔 30 分钟。当前尚未提供新项目公开仓库，不能伪造实名标识。

不依赖用户补充即可推进 M0、M1、下载接口与本地测试、汉化提取。直接接入托管镜像与正式发布前，需要真实公开仓库地址；该信息不阻止前期开发。创建/公开远程仓库和发布 Release 应按用户对外发布授权执行。

## 8. 推荐执行顺序

M0 → M1 → M2 → M3 → M4 → M5 → M6。

优先交付 M0+M1 作为第一个可运行桌面版本：用真实探测状态替换演示信息，保持已完成的 UI。其次完成下载与安装，再进入风险更高、兼容性维护成本更大的汉化及恢复。每个里程碑附运行结果、测试证据、限制与下一阶段入口。
