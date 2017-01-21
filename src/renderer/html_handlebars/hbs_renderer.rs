use renderer::html_handlebars::helpers;
use renderer::Renderer;
use book::{MDBook, Book};
use book::chapter::{Chapter, TranslationLink};
use book::toc::{TocItem, TocContent};
use utils;

use std::process::exit;
use std::path::PathBuf;
use std::fs;
use std::error::Error;
use std::io::{self, Write};

use handlebars::Handlebars;

use serde_json;
use serde_json::value::ToJson;

pub struct HtmlHandlebars;

impl HtmlHandlebars {
    pub fn new() -> Self {
        HtmlHandlebars
    }
}

impl Renderer for HtmlHandlebars {

    /// Prepares the project and calls `render()`.
    fn build(&self, project_root: &PathBuf, dest_base: &Option<PathBuf>) -> Result<MDBook, Box<Error>> {
        debug!("[fn]: build");

        let mut book_project = MDBook::new(&project_root);

        book_project.read_config();

        if let Some(p) = dest_base.clone() {
            book_project.set_dest_base(&p);
        }

        if !book_project.get_src_base().exists() {
            println!("Source folder doesn't exist: {:?}", book_project.get_src_base());
            exit(2);
        }

        book_project.parse_books();
        book_project.link_translations();

        try!(utils::fs::clean_output_dir(&book_project.get_dest_base()));

        match self.render(&book_project) {
            Ok(_) => {},
            Err(e) => {
                println!("Error: {:#?}", e);
                return Err(e);
            }
        }

        Ok(book_project)
    }

