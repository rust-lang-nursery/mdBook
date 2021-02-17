#![allow(missing_docs)] // FIXME: Document this

pub mod fs;
mod string;
pub(crate) mod toml_ext;
use crate::errors::Error;
use regex::Regex;

use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, Options, Parser, Tag};

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use std::path::PathBuf;

pub use self::string::{
    take_anchored_lines, take_lines, take_rustdoc_include_anchored_lines,
    take_rustdoc_include_lines,
};

/// Replaces multiple consecutive whitespace characters with a single space character.
pub fn collapse_whitespace(text: &str) -> Cow<'_, str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\s\s+").unwrap();
    }
    RE.replace_all(text, " ")
}

/// Convert the given string to a valid HTML element ID.
/// The only restriction is that the ID must not contain any ASCII whitespace.
pub fn normalize_id(content: &str) -> String {
    content
        .chars()
        .filter_map(|ch| {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
}

/// Generate an ID for use with anchors which is derived from a "normalised"
/// string.
pub fn id_from_content(content: &str) -> String {
    let mut content = content.to_string();

    // Skip any tags or html-encoded stuff
    const REPL_SUB: &[&str] = &[
        "<em>",
        "</em>",
        "<code>",
        "</code>",
        "<strong>",
        "</strong>",
        "&lt;",
        "&gt;",
        "&amp;",
        "&#39;",
        "&quot;",
    ];
    for sub in REPL_SUB {
        content = content.replace(sub, "");
    }

    // Remove spaces and hashes indicating a header
    let trimmed = content.trim().trim_start_matches('#').trim();

    normalize_id(trimmed)
}

/// Context for resolving markdown symlinks
pub struct SymlinkResolveContext<'a> {
    /// The key is canonicalized absolute path of source markdown file, value is absolute path of destination html file
    pub to_render_paths: &'a HashMap<PathBuf, PathBuf>,
    /// Path to markdown source dir
    pub src_dir: &'a Path,
    /// Current markdown relative path specified in SUMMARY.md
    pub current_md_relative_path: &'a Path,
}

/// A hack to get original readme.md name, since in the preprocessing stage,
/// all markdown files named readme.md (case insensitive) are renamed to index.md.
fn find_readme_path(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok().and_then(|entries| {
        for entry in entries {
            if let Ok(entry) = entry {
                lazy_static! {
                    static ref RE: Regex = Regex::new(r"(?i)^readme$").unwrap();
                }
                if RE.is_match(
                    entry
                        .path()
                        .file_stem()
                        .and_then(std::ffi::OsStr::to_str)
                        .unwrap_or_default(),
                ) {
                    return Some(entry.path());
                }
            }
        }
        None
    })
}

/// Fix links to the correct location.
///
/// This adjusts links, such as turning `.md` extensions to `.html`.
///
/// `path` is the path to the page being rendered relative to the root of the
/// book. This is used for the `print.html` page so that links on the print
/// page go to the original location. Normal page rendering sets `path` to
/// None. Ideally, print page links would link to anchors on the print page,
/// but that is very difficult.
///
/// `symlink_resolve_ctx` is context to resolve markdown symlinks. If it is
/// `None`, we don't resolve symlinks.
fn adjust_links<'a>(
    event: Event<'a>,
    path: Option<&Path>,
    symlink_resolve_ctx: &Option<SymlinkResolveContext<'_>>,
) -> Event<'a> {
    lazy_static! {
        static ref SCHEME_LINK: Regex = Regex::new(r"^[a-z][a-z0-9+.-]*:").unwrap();
        static ref MD_LINK: Regex = Regex::new(r"(?P<link>.*)\.md(?P<anchor>#.*)?").unwrap();
    }

    fn fix<'a>(
        dest: CowStr<'a>,
        path: Option<&Path>,
        symlink_resolve_ctx: &Option<SymlinkResolveContext<'_>>,
    ) -> CowStr<'a> {
        if dest.starts_with('#') {
            // Fragment-only link.
            if let Some(path) = path {
                let mut base = path.display().to_string();
                if base.ends_with(".md") {
                    base.replace_range(base.len() - 3.., ".html");
                }
                return format!("{}{}", base, dest).into();
            } else {
                return dest;
            }
        }
        // Don't modify links with schemes like `https`.
        if !SCHEME_LINK.is_match(&dest) {
            // This is a relative link, adjust it as necessary.
            let mut fixed_link = String::new();
            if let Some(path) = path {
                let base = path
                    .parent()
                    .expect("path can't be empty")
                    .to_str()
                    .expect("utf-8 paths only");
                if !base.is_empty() {
                    write!(fixed_link, "{}/", base).unwrap();
                }
            }

            if let Some(caps) = MD_LINK.captures(&dest) {
                let mut find_and_convert_target_md_path_success = false;
                if let Some(SymlinkResolveContext {
                    to_render_paths,
                    src_dir,
                    current_md_relative_path,
                }) = symlink_resolve_ctx
                {
                    let mut current_md_relative_path = current_md_relative_path.to_path_buf();
                    let mut target_md_relative_path =
                        PathBuf::from(&format!("{}.md", &caps["link"]));
                    if target_md_relative_path.ends_with("index.md") {
                        if let Some(parent) = target_md_relative_path.parent() {
                            if let Some(readme_path) = find_readme_path(parent) {
                                target_md_relative_path = readme_path;
                            }
                        }
                    }
                    let target_md_path = if target_md_relative_path.is_absolute() {
                        target_md_relative_path.clone()
                    } else {
                        src_dir.join(&target_md_relative_path)
                    };
                    if current_md_relative_path.ends_with("index.md") {
                        if let Some(parent) = current_md_relative_path.parent() {
                            if let Some(readme_path) = find_readme_path(parent) {
                                current_md_relative_path = readme_path;
                            }
                        }
                    }
                    let current_md_path = if current_md_relative_path.is_absolute() {
                        current_md_relative_path.clone()
                    } else {
                        src_dir.join(&current_md_relative_path)
                    };
                    if let (Ok(target_md_path), Ok(current_md_path)) = (
                        std::fs::canonicalize(&target_md_path),
                        std::fs::canonicalize(&current_md_path),
                    ) {
                        if let Some(target_html_path) = to_render_paths.get(&target_md_path) {
                            if let Some(current_html_path) = to_render_paths.get(&current_md_path) {
                                if let Some(current_parent) = current_html_path.parent() {
                                    if let Some(target_relative_html_path) =
                                        pathdiff::diff_paths(target_html_path, current_parent)
                                    {
                                        fixed_link
                                            .push_str(target_relative_html_path.to_str().unwrap());
                                        find_and_convert_target_md_path_success = true;
                                    }
                                } // This should be true since current html path is absolute
                            } // This should be true since current markdown path are taken from SUMMARY.md
                        } else {
                            warn!(
                                "Links to markdown file {}, which is not translated.",
                                target_md_relative_path.to_str().unwrap()
                            );
                        }
                    }
                }
                if !find_and_convert_target_md_path_success {
                    fixed_link.push_str(&caps["link"]);
                    fixed_link.push_str(".html");
                }
                if let Some(anchor) = caps.name("anchor") {
                    fixed_link.push_str(anchor.as_str());
                }
            } else {
                fixed_link.push_str(&dest);
            };
            return CowStr::from(fixed_link);
        }
        dest
    }

    fn fix_html<'a>(
        html: CowStr<'a>,
        path: Option<&Path>,
        symlink_resolve_ctx: &Option<SymlinkResolveContext<'_>>,
    ) -> CowStr<'a> {
        // This is a terrible hack, but should be reasonably reliable. Nobody
        // should ever parse a tag with a regex. However, there isn't anything
        // in Rust that I know of that is suitable for handling partial html
        // fragments like those generated by pulldown_cmark.
        //
        // There are dozens of HTML tags/attributes that contain paths, so
        // feel free to add more tags if desired; these are the only ones I
        // care about right now.
        lazy_static! {
            static ref HTML_LINK: Regex =
                Regex::new(r#"(<(?:a|img) [^>]*?(?:src|href)=")([^"]+?)""#).unwrap();
        }

        HTML_LINK
            .replace_all(&html, |caps: &regex::Captures<'_>| {
                let fixed = fix(caps[2].into(), path, &symlink_resolve_ctx);
                format!("{}{}\"", &caps[1], fixed)
            })
            .into_owned()
            .into()
    }

    match event {
        Event::Start(Tag::Link(link_type, dest, title)) => Event::Start(Tag::Link(
            link_type,
            fix(dest, path, &symlink_resolve_ctx),
            title,
        )),
        Event::Start(Tag::Image(link_type, dest, title)) => Event::Start(Tag::Image(
            link_type,
            fix(dest, path, &symlink_resolve_ctx),
            title,
        )),
        Event::Html(html) => Event::Html(fix_html(html, path, &symlink_resolve_ctx)),
        _ => event,
    }
}

