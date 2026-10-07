import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const locales=['en','pt-br','es','fr'];
const dictionaries=Object.fromEntries(await Promise.all(locales.map(async locale=>[locale,JSON.parse(await readFile(new URL(`../src/i18n/${locale}.json`,import.meta.url),'utf8'))])));
test('All locales contain the complete, nonempty translation catalog',()=>{
 const keys=Object.keys(dictionaries.en).sort();
 assert.ok(keys.length>150);
 for(const locale of locales){assert.deepEqual(Object.keys(dictionaries[locale]).sort(),keys,locale);for(const [key,value] of Object.entries(dictionaries[locale]))assert.ok(typeof value==='string'&&value.trim(),`${locale}: ${key}`);}
});
test('All literal Astro translation keys exist in every language',async()=>{
 for(const filename of ['Landing.astro','ProductDemo.astro','LanguageSwitcher.astro','AgentConversation.astro']){
  const source=await readFile(new URL(`../src/components/${filename}`,import.meta.url),'utf8');
  const matches=source.matchAll(/\bt\(("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')\)/g);
  for(const match of matches){const key=match[1][0]==='"'?JSON.parse(match[1]):match[1].slice(1,-1);assert.ok(Object.hasOwn(dictionaries.en,key),`${filename}: ${key}`);}
 }
});
test('Demo interface messages, notes and accessibility labels are localized',()=>{
 const required=['Configuração MCP copiada.','Tipos de bloco','Blocos básicos','Um lugar para pensar','O próximo lançamento','Notas de desenvolvimento','Usar tema escuro na demonstração','Escreva ou digite /…','Novo bloco da demonstração','Selecionar idioma'];
 for(const key of required)for(const locale of locales)assert.ok(dictionaries[locale][key],`${locale}: ${key}`);
});
