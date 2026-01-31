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
#[derive(Tabled, Debug, PartialEq)]
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

fn output_table(rows: &[ComponentRow]) -> Result<(), Box<dyn std::error::Error>> {
    let mut table = Table::new(rows);
    table.with(Style::modern());
    writeln!(io::stdout(), "{}", table)?;
    Ok(())
}

fn output_csv<W: Write>(
    rows: &[ComponentRow],
    writer: W,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_writer(writer);
    // Always write header, even if rows is empty
    wtr.write_record(["Type", "Member"])?;
    for row in rows {
        wtr.write_record([&row.metadata_type, &row.member])?;
    }
    wtr.flush()?;
    Ok(())
}

fn output_tsv<W: Write>(
    rows: &[ComponentRow],
    writer: W,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(writer);
    // Always write header, even if rows is empty
    wtr.write_record(["Type", "Member"])?;
    for row in rows {
        wtr.write_record([&row.metadata_type, &row.member])?;
    }
    wtr.flush()?;
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
        OutputFormat::Csv => output_csv(&rows, io::stdout()),
        OutputFormat::Tsv => output_tsv(&rows, io::stdout()),
    };

    if let Err(e) = result {
        // Check if it's a broken pipe error
        if let Some(io_err) = e.downcast_ref::<io::Error>()
            && io_err.kind() == io::ErrorKind::BrokenPipe
        {
            return;
        }
        eprintln!("Error writing output: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create Package for testing
    fn make_package(types: Vec<(&str, Vec<&str>)>) -> Package {
        Package {
            types: types
                .into_iter()
                .map(|(name, members)| Types {
                    name: name.to_string(),
                    members: members.into_iter().map(|m| m.to_string()).collect(),
                })
                .collect(),
        }
    }

    // ==================== flatten_components tests ====================

    #[test]
    fn flatten_empty_package() {
        let package = make_package(vec![]);
        let rows = flatten_components(&package, &SortOrder::ByType);
        assert!(rows.is_empty());
    }

    #[test]
    fn flatten_empty_types() {
        let package = make_package(vec![("ApexClass", vec![])]);
        let rows = flatten_components(&package, &SortOrder::ByType);
        assert!(rows.is_empty());
    }

    #[test]
    fn flatten_single_type_single_member() {
        let package = make_package(vec![("ApexClass", vec!["MyClass"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            ComponentRow {
                metadata_type: "ApexClass".to_string(),
                member: "MyClass".to_string(),
            }
        );
    }

    #[test]
    fn flatten_single_type_multiple_members() {
        let package = make_package(vec![("ApexClass", vec!["ClassA", "ClassB", "ClassC"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].member, "ClassA");
        assert_eq!(rows[1].member, "ClassB");
        assert_eq!(rows[2].member, "ClassC");
    }

    #[test]
    fn flatten_multiple_types_multiple_members() {
        let package = make_package(vec![
            ("ApexClass", vec!["ClassA"]),
            ("ApexTrigger", vec!["TriggerA", "TriggerB"]),
        ]);
        let rows = flatten_components(&package, &SortOrder::AsIs);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].metadata_type, "ApexClass");
        assert_eq!(rows[1].metadata_type, "ApexTrigger");
        assert_eq!(rows[2].metadata_type, "ApexTrigger");
    }

    #[test]
    fn flatten_by_type_sorts_types_alphabetically() {
        let package = make_package(vec![
            ("CustomObject", vec!["Account"]),
            ("ApexClass", vec!["MyClass"]),
        ]);
        let rows = flatten_components(&package, &SortOrder::ByType);
        assert_eq!(rows[0].metadata_type, "ApexClass");
        assert_eq!(rows[1].metadata_type, "CustomObject");
    }

    #[test]
    fn flatten_as_is_preserves_order() {
        let package = make_package(vec![
            ("CustomObject", vec!["Account"]),
            ("ApexClass", vec!["MyClass"]),
        ]);
        let rows = flatten_components(&package, &SortOrder::AsIs);
        assert_eq!(rows[0].metadata_type, "CustomObject");
        assert_eq!(rows[1].metadata_type, "ApexClass");
    }

    #[test]
    fn flatten_by_type_sorts_members_within_type() {
        let package = make_package(vec![("ApexClass", vec!["Zebra", "Alpha", "Middle"])]);
        let rows = flatten_components(&package, &SortOrder::ByType);
        assert_eq!(rows[0].member, "Alpha");
        assert_eq!(rows[1].member, "Middle");
        assert_eq!(rows[2].member, "Zebra");
    }

    // ==================== output_csv tests ====================

    #[test]
    fn output_csv_empty_rows() {
        let rows: Vec<ComponentRow> = vec![];
        let mut buffer = Vec::new();
        output_csv(&rows, &mut buffer).unwrap();
        assert_eq!(String::from_utf8(buffer).unwrap(), "Type,Member\n");
    }

    #[test]
    fn output_csv_single_row() {
        let rows = vec![ComponentRow {
            metadata_type: "ApexClass".to_string(),
            member: "MyClass".to_string(),
        }];
        let mut buffer = Vec::new();
        output_csv(&rows, &mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Type,Member\nApexClass,MyClass\n"
        );
    }

    #[test]
    fn output_csv_multiple_rows() {
        let rows = vec![
            ComponentRow {
                metadata_type: "ApexClass".to_string(),
                member: "ClassA".to_string(),
            },
            ComponentRow {
                metadata_type: "ApexTrigger".to_string(),
                member: "TriggerA".to_string(),
            },
        ];
        let mut buffer = Vec::new();
        output_csv(&rows, &mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Type,Member\nApexClass,ClassA\nApexTrigger,TriggerA\n"
        );
    }

    // ==================== output_tsv tests ====================

    #[test]
    fn output_tsv_empty_rows() {
        let rows: Vec<ComponentRow> = vec![];
        let mut buffer = Vec::new();
        output_tsv(&rows, &mut buffer).unwrap();
        assert_eq!(String::from_utf8(buffer).unwrap(), "Type\tMember\n");
    }

    #[test]
    fn output_tsv_single_row() {
        let rows = vec![ComponentRow {
            metadata_type: "ApexClass".to_string(),
            member: "MyClass".to_string(),
        }];
        let mut buffer = Vec::new();
        output_tsv(&rows, &mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Type\tMember\nApexClass\tMyClass\n"
        );
    }
}
