// This file is part of the uutils diffutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// #[cfg(target_os = "linux")]

use assert_cmd::cargo::cargo_bin_cmd;
use cmp::parser_cmp::{Config, SkipU64, uu_app};
use predicates::prelude::predicate;
use std::{ffi::OsString, fs::File, io::Write};
use uudiff::error::UResult;
use uutests::{at_and_ucmd, new_ucmd};

fn os(s: &str) -> OsString {
    OsString::from(s)
}

/// Simplify call of parser, just pass a normal string like in the terminal.
fn parse(args: &str) -> UResult<Config> {
    let opts = args
        .split(' ')
        .filter(|arg| !arg.is_empty())
        .map(OsString::from);

    let matches = uudiff::clap_localization::handle_clap_result(uu_app(), opts)?;
    let config: Config = matches.try_into()?;

    Ok(config)
}

#[test]
fn test_files_equal() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_equal.txt")
        .succeeds()
        .no_output();
}

#[test]
fn test_files_different() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .fails_with_code(1)
        .stdout_is("lorem_ipsum.txt lorem_ipsum_diff.txt differ: char 190, line 4\n");
}

#[test]
fn cmp_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = tempfile::tempdir()?;

    let a_path = tmp_dir.path().join("a");
    let mut a = File::create(&a_path).unwrap();
    a.write_all(b"a\n").unwrap();

    // TODO cmp is not yet compiled automatically
    let mut cmd = cargo_bin_cmd!("diffutils");
    cmd.arg("cmp");
    cmd.arg(&a_path);
    cmd.write_stdin("a\n");
    cmd.assert()
        .code(predicate::eq(0))
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    let mut cmd = cargo_bin_cmd!("diffutils");
    cmd.env("LC_ALL", "C");
    cmd.arg("cmp");
    cmd.arg(&a_path);
    cmd.write_stdin("b\n");
    cmd.assert()
        .code(predicate::eq(1))
        .failure()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::ends_with(" - differ: char 1, line 1\n"));

    Ok(())
}

#[test]
fn test_invalid_file_is_dir() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a_dir");

    ucmd.arg("a_dir")
        .fails_with_code(2)
        .stderr_is("cmp: 'a_dir' is a directory\n");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!()
        .arg("--definitely-invalid")
        .fails_with_code(2)
        .stderr_contains("unexpected argument '--definitely-invalid' found");
}

#[test]
fn test_parser_no_arg() {
    new_ucmd!()
        .fails_with_code(2)
        .stderr_contains("cmp: missing operand after 'cmp'");
}

#[test]
/// --ver ambiguous --version --verbose
fn test_parser_ambiguous() {
    new_ucmd!()
        .arg("--ver")
        .fails_with_code(2)
        .stderr_contains("--verbose")
        .stderr_contains("--version");
}

#[test]
// multiple tests in one for historical reasons
fn test_parser_positional() {
    // file_1 only
    assert_eq!(
        parse("cmp foo").unwrap(),
        Config {
            from: os("foo"),
            to: os("-"),
            ..Default::default()
        }
    );

    // double dash without operand: following is interpreted as file
    assert_eq!(
        parse("cmp foo -- --help").unwrap(),
        Config {
            from: os("foo"),
            to: os("--help"),
            ..Default::default()
        }
    );

    // --ignore-initial for file_1 as operand
    assert_eq!(
        parse("cmp foo bar 1K").unwrap(),
        Config {
            from: os("foo"),
            to: os("bar"),
            skip_bytes_from: Some(1024),
            skip_bytes_to: None,
            ..Default::default()
        }
    );
}

#[test]
/// --bytes with value greater than u64
fn test_parser_bytes_value_too_large() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .arg("--bytes")
        .arg("1ZB")
        .fails_with_code(2)
        .stderr_contains("cmp: invalid unit in '1ZB' for option --bytes");

    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .arg("--bytes")
        .arg("99999999999999999999999999999999999999999999999999999999999")
        .fails_with_code(2)
        .stderr_contains("cmp: invalid value '99999999999999999999999999999999999999999999999999999999999' (too large) for option --bytes");
}

#[test]
/// --bytes with value negative
fn test_parser_bytes_negative() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .arg("--bytes=-1")
        .fails_with_code(2)
        .stderr_contains("cmp: invalid value '-1' for option --bytes");
}

#[test]
/// --ignore-initial with value greater than u64)
fn test_parser_ignore_initial_value_too_large() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .arg("1")
        .arg("2Y")
        .fails_with_code(2)
        .stderr_contains("cmp: invalid unit in '2Y' for option --ignore-initial");

    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .arg("--ignore-initial")
        .arg("99999999999999999999999999999999999999999999999999999999999")
        .fails_with_code(2)
        .stderr_contains("cmp: invalid value '99999999999999999999999999999999999999999999999999999999999' (too large) for option --ignore-initial");
}

