use assert_cmd::{assert::OutputAssertExt, cargo::cargo_bin_cmd};
use predicates::prelude::*;

#[test]
fn table_output() {
    let output = cargo_bin_cmd!("csc-view")
        .arg("tests/fixtures/simple.xml")
        .output()
        .unwrap();
    output.assert().success().stdout(
        predicate::str::contains("ApexClass").and(predicate::str::contains("MyController")),
    );
}

#[test]
fn csv_output() {
    let output = cargo_bin_cmd!("csc-view")
        .args(["--format", "csv"])
        .arg("tests/fixtures/simple.xml")
        .output()
        .unwrap();
    output.assert().success().stdout(
        predicate::str::contains("Type,Parent,Member")
            .count(1)
            .and(predicate::str::contains("ApexPage,,MyPage").count(1)),
    );
}

#[test]
fn tsv_output() {
    let output = cargo_bin_cmd!("csc-view")
        .args(["--format", "tsv"])
        .arg("tests/fixtures/simple.xml")
        .output()
        .unwrap();
    output.assert().success().stdout(
        predicate::str::contains("Type\tParent\tMember")
            .count(1)
            .and(predicate::str::contains("ApexPage\t\tMyPage").count(1)),
    );
}

#[test]
fn sort_as_is() {
    let output = cargo_bin_cmd!("csc-view")
        .args(["--format", "csv"])
        .args(["--sort", "as-is"])
        .arg("tests/fixtures/simple.xml")
        .output()
        .unwrap();
    output
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"ApexPage[\s\S]*ApexClass").unwrap());
}

#[test]
fn split_parent_default() {
    let output = cargo_bin_cmd!("csc-view")
        .args(["--format", "csv"])
        .arg("tests/fixtures/with_split_parent.xml")
        .output()
        .unwrap();
    output.assert().success().stdout(
        predicate::str::contains("CustomField,Account,CustomField__c")
            .and(predicate::str::contains("Layout,Account,Account Layout")),
    );
}

#[test]
fn no_split_parent_flag() {
    let output = cargo_bin_cmd!("csc-view")
        .args(["--format", "csv"])
        .arg("--no-split-parent")
        .arg("tests/fixtures/with_split_parent.xml")
        .output()
        .unwrap();
    output.assert().success().stdout(
        predicate::str::contains("CustomField,,Account.CustomField__c")
            .and(predicate::str::contains("Layout,,Account-Account Layout")),
    );
}
