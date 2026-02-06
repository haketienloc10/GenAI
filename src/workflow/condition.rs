use anyhow::Result;

use crate::util::templating::render_template;
use crate::workflow::context::ExecutionContext;

pub fn evaluate_if(expr: &str, ctx: &ExecutionContext) -> Result<bool> {
    let resolved = render_template(expr, ctx.as_map())?;
    let trimmed = resolved.trim();

    if let Some(left) = trimmed.strip_suffix("== ''") {
        return Ok(left.trim().is_empty());
    }
    if let Some(left) = trimmed.strip_suffix("!= ''") {
        return Ok(!left.trim().is_empty());
    }
    if let Some((left, right)) = trimmed.split_once("==") {
        let left = left.trim();
        let right = right.trim().trim_matches('\'');
        return Ok(left == right);
    }

    Ok(false)
}
