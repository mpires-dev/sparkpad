//! Parser-based code colors, independent of the native input's auto-growing layout.
use gpui::HighlightStyle;
use gpui_component::highlighter::{HighlightTheme, Language, LanguageRegistry, SyntaxHighlighter};
use ropey::Rope;
use std::{
    ops::Range,
    sync::{Arc, Once},
};

pub const LANGUAGES: &[(&str, &str)] = &[
    ("text", "Texto simples"),
    ("javascript", "JavaScript / JSX"),
    ("typescript", "TypeScript"),
    ("tsx", "TSX (React + TypeScript)"),
    ("bash", "Bash / Shell"),
    ("c", "C"),
    ("cpp", "C++"),
    ("csharp", "C#"),
    ("cmake", "CMake"),
    ("css", "CSS"),
    ("diff", "Diff"),
    ("ejs", "EJS"),
    ("elixir", "Elixir"),
    ("erb", "ERB"),
    ("go", "Go"),
    ("graphql", "GraphQL"),
    ("html", "HTML"),
    ("java", "Java"),
    ("json", "JSON"),
    ("make", "Makefile"),
    ("markdown", "Markdown"),
    ("proto", "Protocol Buffers"),
    ("python", "Python"),
    ("ruby", "Ruby"),
    ("rust", "Rust"),
    ("scala", "Scala"),
    ("sql", "SQL"),
    ("swift", "Swift"),
    ("toml", "TOML"),
    ("yaml", "YAML"),
    ("zig", "Zig"),
];

pub fn canonical(language: &str) -> &'static str {
    let name = language
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    Language::from_str(match name.as_str() {
        "jsx" => "javascript",
        "c#" => "csharp",
        "shell" | "shellscript" | "zsh" => "bash",
        "plaintext" | "plain" => "text",
        other => other,
    })
    .name()
}
pub fn label(language: &str) -> String {
    let id = canonical(language);
    if id == "text" && !language.is_empty() && !matches!(language, "text" | "plain" | "plaintext") {
        return format!("{language} (texto)");
    }
    LANGUAGES
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, label)| (*label).to_owned())
        .unwrap_or_else(|| language.to_owned())
}

// The older component registry omits queries for several shipped grammars and
// does not resolve inherited C++/TSX queries. Patch configs once, before parsing.
fn prepare_languages() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let registry = LanguageRegistry::singleton();
        let register = |language: &str, query: String| {
            let mut config = registry.language(language).expect("bundled grammar");
            config.highlights = query.into();
            registry.register(language, &config);
        };
        let c = registry.language("c").unwrap().highlights;
        let cpp = registry.language("cpp").unwrap().highlights;
        register("cpp", format!("{c}\n{cpp}"));
        let js = registry.language("javascript").unwrap().highlights;
        let ts = registry.language("typescript").unwrap().highlights;
        let jsx = include_str!("../assets/syntax/jsx.scm");
        register("javascript", format!("{js}\n{jsx}"));
        register("tsx", format!("{ts}\n{jsx}"));
        for (language, embedded) in [("ejs", "javascript"), ("erb", "ruby")] {
            let mut config = registry.language(language).unwrap();
            config.injection_languages = vec!["html".into(), embedded.into()];
            config.injections = config.injections.replace("javascript", embedded).into();
            registry.register(language, &config);
        }
        for (language, query) in [
            ("csharp", include_str!("../assets/syntax/c-sharp.scm")),
            ("cmake", include_str!("../assets/syntax/cmake.scm")),
            ("swift", include_str!("../assets/syntax/swift.scm")),
            ("proto", include_str!("../assets/syntax/proto.scm")),
            ("graphql", include_str!("../assets/syntax/graphql.scm")),
        ] {
            register(language, query.to_owned());
        }
    });
}

