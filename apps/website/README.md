# @sparkpad/website

Astro landing page in the Sparkpad monorepo. The native application remains at the repository root; the website is an independent npm workspace. Static output, no React runtime or translation requests.

## Development

From the repository root:

```sh
npm install
npm run dev:web       # http://localhost:4173
npm run check:web     # Astro/TypeScript and localization checks
npm run build:web     # apps/website/dist
npm run preview:web   # serve the generated static pages
```

## Languages

- `/`: English (default)
- `/pt-br/`: Português (Brasil)
- `/es/`: Español
- `/fr/`: Français

`src/i18n/` contains complete dictionaries shared by server-rendered copy and the interactive preview. `Landing.astro` is the shared presentation; `ProductDemo.astro` contains the preview; `LanguageSwitcher.astro` provides accessible native links. Language switching preserves the current section's URL fragment. Pages include translated titles, descriptions, document languages and `hreflang` alternatives. The root is always English, regardless of browser settings.

`src/styles/global.css` preserves the original visual design. `src/scripts/demo.js` handles demo interactions only. Demo edits stay in memory and never access native app notes.

## macOS download

Generate the local Apple Silicon bundle from the repository root:

```sh
bash scripts/bundle.sh
mkdir -p apps/website/public/downloads
ditto -c -k --sequesterRsrc --keepParent dist/Sparkpad.app apps/website/public/downloads/Sparkpad-macOS.zip
```

Download archives are ignored by Git. Before public hosting, replace local links with a published release artifact.

## Assets

The application icon and fonts are Sparkpad assets, with font licenses in `public/assets/fonts/`. Landing UI icons are Solar Bold Duotone by 480 Design (CC BY 4.0), sourced from `@solar-icons/static` 2.3.2. Only the used SVGs are stored in `src/assets/solar/` and rendered inline by `SolarIcon.astro`, without a browser icon runtime. Colors use CSS variables; source attribution is visible in the footer and notices are in `public/assets/licenses/`. Icons inside the app mockups remain Iconoir (MIT), from the vendored design system. Sky clouds are local preview reference assets from `https://cap.so/backgrounds/clouds/cumulus-a.webp` and `bank-a.webp`; replace or review their licensing before public distribution. The composition is inspired by https://cap.so/.

Language flags are local round SVGs from [Circle Flags](https://github.com/HatScripts/circle-flags) (MIT); the license is stored in `public/assets/licenses/circle-flags-MIT.txt`.

Conversation loading indicators use the diagonal [Blocks spinner from loading.dev](https://loading.dev/spinners/blocks), by Jakub Krehel (MIT). The small Astro adaptation retains its grid, sweep timing and reduced-motion support; the license is in `public/assets/licenses/loading-dev-MIT.txt`.