    /// Renders the chapters and copies static assets.
    fn render(&self, book_project: &MDBook) -> Result<(), Box<Error>> {
        debug!("[fn]: render");

        debug!("[*]: Check if book's base output folder exists");
        if let Err(_) = fs::create_dir_all(&book_project.get_dest_base()) {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Unexpected error when constructing path")
            ));
        }

        // Copy book's static assets

        if book_project.get_project_root().join("assets").exists() {

            let a = book_project.get_project_root().join("assets");
            let base = a.to_str().unwrap();

            let b = a.join("**").join("*");
            let include_glob = b.to_str().unwrap();

            let c = a.join("_*");
            let exclude_glob = c.to_str().unwrap();

            // Ignoring all errors. Should try to see which types are worth returning.

            match utils::fs::copy_files(include_glob,
                                        base,
                                        vec![exclude_glob],
                                        &book_project.get_dest_base()) {
                Ok(_) => {},
                Err(_) => {},
            }
        }

        // Copy template's static assets

        // If there is a template dir in the books's project folder, copy asset
        // files from there, otherwise copy from embedded assets.

        if book_project.get_template_dir().exists() {

            let a = book_project.get_template_dir();
            let base = a.to_str().unwrap();

            let b = a.join("**").join("*");
            let include_glob = b.to_str().unwrap();

            let c = a.join("_*");
            let exclude_glob = c.to_str().unwrap();

            // don't try!(), copy_files() will send error values when trying to copy folders that are part of the file glob
            //
            // Error {
            //     repr: Custom(
            //         Custom {
            //             kind: Other,
            //             error: StringError(
            //                 "Err(Error { repr: Custom(Custom { kind: InvalidInput, error: StringError(\"the source path is not an existing regular file\") }) })\n"
            //             )
            //         }
            //     )
            // }

            // Ignoring all errors. Should try to see which types are worth returning.

            match utils::fs::copy_files(include_glob,
                                        base,
                                        vec![exclude_glob],
                                        &book_project.get_dest_base()) {
                Ok(_) => {},
                Err(_) => {},
            }
        } else {
            try!(utils::fs::copy_data("data/assets/_html-template/**/*",
                                      "data/assets/_html-template/",
                                      vec!["data/assets/_html-template/_*"],
                                      &book_project.get_dest_base()));
        }

        debug!("[*]: start rendering");
        let mut handlebars = Handlebars::new();

        let translation_indexes = book_project.translation_index_links();

        let first_path_that_exists = |paths: &Vec<PathBuf>| -> Option<PathBuf> {
            for p in paths.iter() {
                if p.exists() {
                    return Some(PathBuf::from(p));
                }
            }
            None
        };

        {
            // Look for the page template in these paths
            let mut search_paths: Vec<PathBuf> = vec![];

            // default scheme: assets/_html-template/_layouts/page.hbs
            search_paths.push(book_project.get_template_dir().join("_layouts").join("page.hbs"));
            // maybe the user doesn't use _layouts folder: assets/_html-template/page.hbs
            search_paths.push(book_project.get_template_dir().join("page.hbs"));
            // also look for index.hbs which was the template name in v0.0.15
            search_paths.push(book_project.get_template_dir().join("index.hbs"));

            let s = if let Some(p) = first_path_that_exists(&search_paths) {
                try!(utils::fs::file_to_string(&p))
            } else {
                try!(utils::fs::get_data_file("data/assets/_html-template/_layouts/page.hbs"))
            };

            // Register template
            debug!("[*]: Register handlebars template");
            try!(handlebars.register_template_string("page", s));
        }

        // Register helpers
        debug!("[*]: Register handlebars helpers");
        handlebars.register_helper("toc", Box::new(helpers::toc::RenderToc));
        handlebars.register_helper("previous", Box::new(helpers::navigation::previous));
        handlebars.register_helper("next", Box::new(helpers::navigation::next));
        handlebars.register_helper("translation-links", Box::new(helpers::translations::TranslationLinksHelper));
        handlebars.register_helper("translation-indexes", Box::new(helpers::translations::TranslationIndexesHelper));
        handlebars.register_helper("customcss", Box::new(helpers::customcss::CustomCssHelper));

        let mut custom_css_path: Option<PathBuf> = None;
        {
            // See if the user has a custom.css
            let mut search_paths: Vec<PathBuf> = vec![];

            search_paths.push(book_project.get_project_root().join("assets").join("css").join("custom.css"));
            search_paths.push(book_project.get_project_root().join("assets").join("stylesheets").join("custom.css"));
            search_paths.push(book_project.get_project_root().join("assets").join("custom.css"));

            if let Some(p) = first_path_that_exists(&search_paths) {
                match p.strip_prefix(&book_project.get_project_root().join("assets")) {
                    Ok(x) => { custom_css_path = Some(PathBuf::from(x)); },
                    Err(_) => {},
                }
            }
        }

        // Render the chapters of each book
        for (_, book) in &book_project.translations {

            // Check if book's dest directory exists

            // If this is a single book, config.dest default is
            // `project_root/book`, and the earlier check will cover this.

            // If this is multi-language book, config.dest will
            // `project_book/book/key`, and so we check here for each book.

            debug!("[*]: Check if book's destination directory exists");
            if let Err(_) = fs::create_dir_all(book.config.get_dest()) {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    "Unexpected error when constructing destination path")
                ));
            }

            // If this is the main book of a multi-language book, add an
            // index.html to the project dest folder

            if book.config.is_multilang && book.config.is_main_book {
                match book.toc[0] {
                    TocItem::Numbered(ref i) |
                    TocItem::Unnumbered(ref i) |
                    TocItem::Unlisted(ref i) => {
                        let mut chapter: Chapter = i.chapter.clone();
                        chapter.set_dest_path(PathBuf::from("index.html".to_string()));

                        // almost the same as process_chapter(), but we have to
                        // manipulate path_to_root in data and rendered_path

                        let mut data = try!(make_data(&book, &chapter, &translation_indexes, &book_project.livereload_script, &custom_css_path));

                        data.remove("path_to_root");
                        data.insert("path_to_root".to_owned(), "".to_json());

                        // Render the handlebars template with the data
                        debug!("[*]: Render template");
                        let rendered_content = try!(handlebars.render("page", &data));

                        let p = chapter.get_dest_path().unwrap();
                        let rendered_path = &book_project.get_dest_base().join(&p);

                        debug!("[*]: Create file {:?}", rendered_path);

                        // Write to file
                        let mut file = try!(utils::fs::create_file(rendered_path));
                        info!("[*] Creating {:?} ✓", rendered_path);

                        try!(file.write_all(&rendered_content.into_bytes()));
                    },
                    TocItem::Spacer => {},
                }
            }

            // Render a file for every entry in the book
            try!(self.process_items(&book.toc, &book, &translation_indexes, &book_project.livereload_script, &custom_css_path, &handlebars));

            // Write print.html
            if let Some(content) = self.collect_print_content_markdown(&book.toc, &book) {

                let mut chapter: Chapter = Chapter::new(book.config.title.to_owned(), PathBuf::from(""));

                chapter.set_dest_path(PathBuf::from("print.html"));
                chapter.content = Some(content);

                try!(self.process_chapter(&chapter, &book, &None, &None, &custom_css_path, &handlebars));
            }
        }

        Ok(())
    }
}

