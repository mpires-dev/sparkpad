import {defineConfig} from 'astro/config';
export default defineConfig({
  site: 'https://sparkpad.mplabs.sh',
  output: 'static',
  devToolbar: {enabled: false},
  trailingSlash: 'always',
  i18n: {
    locales: ['en', 'pt-br', 'es', 'fr'],
    defaultLocale: 'en',
    routing: {prefixDefaultLocale: false},
  },
  server: {host: '127.0.0.1', port: 4173},
});
