use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::new(assert_cmd::cargo_bin!("flyr"))
}

#[test]
fn top_level_help() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Search Google Flights from the terminal",
        ))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("Examples:"))
        .stdout(predicate::str::contains("flyr search -f JFK -t LHR"));
}

#[test]
fn top_level_version() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("flyr 1.4.0"));
}

#[test]
fn search_help_shows_all_sections() {
    cmd()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-f, --from <IATA>"))
        .stdout(predicate::str::contains("-t, --to <IATA>"))
        .stdout(predicate::str::contains("-d, --date <YYYY-MM-DD>"))
        .stdout(predicate::str::contains("--leg"))
        .stdout(predicate::str::contains("--return-date"))
        .stdout(predicate::str::contains("--trip <TYPE>"))
        .stdout(predicate::str::contains("--seat <CLASS>"))
        .stdout(predicate::str::contains("--max-stops <N>"))
        .stdout(predicate::str::contains("--airlines <AA,DL,...>"))
        .stdout(predicate::str::contains("--adults <N>"))
        .stdout(predicate::str::contains("--children <N>"))
        .stdout(predicate::str::contains("--infants-in-seat <N>"))
        .stdout(predicate::str::contains("--infants-on-lap <N>"))
        .stdout(predicate::str::contains("--lang <CODE>"))
        .stdout(predicate::str::contains("--currency <CODE>"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--pretty"))
        .stdout(predicate::str::contains("--proxy <URL>"))
        .stdout(predicate::str::contains("--timeout <SECS>"))
        .stdout(predicate::str::contains("--top <N>"))
        .stdout(predicate::str::contains("--compact"))
        .stdout(predicate::str::contains("Examples:"))
        .stdout(predicate::str::contains("One-way:"))
        .stdout(predicate::str::contains("Round-trip:"))
        .stdout(predicate::str::contains("Multi-city:"))
        .stdout(predicate::str::contains("Business:"))
        .stdout(predicate::str::contains("JSON output:"))
        .stdout(predicate::str::contains("Agent-optimized:"));
}

#[test]
fn search_help_shows_value_hints() {
    cmd()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Departure airport IATA code"))
        .stdout(predicate::str::contains("Arrival airport IATA code"))
        .stdout(predicate::str::contains("Departure date in YYYY-MM-DD"))
        .stdout(predicate::str::contains(
            "economy, premium-economy, business, first",
        ))
        .stdout(predicate::str::contains("one-way, round-trip, multi-city"))
        .stdout(predicate::str::contains("0 = nonstop only"))
        .stdout(predicate::str::contains("round-trip"));
}

#[test]
fn short_help_shows_compact_descriptions() {
    cmd()
        .args(["search", "-h"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Departure airport code"))
        .stdout(predicate::str::contains("Arrival airport code"))
        .stdout(predicate::str::contains("Departure date"))
        .stdout(predicate::str::contains("auto-sets round-trip"))
        .stdout(predicate::str::contains("0 = nonstop only"));
}

#[test]
fn search_help_shows_defaults() {
    cmd()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[default: one-way]"))
        .stdout(predicate::str::contains("[default: economy]"))
        .stdout(predicate::str::contains("[default: 1]"))
        .stdout(predicate::str::contains("[default: en]"))
        .stdout(predicate::str::contains("[default: USD]"))
        .stdout(predicate::str::contains("[default: 30]"));
}

#[test]
fn short_flags_work() {
    let output = cmd().args(["search", "-h"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("-f, --from"));
    assert!(stdout.contains("-t, --to"));
    assert!(stdout.contains("-d, --date"));
}

#[test]
fn missing_all_route_args_fails() {
    cmd()
        .arg("search")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--from is required"));
}

#[test]
fn missing_to_fails() {
    cmd()
        .args(["search", "-f", "HEL", "-d", "2026-03-01"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--to is required"));
}

#[test]
fn missing_date_fails() {
    cmd()
        .args(["search", "-f", "HEL", "-t", "BCN"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--date is required"));
}

#[test]
fn invalid_airport_code_too_short() {
    cmd()
        .args(["search", "-f", "X1", "-t", "BCN", "-d", "2026-03-01"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid airport code"));
}

#[test]
fn invalid_airport_code_numeric() {
    cmd()
        .args(["search", "-f", "123", "-t", "BCN", "-d", "2026-03-01"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid airport code"));
}

#[test]
fn invalid_seat_class() {
    cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--seat",
            "luxury",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid seat class"));
}

#[test]
fn invalid_trip_type() {
    cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--trip",
            "zigzag",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid trip type"));
}

#[test]
fn invalid_date_format() {
    cmd()
        .args(["search", "-f", "HEL", "-t", "BCN", "-d", "01-03-2026"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid date"));
}

#[test]
fn too_many_passengers() {
    cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--adults",
            "5",
            "--children",
            "5",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds maximum of 9"));
}

#[test]
fn infants_exceed_adults() {
    cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--adults",
            "1",
            "--infants-on-lap",
            "2",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("infants on lap cannot exceed"));
}

#[test]
fn malformed_leg_fails() {
    cmd()
        .args(["search", "--leg", "2026-03-01 LAX"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--leg must be"));
}

#[test]
fn json_mode_error_is_structured() {
    let output = cmd()
        .args([
            "search",
            "-f",
            "X1",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON error");
    assert_eq!(parsed["error"]["kind"], "invalid_airport");
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("must be exactly 3 letters"));
}

#[test]
fn json_mode_validation_error() {
    let output = cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--adults",
            "5",
            "--children",
            "5",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON error");
    assert_eq!(parsed["error"]["kind"], "validation_error");
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("exceeds maximum of 9"));
}

#[test]
fn json_mode_invalid_seat_error() {
    let output = cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--seat",
            "luxury",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON error");
    assert_eq!(parsed["error"]["kind"], "validation_error");
}

#[test]
fn json_mode_proxy_error() {
    let output = cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--proxy",
            "not-a-url",
            "--timeout",
            "3",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON error");
    assert_eq!(parsed["error"]["kind"], "proxy_error");
}

#[test]
fn human_error_has_actionable_hint() {
    cmd()
        .args(["search", "-f", "X1", "-t", "BCN", "-d", "2026-03-01"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be exactly 3 letters"));
}

#[test]
fn human_error_proxy_has_hint() {
    cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--proxy",
            "not-a-url",
            "--timeout",
            "3",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("proxy error"));
}

#[test]
fn human_error_date_has_example() {
    cmd()
        .args(["search", "-f", "HEL", "-t", "BCN", "-d", "01-03-2026"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("YYYY-MM-DD"));
}

#[test]
fn exit_code_2_for_validation() {
    cmd()
        .args(["search", "-f", "X1", "-t", "BCN", "-d", "2026-03-01"])
        .assert()
        .code(2);
}

#[test]
fn exit_code_3_for_proxy_error() {
    cmd()
        .args([
            "search",
            "-f",
            "HEL",
            "-t",
            "BCN",
            "-d",
            "2026-03-01",
            "--proxy",
            "not-a-url",
            "--timeout",
            "3",
        ])
        .assert()
        .code(3);
}

#[test]
fn no_subcommand_shows_help() {
    cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn unknown_subcommand_fails() {
    cmd().arg("fly").assert().failure();
}

#[test]
fn leg_with_multi_dest_fails() {
    cmd()
        .args(["search", "--leg", "2026-03-01 HEL BCN", "-t", "BCN,ATH"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--leg cannot be used with comma-separated",
        ));
}

#[test]
fn top_level_help_shows_agent_example() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--compact --top 3"));
}

#[test]
fn search_long_about_mentions_agents() {
    cmd()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("For AI agents"));
}