impl HtmlHandlebars {

    fn process_items(&self,
                     items: &Vec<TocItem>,
                     book: &Book,
                     translation_indexes: &Option<Vec<TranslationLink>>,
                     livereload_script: &Option<String>,
                     custom_css_path: &Option<PathBuf>,
                     handlebars: &Handlebars)
                     -> Result<(), Box<Error>> {

        for item in items.iter() {
            match *item {
                TocItem::Numbered(ref i) |
                TocItem::Unnumbered(ref i) |
                TocItem::Unlisted(ref i) => {
                    if let Some(_) = i.chapter.get_dest_path() {
                        try!(self.process_chapter(&i.chapter, book, translation_indexes, livereload_script, custom_css_path, handlebars));
                    }

                    if let Some(ref subs) = i.sub_items {
                        try!(self.process_items(&subs, book, translation_indexes, livereload_script, custom_css_path, handlebars));
                    }

                },
                TocItem::Spacer => {},
            }
        }

        Ok(())
    }

    fn collect_print_content_markdown(&self, items: &Vec<TocItem>, book: &Book) -> Option<String> {
        let mut text = "".to_string();

        for item in items.iter() {
            match *item {
                TocItem::Numbered(ref i) |
                TocItem::Unnumbered(ref i) |
                TocItem::Unlisted(ref i) => {
                    if let Some(content) = i.chapter.content.clone() {
                        text.push_str(&content);
                    }

                    if let Some(ref subs) = i.sub_items {
                        if let Some(x) = self.collect_print_content_markdown(subs, book) {
                            text.push_str(&x);
                        }
                    }

                },
                TocItem::Spacer => {},
            }
        }

        if text.len() > 0 {
            Some(text)
        } else {
            None
        }

    }

    fn process_chapter(&self,
                       chapter: &Chapter,
                       book: &Book,
                       translation_indexes: &Option<Vec<TranslationLink>>,
                       livereload_script: &Option<String>,
                       custom_css_path: &Option<PathBuf>,
                       handlebars: &Handlebars)
                       -> Result<(), Box<Error>> {

        let data = try!(make_data(book, chapter, translation_indexes, livereload_script, custom_css_path));

        // Render the handlebars template with the data
        debug!("[*]: Render template");
        let rendered_content = try!(handlebars.render("page", &data));

        let p = match chapter.get_dest_path() {
            Some(x) => x,
            None => {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    format!("process_chapter(), dest_path is None: {:#?}", chapter))
                ));
            }
        };

        let rendered_path = &book.config.get_dest().join(&p);

        debug!("[*]: Create file {:?}", rendered_path);

        // Write to file
        let mut file = try!(utils::fs::create_file(rendered_path));
        info!("[*] Creating {:?} ✓", rendered_path);

        try!(file.write_all(&rendered_content.into_bytes()));

        Ok(())
    }
}

