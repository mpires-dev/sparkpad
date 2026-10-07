// A deterministic timeline keeps streaming, tool calls and replay in sync.
// Only cards in view run animation frames; no requests or actual MCP calls are made.
const card = document.querySelector('[data-agent-conversation]');
if (card) {
  const dictionary = JSON.parse(document.getElementById('translations').textContent);
  const t = key => dictionary[key] ?? key;
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
  const timeline = card.querySelector('.conversation-timeline');
  const prompt = card.querySelector('[data-prompt]');
  const agent = card.querySelector('.chat-agent');
  const promptOutput = card.querySelector('[data-stream="prompt"]');
  const responseOutput = card.querySelector('[data-stream="response"]');
  const stage = card.querySelector('[data-stage-label]');
  const tools = [...card.querySelectorAll('[data-tool]')];
  const splitText = value => {
    if (typeof Intl.Segmenter === 'function') return [...new Intl.Segmenter(document.documentElement.lang, {granularity: 'grapheme'}).segment(value)].map(part => part.segment);
    return Array.from(value);
  };
  const promptText = splitText(promptOutput.textContent);
  const responseText = splitText(responseOutput.textContent);
  const toolTexts = tools.map(tool => splitText(tool.querySelector('.tool-result').dataset.result));
  const duration = 15000;
  const phases = [
    {start: 6200, complete: 7350},
    {start: 8050, complete: 9350},
    {start: 10050, complete: 11250},
  ];
  let elapsed = 0;
  let previousTime = null;
  let frame = null;
  let inView = false;
  let cycles = 0;
  const stream = (output, text, start, length, time) => {
    const fraction = Math.max(0, Math.min(1, (time - start) / length));
    const next = text.slice(0, Math.floor(text.length * fraction)).join('');
    if (output.textContent !== next) output.textContent = next;
    output.classList.toggle('is-streaming', fraction > 0 && fraction < 1);
  };
  function render(time) {
    const fade = time > 14000 ? Math.max(0, (duration - time) / 650) : 1;
    timeline.style.opacity = String(fade);
    prompt.classList.toggle('is-visible', time >= 350);
    agent.classList.toggle('is-visible', time >= 4050);
    stream(promptOutput, promptText, 500, 2900, time);
    stream(responseOutput, responseText, 4400, 1250, time);
    tools.forEach((tool, index) => {
      const {start, complete} = phases[index];
      const visible = time >= start;
      const done = time >= complete;
      tool.classList.toggle('is-visible', visible);
      tool.classList.toggle('is-running', visible && !done);
      tool.classList.toggle('is-complete', done);
      tool.dataset.state = done ? 'complete' : visible ? 'running' : 'pending';
      const result = tool.querySelector('.tool-result');
      if (!done) {
        const label = visible ? t(index === 2 ? 'Organizando páginas…' : 'Criando nota…') : '';
        if (result.textContent !== label) result.textContent = label;
        result.classList.remove('is-streaming');
      } else stream(result, toolTexts[index], complete, 450, time);
    });
    const label = time < 3400 ? t('Escrevendo…') : time < 4400 ? t('Pensando…') : time < 6200 ? t('Escrevendo…') : time < 10050 ? t('Criando nota…') : time < 11700 ? t('Organizando páginas…') : t('Tudo organizado');
    if (stage.textContent !== label) stage.textContent = label;
    card.dataset.phase = time < 3400 ? 'prompt' : time < 6200 ? 'response' : time < 11700 ? 'tools' : 'complete';
    card.dataset.cycle = String(cycles);
  }
  function tick(now) {
    frame = null;
    if (reducedMotion.matches) { sync(); return; }
    if (!shouldRun()) { previousTime = null; return; }
    if (previousTime !== null) {
      elapsed += Math.min(now - previousTime, 100);
      if (elapsed >= duration) { elapsed %= duration; cycles++; }
    }
    previousTime = now;
    render(elapsed);
    frame = requestAnimationFrame(tick);
  }
  function shouldRun() { return inView && !document.hidden && !reducedMotion.matches; }
  function sync() {
    if (frame !== null) cancelAnimationFrame(frame);
    frame = null;
    previousTime = null;
    if (reducedMotion.matches) {
      card.classList.remove('is-animated');
      render(12500);
      stage.textContent = t('Demonstração em loop');
    } else {
      card.classList.add('is-animated');
      render(elapsed);
      if (shouldRun()) frame = requestAnimationFrame(tick);
    }
  }
  const observer = new IntersectionObserver(entries => { inView = entries[0].isIntersecting; sync(); }, {threshold: 0.2});
  observer.observe(card);
  document.addEventListener('visibilitychange', sync);
  reducedMotion.addEventListener('change', sync);
  sync();
}
