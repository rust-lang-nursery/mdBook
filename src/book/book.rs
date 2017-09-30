use std::path::Path;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;

use super::summary::{parse_summary, Summary, Link, SummaryItem, SectionNumber};
use errors::*;


/// Load a book into memory from its `src/` directory.
pub fn load_book<P: AsRef<Path>>(src_dir: P) -> Result<Book> {
    let src_dir = src_dir.as_ref();
    let summary_md = src_dir.join("SUMMARY.md");

    let mut summary_content = String::new();
    File::open(summary_md)
        .chain_err(|| "Couldn't open SUMMARY.md")?
        .read_to_string(&mut summary_content)?;

    let summary = parse_summary(&summary_content).chain_err(
        || "Summary parsing failed",
    )?;

    load_book_from_disk(&summary, src_dir)
}


/// A dumb tree structure representing a book.
///
/// For the moment a book is just a collection of `BookItems`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Book {
    /// The sections in this book.
    pub sections: Vec<BookItem>,
}

impl Book {
    /// Create an empty book.
    pub fn new() -> Self {
        Default::default()
    }

    /// Get a depth-first iterator over the items in the book.
    pub fn iter(&self) -> BookItems {
        BookItems { items: self.sections.iter().collect() }
    }
}

/// Enum representing any type of item which can be added to a book.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BookItem {
    /// A nested chapter.
    Chapter(Chapter),
    /// A section separator.
    Separator,
}

/// The representation of a "chapter", usually mapping to a single file on
/// disk however it may contain multiple sub-chapters.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    /// The chapter's name.
    pub name: String,
    /// The chapter's contents.
    pub content: String,
    /// The chapter's section number, if it has one.
    pub number: Option<SectionNumber>,
    /// Nested items.
    pub sub_items: Vec<BookItem>,
}

impl Chapter {
    /// Create a new chapter with the provided content.
    pub fn new(name: &str, content: String) -> Chapter {
        Chapter {
            name: name.to_string(),
            content: content,
            ..Default::default()
        }
    }
}

/// Use the provided `Summary` to load a `Book` from disk.
///
/// You need to pass in the book's source directory because all the links in
/// `SUMMARY.md` give the chapter locations relative to it.
pub fn load_book_from_disk<P: AsRef<Path>>(summary: &Summary, src_dir: P) -> Result<Book> {
    debug!("[*] Loading the book from disk");
    let src_dir = src_dir.as_ref();

    let prefix = summary.prefix_chapters.iter();
    let numbered = summary.numbered_chapters.iter();
    let suffix = summary.suffix_chapters.iter();

    let summary_items = prefix.chain(numbered).chain(suffix);

    let chapters = summary_items
        .map(|i| load_summary_item(i, src_dir))
        .collect::<Result<_>>()
        .chain_err(|| "Couldn't load chapters from disk")?;

    Ok(Book { sections: chapters })
}

fn load_summary_item<P: AsRef<Path>>(item: &SummaryItem, src_dir: P) -> Result<BookItem> {
    match *item {
        SummaryItem::Separator => Ok(BookItem::Separator),
        SummaryItem::Link(ref link) => load_chapter(link, src_dir).map(|c| BookItem::Chapter(c)),
    }
}

fn load_chapter<P: AsRef<Path>>(link: &Link, src_dir: P) -> Result<Chapter> {
    debug!("[*] Loading {} ({})", link.name, link.location.display());
    let src_dir = src_dir.as_ref();

    let location = if link.location.is_absolute() {
        link.location.clone()
    } else {
        src_dir.join(&link.location)
    };

    let mut f = File::open(location).chain_err(|| {
        format!("Chapter file not found, {}", link.location.display())
    })?;

    let mut content = String::new();
    f.read_to_string(&mut content)?;

    let mut ch = Chapter::new(&link.name, content);
    ch.number = link.number.clone();

    let sub_items = link.nested_items
        .iter()
        .map(|i| load_summary_item(i, src_dir))
        .collect::<Result<Vec<_>>>()?;

    ch.sub_items = sub_items;

    Ok(ch)
}

/// A depth-first iterator over the items in a book.
///
/// # Note
///
/// This struct shouldn't be created directly, instead prefer the
/// [`Book::iter()`] method.
///
/// [`Book::iter()`]: struct.Book.html#method.iter
pub struct BookItems<'a> {
    items: VecDeque<&'a BookItem>,
}

impl<'a> Iterator for BookItems<'a> {
    type Item = &'a BookItem;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.items.pop_front();

        if let Some(&BookItem::Chapter(ref ch)) = item {
            // if we wanted a breadth-first iterator we'd `extend()` here
            for sub_item in ch.sub_items.iter().rev() {
                self.items.push_front(sub_item);
            }
        }