fn make_data(book: &Book,
             chapter: &Chapter,
             translation_indexes: &Option<Vec<TranslationLink>>,
             livereload_script: &Option<String>,
             custom_css_path: &Option<PathBuf>)
             -> Result<serde_json::Map<String, serde_json::Value>, Box<Error>> {

    debug!("[fn]: make_data");

    let mut data = serde_json::Map::new();

    // Book data

    data.insert("language".to_owned(), book.config.language.code.to_json());
    data.insert("page-title".to_owned(), format!("{} - {}", chapter.title, book.config.title).to_json());
    data.insert("chapter-title".to_owned(), chapter.title.to_json());
    data.insert("description".to_owned(), book.config.description.to_json());
    data.insert("custom-css-path".to_owned(), custom_css_path.to_json());

    if let Some(ref x) = *livereload_script {
        data.insert("livereload".to_owned(), x.to_json());
    }

    // Chapter data

    match chapter.get_dest_path() {
        Some(mut path) => {
            if book.config.is_multilang {
                path = PathBuf::from(&book.config.language.code).join(&path);
            }
            match path.to_str() {
                Some(p) => {
                    data.insert("path".to_owned(), p.to_json());
                    data.insert("path_to_root".to_owned(), utils::fs::path_to_root(&path).to_json());
                },
                None => {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        "Could not convert path to str")
                    ))
                },
            }
        },
        None => {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                format!("make_data(), dest_path is None: {:#?}", chapter))
            ));
        }
    }

    match chapter.content.clone() {
        Some(mut content) => {
            content = utils::render_markdown(&content);

            // Parse for playpen links
            if let Some(a) = chapter.get_src_path() {
                if let Some(p) = book.config.get_src().join(&a).parent() {
                    content = helpers::playpen::render_playpen(&content, p);
                }
            }

            data.insert("content".to_owned(), content.to_json());
        },
        None => {
            debug!("Chapter has dest_path but doesn't have content: {:#?}", chapter);
        },
    }

    if let Some(ref links) = *translation_indexes {
        data.insert("translation-indexes".to_owned(), links.to_json());
    }

    if let Some(ref links) = chapter.translation_links {
        data.insert("translation-links".to_owned(), links.to_json());
    }

    let chapters = try!(items_to_chapters(&book.toc, &book));

    data.insert("chapters".to_owned(), chapters.to_json());
    debug!("[*]: JSON constructed");
    Ok(data)
}

fn items_to_chapters(items: &Vec<TocItem>, book: &Book)
                 -> Result<Vec<serde_json::Map<String, serde_json::Value>>, Box<Error>> {

    let mut chapters = vec![];

    for item in items.iter() {

        match *item {
            TocItem::Numbered(ref i) |
            TocItem::Unnumbered(ref i) => {
                match process_chapter_and_subs(i, book) {
                    Ok(mut x) => chapters.append(&mut x),
                    Err(e) => return Err(e),
                }
            },
            TocItem::Spacer => {
                let mut chapter = serde_json::Map::new();
                chapter.insert("spacer".to_owned(), "_spacer_".to_json());
                chapters.push(chapter);
            },
            TocItem::Unlisted(_) => {},
        }
    }

    Ok(chapters)
}

fn process_chapter_and_subs(i: &TocContent, book: &Book)
                            -> Result<Vec<serde_json::Map<String, serde_json::Value>>, Box<Error>> {

    let mut chapters_data = vec![];

    // Create the data to inject in the template
    let mut data = serde_json::Map::new();
    let chapter = &i.chapter;

    if let Some(_) = i.section {
        let s = i.section_as_string();
        data.insert("section".to_owned(), s.to_json());
    }

    data.insert("title".to_owned(), chapter.title.to_json());

    match chapter.get_dest_path() {
        Some(mut path) => {
            if book.config.is_multilang {
                path = PathBuf::from(&book.config.language.code).join(&path);
            }
            match path.to_str() {
                Some(p) => {
                    data.insert("path".to_owned(), p.to_json());
                    data.insert("path_to_root".to_owned(), utils::fs::path_to_root(&path).to_json());
                },
                None => {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        "Could not convert path to str")
                    ))
                },
            }
        },
        // is draft chapter
        None => {}
    }

    chapters_data.push(data);

    if let Some(ref subs) = i.sub_items {
        let mut sub_chs = try!(items_to_chapters(&subs, book));
        chapters_data.append(&mut sub_chs);
    }

    Ok(chapters_data)
}
