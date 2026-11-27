import {bridge,type InstallState} from './bridge';
const esc=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const labels:Record<string,string>={checking:'安装前检查',ready:'等待确认',launching:'正在打开安装器',awaitingInstaller:'等待系统安装',installed:'安装已确认',failed:'未能继续',interrupted:'上次流程已中断',unconfirmed:'结果未确认',stopped:'已结束等待'};
export class InstallUI {
  state:InstallState|null=null;
  busy=false;
  error='';
  constructor(private changed:()=>void,private notify:(text:string)=>void,private host:()=>string=()=> 'windows'){}
  async init(){
    try{this.state=await bridge.installation();this.changed()}catch(error){this.error=String(error);this.changed()}
    // 仅查询系统安装是否达成；不模拟系统安装进度。
    window.setInterval(()=>{if(this.state?.stage==='awaitingInstaller'&&!this.busy)void this.action('install-recheck')},15000);
  }
  active(){return !!this.state&&['checking','ready','launching','awaitingInstaller'].includes(this.state.stage)}
  async action(name:string,forceExit=false){
    if(this.busy)return;
    if(name==='install-confirm'){await this.confirm();return}
    this.busy=true;this.error='';this.changed();
    const previousStage=this.state?.stage;
    try{
      if(name==='install-prepare')this.state=await bridge.prepareInstall();
      else if(name==='install-launch'&&this.state)this.state=await bridge.launchInstall(this.state.id,forceExit);
      else if(name==='install-recheck')this.state=await bridge.recheckInstall();
      else if(name==='install-stop')this.state=await bridge.stopInstall();
      if(this.state?.stage==='failed')this.notify('安装检查未通过：'+this.state.message);
      if(this.state?.stage==='installed'&&previousStage!=='installed'){this.notify(this.host()==='macos'?'macOS 应用安装与签名校验成功':'已通过 Windows 包注册信息确认安装版本');window.dispatchEvent(new Event('installation-confirmed'))}
    }catch(error){this.error=String(error)}finally{this.busy=false;this.changed()}
    if(name==='install-prepare'&&this.state?.stage==='ready')await this.confirm();
  }
  async confirm(){
    if(this.state?.stage!=='ready'||!this.state.info)return;
    const installId=this.state.id;
    let running=false;
    this.busy=true;this.changed();
    try{const runtime=await bridge.localizationRuntime();running=runtime.managedRunning||runtime.originalRunning||runtime.unknownRunning}
    catch(error){this.error='无法检测 Claude 运行状态：'+String(error);return}
    finally{this.busy=false;this.changed()}
    if(this.state?.id!==installId||this.state.stage!=='ready')return;
    const info=this.state.info!;
    const mac=this.host()==='macos';
    const dialog=document.querySelector<HTMLDialogElement>('#modal')!;
    if(dialog.open){this.notify('安装检查已完成，请在下载与更新页确认安装');return}
    document.querySelector('#modal-content')!.innerHTML=`<h2>${running?'强制退出 Claude 并更新？':'安装更新已准备好'}</h2><p>Claude ${esc(info.version)} · ${esc(info.architecture)}<br>发布者：Anthropic, PBC<br>已通过 SHA256、${mac?'macOS Developer ID':'Windows'} 签名与身份检查。${running?'Claude 正在运行（包括中文副本）。继续会强制结束所有 Claude 进程，正在生成的内容或未保存的操作可能丢失。请先保存。':''}${mac?'确认后将安装到当前用户的 ~/Applications，旧版原应用会保留为备份。已有 /Applications 安装不会被覆盖。中文副本需要在更新后重新制作。':'确认后将打开 Windows 安装器，由你完成安装。已有普通安装会保留，不会自动卸载或迁移其数据。'}</p><div class="modal-actions"><button class="subtle-button" id="cancel-install">返回</button><button class="primary-button" id="confirm-install">${running?'强制退出并继续更新':'继续安装更新'}</button></div>`;
    document.querySelector('#cancel-install')!.addEventListener('click',()=>dialog.close());
    document.querySelector('#confirm-install')!.addEventListener('click',()=>{dialog.close();void this.action('install-launch',running)});
    dialog.showModal();
  }
  view(){
    const state=this.state;const system=this.host()==='macos'?'macOS':'Windows';
    if(!state?.stage&&!this.busy&&!this.error)return '';
    const canRecheck=!!state&&['awaitingInstaller','interrupted','unconfirmed','stopped',...(this.host()==='macos'?['failed']:[])].includes(state.stage);
    const canStop=!!state&&['ready','awaitingInstaller','interrupted','unconfirmed'].includes(state.stage);
    return `<div class="section-title"><h2>${system} 安装</h2><span>只依据实际安装结果确认成功</span></div><section class="panel install-live" aria-label="${system} 安装状态"><div class="panel-top"><h2>${this.busy?'正在处理…':esc(labels[state?.stage||'']||'安装检查')}</h2></div><div class="details"><p class="native-description" role="status">${esc(state?.message||'正在检查安装包，请稍候。')}</p>${this.error?`<p class="task-error" role="alert">${esc(this.error)}</p>`:''}${state?.backupPath?`<p class="native-description path-display">旧版原应用备份：${esc(state.backupPath)}</p>`:''}${state?.info?`<div class="detail-row"><span>目标安装包</span><strong>Claude ${esc(state.info.version)} / ${esc(state.info.architecture)}</strong></div><div class="detail-row"><span>签名状态</span><strong>${esc(state.info.signatureStatus)}</strong></div>`:''}<div class="task-actions">${state?.stage==='ready'?`<button class="primary-button" data-native="install-confirm" ${this.busy?'disabled':''}>确认安装</button>`:''}${canRecheck?`<button class="subtle-button" data-native="install-recheck" ${this.busy?'disabled':''}>重新检测结果</button>`:''}${canStop?`<button class="subtle-button" data-native="install-stop" ${this.busy?'disabled':''}>${state?.stage==='ready'?'取消本次流程':'结束等待'}</button>`:''}${state?.stage==='installed'?'<button class="subtle-button" data-native="detect">刷新本机状态</button>':''}</div>${canRecheck?'<p class="native-description">每 15 秒检测一次，最多等待 10 分钟。若你已取消或系统提示失败，可结束等待；此按钮不会中止 Windows 安装。</p>':''}</div></section>`;
  }
  banner(){const system=this.host()==='macos'?'macOS':'Windows';return this.active()?`<button class="task-banner" data-goto="downloads"><span>${system} 安装 · ${esc(labels[this.state!.stage])}</span><span>查看进度</span></button>`:''}
}
