# Configuration

You can configure the parameters for your book in the ***book.toml*** file.

Here is an example of what a ***book.toml*** file might look like:

```toml
[book]
title = "Example book"
author = "John Doe"
description = "The example book covers examples."

[output.html]
destination = "my-example-book"
additional-css = ["custom.css"]

[output.html.search]
enable = true
limit-results = 15
```

## Supported configuration options

It is important to note that **any** relative path specified in the in the configuration will
always be taken relative from the root of the book where the configuration file is located.


### General metadata

This is general information about your book.

- **title:** The title of the book
- **authors:** The author(s) of the book
- **description:** A description for the book, which is added as meta
  information in the html `<head>` of each page
- **src:** By default, the source directory is found in the directory named
  `src` directly under the root folder. But this is configurable with the `src`
  key in the configuration file.
- **build-dir:** The directory to put the rendered book in. By default this is
  `book/` in the book's root directory.

**book.toml**
```toml
[book]
title = "Example book"
authors = ["John Doe", "Jane Doe"]
description = "The example book covers examples."
src = "my-src"  # the source files will be found in `root/my-src` instead of `root/src`
build-dir = "build"
```

### HTML renderer options
The HTML renderer has a couple of options as well. All the options for the
renderer need to be specified under the TOML table `[output.html]`.

The following configuration options are available:

- **theme:** mdBook comes with a default theme and all the resource files
  needed for it. But if this option is set, mdBook will selectively overwrite
  the theme files with the ones found in the specified folder.
- **curly-quotes:** Convert straight quotes to curly quotes, except for
  those that occur in code blocks and code spans. Defaults to `false`.
- **google-analytics:** If you use Google Analytics, this option lets you
  enable it by simply specifying your ID in the configuration file.
- **additional-css:** If you need to slightly change the appearance of your
  book without overwriting the whole style, you can specify a set of
  stylesheets that will be loaded after the default ones where you can
  surgically change the style.
- **additional-js:** If you need to add some behaviour to your book without
  removing the current behaviour, you can specify a set of javascript files
  that will be loaded alongside the default one.
- **playpen:** A subtable for configuring various playpen settings.
- **search:** A subtable for configuring the browser based search functionality.

Available configuration options for the `[output.html.search]` table:

- **enable:** Enable or disable the search function. Disabling can improve compilation time by a factor of two. Defaults to `true`.
- **limit-results:** The maximum number of search results. Defaults to `30`.
- **teaser-word-count:** The number of words used for a search result teaser. Defaults to `30`.
- **use-boolean-and:** Define the logical link between multiple search words. If true, all search words must appear in each result. Defaults to `true`.
- **boost-title:** Boost factor for the search result score if a search word appears in the header. Defaults to `2`.
- **boost-hierarchy:** Boost factor for the search result score if a search word appears in the hierarchy. The hierarchy contains all titles of the parent documents and all parent headings. Defaults to `1`.
- **boost-paragraph:** Boost factor for the search result score if a search word appears in the text. Defaults to `1`.
- **expand:** True if the searchword `micro` should match `microwave`. Defaults to `true`.
- **split-until-heading:** Documents are split into smaller parts, seperated by headings. This defines, until which level of heading documents should be split. Defaults to `3`. (`### This is a level 3 heading`)

Available configuration options for the `[output.html.playpen]` table:

- **editor:** Source folder for the editors javascript files. Defaults to `""`.
- **editable:** Allow editing the source code. Defaults to `false`.

This shows all available options in the **book.toml**:
```toml
[book]
title = "Example book"
authors = ["John Doe", "Jane Doe"]
description = "The example book covers examples."
src = "my-src"  # the source files will be found in `root/my-src` instead of `root/src`
build-dir = "build"

[output.html]
theme = "my-theme"
curly-quotes = true
google-analytics = "123456"
additional-css = ["custom.css", "custom2.css"]
additional-js = ["custom.js"]

[output.html.search]
enable = true
limit-results = 30
teaser-word-count = 30
use-boolean-and = true
boost-title = 2
boost-hierarchy = 1
boost-paragraph = 1
expand = true
split-until-heading = 3

[output.html.playpen]
editor = "./path/to/editor"
editable = false
```


## For Developers

If you are developing a plugin or alternate backend then whenever your code is
called you will almost certainly be passed a reference to the book's `Config`. 
This can be treated roughly as a nested hashmap which lets you call methods like
`get()` and `get_mut()` to get access to the config's contents.

By convention, plugin developers will have their settings as a subtable inside
`plugins` (e.g. a link checker would put its settings in `plugins.link_check`) 
and backends should put their configuration under `output`, like the HTML 
renderer does in the previous examples.

As an example, some hypothetical `random` renderer would typically want to load
its settings from the `Config` at the very start of its rendering process. The
author can take advantage of serde to deserialize the generic `toml::Value` 
object retrieved from `Config` into a struct specific to its use case.

```rust
#[derive(Debug, Deserialize, PartialEq)]
struct RandomOutput {
    foo: u32,
    bar: String,
    baz: Vec<bool>,
}

let src = r#"
[output.random]
foo = 5
bar = "Hello World"
baz = [true, true, false]
"#;

let book_config = Config::from_str(src)?; // usually passed in by mdbook
let random: Value = book_config.get("output.random").unwrap_or_default();
let got: RandomOutput = random.try_into()?;

assert_eq!(got, should_be);

if let Some(baz) = book_config.get_deserialized::<Vec<bool>>("output.random.baz") {
  println!("{:?}", baz); // prints [true, true, false]

  // do something interesting with baz
}

// start the rendering process
```