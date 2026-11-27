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