/// Wrapper around the pulldown-cmark parser for rendering markdown to HTML.
///
/// `symlink_resolve_ctx` is context to resolve markdown symlinks. If it is
/// `None`, we don't resolve symlinks.
pub fn render_markdown(
    text: &str,
    curly_quotes: bool,
    symlink_resolve_ctx: &Option<SymlinkResolveContext<'_>>,
) -> String {
    render_markdown_with_path(text, curly_quotes, None, symlink_resolve_ctx)
}

pub fn new_cmark_parser(text: &str) -> Parser<'_> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    Parser::new_ext(text, opts)
}

pub fn render_markdown_with_path(
    text: &str,
    curly_quotes: bool,
    path: Option<&Path>,
    symlink_resolve_ctx: &Option<SymlinkResolveContext<'_>>,
) -> String {
    let mut s = String::with_capacity(text.len() * 3 / 2);
    let p = new_cmark_parser(text);
    let mut converter = EventQuoteConverter::new(curly_quotes);
    let events = p
        .map(clean_codeblock_headers)
        .map(|event| adjust_links(event, path, &symlink_resolve_ctx))
        .map(|event| converter.convert(event));

    html::push_html(&mut s, events);
    s
}

struct EventQuoteConverter {
    enabled: bool,
    convert_text: bool,
}

