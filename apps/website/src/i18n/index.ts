import en from './en.json';
import pt from './pt-br.json';
import es from './es.json';
import fr from './fr.json';
export const locales = ['en', 'pt-br', 'es', 'fr'] as const;
export type Locale = typeof locales[number];
export type TranslationKey = keyof typeof en;
export const languages = [
  {locale: 'en', label: 'English', short: 'EN', lang: 'en'},
  {locale: 'pt-br', label: 'Português (Brasil)', short: 'PT', lang: 'pt-BR'},
  {locale: 'es', label: 'Español', short: 'ES', lang: 'es'},
  {locale: 'fr', label: 'Français', short: 'FR', lang: 'fr'},
] as const;
const dictionaries: Record<Locale, Record<TranslationKey, string>> = {'en': en, 'pt-br': pt, es, fr};
export function localePath(locale: Locale): string { return locale === 'en' ? '/' : `/${locale}/`; }
export function getDictionary(locale: Locale) { return dictionaries[locale]; }
export function translator(locale: Locale) { return (key: TranslationKey) => dictionaries[locale][key]; }
