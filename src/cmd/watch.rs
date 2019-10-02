use crate::{get_book_dir, open};
use clap::{App, ArgMatches, SubCommand};
use ignore::gitignore::Gitignore;
use mdbook::errors::Result;
use mdbook::utils;
use mdbook::MDBook;
use notify::Watcher;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;

// Create clap subcommand arguments
pub fn make_subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("watch")
        .about("Watches a book's files and rebuilds it on changes")
        .arg_from_usage(
            "-d, --dest-dir=[dest-dir] 'Output directory for the book{n}\
             Relative paths are interpreted relative to the book's root directory.{n}\
             If omitted, mdBook uses build.build-dir from book.toml or defaults to `./book`.'",
        )
        .arg_from_usage(
            "[dir] 'Root directory for the book{n}\
             (Defaults to the Current Directory when omitted)'",
        )
        .arg_from_usage("-o, --open 'Open the compiled book in a web browser'")
        .arg_from_usage("-i, --gitignore 'Respect .gitignore and skip changes to ignored files'")
}

// Watch command implementation
pub fn execute(args: &ArgMatches) -> Result<()> {
    let book_dir = get_book_dir(args);
    let book = MDBook::load(&book_dir)?;

    if args.is_present("open") {
        book.build()?;
        open(book.build_dir_for("html").join("index.html"));
    }

    let gitignore_path: Option<PathBuf> = if args.is_present("gitignore") {
        let mut path = book_dir.clone();
        path.push(".gitignore");
        Some(path)
    } else {
        None
    };

    trigger_on_change(&book, gitignore_path, |paths, book_dir| {
        info!("Files changed: {:?}\nBuilding book...\n", paths);
        let result = MDBook::load(&book_dir).and_then(|b| b.build());

        if let Err(e) = result {
            error!("Unable to build the book");
            utils::log_backtrace(&e);
        }
    });

    Ok(())
}

/// Calls the closure when a book source file is changed, blocking indefinitely.
pub fn trigger_on_change<F>(book: &MDBook, gitignore_path: Option<PathBuf>, closure: F)
where
    F: Fn(Vec<PathBuf>, &Path),
{
    use notify::DebouncedEvent::*;
    use notify::RecursiveMode::*;

    let gitignore = match gitignore_path {
        Some(path) => {
            info!("Skipping updates for files ignored by .gitignore");
            Gitignore::new(path).0
        }
        _ => Gitignore::empty(),
    };

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    let mut watcher = match notify::watcher(tx, Duration::from_secs(1)) {
        Ok(w) => w,
        Err(e) => {
            error!("Error while trying to watch the files:\n\n\t{:?}", e);
            std::process::exit(1)
        }
    };

    // Add the source directory to the watcher
    if let Err(e) = watcher.watch(book.source_dir(), Recursive) {
        error!("Error while watching {:?}:\n    {:?}", book.source_dir(), e);
        std::process::exit(1);
    };

    let _ = watcher.watch(book.theme_dir(), Recursive);

    // Add the book.toml file to the watcher if it exists
    let _ = watcher.watch(book.root.join("book.toml"), NonRecursive);

    info!("Listening for changes...");

    loop {
        let first_event = rx.recv().unwrap();
        sleep(Duration::from_millis(50));
        let other_events = rx.try_iter();

        let all_events = std::iter::once(first_event).chain(other_events);

        let paths: Vec<PathBuf> = all_events
            .filter_map(|event| {
                debug!("Received filesystem event: {:?}", event);

                match event {
                    Create(path) | Write(path) | Remove(path) | Rename(_, path) => {
                        if !gitignore.matched(&path, false).is_ignore() {
                            Some(path)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect();

        if !paths.is_empty() {
            closure(paths, &book.root);
        }
    }
}
