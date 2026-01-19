//! Output formatters

pub mod csv;
pub mod files;
pub mod json;
pub mod markdown;
pub mod terminal;
pub mod xml;

use crate::app::OutputFormat;
use agentroot_core::SearchResult;

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
