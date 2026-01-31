use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tabled::{Table, Tabled, settings::Style};

/// Salesforce package.xml viewer - displays metadata components in readable formats
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the package.xml file
    path: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,

    /// Sort order
    #[arg(short, long, value_enum, default_value_t = SortOrder::ByType)]
    sort: SortOrder,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Table,
    Csv,
    Tsv,
}

#[derive(Clone, ValueEnum)]
enum SortOrder {
    ByType,
    AsIs,
}

// XML deserialization structures
#[derive(Debug, Deserialize)]
struct Package {
    #[serde(default)]
    types: Vec<Types>,
}

#[derive(Debug, Deserialize)]
struct Types {
    #[serde(default)]
    members: Vec<String>,
    name: String,
}

// Output structure
#[derive(Tabled)]
struct ComponentRow {
    #[tabled(rename = "Type")]
    metadata_type: String,
    #[tabled(rename = "Member")]
    member: String,
}

fn parse_package_xml(path: &PathBuf) -> Result<Package, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let package: Package = quick_xml::de::from_str(&content)?;
    Ok(package)
}

fn flatten_components(package: &Package, sort_order: &SortOrder) -> Vec<ComponentRow> {
    let mut rows: Vec<ComponentRow> = package
        .types
        .iter()
        .flat_map(|t| {
            t.members.iter().map(|m| ComponentRow {
                metadata_type: t.name.clone(),
                member: m.clone(),
            })
        })
        .collect();

    if matches!(sort_order, SortOrder::ByType) {
        rows.sort_by(|a, b| {
            a.metadata_type
                .cmp(&b.metadata_type)
                .then_with(|| a.member.cmp(&b.member))
        });
    }

    rows
}

fn output_table(rows: &[ComponentRow]) -> io::Result<()> {
    let mut table = Table::new(rows);
    table.with(Style::modern());
    writeln!(io::stdout(), "{}", table)
}

fn output_csv(rows: &[ComponentRow]) -> io::Result<()> {
    let mut stdout = io::stdout();
    writeln!(stdout, "Type,Member")?;
    for row in rows {
        writeln!(stdout, "{},{}", row.metadata_type, row.member)?;
    }
    Ok(())
}

fn output_tsv(rows: &[ComponentRow]) -> io::Result<()> {
    let mut stdout = io::stdout();
    writeln!(stdout, "Type\tMember")?;
    for row in rows {
        writeln!(stdout, "{}\t{}", row.metadata_type, row.member)?;
    }
    Ok(())
}

fn main() {
    let args = Cli::parse();

    let package = match parse_package_xml(&args.path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: Failed to parse {}: {}", args.path.display(), e);
            std::process::exit(1);
        }
    };

    let rows = flatten_components(&package, &args.sort);

    let result = match args.format {
        OutputFormat::Table => output_table(&rows),
        OutputFormat::Csv => output_csv(&rows),
        OutputFormat::Tsv => output_tsv(&rows),
    };

    if let Err(e) = result
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        eprintln!("Error writing output: {}", e);
        std::process::exit(1);
    }
}
