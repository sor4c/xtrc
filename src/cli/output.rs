//! Presentation Engine, Output Formatters, and Unix Stream Contracts for xtrc.
//!
//! // @trace SPEC-CLI-OUTPUT, SPEC-CLI-MATRIX, SPEC-CLI-TREE, SPEC-CLI-EXPORT
//! // @trace REQ-FMT-JSON, REQ-UNIX-STREAMS, REQ-EXIT-CODES

use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::str::FromStr;

use crate::error::CliError;

/// Supported output formatting strategies across CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Interactive terminal table with calculated column widths.
    Table,
    /// GitHub-flavored markdown table.
    Markdown,
    /// Comma-Separated Values conforming to RFC 4180.
    Csv,
    /// Machine-readable universal JSON document.
    Json,
    /// Standalone human-readable HTML document.
    Html,
}

impl FromStr for OutputFormat {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "markdown" | "md" => Ok(Self::Markdown),
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            "html" | "htm" => Ok(Self::Html),
            other => Err(CliError::InvalidFormat {
                format: other.to_string(),
            }),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Markdown => write!(f, "markdown"),
            Self::Csv => write!(f, "csv"),
            Self::Json => write!(f, "json"),
            Self::Html => write!(f, "html"),
        }
    }
}

/// Supported presentation formats for the matrix subcommand (type alias for consistency).
pub type MatrixFormat = OutputFormat;

/// Runtime stream environment detecting TTY presence and managing ANSI color states.
#[derive(Debug, Clone, Copy)]
pub struct StreamEnvironment {
    /// True if standard output is connected to an interactive terminal.
    pub stdout_is_terminal: bool,
    /// True if standard error is connected to an interactive terminal.
    pub stderr_is_terminal: bool,
}

impl StreamEnvironment {
    /// Probes stdout and stderr streams and configures global color overrides.
    pub fn probe() -> Self {
        let stdout_is_terminal = io::stdout().is_terminal();
        let stderr_is_terminal = io::stderr().is_terminal();

        if !stdout_is_terminal {
            colored::control::set_override(false);
        }

        Self {
            stdout_is_terminal,
            stderr_is_terminal,
        }
    }

    /// Determines the default output format for matrix command:
    /// `Table` on interactive TTY, `Markdown` when stdout is piped/redirected.
    pub fn default_matrix_format(&self) -> OutputFormat {
        if self.stdout_is_terminal {
            OutputFormat::Table
        } else {
            OutputFormat::Markdown
        }
    }
}

/// Checks whether standard output is connected to an interactive terminal (TTY).
#[inline]
pub fn is_terminal_stdout() -> bool {
    io::stdout().is_terminal()
}

/// Emits text content to standard output through a locked, buffered writer.
pub fn write_buffered_stdout(content: &str) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    writer.write_all(content.as_bytes()).map_err(CliError::Io)?;
    writer.flush().map_err(CliError::Io)?;
    Ok(())
}

/// Escapes a CSV field according to RFC 4180.
pub fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_string()
    }
}

/// Renders a slice of headers and rows as a GitHub-flavored Markdown table.
pub fn render_markdown_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str("| ");
    out.push_str(&headers.join(" | "));
    out.push_str(" |\n| ");
    let separators: Vec<&str> = headers.iter().map(|_| "---").collect();
    out.push_str(&separators.join(" | "));
    out.push_str(" |\n");

    for row in rows {
        out.push_str("| ");
        let escaped_cells: Vec<String> = row
            .iter()
            .map(|c| c.replace('|', "\\|").replace('\n', " "))
            .collect();
        out.push_str(&escaped_cells.join(" | "));
        out.push_str(" |\n");
    }

    out
}

/// Renders a slice of headers and rows as RFC 4180 compliant CSV.
pub fn render_csv(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    let escaped_headers: Vec<String> = headers.iter().map(|h| escape_csv_field(h)).collect();
    out.push_str(&escaped_headers.join(","));
    out.push('\n');

    for row in rows {
        let escaped_cells: Vec<String> = row.iter().map(|cell| escape_csv_field(cell)).collect();
        out.push_str(&escaped_cells.join(","));
        out.push('\n');
    }

    out
}

/// Renders a slice of headers and rows as an aligned text table.
pub fn render_aligned_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                let cell_len = cell.chars().count();
                if cell_len > col_widths[i] {
                    col_widths[i] = cell_len;
                }
            }
        }
    }

    let mut out = String::new();

    // 1. Header row
    for (i, header) in headers.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        let width = col_widths[i];
        out.push_str(&format!("{:<width$}", header, width = width));
    }
    out.push('\n');

    // 2. Underline separator
    for (i, width) in col_widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&"-".repeat(*width));
    }
    out.push('\n');

    // 3. Data rows
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            let width = col_widths.get(i).copied().unwrap_or(cell.chars().count());
            out.push_str(&format!("{:<width$}", cell, width = width));
        }
        out.push('\n');
    }

    out
}

