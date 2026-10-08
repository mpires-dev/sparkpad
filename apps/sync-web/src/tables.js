import DOMPurify from 'dompurify';
import {marked} from 'marked';

function ranges(raw) {
 let offset=0;const rows=[];
 for(const [index,line] of raw.trimEnd().split('\n').entries()){
  if(index===1){offset+=line.length+1;continue;}
  const cuts=[-1];let slashes=0;
  for(let i=0;i<line.length;i++){const c=line[i];if(c==='|'&&slashes%2===0)cuts.push(i);slashes=c==='\\'?slashes+1:0;}
  cuts.push(line.length);const cells=[];
  for(let i=0;i<cuts.length-1;i++){const a=cuts[i]+1,b=cuts[i+1],text=line.slice(a,b);if((i===0||i===cuts.length-2)&&!text.trim()&&((i===0&&cuts[1]<line.length)||(i>0&&cuts[i]>=0)))continue;
   const start=offset+a+text.length-text.trimStart().length;cells.push({start,end:start+text.trim().length,raw:text.trim()});}
  rows.push(cells);offset+=line.length+1;
 }
 return rows;
}
function escapeText(text){return text.replace(/\\/g,'\\\\').replace(/([`*_\[\]<>~|])/g,'\\$1').replace(/\r?\n/g,'<br>');}
function inlineMarkdown(node){
 if(node.nodeType===Node.TEXT_NODE)return escapeText(node.textContent);
 if(node.nodeType!==Node.ELEMENT_NODE)return '';
 const children=()=>[...node.childNodes].map(inlineMarkdown).join('');
 switch(node.tagName){case 'STRONG':case 'B':return `**${children()}**`;case 'EM':case 'I':return `*${children()}*`;case 'DEL':case 'S':return `~~${children()}~~`;case 'CODE':{const value=node.textContent;const longest=Math.max(0,...[...value.matchAll(/`+/g)].map(m=>m[0].length));const fence='`'.repeat(longest+1);return `${fence} ${value.replace(/\|/g,'\\|')} ${fence}`;}case 'A':{const href=node.getAttribute('href')||'';return /^https?:|^mailto:|^#/.test(href)?`[${children()}](${href.replace(/ /g,'%20').replace(/\)/g,'%29')})`:children();}case 'IMG':{const url=node.dataset.imageSource||node.getAttribute('src')||'';return /^https?:|^asset:[a-f0-9]{64}\./.test(url)?`![${escapeText(node.alt||'')}](${url.replace(/ /g,'%20').replace(/\)/g,'%29')})`:escapeText(node.alt||'');}case 'BR':return '<br>';case 'DIV':case 'P':return `${children()}<br>`;default:return children();}
}
function serialize(rows,align){
 const lines=rows.map(row=>`| ${row.join(' | ')} |`);lines.splice(1,0,`| ${align.map(a=>({left:':---',right:'---:',center:':---:'}[a]||'---')).join(' | ')} |`);return lines.join('\n')+'\n';
}
/** Bind visual table cells without rebuilding their DOM on each local keystroke. */
export function editableTables(preview,source,adapter){
 const tokens=[];marked.walkTokens(marked.lexer(source),t=>{if(t.type==='table')tokens.push(t);});let search=0;
 [...preview.querySelectorAll('table')].forEach((table,ordinal)=>{
  const original=tokens[ordinal];if(!original)return;const start=source.indexOf(original.raw,search);if(start<0)return;search=start+original.raw.length;
  const anchor=adapter.anchor(start);
  const current=()=>{const body=adapter.body(),offset=adapter.position(anchor);if(offset===null)return null;const token=marked.lexer(body.slice(offset))[0];if(token?.type!=='table')return null;return {body,offset,token,rows:ranges(token.raw)};};
  const frame=document.createElement('div');frame.className='table-frame';table.replaceWith(frame);const scroll=document.createElement('div');scroll.className='table-scroll';scroll.append(table);frame.append(scroll);
  const cells=[...table.rows].map(row=>[...row.cells]);
  cells.forEach((row,r)=>row.forEach((cell,c)=>{
   const editor=document.createElement('div');editor.className='table-cell-editor';editor.contentEditable='true';editor.role='textbox';editor.setAttribute('aria-label',`${r===0?'Header':'Row '+r}, column ${c+1}`);editor.dataset.table=ordinal;editor.dataset.row=r;editor.dataset.column=c;editor.innerHTML=cell.innerHTML;cell.replaceChildren(editor);
   editor.addEventListener('input',()=>{const state=current();if(!state)return;const raw=[...editor.childNodes].map(inlineMarkdown).join('').replace(/<br><br>$/,'<br>');const target=state.rows[r]?.[c];
    if(target){adapter.replace(state.offset+target.start,target.end-target.start,raw,true);}else{const rows=state.rows.map(row=>Array.from({length:state.token.header.length},(_,i)=>row[i]?.raw||''));if(!rows[r])return;rows[r][c]=raw;adapter.replace(state.offset,state.token.raw.length,serialize(rows,state.token.align),true);}
   });
   editor.addEventListener('paste',event=>{event.preventDefault();const html=event.clipboardData.getData('text/html');if(html){const safe=DOMPurify.sanitize(html,{ALLOWED_TAGS:['strong','b','em','i','del','s','code','a','br'],ALLOWED_ATTR:['href']});document.execCommand('insertHTML',false,safe);}else document.execCommand('insertText',false,event.clipboardData.getData('text/plain'));});
   editor.addEventListener('keydown',event=>{
    if(event.key==='Tab'||(event.key==='Enter'&&!event.shiftKey)){event.preventDefault();const cols=cells[0].length;let nr=r,nc=c;
     if(event.key==='Enter'){nr++;}else{const next=r*cols+c+(event.shiftKey?-1:1);if(next<0)return;nr=Math.floor(next/cols);nc=next%cols;}
     if(nr>=cells.length){structure('row',null,{r:nr,c:nc});}else cells[nr][nc].querySelector('.table-cell-editor').focus();
    }
    if((event.metaKey||event.ctrlKey)&&['b','i'].includes(event.key.toLowerCase())){event.preventDefault();document.execCommand(event.key.toLowerCase()==='b'?'bold':'italic');}
    if((event.metaKey||event.ctrlKey)&&event.key.toLowerCase()==='z'){event.preventDefault();adapter.history(event.shiftKey);}
   });
  }));
  const structure=(operation,index,focus)=>{
   const state=current();if(!state)return;const align=[...state.token.align],cols=state.token.header.length,rows=state.rows.map(row=>Array.from({length:cols},(_,i)=>row[i]?.raw||''));
   if(operation==='row')rows.push(Array(cols).fill(''));
   if(operation==='column'){align.push(null);rows.forEach(row=>row.push(''));}
   if(operation==='remove-row'&&index>0)rows.splice(index,1);
   if(operation==='remove-column'){if(cols===1)return;align.splice(index,1);rows.forEach(row=>row.splice(index,1));}
   adapter.replace(state.offset,state.token.raw.length,serialize(rows,align),false);
   if(focus)preview.querySelector(`[data-table="${ordinal}"][data-row="${focus.r}"][data-column="${focus.c}"]`)?.focus();
  };
  cells[0].forEach((cell,c)=>{const button=document.createElement('button');button.type='button';button.className='table-remove-column';button.textContent='−';button.title='Remove column';button.setAttribute('aria-label',`Remove column ${c+1}`);button.disabled=cells[0].length===1;button.addEventListener('click',()=>structure('remove-column',c));cell.append(button);});
  [...table.rows].forEach((row,r)=>{const cell=row.insertCell();cell.className='table-row-control';if(r>0){const button=document.createElement('button');button.type='button';button.textContent='−';button.setAttribute('aria-label',`Remove row ${r}`);button.title='Remove row';button.addEventListener('click',()=>structure('remove-row',r));cell.append(button);}});
  const tools=document.createElement('div');tools.className='table-tools';for(const [op,label]of [['row','+ Row'],['column','+ Column']]){const button=document.createElement('button');button.type='button';button.textContent=label;button.addEventListener('click',()=>structure(op));tools.append(button);}frame.append(tools);
 });
}
export function tableFocus(preview){const cell=document.activeElement;if(!cell?.matches('.table-cell-editor')||!preview.contains(cell))return null;const selection=getSelection();let offset=0;if(selection?.rangeCount&&cell.contains(selection.anchorNode)){const range=selection.getRangeAt(0).cloneRange();range.selectNodeContents(cell);range.setEnd(selection.anchorNode,selection.anchorOffset);offset=range.toString().length;}return {table:cell.dataset.table,row:cell.dataset.row,column:cell.dataset.column,offset};}
export function restoreTableFocus(preview,focus){if(!focus)return;const cell=preview.querySelector(`[data-table="${focus.table}"][data-row="${focus.row}"][data-column="${focus.column}"]`);if(!cell)return;cell.focus();const walk=document.createTreeWalker(cell,NodeFilter.SHOW_TEXT);let node,offset=focus.offset;while((node=walk.nextNode())){if(offset<=node.length){const range=document.createRange();range.setStart(node,offset);range.collapse(true);getSelection().removeAllRanges();getSelection().addRange(range);return;}offset-=node.length;}}
