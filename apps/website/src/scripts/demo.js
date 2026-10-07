const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const dictionary = JSON.parse($('#translations').textContent);
const t = key => dictionary[key] ?? key;
function localizeDemo(root) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const node = walker.currentNode;
    const source = node.textContent.trim();
    if (Object.hasOwn(dictionary, source)) node.textContent = node.textContent.replace(source, t(source));
  }
  $$('[aria-label], [data-placeholder]', root).forEach(element => {
    for (const attr of ['aria-label','data-placeholder']) {
      const value = element.getAttribute(attr); if (value) element.setAttribute(attr, t(value));
    }
  });
}
const preview = $('#app-preview');
const documentArea = $('#demo-document');
const notes = {
  welcome: { emoji: '✳️', title: 'Um lugar para pensar', body: '<p contenteditable="true" spellcheck="false">Algumas ideias precisam de espaço. Outras, só de um lugar para começar. <strong>Este é o seu.</strong></p><h3 contenteditable="true">Um pouco de clareza para o dia</h3><div class="editor-line"><input type="checkbox" checked aria-label="Separar o que importa">Separar o que importa</div><div class="editor-line"><input type="checkbox" aria-label="Dar forma à próxima ideia">Dar forma à próxima ideia</div><div class="editor-line"><input type="checkbox" aria-label="Criar algo que faça sentido">Criar algo que faça sentido</div><p contenteditable="true">Uma nota pode virar um plano. Um plano pode virar um projeto.<br>E o próximo passo pode começar com <code>uma linha</code>.</p>' },
  ideas: { emoji: '💡', title: 'Ideias em movimento', body: '<p contenteditable="true">Um espaço para todas aquelas ideias que aparecem no meio do caminho.</p><h3 contenteditable="true">Pequenas ideias. Boas possibilidades.</h3><div class="editor-line" contenteditable="true"><span>01</span> Um app que deixa o pensamento fluir.</div><div class="editor-line" contenteditable="true"><span>02</span> Um lugar para guardar referências e próximos passos.</div><div class="editor-line" contenteditable="true"><span>03</span> Menos organização pela organização. Mais espaço para criar.</div><p contenteditable="true">A próxima ideia não precisa estar pronta para ter um lugar aqui.</p>' },
  launch: { emoji: '🚀', title: 'O próximo lançamento', body: '<p contenteditable="true">Uma ideia saiu do papel. Agora é hora de preparar o próximo passo.</p><h3 contenteditable="true">Antes de colocar no mundo</h3><div class="editor-line"><input type="checkbox" checked aria-label="Encontrar o problema certo">Encontrar o problema certo</div><div class="editor-line"><input type="checkbox" checked aria-label="Construir a primeira versão">Construir a primeira versão</div><div class="editor-line"><input type="checkbox" aria-label="Conversar com quem vai usar">Conversar com quem vai usar</div><div class="editor-line"><input type="checkbox" aria-label="Aprender com o lançamento">Aprender com o lançamento</div><h3 contenteditable="true">Uma etapa de cada vez</h3><p contenteditable="true">Organize as notas em páginas filhas. Deixe o seu agente ajudar a transformar as ideias em um checklist.</p>' },
  code: { emoji: '⌘', title: 'Notas de desenvolvimento', body: `<p contenteditable="true">Contexto, decisões e pequenos trechos de código. Tudo perto do que você está construindo.</p><h3 contenteditable="true">Uma ideia, em TypeScript</h3><pre><code><span class="code-purple">export const</span> Spark = () =&gt; {
  <span class="code-purple">return</span> (
    &lt;<span class="code-yellow">Note</span> local <span class="code-green">title="Uma ideia"</span>&gt;
      ${t('Um pequeno começo.')}
    &lt;/<span class="code-yellow">Note</span>&gt;
  );
};</code></pre><p contenteditable="true">No app, o bloco de código tem seletor de linguagem, cópia e indentação com <code>Tab</code>.</p>` },
};
let activeNote = 'welcome';
let font = 0;
let toastTimer;
function notify(message) { const toast = $('.toast'); toast.textContent = t(message); toast.hidden = false; clearTimeout(toastTimer); toastTimer = setTimeout(() => { toast.hidden = true; }, 2600); }
function rememberNote() { if (notes[activeNote]) { const title = $('h2', documentArea); const body = $('.editable-body', documentArea); if (title && body) { notes[activeNote].title = title.textContent; notes[activeNote].body = body.innerHTML; } } }
function showNote(id) {
  rememberNote();
  activeNote = id;
  const note = notes[id];
  documentArea.innerHTML = `<span class="page-emoji" aria-hidden="true">${note.emoji}</span><h2 contenteditable="true" aria-label="Título da nota de demonstração"></h2><div class="editable-body">${note.body}</div><p class="new-block" contenteditable="true" aria-label="Adicionar texto à demonstração" data-placeholder="Escreva ou digite /…"></p>`;
  $('h2', documentArea).textContent = t(note.title);
  localizeDemo(documentArea);
  $$('.note-item').forEach(button => { const selected = button.dataset.note === id; button.classList.toggle('selected', selected); if (selected) button.setAttribute('aria-current', 'page'); else button.removeAttribute('aria-current'); });
  $('.demo-scroll').scrollTop = 0;
}
function selectMode(mode, focus = false) {
  $$('.mode-tabs [role=tab]').forEach(tab => { const selected = tab.dataset.mode === mode; tab.setAttribute('aria-selected', selected); tab.tabIndex = selected ? 0 : -1; if (selected) { preview.setAttribute('aria-labelledby', tab.id); if (focus) tab.focus(); } });
  showNote(mode === 'write' ? 'welcome' : mode === 'organize' ? 'launch' : 'code');
  $('#mcp-status').textContent = t(mode === 'connect' ? 'MCP · demonstração' : 'MCP disponível');
}
$$('.mode-tabs [role=tab]').forEach((tab, index, tabs) => {
  tab.addEventListener('click', () => selectMode(tab.dataset.mode));
  tab.addEventListener('keydown', event => { let target; if (event.key === 'ArrowRight') target = (index + 1) % tabs.length; if (event.key === 'ArrowLeft') target = (index - 1 + tabs.length) % tabs.length; if (event.key === 'Home') target = 0; if (event.key === 'End') target = tabs.length - 1; if (target !== undefined) { event.preventDefault(); selectMode(tabs[target].dataset.mode, true); } });
});
$$('[data-show-mode]').forEach(link => link.addEventListener('click', () => selectMode(link.dataset.showMode)));
$('.app-sidebar').addEventListener('click', event => { const item = event.target.closest('[data-note]'); if (item) showNote(item.dataset.note); });
$$('.group-heading').forEach(button => button.addEventListener('click', () => { const open = button.getAttribute('aria-expanded') === 'true'; button.setAttribute('aria-expanded', !open); document.getElementById(button.getAttribute('aria-controls')).hidden = open; }));
$('.sidebar-toggle').addEventListener('click', () => { preview.classList.add('sidebar-closed'); preview.classList.remove('mobile-sidebar'); $('.restore-sidebar').hidden = false; });
$('.restore-sidebar').addEventListener('click', () => { preview.classList.remove('sidebar-closed'); preview.classList.add('mobile-sidebar'); $('.restore-sidebar').hidden = true; });
$('.demo-theme').addEventListener('click', () => { const dark = preview.classList.toggle('dark'); $('.demo-theme').setAttribute('aria-label', t(dark ? 'Usar tema claro na demonstração' : 'Usar tema escuro na demonstração')); });
$('.demo-font').addEventListener('click', () => { font = (font + 1) % 3; preview.classList.toggle('serif', font === 1); preview.classList.toggle('mono', font === 2); notify(`${t('Fonte')}: ${t(['Sans serif', 'Libron', 'JetBrains Mono'][font])}`); });
$('.demo-width').addEventListener('click', () => preview.classList.toggle('narrow'));
$('.demo-pin').addEventListener('click', event => { const button = event.currentTarget; const pinned = button.getAttribute('aria-pressed') !== 'true'; button.setAttribute('aria-pressed', pinned); notify(pinned ? 'No app, o pin mantém a nota acima das outras janelas.' : 'No app, a janela volta ao comportamento normal do Mac.'); });
$('.new-demo-note').addEventListener('click', () => { const id = `note-${Date.now()}`; notes[id] = { emoji: '📄', title: 'Uma nova ideia', body: '<p contenteditable="true">O que você quer colocar no papel?</p>' }; const button = document.createElement('button'); button.className = 'note-item'; button.dataset.note = id; button.innerHTML = '<span class="note-emoji">📄</span>Uma nova ideia'; localizeDemo(button); $('#demo-notes').append(button); $('#demo-notes').hidden = false; $('[aria-controls="demo-notes"]').setAttribute('aria-expanded', 'true'); showNote(id); $('h2', documentArea).focus(); });
const mcpConfig = JSON.stringify({ mcpServers: { sparkpad: { command: '/Applications/Sparkpad.app/Contents/MacOS/sparkpad', args: ['mcp'] } } }, null, 2);
$('.copy-setup').addEventListener('click', async () => { try { await navigator.clipboard.writeText(mcpConfig); notify('Configuração MCP copiada.'); } catch { notify('Não foi possível copiar. A configuração está nas instruções do GitHub.'); } });
const mobileMenu = $('.mobile-menu');
mobileMenu.addEventListener('click', () => { const open = mobileMenu.getAttribute('aria-expanded') !== 'true'; mobileMenu.setAttribute('aria-expanded', open); $('#mobile-nav').hidden = !open; });
$$('#mobile-nav a').forEach(link => link.addEventListener('click', () => { mobileMenu.setAttribute('aria-expanded', 'false'); $('#mobile-nav').hidden = true; }));
document.addEventListener('keydown', event => { if (event.key === 'Escape') { $('#mobile-nav').hidden = true; mobileMenu.setAttribute('aria-expanded', 'false'); closeSlash(); } });
let slashTarget;
let slashChoice = 0;
const slashKinds = [['Texto', 'p', 'text'], ['Título', 'h3', 'text-size'], ['Lista', 'ul', 'list'], ['Código', 'pre', 'code-brackets']];
function closeSlash() { $('.demo-slash')?.remove(); slashTarget = undefined; }
function chooseSlash(index) {
  const target = slashTarget; closeSlash(); if (!target) return;
  const [, tag] = slashKinds[index]; const element = document.createElement(tag); element.contentEditable = 'true'; element.className = tag === 'p' ? 'editor-line' : ''; element.setAttribute('aria-label', t('Novo bloco da demonstração'));
  if (tag === 'ul') { element.innerHTML = '<li>Uma nova ideia</li>'; } else { element.textContent = t(tag === 'pre' ? '// Seu código aqui' : 'Uma nova ideia'); }
  localizeDemo(element); $('.editable-body').append(element); target.textContent = ''; element.focus();
}
function markSlash() { $$('.demo-slash button').forEach((button, index) => { button.classList.toggle('chosen', index === slashChoice); button.setAttribute('aria-current', index === slashChoice ? 'true' : 'false'); }); }
documentArea.addEventListener('input', event => {
  const target = event.target.closest('.new-block');
  if (!target) return;
  if (!target.textContent.startsWith('/')) { closeSlash(); return; }
  if (slashTarget === target) return;
  closeSlash(); slashTarget = target; slashChoice = 0;
  const menu = document.createElement('div'); menu.className = 'demo-slash'; menu.setAttribute('role', 'menu'); menu.setAttribute('aria-label', t('Tipos de bloco')); menu.innerHTML = '<span>Blocos básicos</span>';
  slashKinds.forEach(([label, , icon], index) => { const button = document.createElement('button'); button.type = 'button'; button.setAttribute('role', 'menuitem'); button.innerHTML = `<img class="icon" src="/assets/icons/${icon}.svg" alt="">${label}`; button.addEventListener('mousedown', event => event.preventDefault()); button.addEventListener('click', () => chooseSlash(index)); menu.append(button); });
  localizeDemo(menu); documentArea.append(menu); markSlash(); menu.scrollIntoView({block:'nearest',behavior:'smooth'});
});
documentArea.addEventListener('keydown', event => { if (!slashTarget) return; if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); slashChoice = (slashChoice + (event.key === 'ArrowDown' ? 1 : -1) + slashKinds.length) % slashKinds.length; markSlash(); } if (event.key === 'Enter') { event.preventDefault(); chooseSlash(slashChoice); } });
document.addEventListener('pointerdown', event => { if (slashTarget && !event.target.closest('.demo-slash') && event.target !== slashTarget) closeSlash(); });
showNote('welcome');

$$('.language-switcher a').forEach(link => link.addEventListener('click', () => { if (location.hash) link.href = link.pathname + location.hash; }));
document.addEventListener('pointerdown', event => { $$('.language-switcher[open]').forEach(menu => { if (!menu.contains(event.target)) menu.open = false; }); });
document.addEventListener('keydown', event => { if (event.key === 'Escape') { $$('.language-switcher[open]').forEach(menu => { menu.open = false; $('summary', menu).focus(); }); } });
