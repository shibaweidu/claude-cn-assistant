// Claude 中文助手：仅汉化页面；不安装扩展、权限处理器或语音功能。
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const electron = require('electron');
const source = fs.readFileSync(path.join(__dirname, 'page-shim.js'), 'utf8');
const armed = new WeakSet();
const target = url => {try {const u=new URL(url);return (u.protocol==='app:'&&u.hostname==='localhost')||(u.protocol==='https:'&&/^(?:[^.]+\.)*claude\.(ai|com)$/.test(u.hostname));}catch{return false;}};
function watch(wc) {
  if (!wc || wc.isDestroyed()) return;
  if (!['window','webview'].includes(wc.getType())) return;
  const arm = async () => {
    if (wc.isDestroyed() || !target(wc.getURL()) || armed.has(wc)) return;
    if (wc.session!==electron.session.defaultSession) return;
    try {
      if (wc.debugger.isAttached()) return;
      wc.debugger.attach('1.3');
      await wc.debugger.sendCommand('Page.enable');
      await wc.debugger.sendCommand('Page.addScriptToEvaluateOnNewDocument',{source});
      await wc.debugger.sendCommand('Runtime.evaluate',{expression:source});
      armed.add(wc);
      // 已加载的页面需要重新读取语言。最多重载一次，不产生循环。
      if (!wc.isLoading()) wc.reload();
      globalThis.__claudeCnPages=(globalThis.__claudeCnPages||0)+1;
    } catch(error) {fs.appendFileSync(path.join(__dirname,'hook.log'),String(error)+'\n');}
  };
  wc.on('did-start-navigation',()=>{void arm()});
  wc.on('did-finish-load',()=>{void arm()});
  void arm();
}
electron.app.on('web-contents-created',(_event,wc)=>watch(wc));
electron.app.whenReady().then(()=>electron.webContents.getAllWebContents().forEach(watch));
globalThis.__claudeCnHookInstalled=true;