impl EventQuoteConverter {
    fn new(enabled: bool) -> Self {
        EventQuoteConverter {
            enabled,
            convert_text: true,
        }
    }

    fn convert<'a>(&mut self, event: Event<'a>) -> Event<'a> {
        if !self.enabled {
            return event;
        }

        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                self.convert_text = false;
                event
            }
            Event::End(Tag::CodeBlock(_)) => {
                self.convert_text = true;
                event
            }
            Event::Text(ref text) if self.convert_text => {
                Event::Text(CowStr::from(convert_quotes_to_curly(text)))
            }
            _ => event,
        }
    }
}

fn clean_codeblock_headers(event: Event<'_>) -> Event<'_> {
    match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) => {
            let info: String = info.chars().filter(|ch| !ch.is_whitespace()).collect();

            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from(info))))
        }
        _ => event,
    }
}

fn convert_quotes_to_curly(original_text: &str) -> String {
    // We'll consider the start to be "whitespace".
    let mut preceded_by_whitespace = true;

    original_text
        .chars()
        .map(|original_char| {
            let converted_char = match original_char {
                '\'' => {
                    if preceded_by_whitespace {
                        '‘'
                    } else {
                        '’'
                    }
                }
                '"' => {
                    if preceded_by_whitespace {
                        '“'
                    } else {
                        '”'
                    }
                }
                _ => original_char,
            };

            preceded_by_whitespace = original_char.is_whitespace();

            converted_char
        })
        .collect()
}

/// Prints a "backtrace" of some `Error`.
pub fn log_backtrace(e: &Error) {
    error!("Error: {}", e);

    for cause in e.chain().skip(1) {
        error!("\tCaused By: {}", cause);
    }
}

#[cfg(test)]
mod tests {
    mod render_markdown {
        use super::super::render_markdown;

        #[test]
        fn preserves_external_links() {
            assert_eq!(
                render_markdown("[example](https://www.rust-lang.org/)", false, &None),
                "<p><a href=\"https://www.rust-lang.org/\">example</a></p>\n"
            );
        }

        #[test]
        fn it_can_adjust_markdown_links() {
            assert_eq!(
                render_markdown("[example](example.md)", false, &None),
                "<p><a href=\"example.html\">example</a></p>\n"
            );
            assert_eq!(
                render_markdown("[example_anchor](example.md#anchor)", false, &None),
                "<p><a href=\"example.html#anchor\">example_anchor</a></p>\n"
            );

            // this anchor contains 'md' inside of it
            assert_eq!(
                render_markdown("[phantom data](foo.html#phantomdata)", false, &None),
                "<p><a href=\"foo.html#phantomdata\">phantom data</a></p>\n"
            );
        }

        #[test]
        fn it_can_keep_quotes_straight() {
            assert_eq!(render_markdown("'one'", false, &None), "<p>'one'</p>\n");
        }

