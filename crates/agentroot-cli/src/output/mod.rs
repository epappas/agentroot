//! Output formatters

pub mod json;
pub mod csv;
pub mod xml;
pub mod markdown;
pub mod files;
pub mod terminal;

use agentroot_core::SearchResult;
use crate::app::OutputFormat;

/// Format options
pub struct FormatOptions {
    pub full: bool,
    #[allow(dead_code)]
    pub query: Option<String>,
    pub line_numbers: bool,
}

/// Format search results
pub fn format_search_results(
    results: &[SearchResult],
    format: OutputFormat,
    options: &FormatOptions,
) -> String {
    match format {
        OutputFormat::Json => json::format_results(results, options),
        OutputFormat::Csv => csv::format_results(results, options),
        OutputFormat::Xml => xml::format_results(results, options),
        OutputFormat::Md => markdown::format_results(results, options),
        OutputFormat::Files => files::format_results(results),
        OutputFormat::Cli => terminal::format_results(results, options),
    }
}
