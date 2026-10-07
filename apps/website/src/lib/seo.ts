import {languages, localePath, translator, type Locale} from '../i18n';

export const siteUrl = 'https://sparkpad.mplabs.sh';
export const repositoryUrl = 'https://github.com/mpires-dev/sparkpad';
export const absoluteUrl = (path: string) => new URL(path, siteUrl).href;
export const languageTag = (locale: Locale) => locale === 'pt-br' ? 'pt-BR' : locale;
export const ogLocales: Record<Locale, string> = {en: 'en_US', 'pt-br': 'pt_BR', es: 'es_ES', fr: 'fr_FR'};
const titles: Record<Locale, string> = {
  en: 'Sparkpad — Local-first notes for Mac and AI agents',
  'pt-br': 'Sparkpad — Notas locais para Mac e agentes de IA',
  es: 'Sparkpad — Notas locales para Mac y agentes de IA',
  fr: 'Sparkpad — Notes locales pour Mac et agents IA',
};

export function pageMetadata(locale: Locale) {
  const t = translator(locale);
  return {
    title: titles[locale],
    description: t('Seu bloco de notas nativo para Mac. Markdown, páginas aninhadas e MCP para trabalhar com seus agentes. Local, leve e open source.'),
    canonical: absoluteUrl(localePath(locale)),
    image: absoluteUrl(`/assets/social/sparkpad-${locale}.png`),
    imageAlt: `${t('Pense. Escreva.')} ${t('Conecte.')} — Sparkpad`,
  };
}

export function structuredData(locale: Locale) {
  const meta = pageMetadata(locale);
  const lang = languageTag(locale);
  const appId = `${siteUrl}/#application`;
  const websiteId = `${siteUrl}/#website`;
  const personId = `${siteUrl}/#creator`;
  return {
    '@context': 'https://schema.org',
    '@graph': [
      {'@type': 'Person', '@id': personId, name: 'Matheus Pires', sameAs: ['https://github.com/mpires-dev']},
      {
        '@type': 'WebSite', '@id': websiteId, name: 'Sparkpad', url: `${siteUrl}/`,
        inLanguage: languages.map(language => language.lang), publisher: {'@id': personId},
      },
      {
        '@type': 'SoftwareApplication', '@id': appId, name: 'Sparkpad',
        url: `${siteUrl}/`, description: meta.description, applicationCategory: 'ProductivityApplication',
        operatingSystem: 'macOS', processorRequirements: 'Apple Silicon',
        image: meta.image, screenshot: absoluteUrl('/assets/social/app-preview.png'),
        downloadUrl: absoluteUrl('/downloads/Sparkpad-macOS.zip'),
        license: `${repositoryUrl}/blob/main/LICENSE`, isAccessibleForFree: true,
        offers: {'@type': 'Offer', price: '0', priceCurrency: 'USD', url: meta.canonical},
        author: {'@id': personId}, sameAs: [repositoryUrl],
      },
      {
        '@type': 'WebPage', '@id': `${meta.canonical}#webpage`, url: meta.canonical,
        name: meta.title, description: meta.description, inLanguage: lang,
        isPartOf: {'@id': websiteId}, about: {'@id': appId}, primaryImageOfPage: {
          '@type': 'ImageObject', url: meta.image, width: 1200, height: 630, caption: meta.imageAlt,
        },
      },
    ],
  };
}

export function sitemapXml() {
  const alternates = [...languages.map(language => ({lang: language.lang, url: absoluteUrl(localePath(language.locale))})), {lang: 'x-default', url: absoluteUrl('/')}];
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n${languages.map(language => `  <url>\n    <loc>${absoluteUrl(localePath(language.locale))}</loc>\n${alternates.map(alternate => `    <xhtml:link rel="alternate" hreflang="${alternate.lang}" href="${alternate.url}" />`).join('\n')}\n  </url>`).join('\n')}\n</urlset>\n`;
}