/// Multi-column 2D tabular formatter supporting Terminal, Markdown, and CSV outputs.
#[derive(Debug, Clone, Default)]
pub struct TabularFormatter {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl TabularFormatter {
    /// Creates a new tabular formatter with specified header labels.
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
        }
    }

    /// Appends a data row to the table.
    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// Renders the table in GitHub-flavored Markdown.
    pub fn render_markdown(&self) -> String {
        let headers: Vec<&str> = self.headers.iter().map(|s| s.as_str()).collect();
        render_markdown_table(&headers, &self.rows)
    }

    /// Renders the table in RFC 4180 CSV format.
    pub fn render_csv(&self) -> String {
        let headers: Vec<&str> = self.headers.iter().map(|s| s.as_str()).collect();
        render_csv(&headers, &self.rows)
    }

    /// Renders the table for interactive terminal display with box-drawing borders.
    pub fn render_terminal_table(&self) -> String {
        let col_count = self.headers.len();
        if col_count == 0 {
            return String::new();
        }

        let mut widths = vec![0; col_count];
        for (i, h) in self.headers.iter().enumerate() {
            widths[i] = h.chars().count();
        }
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    widths[i] = widths[i].max(cell.chars().count());
                }
            }
        }

        let mut out = String::new();

        // Top border: ┌───┬───┐
        out.push('┌');
        for (i, &w) in widths.iter().enumerate() {
            out.push_str(&"─".repeat(w + 2));
            if i + 1 < col_count {
                out.push('┬');
            }
        }
        out.push_str("┐\n");

        // Header: │ Title │
        out.push('│');
        for (i, h) in self.headers.iter().enumerate() {
            out.push_str(&format!(" {:<w$} │", h, w = widths[i]));
        }
        out.push('\n');

        // Middle separator: ├───┼───┤
        out.push('├');
        for (i, &w) in widths.iter().enumerate() {
            out.push_str(&"─".repeat(w + 2));
            if i + 1 < col_count {
                out.push('┼');
            }
        }
        out.push_str("┤\n");

        // Data rows
        for row in &self.rows {
            out.push('│');
            for (i, width) in widths.iter().enumerate() {
                let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                out.push_str(&format!(" {:<w$} │", cell, w = *width));
            }
            out.push('\n');
        }

        // Bottom border: └───┴───┘
        out.push('└');
        for (i, &w) in widths.iter().enumerate() {
            out.push_str(&"─".repeat(w + 2));
            if i + 1 < col_count {
                out.push('┴');
            }
        }
        out.push_str("┘\n");

        out
    }
}

/// Pure Unix stream emission helpers.
pub struct StreamEmitter;

impl StreamEmitter {
    /// Writes pure payload data to `stdout` followed by newline.
    pub fn write_stdout(data: &str) -> Result<(), CliError> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        write!(handle, "{data}").map_err(CliError::Io)?;
        if !data.ends_with('\n') {
            writeln!(handle).map_err(CliError::Io)?;
        }
        handle.flush().map_err(CliError::Io)?;
        Ok(())
    }

    /// Writes diagnostic or progress message to `stderr`.
    pub fn write_stderr(msg: &str) {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(handle, "{msg}");
        let _ = handle.flush();
    }

    /// Writes diagnostic warning notice to `stderr`.
    pub fn write_warning(msg: &str) {
        Self::write_stderr(&format!("[WARN] {msg}"));
    }

    /// Writes fatal error message to `stderr`.
    pub fn write_error(msg: &str) {
        Self::write_stderr(&format!("Error: {msg}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_buffered_stdout_basic() {
        assert!(write_buffered_stdout("").is_ok());
    }

    #[test]
    fn test_format_parsing() {
        assert_eq!(
            OutputFormat::from_str("table").ok(),
            Some(OutputFormat::Table)
        );
        assert_eq!(
            OutputFormat::from_str("TABLE").ok(),
            Some(OutputFormat::Table)
        );
        assert_eq!(
            OutputFormat::from_str("markdown").ok(),
            Some(OutputFormat::Markdown)
        );
        assert_eq!(
            OutputFormat::from_str("md").ok(),
            Some(OutputFormat::Markdown)
        );
        assert_eq!(OutputFormat::from_str("csv").ok(), Some(OutputFormat::Csv));
        assert_eq!(
            OutputFormat::from_str("json").ok(),
            Some(OutputFormat::Json)
        );
        assert!(OutputFormat::from_str("invalid_format").is_err());
    }

    #[test]
    fn test_markdown_rendering() {
        let headers = ["ID", "Title", "Status"];
        let rows = vec![
            vec![
                "REQ-01".to_string(),
                "Verify flow".to_string(),
                "Covered".to_string(),
            ],
            vec![
                "REQ-02".to_string(),
                "Pipe | test".to_string(),
                "Uncovered".to_string(),
            ],
        ];
        let md = render_markdown_table(&headers, &rows);
        assert!(md.contains("| ID | Title | Status |"));
        assert!(md.contains("| --- | --- | --- |"));
        assert!(md.contains("| REQ-01 | Verify flow | Covered |"));
        assert!(md.contains(r#"| REQ-02 | Pipe \| test | Uncovered |"#));
    }

    #[test]
    fn test_csv_escaping() {
        let headers = ["ID", "Title", "Notes"];
        let rows = vec![
            vec![
                "REQ-01".to_string(),
                "Standard title".to_string(),
                "None".to_string(),
            ],
            vec![
                "REQ-02".to_string(),
                "Title, with comma".to_string(),
                "Quote \"test\"".to_string(),
            ],
        ];
        let csv = render_csv(&headers, &rows);
        assert!(csv.contains("ID,Title,Notes\n"));
        assert!(csv.contains("REQ-01,Standard title,None\n"));
        assert!(csv.contains("REQ-02,\"Title, with comma\",\"Quote \"\"test\"\"\"\n"));
    }

    #[test]
    fn test_aligned_table_rendering() {
        let headers = ["ID", "Status"];
        let rows = vec![
            vec!["REQ-LONG-ID-01".to_string(), "Covered".to_string()],
            vec!["REQ-02".to_string(), "Uncovered".to_string()],
        ];
        let table = render_aligned_table(&headers, &rows);
        assert!(table.contains("ID"));
        assert!(table.contains("REQ-LONG-ID-01"));
        assert!(table.contains("---"));
    }
}
