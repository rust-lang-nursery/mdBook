use std::path::Path;

use handlebars::*;

/// Link to the current page.
fn active_link(rc: &RenderContext) -> Result<String, RenderError> {
    let path = rc.evaluate_absolute("path")?
        .as_str()
        .ok_or_else(|| RenderError::new("Expected `path` to be string"))?;

    let link = Path::new(path)
        .with_extension("html")
        .to_str()
        .unwrap()
        .replace("\\", "/");

    Ok(link)
}

/// Wrap a block in <a> if a link exists.
pub fn link(h: &Helper, r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    let link = rc.evaluate("link")?.clone();

    if let Some(link) = link.as_str() {
        rc.writer.write_all(b"<a")?;

        let active = active_link(rc)?;

        if active == link {
            rc.writer.write_all(b" class=\"active\"")?;
        }

        rc.writer.write_all(b" href=\"")?;
        rc.writer.write_all(link.as_bytes())?;
        rc.writer.write_all(b"\">")?;
    }

    h.template()
        .ok_or_else(|| RenderError::new("No template for link"))
        .and_then(|t| t.render(r, rc))?;

    if link.is_string() {
        rc.writer.write_all(b"</a>")?;
    }

    Ok(())
}
