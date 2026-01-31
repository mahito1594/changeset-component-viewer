# changeset-component-viewer

A CLI tool that parses Salesforce `package.xml` files and displays metadata components in human-readable formats.

> [!TIP]
> To retrieve `package.xml` of a changeset, run
>
> ```bash
> sf project retrieve start --package-name <name of your changeset>
> ```
>
> For details, see [Salesforce CLI Command Reference](https://developer.salesforce.com/docs/atlas.en-us.sfdx_cli_reference.meta/sfdx_cli_reference/cli_reference_project_commands_unified.htm#cli_reference_project_retrieve_start_unified).

## Features

- Lists metadata types and members from `package.xml`
- Multiple output formats (table, CSV, TSV)
- Customizable sort order (by type or preserve original order)

## Installation

### Use `cargo install`

```bash
cargo install --git https://github.com/mahito1594/changeset-component-viewer
```

## Usage

```bash
csc-view <PATH> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<PATH>` | Path to the `package.xml` file |

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --format <FORMAT>` | Output format (`table`, `csv`, `tsv`) | `table` |
| `-s, --sort <SORT>` | Sort order (`by-type`, `as-is`) | `by-type` |
| `-h, --help` | Print help | - |
| `-V, --version` | Print version | - |

### Examples

#### Display as table (default)

```bash
csc-view path/to/package.xml
```

Output:

```
┌──────────────┬─────────────────┐
│ Type         │ Member          │
├──────────────┼─────────────────┤
│ ApexClass    │ AccountHandler  │
│ ApexClass    │ ContactService  │
│ ApexTrigger  │ AccountTrigger  │
│ CustomObject │ MyObject__c     │
└──────────────┴─────────────────┘
```

#### Output as CSV

```bash
csc-view path/to/package.xml -f csv
```

Output:

```
Type,Member
ApexClass,AccountHandler
ApexClass,ContactService
ApexTrigger,AccountTrigger
CustomObject,MyObject__c
```

#### Output as TSV

```bash
csc-view path/to/package.xml -f tsv
```

## Supported package.xml format

See [Metadata API Developer Guide](https://developer.salesforce.com/docs/atlas.en-us.api_meta.meta/api_meta/manifest_samples.htm).
For example, we support:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Package xmlns="http://soap.sforce.com/2006/04/metadata">
    <types>
        <members>AccountHandler</members>
        <members>ContactService</members>
        <name>ApexClass</name>
    </types>
    <types>
        <members>AccountTrigger</members>
        <name>ApexTrigger</name>
    </types>
    <version>62.0</version>
</Package>
```

## License

[Unlicense](LICENSE) - Public Domain