#[derive(Default)]
pub struct CodeHighlight {
    language: String,
    text: String,
    parser: Option<SyntaxHighlighter>,
    theme: Option<Arc<HighlightTheme>>,
    styles: Vec<(Range<usize>, HighlightStyle)>,
}
fn end_point(text: &str) -> tree_sitter::Point {
    tree_sitter::Point::new(
        text.bytes().filter(|&b| b == b'\n').count(),
        text.rsplit('\n').next().unwrap_or("").len(),
    )
}
impl CodeHighlight {
    pub fn styles(
        &mut self,
        language: &str,
        text: &str,
        theme: Arc<HighlightTheme>,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        prepare_languages();
        let language = canonical(language);
        let language_changed = self.language != language;
        let text_changed = self.text != text;
        if language_changed {
            self.parser = (language != "text").then(|| SyntaxHighlighter::new(language));
            self.language = language.into();
        }
        if language_changed || text_changed {
            if let Some(parser) = &mut self.parser {
                // Describe a complete replacement with correct UTF-8 byte offsets and points.
                // The engine still reuses the parser; unchanged renders do no parsing at all.
                let old = if language_changed { "" } else { &self.text };
                parser.update(
                    Some(tree_sitter::InputEdit {
                        start_byte: 0,
                        old_end_byte: old.len(),
                        new_end_byte: text.len(),
                        start_position: tree_sitter::Point::new(0, 0),
                        old_end_position: end_point(old),
                        new_end_position: end_point(text),
                    }),
                    &Rope::from(text),
                );
            }
            self.text = text.into();
        }
        if language_changed || text_changed || self.theme.as_deref() != Some(theme.as_ref()) {
            self.styles = self
                .parser
                .as_ref()
                .map(|p| p.styles(&(0..text.len()), &theme))
                .unwrap_or_else(|| {
                    if text.is_empty() {
                        vec![]
                    } else {
                        vec![(0..text.len(), HighlightStyle::default())]
                    }
                });
            self.theme = Some(theme);
        }
        self.styles.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn languages_highlight_unicode_edits_and_theme_switches() {
        let dark = HighlightTheme::default_dark();
        let mut cache = CodeHighlight::default();
        prepare_languages();
        for &(language, _) in LANGUAGES.iter().filter(|(l, _)| *l != "text") {
            let config = LanguageRegistry::singleton().language(language).unwrap();
            let query = format!(
                "{}{}{}",
                config.injections, config.locals, config.highlights
            );
            tree_sitter::Query::new(&config.language, &query)
                .unwrap_or_else(|err| panic!("invalid {language} query: {err}"));
            let sample = match language {
                "diff" => "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n",
                "html" | "ejs" | "erb" => "<div class=\"hello\">hello</div>",
                "markdown" => "# Title\n\n**bold** `code`\n",
                "json" => "{\"value\": true}",
                "yaml" => "value: true\n",
                "toml" => "value = true\n",
                "make" => "all:\n\techo hello\n",
                "cmake" => "set(VALUE hello)\n",
                "bash" => "echo \"hello\"\n",
                "sql" => "SELECT name FROM users;",
                "proto" => "syntax = \"proto3\";\nmessage Hello { string name = 1; }",
                "graphql" => "query { users { name } }",
                "css" => ".hello { color: red; }",
                _ => "const ação = \"coração\";\n// comment\n",
            };
            let styles = cache.styles(language, sample, dark.clone());
            assert!(
                styles.iter().any(|(_, s)| s.color.is_some()),
                "grammar missing: {language}"
            );
        }
        let first = "const ação = \"coração\";\n// comment\n";
        let styles = cache.styles("js", first, dark.clone());
        assert_eq!(
            styles.iter().map(|(r, _)| r.len()).sum::<usize>(),
            first.len()
        );
        assert!(styles
            .iter()
            .all(|(r, _)| first.is_char_boundary(r.start) && first.is_char_boundary(r.end)));
        assert_eq!(styles, cache.styles("javascript", first, dark.clone()));
        let color_at = |offset| {
            styles
                .iter()
                .find(|(r, _)| r.contains(&offset))
                .unwrap()
                .1
                .color
        };
        assert!(color_at(0).is_some(), "keyword must be colored");
        assert_ne!(
            color_at(0),
            color_at(first.find("coração").unwrap()),
            "strings must differ from keywords"
        );
        assert_ne!(
            color_at(0),
            color_at(first.find("comment").unwrap()),
            "comments must differ from keywords"
        );
        let next = "/* multiline\n comentário */\nconst n = 42;";
        let edited = cache.styles("js", next, dark.clone());
        let fresh = CodeHighlight::default().styles("js", next, dark.clone());
        assert_eq!(edited, fresh, "edits must match a fresh parse");
        assert_ne!(
            edited,
            cache.styles("js", next, HighlightTheme::default_light())
        );
        assert!(cache
            .styles("text", next, dark.clone())
            .iter()
            .all(|(_, s)| s.color.is_none()));
        assert!(cache
            .styles("unknown", next, dark)
            .iter()
            .all(|(_, s)| s.color.is_none()));
        for text in [
            "",
            "const x = 1;",
            "\n\n",
            "// ação\nlet x = `hello ${42}`;",
        ] {
            let styles = cache.styles("js", text, HighlightTheme::default_dark());
            assert_eq!(
                styles,
                CodeHighlight::default().styles("js", text, HighlightTheme::default_dark())
            );
        }
        let long = format!("{}const RESULT = 42;", "// coração\n".repeat(1024));
        let colors = cache.styles("js", &long, HighlightTheme::default_dark());
        let offset = long.find("RESULT").unwrap();
        assert_eq!(
            colors
                .iter()
                .find(|(r, _)| r.contains(&offset))
                .unwrap()
                .1
                .color,
            HighlightTheme::default_dark()
                .style("constant")
                .unwrap()
                .color,
            "regex predicates must inspect the token even across rope chunks"
        );
        let react = "type Props = { label: string };\nconst Card = ({ label }: Props) => <section>{label}</section>;";
        let colors = cache.styles("tsx", react, HighlightTheme::default_dark());
        let tag = react.find("section").unwrap();
        assert_eq!(
            colors
                .iter()
                .find(|(r, _)| r.contains(&tag))
                .unwrap()
                .1
                .color,
            HighlightTheme::default_dark().style("tag").unwrap().color,
            "TSX must highlight JSX tags within TypeScript components"
        );
        assert_eq!(canonical("C#"), "csharp");
        assert_eq!(canonical("jsx"), "javascript");
    }
}
