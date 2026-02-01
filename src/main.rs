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

    /// Disable splitting Parent.Member format into separate columns
    #[arg(long, default_value_t = false)]
    no_split_parent: bool,
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
    #[tabled(rename = "Parent")]
    parent: String,
    #[tabled(rename = "Member")]
    member: String,
}

fn parse_package_xml(path: &PathBuf) -> Result<Package, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let package: Package = quick_xml::de::from_str(&content)?;
    Ok(package)
}

/// Types whose members should be split on the first `.` into Parent and Member columns
const SPLITTABLE_BY_DOT: &[&str] = &[
    "AssignmentRule",
    "CustomField",
    "ListView",
    "RecordType",
    "SharingCriteriaRule",
    "SharingOwnerRule",
    "SharingTerritoryRule",
];

/// Types whose members should be split on the first `-` into Parent and Member columns
const SPLITTABLE_BY_HYPHEN: &[&str] = &["Layout"];

fn flatten_components(
    package: &Package,
    sort_order: &SortOrder,
    split_parent: bool,
) -> Vec<ComponentRow> {
    let mut rows: Vec<ComponentRow> = package
        .types
        .iter()
        .flat_map(|t| {
            t.members.iter().map(|m| {
                let (parent, member) =
                    if split_parent && SPLITTABLE_BY_DOT.contains(&t.name.as_str()) {
                        match m.split_once('.') {
                            Some((p, rest)) => (p.to_string(), rest.to_string()),
                            None => (String::new(), m.clone()),
                        }
                    } else if split_parent && SPLITTABLE_BY_HYPHEN.contains(&t.name.as_str()) {
                        match m.split_once('-') {
                            Some((p, rest)) => (p.to_string(), rest.to_string()),
                            None => (String::new(), m.clone()),
                        }
                    } else {
                        (String::new(), m.clone())
                    };

                ComponentRow {
                    metadata_type: t.name.clone(),
                    parent,
                    member,
                }
            })
        })
        .collect();

    if matches!(sort_order, SortOrder::ByType) {
        rows.sort_by(|a, b| {
            a.metadata_type
                .cmp(&b.metadata_type)
                .then_with(|| a.parent.cmp(&b.parent))
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
    wtr.write_record(["Type", "Parent", "Member"])?;
    for row in rows {
        wtr.write_record([&row.metadata_type, &row.parent, &row.member])?;
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
    wtr.write_record(["Type", "Parent", "Member"])?;
    for row in rows {
        wtr.write_record([&row.metadata_type, &row.parent, &row.member])?;
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

    let rows = flatten_components(&package, &args.sort, !args.no_split_parent);

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
        let rows = flatten_components(&package, &SortOrder::ByType, false);
        assert!(rows.is_empty());
    }

    #[test]
    fn flatten_empty_types() {
        let package = make_package(vec![("ApexClass", vec![])]);
        let rows = flatten_components(&package, &SortOrder::ByType, false);
        assert!(rows.is_empty());
    }

    #[test]
    fn flatten_single_type_single_member() {
        let package = make_package(vec![("ApexClass", vec!["MyClass"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, false);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            ComponentRow {
                metadata_type: "ApexClass".to_string(),
                parent: String::new(),
                member: "MyClass".to_string(),
            }
        );
    }

    #[test]
    fn flatten_single_type_multiple_members() {
        let package = make_package(vec![("ApexClass", vec!["ClassA", "ClassB", "ClassC"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, false);
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
        let rows = flatten_components(&package, &SortOrder::AsIs, false);
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
        let rows = flatten_components(&package, &SortOrder::ByType, false);
        assert_eq!(rows[0].metadata_type, "ApexClass");
        assert_eq!(rows[1].metadata_type, "CustomObject");
    }

    #[test]
    fn flatten_as_is_preserves_order() {
        let package = make_package(vec![
            ("CustomObject", vec!["Account"]),
            ("ApexClass", vec!["MyClass"]),
        ]);
        let rows = flatten_components(&package, &SortOrder::AsIs, false);
        assert_eq!(rows[0].metadata_type, "CustomObject");
        assert_eq!(rows[1].metadata_type, "ApexClass");
    }

    #[test]
    fn flatten_by_type_sorts_members_within_type() {
        let package = make_package(vec![("ApexClass", vec!["Zebra", "Alpha", "Middle"])]);
        let rows = flatten_components(&package, &SortOrder::ByType, false);
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
        assert_eq!(String::from_utf8(buffer).unwrap(), "Type,Parent,Member\n");
    }

    #[test]
    fn output_csv_single_row() {
        let rows = vec![ComponentRow {
            metadata_type: "ApexClass".to_string(),
            parent: String::new(),
            member: "MyClass".to_string(),
        }];
        let mut buffer = Vec::new();
        output_csv(&rows, &mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Type,Parent,Member\nApexClass,,MyClass\n"
        );
    }

    #[test]
    fn output_csv_multiple_rows() {
        let rows = vec![
            ComponentRow {
                metadata_type: "ApexClass".to_string(),
                parent: String::new(),
                member: "ClassA".to_string(),
            },
            ComponentRow {
                metadata_type: "ApexTrigger".to_string(),
                parent: String::new(),
                member: "TriggerA".to_string(),
            },
        ];
        let mut buffer = Vec::new();
        output_csv(&rows, &mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Type,Parent,Member\nApexClass,,ClassA\nApexTrigger,,TriggerA\n"
        );
    }

    // ==================== output_tsv tests ====================

    #[test]
    fn output_tsv_empty_rows() {
        let rows: Vec<ComponentRow> = vec![];
        let mut buffer = Vec::new();
        output_tsv(&rows, &mut buffer).unwrap();
        assert_eq!(String::from_utf8(buffer).unwrap(), "Type\tParent\tMember\n");
    }

    #[test]
    fn output_tsv_single_row() {
        let rows = vec![ComponentRow {
            metadata_type: "ApexClass".to_string(),
            parent: String::new(),
            member: "MyClass".to_string(),
        }];
        let mut buffer = Vec::new();
        output_tsv(&rows, &mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Type\tParent\tMember\nApexClass\t\tMyClass\n"
        );
    }

    // ==================== split_parent tests ====================

    #[test]
    fn flatten_splits_parent_for_custom_field() {
        let package = make_package(vec![("CustomField", vec!["Account.Active__c"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, true);
        assert_eq!(rows[0].parent, "Account");
        assert_eq!(rows[0].member, "Active__c");
    }

    #[test]
    fn flatten_splits_parent_for_record_type() {
        let package = make_package(vec![("RecordType", vec!["Metric.Completion"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, true);
        assert_eq!(rows[0].parent, "Metric");
        assert_eq!(rows[0].member, "Completion");
    }

    #[test]
    fn flatten_splits_only_first_dot() {
        // Account.Sub.Field__c → Parent: "Account", Member: "Sub.Field__c"
        let package = make_package(vec![("CustomField", vec!["Account.Sub.Field__c"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, true);
        assert_eq!(rows[0].parent, "Account");
        assert_eq!(rows[0].member, "Sub.Field__c");
    }

    #[test]
    fn flatten_no_split_when_disabled() {
        let package = make_package(vec![("CustomField", vec!["Account.Active__c"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, false);
        assert_eq!(rows[0].parent, "");
        assert_eq!(rows[0].member, "Account.Active__c");
    }

    #[test]
    fn flatten_no_split_for_non_splittable_type() {
        let package = make_package(vec![("ApexClass", vec!["MyClass.Inner"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, true);
        assert_eq!(rows[0].parent, "");
        assert_eq!(rows[0].member, "MyClass.Inner");
    }

    #[test]
    fn flatten_splits_all_splittable_types() {
        let package = make_package(vec![
            ("AssignmentRule", vec!["Case.My_Rule"]),
            ("CustomField", vec!["Account.Active__c"]),
            ("ListView", vec!["Contact.All_Contacts"]),
            ("RecordType", vec!["Metric.Completion"]),
            ("SharingCriteriaRule", vec!["Account.Share_Rule"]),
            ("SharingOwnerRule", vec!["Lead.Owner_Rule"]),
            ("SharingTerritoryRule", vec!["Account.Territory_Rule"]),
        ]);
        let rows = flatten_components(&package, &SortOrder::AsIs, true);

        // All splittable types should have parent populated
        for row in &rows {
            assert!(
                !row.parent.is_empty(),
                "Type {} should have parent",
                row.metadata_type
            );
        }
    }

    #[test]
    fn flatten_splits_parent_for_layout_by_hyphen() {
        let package = make_package(vec![("Layout", vec!["Account-Account Layout"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, true);
        assert_eq!(rows[0].parent, "Account");
        assert_eq!(rows[0].member, "Account Layout");
    }

    #[test]
    fn flatten_no_dot_in_member_keeps_empty_parent() {
        let package = make_package(vec![("CustomField", vec!["SomeField"])]);
        let rows = flatten_components(&package, &SortOrder::AsIs, true);
        assert_eq!(rows[0].parent, "");
        assert_eq!(rows[0].member, "SomeField");
    }

    #[test]
    fn output_csv_with_parent_column() {
        let rows = vec![ComponentRow {
            metadata_type: "CustomField".to_string(),
            parent: "Account".to_string(),
            member: "Active__c".to_string(),
        }];
        let mut buffer = Vec::new();
        output_csv(&rows, &mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Type,Parent,Member\nCustomField,Account,Active__c\n"
        );
    }
}
