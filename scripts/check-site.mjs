import {readFile,stat} from 'node:fs/promises';
import {resolve,dirname,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
import {pages} from '../site/content.mjs';
const root=resolve(fileURLToPath(new URL('../site-dist/',import.meta.url)));
const titles=new Set(),descriptions=new Set();
for(const page of pages){
 const file=resolve(root,page.slug,'index.html');const html=await readFile(file,'utf8');
 assert.equal((html.match(/<h1[ >]/g)||[]).length,1,`${page.slug}: one H1`);
 assert(!titles.has(page.title),'duplicate title');titles.add(page.title);
 assert(!descriptions.has(page.description),'duplicate description');descriptions.add(page.description);
 assert(html.includes('rel="canonical"'),'canonical missing');
 JSON.parse(html.match(/<script type="application\/ld\+json">(.*?)<\/script>/s)[1]);
 for(const [,url] of html.matchAll(/(?:href|src)="([^"]+)"/g)){
  if(/^(https?:|#|mailto:)/.test(url))continue;
  const path=resolve(dirname(file),decodeURIComponent(url));assert(path===root||path.startsWith(root+sep),'link escaped root');
  const candidate=url.endsWith('/')?resolve(path,'index.html'):path;
  await stat(candidate).catch(()=>{throw Error(`Broken link ${url} in ${page.slug||'home'}`)});
 }
}
const sitemap=await readFile(resolve(root,'sitemap.xml'),'utf8');
assert.equal((sitemap.match(/<loc>/g)||[]).length,pages.length);
assert((await readFile(resolve(root,'robots.txt'),'utf8')).includes('sitemap.xml'));
console.log(`Validated ${pages.length} pages: unique metadata, structured data, local links and sitemap.`);
