use openwork_e2e::{analyze, read_fixture_below, sha256_hex, validate_golden};
use std::fs;
use std::path::Path;

const JULY: &str = include_str!("../../../samples/sales/sales_july.csv");
const AUGUST: &str = include_str!("../../../samples/sales/sales_august.csv");
const ANALYSIS: &str = include_str!("../../../samples/sales/golden/sales-analysis.csv");
const SUMMARY: &str = include_str!("../../../samples/sales/golden/summary.md");
const ANALYZER: &str = include_str!("../../../samples/sales/analyze.awk");

#[test]
fn checked_in_sales_fixture_matches_exact_golden() {
    let analysis = analyze(JULY, AUGUST).expect("analyze fixture");
    assert_eq!(analysis.july_total(), 33_000);
    assert_eq!(analysis.august_total(), 28_500);
    assert_eq!(analysis.change(), -4_500);
    assert_eq!(analysis.customers()[0].customer_name, "Crown");
    assert_eq!(analysis.customers()[0].decline, 3_000);
    assert!(analysis.render_csv().contains("330,28500,297"));
    assert!(
        analysis
            .render_summary()
            .expect("summary")
            .contains("-13.64%")
    );
    validate_golden(&analysis, ANALYSIS, SUMMARY).expect("exact golden");
}

#[test]
fn hashes_pin_every_input_and_golden_file() {
    assert_eq!(
        sha256_hex(JULY.as_bytes()),
        "d5e7375085a2e2b98be702dbcc166f6e684161260af88a32639b8104fc2311cf"
    );
    assert_eq!(
        sha256_hex(AUGUST.as_bytes()),
        "5101c4d45855352425629f4f798c648abba836119b781ff40100db2affc1bdb5"
    );
    assert_eq!(
        sha256_hex(ANALYSIS.as_bytes()),
        "b0d51ee462378286ccd4e60a951881b2be8908d6f5e21a8ae58c7bf606b745e0"
    );
    assert_eq!(
        sha256_hex(SUMMARY.as_bytes()),
        "ec643695fcd8347a3dd984e8cdf255e6d0dbb255c448b7bef55146a612ef8072"
    );
    assert_eq!(
        sha256_hex(ANALYZER.as_bytes()),
        "d6cd17ad781a56bbb762deda456a03b9880b2ad903ea1d9bff8afbcf837a0439"
    );
}

#[test]
fn duplicate_invalid_number_and_unstable_order_are_rejected() {
    let duplicate = JULY.replace("C002,Beta,5000,50\n", "C001,Beta,5000,50\n");
    assert!(analyze(&duplicate, AUGUST).is_err());
    let invalid_number = JULY.replace("8000", "80.00");
    assert!(analyze(&invalid_number, AUGUST).is_err());

    let analysis = analyze(JULY, AUGUST).expect("analysis");
    let unstable = ANALYSIS.replace(
        "C003,Crown,7000,70,4000,50,-3000,3000,-42.86%,-20\nC001,Acme,8000,80,6000,60,-2000,2000,-25.00%,-20\n",
        "C001,Acme,8000,80,6000,60,-2000,2000,-25.00%,-20\nC003,Crown,7000,70,4000,50,-3000,3000,-42.86%,-20\n",
    );
    assert!(validate_golden(&analysis, &unstable, SUMMARY).is_err());
}

#[test]
fn fixture_loader_rejects_traversal_and_symlinks() {
    let root = tempfile::tempdir().expect("root");
    fs::write(root.path().join("safe.txt"), "safe\n").expect("safe fixture");
    assert_eq!(
        read_fixture_below(root.path(), Path::new("safe.txt")).expect("read"),
        "safe\n"
    );
    assert!(read_fixture_below(root.path(), Path::new("../safe.txt")).is_err());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = tempfile::NamedTempFile::new().expect("outside");
        symlink(outside.path(), root.path().join("escape.txt")).expect("symlink");
        assert!(read_fixture_below(root.path(), Path::new("escape.txt")).is_err());
    }
}