        #[test]
        fn it_can_make_quotes_curly_except_when_they_are_in_code() {
            let input = r#"
'one'
```
'two'
```
`'three'` 'four'"#;
            let expected = r#"<p>‘one’</p>
<pre><code>'two'
</code></pre>
<p><code>'three'</code> ‘four’</p>
"#;
            assert_eq!(render_markdown(input, true, &None), expected);
        }

        #[test]
        fn whitespace_outside_of_codeblock_header_is_preserved() {
            let input = r#"
some text with spaces
```rust
fn main() {
// code inside is unchanged
}
```
more text with spaces
"#;

            let expected = r#"<p>some text with spaces</p>
<pre><code class="language-rust">fn main() {
// code inside is unchanged
}
</code></pre>
<p>more text with spaces</p>
"#;
            assert_eq!(render_markdown(input, false, &None), expected);
            assert_eq!(render_markdown(input, true, &None), expected);
        }

        #[test]
        fn rust_code_block_properties_are_passed_as_space_delimited_class() {
            let input = r#"
```rust,no_run,should_panic,property_3
```
"#;

            let expected = r#"<pre><code class="language-rust,no_run,should_panic,property_3"></code></pre>
"#;
            assert_eq!(render_markdown(input, false, &None), expected);
            assert_eq!(render_markdown(input, true, &None), expected);
        }

        #[test]
        fn rust_code_block_properties_with_whitespace_are_passed_as_space_delimited_class() {
            let input = r#"
```rust,    no_run,,,should_panic , ,property_3
```
"#;

            let expected = r#"<pre><code class="language-rust,no_run,,,should_panic,,property_3"></code></pre>
"#;
            assert_eq!(render_markdown(input, false, &None), expected);
            assert_eq!(render_markdown(input, true, &None), expected);
        }

        #[test]
        fn rust_code_block_without_properties_has_proper_html_class() {
            let input = r#"
```rust
```
"#;

            let expected = r#"<pre><code class="language-rust"></code></pre>
"#;
            assert_eq!(render_markdown(input, false, &None), expected);
            assert_eq!(render_markdown(input, true, &None), expected);

            let input = r#"
```rust
```
"#;
            assert_eq!(render_markdown(input, false, &None), expected);
            assert_eq!(render_markdown(input, true, &None), expected);
        }
    }

    mod html_munging {
        use super::super::{id_from_content, normalize_id};

        #[test]
        fn it_generates_anchors() {
            assert_eq!(
                id_from_content("## Method-call expressions"),
                "method-call-expressions"
            );
            assert_eq!(id_from_content("## **Bold** title"), "bold-title");
            assert_eq!(id_from_content("## `Code` title"), "code-title");
        }

        #[test]
        fn it_generates_anchors_from_non_ascii_initial() {
            assert_eq!(
                id_from_content("## `--passes`: add more rustdoc passes"),
                "--passes-add-more-rustdoc-passes"
            );
            assert_eq!(
                id_from_content("## 中文標題 CJK title"),
                "中文標題-cjk-title"
            );
            assert_eq!(id_from_content("## Über"), "Über");
        }

        #[test]
        fn it_normalizes_ids() {
            assert_eq!(
                normalize_id("`--passes`: add more rustdoc passes"),
                "--passes-add-more-rustdoc-passes"
            );
            assert_eq!(
                normalize_id("Method-call 🐙 expressions \u{1f47c}"),
                "method-call--expressions-"
            );
            assert_eq!(normalize_id("_-_12345"), "_-_12345");
            assert_eq!(normalize_id("12345"), "12345");
            assert_eq!(normalize_id("中文"), "中文");
            assert_eq!(normalize_id("にほんご"), "にほんご");
            assert_eq!(normalize_id("한국어"), "한국어");
            assert_eq!(normalize_id(""), "");
        }
    }

    mod convert_quotes_to_curly {
        use super::super::convert_quotes_to_curly;

        #[test]
        fn it_converts_single_quotes() {
            assert_eq!(convert_quotes_to_curly("'one', 'two'"), "‘one’, ‘two’");
        }

        #[test]
        fn it_converts_double_quotes() {
            assert_eq!(convert_quotes_to_curly(r#""one", "two""#), "“one”, “two”");
        }

        #[test]
        fn it_treats_tab_as_whitespace() {
            assert_eq!(convert_quotes_to_curly("\t'one'"), "\t‘one’");
        }
    }
}
