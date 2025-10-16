pub mod json;
pub mod table;
pub mod ascii;

use crate::stats::{OutputFormat, StatsContext};
use anyhow::Result;

pub fn render_repo_placeholder(ctx: &StatsContext, fmt: OutputFormat) -> Result<()> {
    match fmt {
        OutputFormat::Json => json::print(ctx),
        OutputFormat::Table => table::print_repo_placeholder(ctx),
        OutputFormat::Chart => ascii::print_repo_placeholder(ctx),
    }
}
