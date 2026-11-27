const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const ts=require('typescript');
function load(file,deps){
  const exports={};
  const code=ts.transpileModule(fs.readFileSync(file,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
  new Function('require','exports',code)(name=>deps[name],exports);
  return exports;
}
const ready={id:'install-1',stage:'ready',info:{version:'6.20.1',architecture:'x64'}};
function setup(running){
  const calls=[];const listeners={};
  const dialog={open:false,showModal(){this.open=true},close(){this.open=false}};
  const content={innerHTML:''};
  global.document={querySelector(selector){if(selector==='#modal')return dialog;if(selector==='#modal-content')return content;return {addEventListener(event,callback){listeners[selector]=callback}}}};
  global.window={setInterval(){},dispatchEvent(){}};
  const bridge={prepareInstall:async()=>structuredClone(ready),localizationRuntime:async()=>({managedRunning:running,originalRunning:false,unknownRunning:false}),launchInstall:async(id,force)=>{calls.push({id,force});return {...ready,stage:'awaitingInstaller'}}};
  const {InstallUI}=load('src/install-view.ts',{'./bridge':{bridge}});
  return {ui:new InstallUI(()=>{},()=>{}),calls,listeners,dialog,content,bridge};
}
test('download preparation automatically asks before closing running Claude',async()=>{
  const s=setup(true);await s.ui.action('install-prepare');
  assert.equal(s.dialog.open,true);assert.match(s.content.innerHTML,/强制退出并继续更新/);assert.deepEqual(s.calls,[]);
  s.listeners['#cancel-install']();assert.equal(s.dialog.open,false);assert.deepEqual(s.calls,[]);
});
test('explicit confirmation forwards force exit for this installation only',async()=>{
  const s=setup(true);await s.ui.action('install-prepare');s.listeners['#confirm-install']();await new Promise(setImmediate);
  assert.deepEqual(s.calls,[{id:'install-1',force:true}]);
});
test('stopped Claude does not grant force exit permission',async()=>{
  const s=setup(false);await s.ui.action('install-prepare');s.listeners['#confirm-install']();await new Promise(setImmediate);
  assert.deepEqual(s.calls,[{id:'install-1',force:false}]);
});
test('process detection failure never launches or opens confirmation',async()=>{
  const s=setup(false);s.bridge.localizationRuntime=async()=>{throw Error('probe failed')};await s.ui.action('install-prepare');
  assert.equal(s.dialog.open,false);assert.match(s.ui.error,/probe failed/);assert.deepEqual(s.calls,[]);
});
test('completion event starts installation once and only for Windows MSIX',async()=>{
  global.window={setInterval(){}};let event;let prepares=0;
  class InstallUI{busy=false;state=null;active(){return this.busy}async init(){}async action(){prepares++;this.busy=true}}
  const bridge={onTask:async callback=>{event=callback},downloads:async()=>({task:null})};
  const {DownloadUI}=load('src/download-view.ts',{'./bridge':{bridge},'./install-view':{InstallUI}});
  const ui=new DownloadUI(()=>{},()=>{},()=>false);await ui.init();
  event({id:'linux',stage:'ready',assetName:'Claude-linux.deb'});assert.equal(prepares,0);
  event({id:'win',stage:'downloading',assetName:'Claude-win-x64.msix'});
  event({id:'win',stage:'ready',assetName:'Claude-win-x64.msix'});
  event({id:'win',stage:'ready',assetName:'Claude-win-x64.msix'});assert.equal(prepares,1);
});

test('confirmed installation updates task and latest status without offering installation again',()=>{
  const {DownloadUI}=load('src/download-view.ts',{'./bridge':{bridge:{}},'./install-view':{InstallUI:class{active(){return false}view(){return ''}}}});
  const ui=new DownloadUI(()=>{},()=>{},()=>false);
  const task={id:'download-1',stage:'ready',version:'6.28899.6',assetName:'Claude-win-x64.msix',downloaded:100,total:100};
  ui.data.task=task;
  ui.data.release={version:task.version,checkedAt:1,assets:[{platform:'windows',architecture:'x64',name:task.assetName,size:100}]};
  ui.install.state={stage:'installed',taskId:task.id,info:{version:'6.28899.6.0',architecture:'x64'}};
  assert.match(ui.taskView(task),/已安装更新/);
  assert.doesNotMatch(ui.taskView(task),/data-native="install-prepare"/);
  assert.match(ui.view('windows','x64','6.6395.0'),/已更新至最新版本/);
  ui.data.release.version='6.30000.0';
  assert.match(ui.view('windows','x64','6.6395.0'),/发现新版本/);
  ui.data.task={...task,id:'new-download'};
  assert.doesNotMatch(ui.taskView(ui.data.task),/已安装更新/);
});

test('localization requires confirmation before forcing running Claude to exit',async()=>{
 const elements={};const calls=[];
 for(const key of ['#modal','#modal-content','#local-back','#local-accept'])elements[key]={};
 elements['#modal'].showModal=function(){this.open=true};elements['#modal'].close=function(){this.open=false};
 global.document={querySelector:key=>elements[key]};
 const bridge={localizationRuntime:async()=>({originalRunning:true}),prepareLocalized:async force=>{calls.push(force);return {stage:'prepared'}}};
 const {LocalizationUI}=load('src/localization-view.ts',{'./bridge':{bridge}});
 const ui=new LocalizationUI(()=>{});await ui.action('local-confirm');
 assert.match(elements['#modal-content'].innerHTML,/强制退出并汉化/);assert.deepEqual(calls,[]);
 elements['#local-back'].onclick();assert.deepEqual(calls,[]);
 await ui.action('local-confirm');elements['#local-accept'].onclick();await new Promise(setImmediate);
 assert.deepEqual(calls,[true]);assert.equal(ui.state.stage,'prepared');
});
test('localization process detection failure cannot grant force exit permission',async()=>{
 const calls=[];const bridge={localizationRuntime:async()=>{throw Error('probe failed')},prepareLocalized:async force=>calls.push(force)};
 const {LocalizationUI}=load('src/localization-view.ts',{'./bridge':{bridge}});
 const ui=new LocalizationUI(()=>{});await ui.action('local-confirm');
 assert.match(ui.error,/probe failed/);assert.deepEqual(calls,[]);assert.equal(ui.busy,false);
});

test('macOS only offers native DMG installation and confirms user Applications destination',async()=>{
 const s=setup(true);s.ui.host=()=> 'macos';await s.ui.action('install-prepare');
 assert.match(s.content.innerHTML,/macOS Developer ID/);assert.match(s.content.innerHTML,/Applications/);assert.doesNotMatch(s.content.innerHTML,/打开 Windows 安装器/);
 class InstallUI{active(){return false}view(){return ''}}
 const {DownloadUI}=load('src/download-view.ts',{'./bridge':{bridge:{}},'./install-view':{InstallUI}});
 const ui=new DownloadUI(()=>{},()=>{},()=>false,()=> 'macos');
 assert.equal(ui.canInstall({assetName:'Claude-mac-universal.dmg'}),true);
 assert.equal(ui.canInstall({assetName:'Claude-win-x64.msix'}),false);
 assert.doesNotMatch(ui.taskView({id:'windows',assetName:'Claude-win-x64.msix',stage:'ready',downloaded:100,total:100}),/install-prepare/);
});
