import {editableTables,tableFocus,restoreTableFocus} from './tables';
import {marked} from 'marked';
import DOMPurify from 'dompurify';
import {digest as sha256} from 'lib0/hash/sha256';
import * as Y from 'yjs';
import {IndexeddbPersistence} from 'y-indexeddb';
const $=id=>document.getElementById(id), LOCAL='sparkpad-local',REMOTE='sparkpad-remote';
const base=new URL('./',location.href), namespace=`sparkpad:${base.href}`;
const docs=new Map(),loaders=new Map(),requestIds=new Set();
let editingTable=false;
let credentials,workspace,values,socket,authenticated=false,selected=null,undo,refreshFrame,retry=500,reconnectTimer;
const b64=bytes=>{let s='';for(let i=0;i<bytes.length;i+=8192)s+=String.fromCharCode(...bytes.subarray(i,i+8192));return btoa(s);};
const bytes=s=>Uint8Array.from(atob(s),c=>c.charCodeAt(0));
const valid=id=>id==='workspace'||/^[a-f0-9-]{36}$/i.test(id);
const field=(id,key)=>values?.get(`note/${id}/${key}`)||'';
function send(message){if(authenticated&&socket?.readyState===WebSocket.OPEN)socket.send(JSON.stringify(message));}
function request(id){if(!requestIds.has(id)&&docs.has(id)){requestIds.add(id);send({type:'sync',doc:id,vector:b64(Y.encodeStateVector(docs.get(id)))});}}
async function load(id){
 if(loaders.has(id))return loaders.get(id);if(docs.has(id))return docs.get(id);
 const promise=(async()=>{
  const doc=new Y.Doc();docs.set(id,doc);
  const persistence=new IndexeddbPersistence(`${namespace}:${id}`,doc);
  let selection;
  doc.on('beforeTransaction',tx=>{
   selection=null;if(selected!==id||tx.origin===LOCAL)return;
   const element=document.activeElement;if(element!==$('body')&&element!==$('title'))return;
   const text=doc.getText(element===$('body')?'body':'title');
   selection={element,start:Y.createRelativePositionFromTypeIndex(text,Math.min(element.selectionStart,text.length)),end:Y.createRelativePositionFromTypeIndex(text,Math.min(element.selectionEnd,text.length))};
  });
  doc.on('afterTransaction',()=>{
   if(id==='workspace')scheduleRefresh();
   else {scheduleRefresh();if(selected===id){renderText();if(selection){const start=Y.createAbsolutePositionFromRelativePosition(selection.start,doc),end=Y.createAbsolutePositionFromRelativePosition(selection.end,doc);if(start&&end)selection.element.setSelectionRange(start.index,end.index);}}}
  });
  doc.on('update',(data,origin)=>{if(origin===LOCAL||origin===undo){request(id);send({type:'update',doc:id,data:b64(data)});}});
  await persistence.whenSynced;return doc;
 })();loaders.set(id,promise);return promise;
}
function scheduleRefresh(){if(!refreshFrame)refreshFrame=requestAnimationFrame(()=>{refreshFrame=null;renderSidebar();if(selected)renderAppearance(selected);if(selected&&field(selected,'deleted')==='true'){selected=null;$('editor').hidden=true;$('empty').hidden=false;}});}
function noteIds(){const ids=new Set();for(const key of values?.keys()||[]){const m=/^note\/([^/]+)\//.exec(key);if(m&&valid(m[1])&&field(m[1],'deleted')!=='true')ids.add(m[1]);}return [...ids];}
function renderSidebar(){
 if(!values)return;const nav=$('notes');nav.replaceChildren();const ids=noteIds();
 for(const id of ids){if(!docs.has(id))load(id).then(()=>{request(id);scheduleRefresh();});}
 const sort=(a,b)=>Number(field(a,'position')||Number.MAX_SAFE_INTEGER)-Number(field(b,'position')||Number.MAX_SAFE_INTEGER)||a.localeCompare(b);
 const visited=new Set();
 function branch(id,depth){if(visited.has(id))return;visited.add(id);const button=document.createElement('button');button.className=id===selected?'active':'';button.style.paddingLeft=`${12+Math.min(depth,6)*16}px`;
  const icon=document.createElement('span');icon.textContent=displayIcon(field(id,'icon'));const title=document.createElement('span');title.textContent=docs.get(id)?.getText('title').toString()||'Untitled';button.append(icon,title);button.addEventListener('click',()=>openNote(id));nav.append(button);ids.filter(child=>field(child,'parent')===id).sort(sort).forEach(child=>branch(child,depth+1));}
 const roots=ids.filter(id=>!ids.includes(field(id,'parent')));
 roots.filter(id=>!field(id,'group')).sort(sort).forEach(id=>branch(id,0));
 const groups=new Set();for(const key of values.keys()){const m=/^group\/([^/]+)\//.exec(key);if(m&&values.get(`group/${m[1]}/deleted`)!=='true')groups.add(m[1]);}
 for(const group of [...groups].sort((a,b)=>Number(values.get(`group/${a}/position`)||0)-Number(values.get(`group/${b}/position`)||0))){const heading=document.createElement('div');heading.className='group';heading.textContent=values.get(`group/${group}/title`)||'Projects';nav.append(heading);roots.filter(id=>field(id,'group')===group).sort(sort).forEach(id=>branch(id,0));}
 ids.filter(id=>!visited.has(id)).sort(sort).forEach(id=>branch(id,0));
 if(!selected&&ids.length){const previous=localStorage.getItem(`${namespace}:selected`);openNote(ids.includes(previous)?previous:ids[0]);}
}
function displayIcon(icon){return icon&&!icon.startsWith('sparkpad:')&&!icon.startsWith('asset:')?icon:'📄';}
async function openNote(id){
 if(field(id,'deleted')==='true')return;selected=id;localStorage.setItem(`${namespace}:selected`,id);
 const doc=await load(id);if(selected!==id)return;
 request(id);undo?.destroy();undo=new Y.UndoManager([doc.getText('body'),doc.getText('title')],{trackedOrigins:new Set([LOCAL])});
 $('empty').hidden=true;$('editor').hidden=false;$('sidebar').classList.remove('open');renderText();renderSidebar();renderAppearance(id);$('document').scrollTop=0;
}
function renderText(){const doc=docs.get(selected);if(!doc)return;const body=doc.getText('body').toString(),title=doc.getText('title').toString();if($('body').value!==body)$('body').value=body;if($('title').value!==title)$('title').value=title;resize();if(!$('preview').hidden&&!editingTable)renderPreview(body);}
function resize(){$('body').style.height='auto';$('body').style.height=`${Math.max(380,$('body').scrollHeight)}px`;}
function editText(text,newValue){const old=text.toString();if(old===newValue)return;let start=0;while(start<old.length&&start<newValue.length&&old[start]===newValue[start])start++;if(start&&old.charCodeAt(start)>=0xdc00&&old.charCodeAt(start)<=0xdfff)start--;
 let endOld=old.length,endNew=newValue.length;while(endOld>start&&endNew>start&&old[endOld-1]===newValue[endNew-1]){endOld--;endNew--;}
 if(endOld<old.length&&old.charCodeAt(endOld)>=0xdc00&&old.charCodeAt(endOld)<=0xdfff){endOld++;endNew++;}
 if(endOld>start)text.delete(start,endOld-start);if(endNew>start)text.insert(start,newValue.slice(start,endNew));}
for(const id of ['title','body'])$(id).addEventListener('input',()=>{const doc=docs.get(selected);if(doc)doc.transact(()=>editText(doc.getText(id),$(id).value),LOCAL);resize();});
$('body').addEventListener('keydown',e=>{if(e.key==='Tab'){e.preventDefault();const input=$('body'),start=input.selectionStart,end=input.selectionEnd;input.setRangeText('  ',start,end,'end');input.dispatchEvent(new Event('input'));}});
document.addEventListener('keydown',e=>{if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='z'&&undo&&['body','title'].includes(document.activeElement?.id)){e.preventDefault();if(e.shiftKey)undo.redo();else undo.undo();}});
$('create').addEventListener('click',async()=>{const id=crypto.randomUUID?.()||'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g,c=>{const r=crypto.getRandomValues(new Uint8Array(1))[0]&15;return(c==='x'?r:(r&3)|8).toString(16);});const doc=await load(id);doc.transact(()=>doc.getText('title').insert(0,'Untitled'),LOCAL);workspace.transact(()=>{for(const [k,v]of Object.entries({deleted:'false',parent:'',group:'',icon:'',cover:'',position:String(Date.now()),created:String(Math.floor(Date.now()/1000))}))values.set(`note/${id}/${k}`,v);},LOCAL);openNote(id);});
$('remove').addEventListener('click',()=>{if(!selected||!confirm('Delete this page and its subpages from all connected devices?'))return;const deleting=new Set([selected]);let changed=true;while(changed){changed=false;for(const id of noteIds())if(!deleting.has(id)&&deleting.has(field(id,'parent'))){deleting.add(id);changed=true;}}workspace.transact(()=>{for(const id of deleting)values.set(`note/${id}/deleted`,'true');},LOCAL);scheduleRefresh();});
$('menu').addEventListener('click',()=>$('sidebar').classList.toggle('open'));
function tab(preview){$('body').hidden=preview;$('preview').hidden=!preview;$('edit-tab').setAttribute('aria-pressed',String(!preview));$('preview-tab').setAttribute('aria-pressed',String(preview));if(preview)renderPreview($('body').value);}
$('edit-tab').addEventListener('click',()=>tab(false));$('preview-tab').addEventListener('click',()=>tab(true));
const escapeHTML=s=>String(s||'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
marked.use({renderer:{image({href,title,text}){
 const safe=/^https?:\/\//.test(href)||/^asset:[a-f0-9]{64}\.(png|jpg|jpeg|gif|webp|svg)$/.test(href);
 if(!safe)return escapeHTML(text);
 return `<img data-image-source="${escapeHTML(href)}" alt="${escapeHTML(text)}" title="${escapeHTML(title)}">`;
},tablecell({tokens,header,align}){const tag=header?'th':'td';return `<${tag}${['left','center','right'].includes(align)?` align="${align}"`:''}>${this.parser.parseInline(tokens)}</${tag}>`;},checkbox({checked}){return `<input type="checkbox" data-sparkpad-task="true" ${checked?'checked':''}>`;}}});
function taskPositions(source){const result=[];let fence=null,offset=0;for(const line of source.split('\n')){const match=/^\s{0,3}(`{3,}|~{3,})/.exec(line);if(match){if(!fence)fence=match[1];else if(match[1][0]===fence[0]&&match[1].length>=fence.length)fence=null;}else if(!fence&&/^\s*[-*+\d.)]+ \[[ xX]\] /.test(line)){result.push(offset+line.indexOf('[')+1);}offset+=line.length+1;}return result;}
function renderPreview(source){
 const preview=$('preview'),focus=tableFocus(preview);preview.innerHTML=DOMPurify.sanitize(marked.parse(source,{gfm:true}),{USE_PROFILES:{html:true},FORBID_ATTR:['style'],FORBID_TAGS:['iframe','object','form']});
 const positions=taskPositions(source);
 preview.querySelectorAll('[data-sparkpad-task]').forEach((input,index)=>{input.addEventListener('change',()=>{const doc=docs.get(selected),text=doc.getText('body'),position=positions[index];if(position===undefined||text.toString()!==source){renderPreview(text.toString());return;}doc.transact(()=>{text.delete(position,1);text.insert(position,input.checked?'x':' ');},LOCAL);});});
 const doc=docs.get(selected),text=doc.getText('body');
 editableTables(preview,source,{
  anchor:index=>Y.createRelativePositionFromTypeIndex(text,index),
  position:anchor=>Y.createAbsolutePositionFromRelativePosition(anchor,doc)?.index??null,
  body:()=>text.toString(),
  replace:(start,length,value,typing)=>{if(selected!==doc.guid&&docs.get(selected)!==doc)return;editingTable=typing;try{if(!typing)undo?.stopCapturing();doc.transact(()=>{text.delete(start,length);text.insert(start,value);},LOCAL);if(!typing)undo?.stopCapturing();}finally{editingTable=false;}},
  history:redo=>{if(redo)undo?.redo();else undo?.undo();}
 });
 let imageOrdinal=0;
 for(const image of preview.querySelectorAll('[data-image-source]')){
  const ordinal=imageOrdinal++;
  const url=image.dataset.imageSource,width=Math.max(10,Math.min(100,Number(/sparkpad-width=(\d+)/.exec(image.title)?.[1]||100)));image.style.width=`${width}%`;image.title=image.title.replace(/(?:^|\s)sparkpad-width=\d+/g,'').trim();image.loading='lazy';
  if(url.startsWith('asset:'))imageAsset(url).then(src=>{if(src&&image.isConnected)image.src=src;});else image.src=url;
  if(image.closest('.table-cell-editor'))continue;
  const frame=document.createElement('figure');image.replaceWith(frame);frame.append(image);
  if(image.alt){const caption=document.createElement('figcaption');caption.textContent=image.alt;frame.append(caption);}
  const handle=document.createElement('input');handle.type='range';handle.min='10';handle.max='100';handle.value=String(width);handle.setAttribute('aria-label','Image width');handle.title='Resize image';handle.className='image-resize';frame.append(handle);
  handle.addEventListener('input',()=>image.style.width=`${handle.value}%`);
  handle.addEventListener('change',()=>{const doc=docs.get(selected),text=doc.getText('body');if(text.toString()!==source){renderPreview(text.toString());return;}
    const tokens=marked.lexer(source);const images=[];marked.walkTokens(tokens,t=>{if(t.type==='image')images.push(t);});const token=images[ordinal];if(!token)return;
    const start=source.indexOf(token.raw);if(start<0)return;const title=(token.title||'').replace(/(?:^|\s)sparkpad-width=\d+/g,'').trim();const next=`![${token.text.replace(/]/g,'\\]')}](${url} "${(title?title+' ':'').replace(/\\/g,'\\\\').replace(/"/g,'\\"')}sparkpad-width=${handle.value}")`;
    undo?.stopCapturing();doc.transact(()=>{text.delete(start,token.raw.length);text.insert(start,next);},LOCAL);undo?.stopCapturing();
  });
 }
 restoreTableFocus(preview,focus);
}
const assetURLs=new Map();let imageDBPromise;
function imageDB(){return imageDBPromise||=(new Promise((resolve,reject)=>{const request=indexedDB.open(`${namespace}:images`,1);request.onupgradeneeded=()=>request.result.createObjectStore('images',{keyPath:'name'});request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);}));}
async function imageStorage(method,value){const db=await imageDB();return new Promise((resolve,reject)=>{const tx=db.transaction('images',method==='put'?'readwrite':'readonly'),store=tx.objectStore('images');let result;const req=method==='put'?store.put(value):method==='all'?store.getAll():store.get(value);req.onsuccess=()=>result=req.result;tx.oncomplete=()=>resolve(result);tx.onerror=()=>reject(tx.error);});}
async function flushImages(){for(const entry of await imageStorage('all'))if(entry.pending){const r=await fetch(new URL(`api/assets/${entry.name}`,base),{method:'PUT',headers:{Authorization:`Bearer ${credentials.token}`},body:entry.blob});if(!r.ok)throw Error('Cannot upload image');await imageStorage('put',{...entry,pending:false});}}
async function imageAsset(value){const name=value.slice(6);if(!/^[a-f0-9]{64}\.(png|jpg|jpeg|gif|webp|svg)$/.test(name))return;
 if(assetURLs.has(name))return assetURLs.get(name);const cached=await imageStorage('get',name);if(cached){const url=URL.createObjectURL(cached.blob);assetURLs.set(name,url);return url;}
 try{const response=await fetch(new URL(`api/assets/${name}`,base),{headers:{Authorization:`Bearer ${credentials.token}`}});if(response.ok){const raw=await response.blob(),ext=name.split('.').pop(),blob=new Blob([raw],{type:({svg:'image/svg+xml',png:'image/png',jpg:'image/jpeg',jpeg:'image/jpeg',gif:'image/gif',webp:'image/webp'})[ext]});await imageStorage('put',{name,blob,pending:false});const url=URL.createObjectURL(blob);assetURLs.set(name,url);return url;}}catch{}
}
$('insert-image').addEventListener('click',()=>$('image-file').click());
$('image-file').addEventListener('change',async()=>{const file=$('image-file').files[0];$('image-file').value='';if(!file)return;try{
 if(file.size>10_000_000)throw Error('Images must be smaller than 10 MB.');const ext=file.name.split('.').pop().toLowerCase();if(!['png','jpg','jpeg','webp','gif','svg'].includes(ext))throw Error('Use PNG, JPG, WebP, GIF or SVG.');
 const id=selected,body=docs.get(id).getText('body'),range=[Y.createRelativePositionFromTypeIndex(body,$('body').selectionStart),Y.createRelativePositionFromTypeIndex(body,$('body').selectionEnd)],data=new Uint8Array(await file.arrayBuffer()),name=Array.from(sha256(data),b=>b.toString(16).padStart(2,'0')).join('')+'.'+ext;
 await imageStorage('put',{name,blob:file,pending:true});if(authenticated){try{await flushImages();}catch{socket?.close();}}
 const caption=file.name.replace(/\.[^.]+$/,'').replace(/[\[\]\n]/g,'');undo?.stopCapturing();docs.get(id).transact(()=>{const start=Y.createAbsolutePositionFromRelativePosition(range[0],docs.get(id))?.index??body.length,end=Y.createAbsolutePositionFromRelativePosition(range[1],docs.get(id))?.index??start;if(end>start)body.delete(start,end-start);body.insert(start,`\n\n![${caption}](asset:${name})\n\n`);},LOCAL);undo?.stopCapturing();
 }catch(error){alert(error.message||'Cannot insert image');}});
async function renderAppearance(id){const icon=field(id,'icon'),cover=field(id,'cover');$('page-icon').textContent=displayIcon(icon);if(icon.startsWith('asset:')){const url=await imageAsset(icon);if(url&&selected===id){const image=document.createElement('img');image.src=url;image.width=56;image.height=56;image.alt='';image.style.objectFit='contain';$('page-icon').replaceChildren(image);}}
 $('cover').hidden=!cover;if(cover.startsWith('asset:')){const url=await imageAsset(cover);if(url&&selected===id){$('cover').style.backgroundImage=`url("${url}")`;$('cover').style.backgroundSize='cover';}}
 else if(cover){const colors=COVER_PRESETS[cover.replace('sparkpad:cover:','')]||['#38383e','#253f34'];$('cover').style.backgroundImage=`linear-gradient(135deg,${colors[0]},${colors[1]})`;}}
function connectSocket(){
 clearTimeout(reconnectTimer);if(!credentials)return;if(socket){socket.onclose=null;socket.close();}authenticated=false;requestIds.clear();$('status').textContent='Connecting…';const url=new URL('sync',base);url.protocol=url.protocol==='https:'?'wss:':'ws:';socket=new WebSocket(url);
 socket.onopen=()=>socket.send(JSON.stringify({type:'auth',token:credentials.token,version:1}));
 socket.onmessage=async event=>{try{const message=JSON.parse(event.data);if(message.type==='manifest'){await flushImages();authenticated=true;retry=500;$('status').textContent='Synced';for(const id of new Set([...message.docs,...docs.keys()]))if(valid(id)){await load(id);request(id);}scheduleRefresh();}
  else if(message.type==='update'&&valid(message.doc)){const doc=await load(message.doc);Y.applyUpdate(doc,bytes(message.data),REMOTE);request(message.doc);}
  else if(message.type==='sync'&&valid(message.doc)){const doc=await load(message.doc),data=Y.encodeStateAsUpdate(doc,bytes(message.vector));if(data.length>2)send({type:'update',doc:message.doc,data:b64(data)});}
  else if(message.type==='ping')send({type:'pong'});
  else if(message.type==='error'){$('status').textContent='Sync error · saved locally';socket.close();}
 }catch{$('status').textContent='Sync error · saved locally';socket.close();}};
 socket.onclose=()=>{authenticated=false;if(!credentials)return;$('status').textContent='Offline · saved locally';reconnectTimer=setTimeout(connectSocket,retry);retry=Math.min(retry*2,15000);};
 socket.onerror=()=>socket.close();
}
async function start(config){credentials=config;workspace=await load('workspace');values=workspace.getMap('values');$('login').hidden=true;$('workspace').hidden=false;renderSidebar();connectSocket();}
$('connect').addEventListener('submit',async e=>{e.preventDefault();const token=$('key').value.trim();$('login-error').textContent='';try{const response=await fetch(new URL('api/session',base),{headers:{Authorization:`Bearer ${token}`},signal:AbortSignal.timeout(8000)});if(!response.ok)throw Error('Check your connection key and try again.');const config={token};localStorage.setItem(`${namespace}:connection`,JSON.stringify(config));await start(config);}catch(error){$('login-error').textContent=error.message||'Cannot connect to the server.';}});
$('disconnect').addEventListener('click',()=>{credentials=null;clearTimeout(reconnectTimer);socket?.close();localStorage.removeItem(`${namespace}:connection`);location.reload();});
window.addEventListener('online',()=>{if(credentials&&!authenticated)connectSocket();});
try{const saved=JSON.parse(localStorage.getItem(`${namespace}:connection`));if(saved?.token)start(saved);}catch{}
if('serviceWorker'in navigator&&window.isSecureContext)navigator.serviceWorker.register(new URL('sw.js',base)).catch(()=>{});
