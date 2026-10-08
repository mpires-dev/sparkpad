import {build} from 'esbuild';
import {mkdir,copyFile,readFile} from 'node:fs/promises';
await mkdir(new URL('./dist/',import.meta.url),{recursive:true});
const coverSource=await readFile(new URL('../../src/covers.rs',import.meta.url),'utf8');
const presets=Object.fromEntries([...coverSource.matchAll(/id: "([^"]+)".*?colors: \[0x([a-f0-9]+), 0x([a-f0-9]+)\]/g)].map(m=>[m[1],[`#${m[2]}`,`#${m[3]}`]]));
await build({define:{COVER_PRESETS:JSON.stringify(presets)},entryPoints:[new URL('./src/app.js',import.meta.url).pathname],bundle:true,minify:true,format:'esm',outfile:new URL('./dist/app.js',import.meta.url).pathname,target:'es2022'});
for(const name of ['index.html','style.css','manifest.webmanifest','sw.js']) await copyFile(new URL(`./src/${name}`,import.meta.url),new URL(`./dist/${name}`,import.meta.url));
await copyFile(new URL('../../assets/app/icon.png',import.meta.url),new URL('./dist/icon.png',import.meta.url));
