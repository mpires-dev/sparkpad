import {absoluteUrl, repositoryUrl} from '../lib/seo';
export const GET = () => new Response(`# Sparkpad

> Sparkpad is a local-first, open-source native notepad for macOS with a block editor, Markdown storage, nested pages, sidebar groups, and a built-in Model Context Protocol (MCP) server.

Notes are stored locally on the user's Mac in SQLite as Markdown. The app is built with Rust and GPUI. No account or subscription is required. The current downloadable build targets Apple Silicon. The project is licensed under AGPL-3.0-or-later.

The editor supports headings, lists, tasks, inline formatting, code blocks with language selection and syntax highlighting, rich clipboard content, covers, icons, and page fonts. AI clients supporting MCP over stdio can create, edit, and organize notes and their hierarchy. Connecting an AI agent is optional.

## Website

- [English](${absoluteUrl('/')}): Default product page, features, MCP overview, and FAQ.
- [Português (Brasil)](${absoluteUrl('/pt-br/')}): Brazilian Portuguese product page.
- [Español](${absoluteUrl('/es/')}): Spanish product page.
- [Français](${absoluteUrl('/fr/')}): French product page.

## Source and documentation

- [Source code and setup](${repositoryUrl}): Native app, MCP configuration, and build instructions.
- [MCP implementation](${repositoryUrl}/blob/main/src/mcp.rs): Authoritative MCP tool definitions and request handling.
- [License](${repositoryUrl}/blob/main/LICENSE): AGPL-3.0-or-later license.

## Optional

- [macOS download](${absoluteUrl('/downloads/Sparkpad-macOS.zip')}): Apple Silicon app archive.
- [Sitemap](${absoluteUrl('/sitemap.xml')}): Canonical language versions of the landing page.
`, {headers: {'Content-Type': 'text/plain; charset=utf-8'}});