        item
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    use std::io::Write;

    const DUMMY_SRC: &'static str = "
# Dummy Chapter

this is some dummy text.

And here is some more text.
";

    /// Create a dummy `Link` in a temporary directory.
    fn dummy_link() -> (Link, TempDir) {
        let temp = TempDir::new("book").unwrap();

        let chapter_path = temp.path().join("chapter_1.md");
        File::create(&chapter_path)
            .unwrap()
            .write(DUMMY_SRC.as_bytes())
            .unwrap();

        let link = Link::new("Chapter 1", chapter_path);

        (link, temp)
    }

    /// Create a nested `Link` written to a temporary directory.
    fn nested_links() -> (Link, TempDir) {
        let (mut root, temp_dir) = dummy_link();

        let second_path = temp_dir.path().join("second.md");

        File::create(&second_path)
            .unwrap()
            .write_all("Hello World!".as_bytes())
            .unwrap();


        let mut second = Link::new("Nested Chapter 1", &second_path);
        second.number = Some(SectionNumber(vec![1, 2]));

        root.push_item(second.clone());
        root.push_item(SummaryItem::Separator);
        root.push_item(second.clone());

        (root, temp_dir)
    }

    #[test]
    fn load_a_single_chapter_from_disk() {
        let (link, temp_dir) = dummy_link();
        let should_be = Chapter::new("Chapter 1", DUMMY_SRC.to_string());

        let got = load_chapter(&link, temp_dir.path()).unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn cant_load_a_nonexistent_chapter() {
        let link = Link::new("Chapter 1", "/foo/bar/baz.md");

        let got = load_chapter(&link, "");
        assert!(got.is_err());
    }

    #[test]
    fn load_recursive_link_with_separators() {
        let (root, _temp) = nested_links();

        let nested = Chapter {
            name: String::from("Nested Chapter 1"),
            content: String::from("Hello World!"),
            number: Some(SectionNumber(vec![1, 2])),
            sub_items: Vec::new(),
        };
        let should_be = BookItem::Chapter(Chapter {
            name: String::from("Chapter 1"),
            content: String::from(DUMMY_SRC),
            number: None,
            sub_items: vec![
                BookItem::Chapter(nested.clone()),
                BookItem::Separator,
                BookItem::Chapter(nested.clone()),
            ],
        });

        let got = load_summary_item(&SummaryItem::Link(root), "").unwrap();
        assert_eq!(got, should_be);
    }

    #[test]
    fn load_a_book_with_a_single_chapter() {
        let (link, _temp) = dummy_link();
        let summary = Summary {
            numbered_chapters: vec![SummaryItem::Link(link)],
            ..Default::default()
        };
        let should_be = Book {
            sections: vec![
                BookItem::Chapter(Chapter {
                    name: String::from("Chapter 1"),
                    content: String::from(DUMMY_SRC),
                    ..Default::default()
                }),
            ],
        };

        let got = load_book_from_disk(&summary, "").unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn book_iter_iterates_over_sequential_items() {
        let book = Book {
            sections: vec![
                BookItem::Chapter(Chapter {
                    name: String::from("Chapter 1"),
                    content: String::from(DUMMY_SRC),
                    ..Default::default()
                }),
                BookItem::Separator,
            ],
        };

        let should_be: Vec<_> = book.sections.iter().collect();

        let got: Vec<_> = book.iter().collect();

        assert_eq!(got, should_be);
    }

    #[test]
    fn iterate_over_nested_book_items() {
        let book = Book {
            sections: vec![
                BookItem::Chapter(Chapter {
                    name: String::from("Chapter 1"),
                    content: String::from(DUMMY_SRC),
                    number: None,
                    sub_items: vec![
                        BookItem::Chapter(Chapter::new("Hello World", String::new())),
                        BookItem::Separator,
                        BookItem::Chapter(Chapter::new("Goodbye World", String::new())),
                    ],
                }),
                BookItem::Separator,
            ],
        };


        let got: Vec<_> = book.iter().collect();

        assert_eq!(got.len(), 5);

        // checking the chapter names are in the order should be sufficient here...
        let chapter_names: Vec<String> = got.into_iter()
            .filter_map(|i| match *i {
                BookItem::Chapter(ref ch) => Some(ch.name.clone()),
                _ => None,
            })
            .collect();
        let should_be: Vec<_> = vec![
            String::from("Chapter 1"),
            String::from("Hello World"),
            String::from("Goodbye World"),
        ];

        assert_eq!(chapter_names, should_be);
    }
}