#[test]
/// --ignore-initial as operands with 1 2Y (which is greater than u64)
fn test_parser_ignore_initial_too_many_values() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .arg("--ignore-initial")
        .arg("1:2:3")
        .fails_with_code(2)
        .stderr_contains("cmp: invalid unit in '2:3' for option --ignore-initial");
}

#[test]
fn test_parser_too_many_operands() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .arg("1")
        .arg("2")
        .arg("3")
        .fails_with_code(2)
        .stderr_contains("cmp: extra operand '3'");
}

#[test]
fn test_parser_incompatible_silent_and_verbose() {
    new_ucmd!()
        .arg("--silent")
        .arg("--verbose")
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .fails_with_code(2)
        .stderr_contains("cmp: options --verbose and --silent are incompatible");
}

#[test]
// This is not a GNU error, but should be
fn test_parser_incompatible_silent_and_print_bytes() {
    new_ucmd!()
        .arg("--silent")
        .arg("--print-bytes")
        .arg("lorem_ipsum.txt")
        .arg("lorem_ipsum_diff.txt")
        .fails_with_code(2)
        .stderr_contains("cmp: options --print-bytes and --silent are incompatible");
}

#[test]
fn test_execution_modes() {
    // --print-bytes
    let print_bytes = Config {
        from: os("foo"),
        to: os("bar"),
        print_bytes: true,
        ..Default::default()
    };
    assert_eq!(parse("cmp -b foo bar").unwrap(), print_bytes.clone());
    assert_eq!(
        parse("cmp --print-bytes foo bar").unwrap(),
        (print_bytes.clone())
    );
    assert_eq!(parse("cmp --pr foo bar").unwrap(), print_bytes);

    // --verbose
    let verbose = Config {
        from: os("foo"),
        to: os("bar"),
        verbose: true,
        ..Default::default()
    };
    assert_eq!(parse("cmp -l foo bar").unwrap(), verbose.clone());
    assert_eq!(parse("cmp --verbose foo bar").unwrap(), verbose.clone());
    assert_eq!(parse("cmp --verb foo bar").unwrap(), verbose.clone());

    // --verbose & --print-bytes
    let verbose_and_print_bytes = Config {
        from: os("foo"),
        to: os("bar"),
        print_bytes: true,
        verbose: true,
        ..Default::default()
    };
    assert_eq!(
        parse("cmp -l -b foo bar").unwrap(),
        verbose_and_print_bytes.clone()
    );
    assert_eq!(
        parse("cmp -lb foo bar").unwrap(),
        verbose_and_print_bytes.clone()
    );
    assert_eq!(
        parse("cmp -bl foo bar").unwrap(),
        verbose_and_print_bytes.clone()
    );
    assert_eq!(
        parse("cmp --verbose --print-bytes foo bar").unwrap(),
        verbose_and_print_bytes.clone()
    );
    assert_eq!(
        parse("cmp --verb --p foo bar").unwrap(),
        verbose_and_print_bytes.clone()
    );

    // --silent --quiet
    let silent = Config {
        from: os("foo"),
        to: os("bar"),
        silent: true,
        ..Default::default()
    };
    assert_eq!(parse("cmp -s foo bar").unwrap(), silent.clone());
    assert_eq!(parse("cmp --silent foo bar").unwrap(), (silent.clone()));
    assert_eq!(parse("cmp --quiet foo bar").unwrap(), (silent.clone()));
}

