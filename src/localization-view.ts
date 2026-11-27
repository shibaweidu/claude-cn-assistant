import {bridge,type RuntimeState,type LocalState} from './bridge';
const esc=(s:unknown)=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const labels:Record<string,string>={copying:'正在复制',patching:'正在汉化',verifying:'正在校验',committing:'正在切换副本',prepared:'副本已准备',launched:'启动已确认',failed:'制作失败',cancelled:'已取消',interrupted:'上次操作中断'};
export class LocalizationUI{
  state:LocalState|null=null;busy=false;error='';
  runtime:RuntimeState|null=null;runtimeError='';checking=false;
  async refreshRuntime(){
    if(this.checking)return;this.checking=true;
    const before=JSON.stringify([this.runtime,this.runtimeError]);
    try{this.runtime=await bridge.localizationRuntime();this.runtimeError=''}catch(e){this.runtime=null;this.runtimeError=String(e)}
    finally{this.checking=false;if(before!==JSON.stringify([this.runtime,this.runtimeError]))this.changed()}
  }
  constructor(private changed:()=>void){}
  async init(){void this.refreshRuntime();window.setInterval(()=>{void this.refreshRuntime()},5000);try{await bridge.onLocalization(state=>{this.state=state;this.changed()});this.state=await bridge.localization();this.changed()}catch(e){this.error=String(e);this.changed()}}
  async action(name:string,forceExit=false){
    if(name==='local-cancel'){try{await bridge.cancelLocalized()}catch(e){this.error=String(e);this.changed()}return}
    if(this.busy)return;
    if(name==='local-confirm'){
      this.busy=true;this.error='';this.changed();
      try{this.runtime=await bridge.localizationRuntime();this.runtimeError=''}catch(e){this.error='无法检测 Claude 运行状态：'+String(e);return}finally{this.busy=false;this.changed()}
      const running=!!this.runtime&&(this.runtime.managedRunning||this.runtime.originalRunning||this.runtime.unknownRunning);
      const dialog=document.querySelector<HTMLDialogElement>('#modal')!;
      document.querySelector('#modal-content')!.innerHTML=`<h2>${running?'强制退出并汉化？':'制作中文副本？'}</h2><p>会复制本机原版客户端到独立数据目录，加入中文词表，并修改副本的 Electron 调试及资源完整性开关。副本的原始数字签名将不再有效。原版目录保持不变。${running?'继续会强制结束所有 Claude 进程（包括中文副本），正在生成的内容或未保存的操作可能丢失，请先保存。':''}现有受管副本将保留为备份。</p><div class="modal-actions"><button class="subtle-button" id="local-back">返回</button><button class="primary-button" id="local-accept">${running?'强制退出并汉化':'开始制作'}</button></div>`;
      document.querySelector<HTMLButtonElement>('#local-back')!.onclick=()=>dialog.close();document.querySelector<HTMLButtonElement>('#local-accept')!.onclick=()=>{dialog.close();void this.action('local-prepare',running)};dialog.showModal();return;
    }
    this.busy=true;this.error='';this.changed();
    try{this.state=name==='local-launch'?await bridge.launchLocalized():await bridge.prepareLocalized(forceExit)}catch(e){this.error=String(e)}finally{this.busy=false;this.changed();void this.refreshRuntime()}
  }
  view(){const state=this.state;const running=!!this.runtime&&(this.runtime.managedRunning||this.runtime.originalRunning||this.runtime.unknownRunning);
    const runtimeLabel=this.runtimeError?'运行状态检测失败':!this.runtime?'正在检测运行状态':this.runtime.managedRunning?(this.runtime.originalRunning?'中文版与原版运行中':'中文版运行中'):this.runtime.originalRunning?'原版运行中':this.runtime.unknownRunning?'Claude 运行中（版本待确认）':'未运行';const prepared=!!state&&['prepared','launched'].includes(state.stage);
    return `<div class="page-heading"><div><h1>熟悉的语言，自然的体验。</h1><p>从本机原版安装制作独立中文副本，下载源继续使用指定镜像。</p></div></div><section class="panel"><div class="panel-top"><h2>${esc(labels[state?.stage||'']||'中文副本尚未准备')}</h2><span class="runtime-badge ${running?'is-running':''}" role="status" aria-live="polite"><i aria-hidden="true"></i>${esc(runtimeLabel)}</span></div><div class="details">${this.runtimeError?`<p class="task-error">${esc(this.runtimeError)}</p>`:''}<p class="native-description" role="status">${esc(state?.message||'先退出 Claude，再制作受管副本。不会修改原版安装或对话数据。')}</p>${this.error?`<p class="task-error" role="alert">${esc(this.error)}</p>`:''}${state?.stage==='copying'?`<progress class="native-progress" value="${state.copied}" max="${state.total||1}" aria-label="副本复制进度"></progress><p class="native-description">${(state.copied/1024/1024).toFixed(1)} / ${(state.total/1024/1024).toFixed(1)} MB</p>`:''}<div class="task-actions"><button class="primary-button" data-native="local-confirm" ${this.busy?'disabled':''}>${this.busy?'正在处理…':running?'强制退出并汉化':prepared?'重新制作中文副本':'制作中文副本'}</button>${prepared?`<button class="subtle-button" data-native="local-launch" ${this.busy||running?'disabled':''}>${this.runtime?.managedRunning?'中文版运行中':'启动中文版'}</button>`:''}${state?.stage==='copying'?'<button class="subtle-button" data-native="local-cancel">取消复制</button>':''}<button class="subtle-button" data-native="open-data">打开数据目录</button></div>${state?.sourceVersion?`<div class="detail-row"><span>来源版本</span><strong>${esc(state.sourceVersion)}</strong></div>`:''}${state?.managedExe?`<p class="native-description path-display">${esc(state.managedExe)}</p>`:''}${state?.backup?`<p class="native-description path-display">旧副本保留于：${esc(state.backup)}</p>`:''}</div></section><div class="notice">开发版仅支持 Windows 的 ion-dist.asar 资源布局。启动注入需要 Node.js 22+，当前尚未内置运行时。目录布局和未知版本结构会停止处理，不强行修改。启动后仍需核对界面中文覆盖。</div>`;
  }
}
