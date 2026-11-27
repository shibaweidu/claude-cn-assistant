export const repository = 'https://github.com/shibaweidu/claude-cn-assistant';
export const pages = [
  {slug:'',title:'Claude Desktop 汉化助手｜中文界面、镜像下载与更新',description:'Claude Desktop 图形化汉化助手，提供独立中文副本、指定镜像下载、断点续传、更新确认和备份恢复。支持 Windows，提供 Apple Silicon 与 Intel Mac 预览版。',label:'首页',body:`
    <section class="hero"><div class="hero-copy"><p class="release">0.2.0 · 开源预览版</p><h1>Claude Desktop<br>汉化助手</h1><p class="lede">从下载、更新，到熟悉的中文界面。<br>在一个桌面助手里完成。</p><div class="actions"><a class="button primary" href="${repository}/releases">下载预览版 <span aria-hidden="true">↗</span></a><a class="button" href="windows/">开始使用</a></div><p class="fine">Windows · macOS 预览版 · 启动中文版需要 Node.js 22+</p></div><figure class="preview"><img src="assets/assistant-preview.png" width="1440" height="1000" alt="Claude Desktop 汉化助手的浏览器预览：侧栏包含下载与更新、界面汉化、备份与恢复"><figcaption>界面预览 · 示例状态，不代表安装验收结果</figcaption></figure></section>
    <section class="intro"><h2>原版、中文版，都有自己的位置。</h2><p>基于本机原版制作独立中文副本。更新有确认，下载能续传，旧副本留作备份。你可以从助手查看运行状态，再决定何时更新或切换。</p></section>
    <div class="features"><section><h3>下载与更新</h3><p>从指定社区镜像获取客户端，校验 SHA256 和包身份。中断后可继续下载，安装前询问是否退出 Claude。</p><a href="update/">了解更新流程 →</a></section><section><h3>制作中文副本</h3><p>原版安装保留，中文副本独立管理。新版原版安装完成后，重新制作即可使用对应的中文副本。</p><a href="compatibility/">查看兼容范围 →</a></section><section><h3>备份与恢复</h3><p>重新制作和恢复时保留旧副本。检查目录与文件完整性，处理中断的切换流程。</p><a href="troubleshooting/">恢复与故障排查 →</a></section></div>
    <section class="platforms"><div><h2>选对平台，再开始。</h2><p>我们将自动化构建与真实客户端验收分开说明。</p></div><div class="platform-links"><a href="windows/"><strong>Windows</strong><span>x64 预览版 · 已有本机启动实测</span><b aria-hidden="true">→</b></a><a href="macos/"><strong>macOS</strong><span>Apple Silicon / Intel · 实机流程待验收</span><b aria-hidden="true">→</b></a></div></section>
    <section class="callout"><h2>第一次使用？</h2><p>先阅读平台教程。助手不是 Claude Code 命令行汉化工具；不提供账号、订阅或 API 服务，也不属于 Anthropic 官方产品。</p><a href="faq/">阅读常见问题 →</a></section>`},
  {slug:'windows',title:'Claude Desktop Windows 汉化教程｜下载、更新与中文副本',description:'Windows x64 使用 Claude Desktop 汉化助手的步骤、Node.js 依赖、指定镜像安装、退出确认和中文副本启动说明。',label:'Windows 教程',body:`
    <h1>Windows 汉化与安装教程</h1><p class="lede">先安装原版，再制作独立中文副本。</p>
    <h2>开始前</h2><p>使用 Windows x64 预览包，准备 Node.js 22 或更新版本。桌面助手需要 WebView2。Claude 必须具有受支持的 <code>ion-dist.asar</code> 资源布局；不支持的结构会停止处理。</p>
    <h2>下载并运行助手</h2><ol><li>从 <a href="${repository}/releases">Releases</a> 下载 Windows x64 ZIP，解压后运行助手。</li><li>如果要核对下载文件，在 PowerShell 运行 <code>Get-FileHash -Algorithm SHA256 文件路径</code>，与 Release 的 <code>SHA256SUMS.txt</code> 对照。</li><li>预览程序未提供受信任的发布者签名。只使用本仓库发布文件，并遵守设备的组织策略。</li></ol>
    <h2>找到或安装 Claude</h2><p>在概览点击重新检测。没有检测到时，选择真实安装目录中的 Claude.exe。也可以在「下载与更新」获取指定镜像安装包。下载校验后进入安装检查，确认后打开 Windows 系统安装器，仍需在系统窗口完成安装。</p>
    <h2>制作并启动中文版</h2><ol><li>先保存正在进行的工作，进入「界面汉化」。</li><li>点击「制作中文副本」。Claude 正在运行时会显示「强制退出并汉化」，确认后才关闭进程。</li><li>等待复制、汉化和校验完成，点击「启动中文版」。</li><li>核对界面与来源版本。部分词条仍可能为英文。</li></ol>
    <h2>更新后怎么办？</h2><p>原版更新和中文副本分开管理。安装新版原版后，需要重新制作中文副本；原有副本会保留在备份目录。参见<a href="../update/">更新教程</a>。</p>
    <h2>遇到问题</h2><p>助手数据位于 <code>%LOCALAPPDATA%\\ClaudeCnAssistant</code>。在助手中打开数据目录查看日志；反馈前隐去个人信息。参见<a href="../troubleshooting/">故障排查</a>和<a href="../compatibility/">兼容性说明</a>。</p>`},
  {slug:'macos',title:'Claude Desktop Mac 汉化教程｜Apple Silicon 与 Intel 预览版',description:'macOS 版 Claude Desktop 汉化助手的依赖、Apple Silicon 和 Intel 下载、用户目录更新、应用副本签名与权限说明。',label:'Mac 教程',body:`
    <h1>macOS 汉化与安装教程</h1><p class="lede">Apple Silicon 与 Intel 原生构建，当前以预览版提供。</p>
    <aside class="callout"><p>原生自动化测试、应用复制与签名测试及打包已通过。真实 Claude 的完整登录、权限和中文页面流程仍需实机验收；助手尚未做 Apple 公证。</p></aside>
    <h2>选择架构</h2><p>在「关于本机」查看处理器。M 系列芯片使用 Apple Silicon 包；Intel 处理器使用 Intel 包。<a href="${repository}/releases">打开预览版下载</a>。</p>
    <h2>准备运行环境</h2><p>需要 Node.js 22+。Finder 启动的助手会检查 <code>/opt/homebrew/bin/node</code> 与 <code>/usr/local/bin/node</code>，也可使用可访问的 PATH。解压下载文件：Actions ZIP 内还有 tar.gz，需要继续解压；Release 直接提供 tar.gz。不要通过破坏符号链接或执行权限的方式重新打包 .app。</p>
    <h2>检测与镜像更新</h2><p>助手检测 <code>~/Applications/Claude.app</code> 与 <code>/Applications/Claude.app</code>，也支持手动选择真实应用。更新仅从指定镜像取得 DMG；只读挂载后检查版本、架构、Developer ID 签名和系统信任。</p><p>确认安装后，新原版放在 <code>~/Applications/Claude.app</code>，旧的用户目录原版保留为备份；已有系统 Applications 安装不被覆盖。强制退出需要你在本次确认。</p>
    <h2>制作与启动中文版</h2><ol><li>在「界面汉化」制作副本，等待资源修改、逐层重新签名和校验完成。</li><li>确认所有 Claude 进程退出后，启动中文版。</li><li>副本使用本地签名，macOS 隐私权限、钥匙串访问可能需要重新授权。组织策略不允许时请停止使用并联系设备管理员。</li></ol>
    <h2>备份与恢复</h2><p>助手数据位于 <code>~/Library/Application Support/ClaudeCnAssistant</code>。中文副本、旧副本和切换记录在此管理。原版更新后的重新汉化和中断处理见<a href="../update/">更新教程</a>及<a href="../troubleshooting/">故障排查</a>。</p>`},
  {slug:'update',title:'Claude Desktop 更新后如何重新汉化｜镜像下载与恢复流程',description:'了解 Claude Desktop 镜像更新、暂停续传、强制退出确认、实际安装结果检测，以及更新后重建中文副本的步骤。',label:'更新与恢复',body:`
    <h1>更新 Claude，再更新中文副本。</h1><p class="lede">下载完成、安装完成、中文副本更新，是三个不同状态。</p>
    <h2>完整更新流程</h2><ol><li>打开「下载与更新」检查镜像版本。助手运行时也会每 6 小时自动检查。</li><li>下载安装包；网络中断时选择同一版本继续下载，完成后校验完整 SHA256。</li><li>下载完成后自动进入本机安装检查，通过后弹出确认。Claude 运行时可确认强制退出，也可以取消并自行退出后重试。</li><li>Windows 在系统安装器中完成安装；Mac 安装到当前用户 Applications。</li><li>助手依据实际安装结果更新状态，确认成功后显示「已安装更新」。</li><li>进入「界面汉化」，重新制作并启动中文副本。</li></ol>
    <h2>为什么原版更新了，中文版还是旧版本？</h2><p>中文副本独立存储，不会因原版升级自动被替换。检查汉化页的「来源版本」。当前自动重建偏好尚未执行，仍需手动重新制作。</p>
    <h2>中断或想恢复旧副本</h2><p>正常退出 Claude，打开「备份与恢复」，选择校验通过的副本。若显示未完成切换，先使用「检查并恢复中断流程」。恢复会保留被替换的当前副本，不回退账户或对话。</p>
    <h2>下载源</h2><p>公益加速与 GitHub 备用渠道都来自 <a href="https://github.com/Wangnov/claude-app-mirror">Wangnov/claude-app-mirror</a>。不自动回退到官方安装渠道。切换渠道需要重新获取版本清单，不会在下载途中混用来源。</p>`},
  {slug:'faq',title:'Claude Desktop 汉化常见问题｜中文副本、更新与运行依赖',description:'Claude Desktop 汉化助手 FAQ：是否官方、是否支持 Claude Code、为什么制作副本、为何仍有英文、Node.js 要求与更新后的重新汉化。',label:'常见问题',body:`
    <h1>常见问题</h1><p class="lede">先了解助手做什么，再决定如何使用。</p>
    <h2>这是官方工具吗？</h2><p>不是。它是独立社区工具。Claude 名称与商标归其权利人所有；不提供账号、订阅、模型或第三方 API 服务。</p>
    <h2>能汉化 Claude Code 命令行吗？</h2><p>本项目面向 Claude Desktop 桌面应用，不是 Claude Code CLI 汉化工具。</p>
    <h2>为什么不是直接给原版打补丁？</h2><p>助手把汉化放在独立受管副本，保留原版安装，并为副本切换提供恢复记录。代价是额外的磁盘空间；汉化副本的签名状态也会改变。</p>
    <h2>为什么需要 Node.js？</h2><p>当前中文版启动使用 Node.js 22+ 完成主进程调试连接和页面注入，确认目标进程后关闭调试入口。运行时尚未内置。</p>
    <h2>能保证所有界面都变中文吗？</h2><p>不能。不同版本和功能的词条不完全相同，词表仍有缺失。资源结构匹配不等于所有页面均已实测。</p>
    <h2>会自动关闭 Claude 或静默安装吗？</h2><p>不会在未确认时强制退出。下载完成会检查并询问安装；强制退出要本次确认。Windows 还需在系统安装器中完成操作。</p>
    <h2>会修改对话吗？</h2><p>助手不写入 Claude 对话文件。原版与中文副本仍使用 Claude 自身的账户和数据环境，副本不是隔离的新账户。</p>
    <h2>更新后还需要汉化吗？</h2><p>需要重新制作中文副本。原版升级不替换旧中文副本，详见<a href="../update/">更新说明</a>。</p>
    <h2>Linux 和 Windows ARM64 能用吗？</h2><p>目前不提供对应助手和汉化构建。能下载该平台的客户端镜像，不等于该平台的助手已适配。</p>`},
  {slug:'troubleshooting',title:'Claude Desktop 汉化故障排查｜启动失败、下载中断和签名检查',description:'排查 Claude Desktop 汉化助手下载中断、安装检查失败、Node.js 缺失、中文副本启动失败和备份切换中断。',label:'故障排查',body:`
    <h1>从失败的那一步开始排查。</h1><p class="lede">保留错误信息，先确认下载、安装、制作还是启动失败。</p>
    <h2>下载中断或速度慢</h2><p>选择相同版本、文件名、大小与 SHA256 的安装包继续下载。服务器不支持 Range 时会从头写入，不拼接错误文件。可切换公益镜像和 GitHub 备用渠道后重新检查版本；不保证任一渠道的实际速度。</p>
    <h2>下载完但没有安装</h2><p>查看安装区域的具体检查结果。文件校验、签名、发布者、架构或版本不匹配时会停止。已有同版本或更高版本时不需要重复安装；Windows 安装器打开不代表安装完成。</p>
    <h2>启动中文版失败</h2><ol><li>确认 Node.js 版本至少为 22，运行 <code>node --version</code>。</li><li>正常退出原版与旧中文副本，检查汉化页运行状态。</li><li>打开数据目录，查看 <code>localization-launch.log</code>；页面注入错误也可能记录在 <code>managed/cn-runtime/hook.log</code>。</li><li>关键文件校验失败、资源格式不兼容时，不要强行修改哈希；重新制作或报告问题。</li></ol>
    <h2>Mac 签名或权限检查失败</h2><p>原版检查与本地签名副本检查不同。不要把中文副本作为原版来源。助手未做 Apple 公证；请核对下载来源、校验和以及组织策略。签名失效、系统信任检查未通过时会停止安装。</p>
    <h2>制作中断或恢复失败</h2><p>退出 Claude，进入「备份与恢复」处理未完成切换。未通过校验的暂存目录不能当作成功副本恢复。不要删除切换记录来绕过检查；保留目录以便诊断。</p>
    <h2>报告问题</h2><p>前往 <a href="${repository}/issues/new/choose">问题反馈</a>，填写助手版本、Claude 版本、系统和架构、重现步骤、完整错误信息。截图或日志先隐去个人目录、账户和令牌。</p>`},
  {slug:'compatibility',title:'Claude Desktop 汉化兼容性｜支持版本、平台与已知限制',description:'Claude Desktop 汉化助手的 Windows 与 macOS 支持矩阵、ASAR 布局限制、实测版本、词表缺失与预览版验收边界。',label:'兼容性',body:`
    <h1>兼容范围与验证记录</h1><p class="lede">构建通过、结构可处理、真实启动，是不同的证据。</p>
    <div class="table-wrap"><table><thead><tr><th>平台</th><th>当前支持</th><th>验证范围</th></tr></thead><tbody><tr><td>Windows x64</td><td>预览版</td><td>6.6395.0、6.28899.6 曾完成资源副本制作与启动注入检查；不保证全部页面</td></tr><tr><td>Mac Apple Silicon / Intel</td><td>预览版</td><td>原生测试、复制权限／链接、重新签名与打包通过；真实 Claude 完整流程待验收</td></tr><tr><td>Linux / Windows ARM64 助手</td><td>暂不提供</td><td>仅可下载对应客户端镜像</td></tr></tbody></table></div>
    <h2>资源结构限制</h2><p>当前处理 <code>ion-dist.asar</code> 布局，检查语言协商锚点、ASAR 结构和 Electron fuse。目录式资源或未知格式会停止处理，不以修改版本字符串绕过。Mac 还会检查原版系统签名，并为副本重新签名。</p>
    <h2>词表不是全量翻译</h2><p>对本机 Claude 6.28899.6 的静态词条检查发现：英文主词表 46,448 条，其中内置中文词表匹配 30,165 条，缺失 16,283 条；动态词表 51 条中匹配 49 条。缺失键不一定出现在你使用的页面，不能直接当作可见界面的翻译覆盖率。</p>
    <h2>已知限制</h2><ul><li>启动中文版依赖外部 Node.js 22+。</li><li>原版更新后需要重新制作副本；自动重建偏好尚未执行。</li><li>副本签名发生变化，Mac 隐私和钥匙串权限可能需要重新授权。</li><li>不提供账号、订阅、模型服务或官方兼容承诺。</li></ul>
    <h2>参与兼容性验证</h2><p>欢迎在 <a href="${repository}/issues/new/choose">Issues</a> 提供平台、架构、Claude 版本、资源布局及实际失败步骤。请区分静态检查、制作成功与真实页面验收。</p>`},
];
