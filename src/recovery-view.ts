import {bridge,type RecoveryState,type LocalState} from './bridge';
const esc=(s:unknown)=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export class RecoveryUI{
  data:RecoveryState|null=null;busy=false;error='';message='';
  constructor(private changed:()=>void,private applied:(state:LocalState)=>void){}
  async load(){if(this.busy)return;this.busy=true;this.error='';this.changed();try{this.data=await bridge.recovery()}catch(e){this.error=String(e)}finally{this.busy=false;this.changed()}}
  async action(name:string,id?:string){
    if(name==='recovery-refresh'){await this.load();return}
    if(this.busy)return;
    const entry=this.data?.entries.find(e=>e.id===id);
    if(name==='recovery-restore'||name==='recovery-archive'||name==='recovery-reconcile'){
      if(name!=='recovery-reconcile'&&!entry)return;
      const dialog=document.querySelector<HTMLDialogElement>('#modal')!;
      const title=name==='recovery-restore'?'恢复这个副本？':name==='recovery-archive'?'归档这个目录？':'恢复中断的切换？';
      const description=name==='recovery-restore'?`将切换到 ${esc(entry!.version||'所选')} 受管副本。当前副本先保留为另一份备份。只恢复应用副本，不降级原版安装、不回滚对话。旧副本与新版用户数据的兼容性仍需自行检查。`:name==='recovery-archive'?'目录将移到应用数据目录的 archive 文件夹，保留原始文件，可在资源管理器中找回。归档不会释放磁盘空间，也不会删除文件。':'将根据事务记录和实际文件校验结果，恢复旧副本或确认新副本。未完成目录和备份均保留；无法确认时停止。';
      document.querySelector('#modal-content')!.innerHTML=`<h2>${title}</h2><p>${description}<br>请先正常退出 Claude。</p><div class="modal-actions"><button class="subtle-button" id="recovery-back">返回</button><button class="primary-button" id="recovery-confirm">${name==='recovery-archive'?'归档目录':'确认恢复'}</button></div>`;
      document.querySelector('#recovery-back')!.addEventListener('click',()=>dialog.close());document.querySelector('#recovery-confirm')!.addEventListener('click',()=>{dialog.close();void this.execute(name,id)});dialog.showModal();return;
    }
    await this.execute(name,id);
  }
  async execute(name:string,id?:string){
    this.busy=true;this.error='';this.message='';this.changed();
    try{
      let state:LocalState|undefined;
      if(name==='recovery-restore'&&id)state=await bridge.restoreBackup(id);
      else if(name==='recovery-reconcile')state=await bridge.reconcile();
      else if(name==='recovery-current')state=await bridge.useCurrent();
      else if(name==='recovery-archive'&&id){await bridge.archiveEntry(id);this.message='目录已移至 archive，文件仍保留。'}
      if(state){this.applied(state);this.message=state.message}
      this.data=await bridge.recovery();
    }catch(e){this.error=String(e)}finally{this.busy=false;this.changed()}
  }
  view(){const data=this.data;return `<div class="page-heading"><div><h1>每一次改变，都有退路。</h1><p>校验副本、恢复备份，处理未完成的切换。</p></div><button class="subtle-button" data-native="recovery-refresh" ${this.busy?'disabled':''}>${this.busy?'正在校验…':'刷新与校验'}</button></div>${this.error?`<div class="notice" role="alert">${esc(this.error)}</div>`:''}${this.message?`<div class="notice" role="status">${esc(this.message)}</div>`:''}<section class="panel"><div class="panel-top"><h2>当前受管副本</h2><span class="badge warm">${data?.currentValid?'校验通过':'尚不可用'}</span></div><div class="details"><p class="native-description">${esc(data?.currentMessage||'正在读取副本信息…')}</p>${data?.currentValid?`<button class="subtle-button" data-native="recovery-current" ${this.busy||data.pending?'disabled':''}>重新启用当前副本</button>`:''}</div></section>${data?.pending?`<section class="panel wide-panel"><div class="panel-top"><h2>有未完成的目录切换</h2></div><div class="details"><p class="native-description" role="alert">${esc(data.transactionMessage)}</p><button class="primary-button" data-native="recovery-reconcile" ${this.busy?'disabled':''}>检查并恢复中断流程</button></div></section>`:''}<div class="section-title"><h2>备份与暂存目录</h2><span>${data?.entries.length||0} 项</span></div>${data?.entries.length?data.entries.map(entry=>`<section class="panel recovery-entry"><div class="panel-top"><h2>${entry.kind==='backup'?'受管副本备份':'制作暂存目录'} ${esc(entry.version)}</h2><span class="badge ${entry.valid?'':'warm'}">${entry.valid?'校验通过':'不可恢复'}</span></div><div class="details"><p class="native-description">${esc(entry.message)}<br>${new Date(entry.modified*1000).toLocaleString('zh-CN')} · ${(entry.bytes/1024/1024).toFixed(1)} MB</p><p class="path-display task-path">${esc(entry.id)}</p><div class="task-actions"><button class="primary-button" data-native="recovery-restore" data-entry="${esc(entry.id)}" ${!entry.valid||this.busy||data.pending?'disabled':''}>恢复此副本</button><button class="subtle-button" data-native="recovery-archive" data-entry="${esc(entry.id)}" ${this.busy||data.pending?'disabled':''}>归档目录</button></div></div></section>`).join(''):`<section class="panel"><div class="empty-state"><h2>${this.busy?'正在校验目录…':'暂无备份'}</h2><p>重新制作或恢复副本时，旧的当前副本会自动保留。首次制作成功前可能只有未完成的暂存目录。</p></div></section>`}<div class="notice">恢复只切换受管应用副本，不会更改原版安装、账户或对话。归档仅移动目录，不释放磁盘空间。当前汉化实现不写语言配置文件；退出中文版后可从概览启动原版客户端。</div><button class="subtle-button" data-native="open-data">打开数据目录</button>`}
}
