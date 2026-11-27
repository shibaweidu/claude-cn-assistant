'use strict';
// 由 Rust 传入路径，不读取或修改 Claude 用户配置。
const {spawn}=require('node:child_process');
const path=require('node:path');
const net=require('node:net');
const fs=require('node:fs');
const app=process.env.CLAUDE_CN_MANAGED_EXE;
if(!app)throw new Error('未指定受管副本');
const expected=fs.realpathSync(app);
const sameExecutable=(a,b)=>process.platform==='win32'?a.toLowerCase()===b.toLowerCase():a===b;
const hook=path.join(__dirname,'hook.cjs');
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function main(){
  if(Number(process.versions.node.split('.')[0])<22)throw new Error('启动注入需要 Node.js 22 或更新版本');
  const server=net.createServer();await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve)});
  const port=server.address().port;await new Promise(resolve=>server.close(resolve));
  const child=spawn(app,[`--inspect=127.0.0.1:${port}`,'--lang=zh-Hans'],{detached:true,stdio:'ignore',windowsHide:false});
  let childError;child.on('error',error=>{childError=error});child.unref();
  let ws;let verified=false;let completed=false;
  try {
    const deadline=Date.now()+35000;let endpoint;
    while(Date.now()<deadline){
      if(childError)throw childError;
      try {const response=await fetch(`http://127.0.0.1:${port}/json/list`,{signal:AbortSignal.timeout(1200)});const targets=await response.json();endpoint=targets.find(t=>t.type==='node')?.webSocketDebuggerUrl;if(endpoint)break;}catch{}
      await delay(250);
    }
    if(!endpoint)throw new Error('未能连接受管副本，请完全退出 Claude 后重试。');
    const address=new URL(endpoint);if(address.hostname!=='127.0.0.1'||Number(address.port)!==port)throw new Error('启动调试地址不匹配');
    ws=new WebSocket(endpoint);await new Promise((resolve,reject)=>{const timeout=setTimeout(()=>reject(new Error('连接超时')),5000);ws.addEventListener('open',()=>{clearTimeout(timeout);resolve()},{once:true});ws.addEventListener('error',()=>reject(new Error('连接失败')),{once:true})});
    let id=0;
    const evaluate=expression=>new Promise((resolve,reject)=>{
      const current=++id;const timeout=setTimeout(()=>{ws.removeEventListener('message',onMessage);reject(new Error('注入超时'))},5000);
      function onMessage(event){const value=JSON.parse(event.data);if(value.id!==current)return;clearTimeout(timeout);ws.removeEventListener('message',onMessage);if(value.error||value.result?.exceptionDetails)reject(new Error('受管副本注入失败：'+JSON.stringify(value.error||value.result?.exceptionDetails)));else resolve(value.result?.result?.value)}
      ws.addEventListener('message',onMessage);ws.send(JSON.stringify({id:current,method:'Runtime.evaluate',params:{expression,returnByValue:true,awaitPromise:true}}));
    });
    const actual=await evaluate('process.getBuiltinModule("fs").realpathSync(process.execPath)');
    if(!sameExecutable(actual,expected))throw new Error('连接进程不是受管副本，已停止注入');verified=true;
    if(!await evaluate(`process.getBuiltinModule("module").createRequire(process.resourcesPath+"/app.asar/package.json")(${JSON.stringify(hook)});globalThis.__claudeCnHookInstalled===true`))throw new Error('汉化 hook 未就绪');
    let pages=0;for(let i=0;i<24;i++){pages=await evaluate('globalThis.__claudeCnPages||0');if(pages)break;await delay(500)}
    if(!pages)throw new Error('副本已启动，但未确认目标页面汉化注入；请检查 hook.log');
    // 在确认身份后关闭调试入口，随后断开。即使确认页面失败也会进入 finally。
    console.log('汉化启动注入已确认，页面数量：'+pages);
    await evaluate('setTimeout(()=>process.getBuiltinModule("inspector").close(),50);true').catch(()=>{});
    completed=true;
  } finally {
    if(ws&&verified&&ws.readyState===WebSocket.OPEN){ws.send(JSON.stringify({id:99999,method:'Runtime.evaluate',params:{expression:'setTimeout(()=>process.getBuiltinModule("inspector").close(),30);true'}}));await delay(100)}
    ws?.close();
    if(!completed){try{child.kill()}catch{}}
  }
}
main().catch(error=>{console.error(error.message);process.exitCode=1});