#[test]
/// These are all identical:
/// - cmp file_1 file_2 -bl -n 1024
/// - cmp file_1 file_2 -bl -n 1k
/// - cmp file_1 file_2 -bl -n 1K
/// - cmp file_1 file_2 -bl -n 1KiB
/// - cmp file_1 file_2 -bl -n 1kiB
/// - cmp file_1 file_2 -bl -n1kiB
/// - cmp file_1 file_2 -bln1kiB
fn bytes_limit() {
    let mut bytes_limit = Config {
        from: os("foo"),
        to: os("bar"),
        bytes_limit: Some(1000),
        ..Default::default()
    };
    assert_eq!(parse("cmp -n 1000 foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n1000 foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1kB foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1KB foo bar").unwrap(), (bytes_limit.clone()));

    bytes_limit.bytes_limit = Some(1024);
    assert_eq!(parse("cmp -n 1024 foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1k foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1K foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1KiB foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1kiB foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n1024 foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n1k foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n1K foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(
        parse("cmp --bytes=1024 foo bar").unwrap(),
        bytes_limit.clone()
    );
    assert_eq!(
        parse("cmp --bytes=1K foo bar").unwrap(),
        (bytes_limit.clone())
    );
    assert_eq!(
        parse("cmp --bytes 1024 foo bar").unwrap(),
        bytes_limit.clone()
    );
    assert_eq!(
        parse("cmp --bytes 1K foo bar").unwrap(),
        (bytes_limit.clone())
    );
    bytes_limit.print_bytes = true;
    bytes_limit.verbose = true;
    assert_eq!(
        parse("cmp -bln1kiB foo bar").unwrap(),
        (bytes_limit.clone())
    );
    bytes_limit.print_bytes = false;
    bytes_limit.verbose = false;

    // Test large numbers
    // Most modern Linux distributions (like Debian, Ubuntu, or CentOS) compile their core utilities (GNU diffutils) with Large File Support (LFS).
    // This uses the _FILE_OFFSET_BITS=64 flag, which forces the system to use a 64-bit integer ($off\_t$) for file offsets and sizes.
    // Even on a 32-bit processor, cmp can handle files much larger than the system's memory or 4 GB address space.The limit:
    // Technically $9,223,372,036,854,775,807$ bytes.
    // This is a problematic topic. File sizes can be larger than u64. Should the new cmp allow larger numbers (u128)?
    bytes_limit.bytes_limit = Some(1_000_000);
    assert_eq!(parse("cmp -n 1MB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_048_576);
    assert_eq!(parse("cmp -n 1M foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1MiB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_000_000_000);
    assert_eq!(parse("cmp -n 1GB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_073_741_824);
    assert_eq!(parse("cmp -n 1G foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1GiB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_000_000_000_000);
    assert_eq!(parse("cmp -n 1TB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_099_511_627_776);
    assert_eq!(parse("cmp -n 1T foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1TiB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_000_000_000_000_000);
    assert_eq!(parse("cmp -n 1PB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_125_899_906_842_624);
    assert_eq!(parse("cmp -n 1P foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1PiB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_000_000_000_000_000_000);
    assert_eq!(parse("cmp -n 1EB foo bar").unwrap(), (bytes_limit.clone()));
    bytes_limit.bytes_limit = Some(1_152_921_504_606_846_976);
    assert_eq!(parse("cmp -n 1E foo bar").unwrap(), (bytes_limit.clone()));
    assert_eq!(parse("cmp -n 1EiB foo bar").unwrap(), (bytes_limit.clone()));
}

#[test]
fn ignore_initial() {
    let mut skips = Config {
        from: os("foo"),
        to: os("bar"),
        skip_bytes_from: Some(1),
        skip_bytes_to: Some(1),
        ..Default::default()
    };
    assert_eq!(parse("cmp -i 1 foo bar").unwrap(), skips.clone());
    assert_eq!(
        parse("cmp --ignore-initial 1 foo bar").unwrap(),
        skips.clone()
    );
    assert_eq!(parse("cmp --ig 1 foo bar").unwrap(), skips.clone());

    // 2nd value different
    skips.skip_bytes_to = Some(2);
    assert_eq!(
        parse("cmp --ignore-initial=1:2 foo bar").unwrap(),
        skips.clone()
    );

    // uses higher positional arguments when operand and options are both provided
    skips.skip_bytes_from = Some(3);
    skips.skip_bytes_to = Some(4);
    assert_eq!(
        parse("cmp --ignore-initial=1:2 foo bar 3 4").unwrap(),
        skips.clone()
    );

    // large numbers
    skips.skip_bytes_from = Some(1_000_000_000);
    skips.skip_bytes_to = Some(2 * 1_152_921_504_606_846_976);
    assert_eq!(
        parse("cmp --ignore-initial=1GB:2E foo bar").unwrap(),
        skips.clone()
    );

    // All special suffixes for ignore-initial.
    for (i, suffixes) in [
        ["kB", "K"],
        ["MB", "M"],
        ["GB", "G"],
        ["TB", "T"],
        ["PB", "P"],
        ["EB", "E"],
        // These values give an error in GNU cmp
        // #[cfg(feature = "cmp_bytes_limit_128_bit")]
        // ["ZB", "Z"],
        // #[cfg(feature = "cmp_bytes_limit_128_bit")]
        // ["YB", "Y"],
    ]
    .iter()
    .enumerate()
    {
        let values = [
            (1_000 as SkipU64)
                .checked_pow((i + 1) as u32)
                .expect(&format!("number too large for suffix {suffixes:?}")),
            (1024 as SkipU64)
                .checked_pow((i + 1) as u32)
                .expect(&format!("number too large for suffix {suffixes:?}")),
        ];
        for (j, v) in values.iter().enumerate() {
            assert_eq!(
                parse(&format!("cmp -i 1{}:2 foo bar", suffixes[j])).unwrap(),
                Config {
                    from: os("foo"),
                    to: os("bar"),
                    skip_bytes_from: Some(*v),
                    skip_bytes_to: Some(2),
                    ..Default::default()
                }
            );
        }
    }
}
