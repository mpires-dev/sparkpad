# Markdown blocks

Sparkpad stores Markdown and preserves the original source for blocks that have not been edited. The native editor uses the Markdown AST rather than matching block syntax with regular expressions.

| Content | Native editor | Browser |
| --- | --- | --- |
| Paragraphs, soft/hard line breaks | Editable text | Markdown editor and preview |
| H1–H6, including setext headings | Editable headings; slash-menu choices H1–H6 | Preview |
| Unordered and ordered lists | Editable simple items | Preview |
| Nested lists and multi-paragraph items | Rendered compound Markdown; source editing | Preview |
| Task lists | Empire UI checkbox | Interactive checkbox in preview |
| Blockquotes and nested quotes | Editable simple quotes; compound Markdown preview | Preview |
| Fenced and indented code | Native code editor, language, highlighting, indentation and copy | Markdown editor and code preview |
| Thematic breaks | Divider | Divider |
| Images and image references | Image block, file chooser, clipboard image, caption and proportional resizing | Upload, preview and width slider |
| GFM tables | Editable cells, add/remove rows and columns, keyboard navigation | Editable cells, add/remove rows and columns, horizontal scrolling |
| Links, reference links and autolinks | Rich text or compound Markdown preview | Preview |
| Emphasis, strong, inline code and GFM strikethrough | Inline formatting | Preview |
| Escapes and entities | Preserved Markdown and decoded display text | Preview |
| HTML blocks | Preserved source; supported markup has a native preview | Sanitized HTML preview |
| Link/image definitions | Preserved and resolved when parsing the document | Resolved by the Markdown parser |

The table covers CommonMark and GitHub Flavored Markdown. Footnotes, mathematics, executable diagrams, and custom embedded widgets are not part of this supported block set. Unrecognized content remains editable as Markdown instead of being discarded. Complex list/quote structures use the compound Markdown renderer; they do not have individual nested native input controls yet.

## Images

Use **/ → Image** to choose a file, or paste a supported image from the clipboard into a text block. **Caption** edits its alternative text. Drag the right-hand handle to resize while retaining its aspect ratio. Width is a percentage of the available document width, so it also adapts to smaller screens. Uploaded files are copied into Sparkpad's owned image storage and synchronized to the server.

Supported uploads: PNG, JPEG, WebP, GIF and SVG, up to 10 MB. External HTTP/HTTPS images can be displayed using standard Markdown; they still depend on their source being available. Relative image paths need an accessible image asset; import the file for reliable offline and cross-device use.

Uploaded images use a content-addressed URL and store width in the standard Markdown image title:

```markdown
![Diagram](asset:SHA256.png "sparkpad-width=60")
```

The `SHA256` placeholder is replaced with the file's actual 64-character digest. Markdown readers that do not implement the width convention retain the image's alternative text and title. Exporting Markdown to another application requires exporting its referenced image files or replacing the asset URLs.

The browser saves uploaded images in IndexedDB, including uploads made offline, and uploads pending files before reconciling document changes on reconnection.

## Tables

```markdown
| Feature | Status |
| :--- | ---: |
| **Images** | Ready |
| Tables | Ready |
```

The renderer supports a header row, column alignment, inline formatting and escaped pipes. Click any cell to edit it in place, including the header. Use **+ Row** and **+ Column** to grow the table, and the small remove controls to delete a body row or a column. The header and last column cannot be deleted. **Tab / Shift+Tab** move between cells; **Enter** moves to the next row. Advancing beyond the last row adds one automatically. **Cmd/Ctrl+A** selects the focused cell, and undo/redo works for content and structural changes. Bold, italic, links and inline code remain formatted while editing; native formatting shortcuts also work inside cells. Cell edits preserve neighboring Markdown and column alignment. Large tables scroll horizontally within the content area. Changes use the existing offline storage and collaborative synchronization.

The browser uses [Marked](https://github.com/markedjs/marked) for GFM parsing and [DOMPurify](https://github.com/cure53/DOMPurify) to sanitize its HTML output. The native viewer reuses the existing GPUI Markdown renderer. The block audit follows the [GFM specification](https://github.github.com/gfm/).
