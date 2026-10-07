import {sitemapXml} from '../lib/seo';
export const GET = () => new Response(sitemapXml(), {
  headers: {'Content-Type': 'application/xml; charset=utf-8'},
});
