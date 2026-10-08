const CACHE='sparkpad-shell-v2-markdown';
const SHELL=['./','./app.js','./style.css','./icon.png','./manifest.webmanifest'];
self.addEventListener('install',e=>e.waitUntil(caches.open(CACHE).then(c=>c.addAll(SHELL)).then(()=>self.skipWaiting())));
self.addEventListener('activate',e=>e.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(k=>k!==CACHE).map(k=>caches.delete(k)))).then(()=>self.clients.claim())));
self.addEventListener('fetch',e=>{const u=new URL(e.request.url);if(u.origin!==self.location.origin||u.pathname.includes('/api/')||e.request.method!=='GET')return;e.respondWith(fetch(e.request).then(response=>{if(response.ok&&SHELL.some(path=>new URL(path,self.registration.scope).href===u.href)){const copy=response.clone();e.waitUntil(caches.open(CACHE).then(cache=>cache.put(e.request,copy)));}return response;}).catch(()=>caches.match(e.request).then(r=>r||caches.match('./'))));});
