import {bridge, type DownloadState, type DownloadTask} from './bridge';
import {InstallUI} from './install-view';
const esc=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const bytes=(value:number)=>value>=1024*1024?`${(value/1024/1024).toFixed(1)} MB`:`${(value/1024).toFixed(1)} KB`;
const stages:Record<string,string>={connecting:'正在连接',downloading:'正在下载',verifying:'正在校验 SHA256',committing:'正在保存',ready:'完整性校验通过',failed:'下载失败',cancelled:'已暂停',unverified:'等待重新校验'};
const active=(task:DownloadTask|null)=>!!task&&['connecting','downloading','verifying','committing'].includes(task.stage);
function compare(a:string,b:string):number|null {
  if(!/^\d+(\.\d+)*$/.test(a)||!/^\d+(\.\d+)*$/.test(b))return null;
  const left=a.split('.').map(BigInt),right=b.split('.').map(BigInt);
  for(let i=0;i<Math.max(left.length,right.length);i++){if((left[i]??0n)>(right[i]??0n))return 1;if((left[i]??0n)<(right[i]??0n))return -1;}return 0;
}
export class DownloadUI {
  install:InstallUI;
  data:DownloadState={channel:{accelerated:true,repository:'https://github.com/shibaweidu/claude-cn-assistant'},release:null,task:null,lastError:null,retryAt:0,stale:false};
  checking=false;
  working=false;
  error='';
  initialized=false;
  constructor(private changed:(progress?:boolean)=>void,private notify:(text:string)=>void,private auto:()=>boolean,private host:()=>string=()=> 'windows'){this.install=new InstallUI(()=>this.changed(),notify,host)}
  async init(){
    try{
      await bridge.onTask(task=>{const old=this.data.task;this.data.task=task;const progress=old?.id===task.id&&old?.stage===task.stage;this.changed(progress);if(!active(task))this.notify(task.stage==='ready'?'安装包已下载，正在准备安装检查':task.error||'下载已暂停，进度已保留');if(task.stage==='ready'&&(old?.id!==task.id||old.stage!=='ready'))void this.continueInstall(task);});
      this.data=await bridge.downloads();this.initialized=true;this.changed();
      await this.install.init();
      if(this.data.task?.stage==='unverified'&&this.canInstall(this.data.task))this.data.task=await bridge.verify();
      if(this.data.task?.stage==='ready'){
        if(!this.install.state?.stage||['stopped','failed'].includes(this.install.state.stage))void this.continueInstall(this.data.task);
        else if(this.install.state.stage==='ready')void this.install.action('install-confirm');
      }
      this.changed();
      if(this.auto())void this.check(false);
      window.setInterval(()=>{if(this.auto())void this.check(false)},6*3600*1000);
    }catch(e){this.error=String(e);this.changed();}
  }
  canInstall(task:DownloadTask){return this.host()==='macos'?task.assetName.endsWith('.dmg'):this.host()==='windows'&&task.assetName.endsWith('.msix')}
  async continueInstall(task:DownloadTask){
    if(!this.canInstall(task)||this.install.active()||this.install.busy||this.taskInstalled(task))return;
    await this.install.action('install-prepare');
  }
  async check(manual=true){
    if(this.checking)return;this.checking=true;this.error='';this.changed();
    try{this.data=await bridge.checkRelease(manual)}catch(e){this.error=String(e)}finally{this.checking=false;this.changed()}
  }
  async action(name:string,platform:string,architecture:string){
    if(name.startsWith('install-')){await this.install.action(name);return}
    if(name==='release-check'){await this.check();return}
    if(this.working)return;
    this.working=true;this.error='';this.changed();
    try{
      if(name==='package-channel'){this.data=await bridge.channel(!this.data.channel.accelerated);return}
      if(name==='package-download'){
        const task=await bridge.download(platform==='mac'?'macos':platform,platform==='mac'?'universal':architecture);
        // 任务可能在 invoke 返回前已产生进度或完成事件。
        if(this.data.task?.id!==task.id)this.data.task=task;
      }else if(name==='package-cancel'){
        if(this.data.task){await bridge.cancel(this.data.task.id);this.notify('正在暂停下载…')}
      }else if(name==='package-verify'){this.data.task=await bridge.verify();if(this.data.task.stage==='ready')void this.continueInstall(this.data.task);}
      else if(name==='package-reveal'){await bridge.reveal();}
      else if(name==='release-page'){await bridge.openLink('release');}
    }catch(e){this.error=String(e)}finally{this.working=false;this.changed()}
  }
  taskInstalled(task:DownloadTask){return this.install.state?.stage==='installed'&&this.install.state.taskId===task.id}
  view(platform:string,architecture:string,localVersion?:string){
    const {release,task,lastError,retryAt,stale}=this.data;
    const selected=release?.assets.find(a=>a.platform===(platform==='mac'?'macos':platform)&&a.architecture===(platform==='mac'?'universal':architecture));
    const confirmed=this.install.state?.stage==='installed'&&(platform==='windows'&&this.host()==='windows'&&this.install.state.info?.architecture===architecture||platform==='mac'&&this.host()==='macos')?this.install.state.info?.version:undefined;
    const effectiveVersion=confirmed&&(!localVersion||compare(confirmed,localVersion)===1)?confirmed:localVersion;
    const sameHost=platform===(this.host()==='macos'?'mac':this.host());
    const ordering=sameHost&&effectiveVersion&&release?compare(release.version,effectiveVersion):null;
    const upToDate=sameHost&&(ordering===0||ordering===-1);
    const status=ordering===1?'发现新版本':ordering===0?'已更新至最新版本':ordering===-1?'镜像版本早于本机':release?'镜像版本已获取':'尚未检查版本';
    const checked=release?new Date(release.checkedAt*1000).toLocaleString('zh-CN'):'—';
    return `<div class="page-heading"><div><h1>下载与更新</h1><p>原版安装包，来自你指定的社区镜像。</p></div><button class="subtle-button" data-native="release-check" ${this.checking?'disabled':''}>${this.checking?'正在查询…':'检查版本'}</button></div>
      ${this.install.state?.stage==='failed'?`<div class="notice" role="alert">安装未能继续：${esc(this.install.state.message)}<br>请查看下方安装详情。</div>`:''}
      ${this.error||lastError?`<div class="notice" role="alert">${esc(this.error||lastError)}${retryAt>Date.now()/1000?`<br>下次可重试：${esc(new Date(retryAt*1000).toLocaleTimeString('zh-CN'))}`:''}<br><button class="text-button" data-native="release-page">打开镜像发布页</button></div>`:''}
      <div class="notice">下载渠道：${this.data.channel.accelerated?'公益镜像加速':'GitHub Releases'} <button class="text-button" data-native="package-channel" ${active(task)||this.checking||this.working?'disabled':''}>切换到${this.data.channel.accelerated?' GitHub 备用':'公益加速'}</button></div>
      <div class="platform-tabs" role="tablist" aria-label="下载操作系统">${[['windows','Windows'],['mac','macOS'],['linux','Linux']].map(([key,label])=>`<button role="tab" aria-selected="${platform===key}" data-platform="${key}">${label}</button>`).join('')}</div>
      <section class="panel"><div class="download-product"><div class="product-icon" aria-hidden="true">↓</div><div><h2>Claude for ${platform==='windows'?'Windows':platform==='mac'?'macOS':'Linux'}</h2><p>${esc(selected?.name||'检查版本后获取对应安装包')}<br>${esc(status)}${release?` · ${esc(release.version)}`:''}</p></div><button class="primary-button" data-native="package-download" ${!selected||stale||upToDate||active(task)||this.working||this.install.active()||this.install.busy?'disabled':''}>${upToDate?'已是最新版本':active(task)?'正在处理…':(task&&task.version===release?.version&&task.assetName===selected?.name&&['failed','cancelled'].includes(task.stage)&&task.downloaded>0?'继续下载':'下载安装包')}</button></div>
      <div class="download-info"><div><span>架构</span>${platform==='mac'?'<strong>Universal</strong>':`<select id="architecture" aria-label="处理器架构"><option value="x64" ${architecture==='x64'?'selected':''}>x64</option><option value="arm64" ${architecture==='arm64'?'selected':''}>ARM64</option></select>`}</div><div><span>安装包大小</span><strong>${selected?bytes(selected.size):'—'}</strong></div><div><span>最后成功检查</span><strong>${esc(checked)}</strong></div></div>
      ${stale?'<div class="notice">清单已超过 6 小时，请重新检查版本后下载。</div>':''}
      ${selected?`<div class="download-hash"><span>预期 SHA256</span><code>${esc(selected.sha256)}</code></div>`:''}</section>
      ${ordering===-1?'<div class="notice">本机版本更新，无需降级。你仍可保存镜像安装包，但助手不会自动安装。</div>':''}
      ${task?`<div class="section-title"><h2>下载任务</h2><span>切换页面不会中断下载</span></div>${this.taskView(task)}`:''}
      ${this.install.view()}
      <div class="notice">安装包与校验和锁定在同一个 GitHub Release。本机安装前会再次核对 SHA256、签名、身份和架构。macOS 更新安装到当前用户的 Applications，旧版保留备份；其他操作系统的安装包仅保存。${this.auto()?'助手运行时每 6 小时自动检查一次。':'自动检查已关闭。'}</div>`;
  }
  taskView(task:DownloadTask){
    const installed=this.taskInstalled(task);
    const installing=this.install.state?.taskId===task.id&&['launching','awaitingInstaller'].includes(this.install.state.stage);
    const taskLabel=installed?'已安装更新':installing?'等待安装完成':this.install.busy?'正在检查安装条件':stages[task.stage]||task.stage;
    const percent=task.total?Math.min(100,task.downloaded/task.total*100):0;
    return `<section class="panel task-live" aria-label="下载任务"><div class="panel-top"><h2>${esc(task.assetName)}</h2><span class="badge warm" data-task-stage>${esc(taskLabel)}</span></div><div class="details"><div class="operation-label"><span data-task-size>${bytes(task.downloaded)} / ${bytes(task.total)}</span><span data-task-percent>${percent.toFixed(1)}%</span></div><progress class="native-progress" value="${task.downloaded}" max="${task.total||1}" aria-label="下载字节进度"></progress><p class="task-error" role="alert">${esc(task.error||'')}</p>${task.path?`<p class="path-display task-path">${esc(task.path)}</p>`:''}<div class="task-actions">${installed?'<span class="badge">此安装包对应的版本已安装</span>':''}${active(task)?`<button class="subtle-button" data-native="package-cancel" ${!task.cancellable||this.working?'disabled':''}>暂停下载</button>`:task.stage==='ready'?`<button class="subtle-button" data-native="package-reveal" ${this.working?'disabled':''}>打开所在目录</button>${this.canInstall(task)&&!installed?`<button class="primary-button" data-native="install-prepare" ${this.install.active()||this.install.busy?'disabled':''}>${this.install.busy?'正在检查…':installing?'等待安装完成':'检查并安装'}</button>`:''}`:task.stage==='unverified'?`<button class="subtle-button" data-native="package-verify" ${this.working?'disabled':''}>${this.working?'正在校验…':'重新验证文件'}</button>`:`<span>进度已保留。选择相同版本安装包可继续下载。</span>`}</div></div></section>`;
  }
  banner(){const task=this.data.task;return active(task)?`<button class="task-banner" data-goto="downloads"><span>${esc(stages[task!.stage])} · ${esc(task!.assetName)}</span><span data-banner-size>${bytes(task!.downloaded)} / ${bytes(task!.total)}</span></button>`:this.install.banner()}
  updateProgress(){
    const task=this.data.task;if(!task)return;
    document.querySelectorAll('[data-task-size],[data-banner-size]').forEach(el=>el.textContent=`${bytes(task.downloaded)} / ${bytes(task.total)}`);
    document.querySelectorAll('[data-task-percent]').forEach(el=>el.textContent=`${(task.total?Math.min(100,task.downloaded/task.total*100):0).toFixed(1)}%`);
    const progress=document.querySelector<HTMLProgressElement>('.native-progress');if(progress)progress.value=task.downloaded;
  }
}
