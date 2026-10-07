import {absoluteUrl} from '../lib/seo';
export const GET = () => new Response(`User-agent: *\nAllow: /\n\nSitemap: ${absoluteUrl('/sitemap.xml')}\n`, {
  headers: {'Content-Type': 'text/plain; charset=utf-8'},
});
