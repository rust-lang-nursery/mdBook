use std::path::Path;
use std::collections::BTreeMap;

use serde_json;
use handlebars::{Context, Handlebars, Helper, RenderContext, RenderError, Renderable};


type StringMap = BTreeMap<String, String>;

/// Target for `find_chapter`.
enum Target {
    Previous,
    Next,
}

impl Target {
    /// Returns target if found.
    fn find(&self,
            base_path: &String,
            current_path: &String,
            current_item: &StringMap,
            previous_item: StringMap,
            ) -> Result<Option<StringMap>, RenderError> {
        match self {
            &Target::Next => {
                let previous_path = previous_item.get("path").ok_or_else(|| {
                    RenderError::new("No path found for chapter in JSON data")
                })?;

                if previous_path == base_path {
                    return Ok(Some(current_item.clone()));
                }
            },

            &Target::Previous => {
                if current_path == base_path {
                    return Ok(Some(previous_item));
                }
            }
        }

        Ok(None)
    }
}

fn find_chapter(
    rc: &mut RenderContext,
    target: Target
) -> Result<Option<StringMap>, RenderError> {
    debug!("[*]: Get data from context");

    let chapters = rc.evaluate_absolute("chapters").and_then(|c| {
        serde_json::value::from_value::<Vec<StringMap>>(c.clone())
            .map_err(|_| RenderError::new("Could not decode the JSON data"))
    })?;

    let base_path = rc.evaluate_absolute("path")?
                      .as_str()
                      .ok_or_else(|| RenderError::new("Type error for `path`, string expected"))?
                      .replace("\"", "");

    let mut previous: Option<StringMap> = None;

    debug!("[*]: Search for chapter");

    for item in chapters {
        match item.get("path") {
            Some(path) if !path.is_empty() => {
                if let Some(previous) = previous {
                    if let Some(item) = target.find(&base_path, &path, &item, previous)? {
                        return Ok(Some(item));
                    }
                }

                previous = Some(item.clone());
            }
            _ => continue,
        }
    }

   Ok(None)
}

fn render(
    _h: &Helper,
    r: &Handlebars,
    rc: &mut RenderContext,
    chapter: &StringMap,
) -> Result<(), RenderError> {
    debug!("[*]: Creating BTreeMap to inject in context");

    let mut context = BTreeMap::new();

    chapter.get("name")
            .ok_or_else(|| RenderError::new("No title found for chapter in JSON data"))
            .map(|name| context.insert("title".to_owned(), json!(name)))?;

    chapter.get("path")
            .ok_or_else(|| RenderError::new("No path found for chapter in JSON data"))
            .and_then(|p| {
                    Path::new(p).with_extension("html")
                    .to_str()
                    .ok_or_else(|| RenderError::new("Link could not be converted to str"))
                    .map(|p| context.insert("link".to_owned(), json!(p.replace("\\", "/"))))
            })?;

    debug!("[*]: Render template");

    _h.template()
        .ok_or_else(|| RenderError::new("Error with the handlebars template"))
        .and_then(|t| {
            let mut local_rc = rc.with_context(Context::wraps(&context)?);
            t.render(r, &mut local_rc)
        })?;

    Ok(())
}

pub fn previous(_h: &Helper, r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    debug!("[fn]: previous (handlebars helper)");

    if let Some(previous) = find_chapter(rc, Target::Previous)? {
        render(_h, r, rc, &previous)?;
    }

    Ok(())
}

pub fn next(_h: &Helper, r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    debug!("[fn]: next (handlebars helper)");

    if let Some(next) = find_chapter(rc, Target::Next)? {
        render(_h, r, rc, &next)?;
    }

    Ok(())
}
