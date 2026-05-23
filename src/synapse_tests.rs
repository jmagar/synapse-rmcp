use super::{validate_command, validate_safe_path};

#[test]
fn guardrails_accept_safe_paths_and_commands() {
    assert!(validate_safe_path("/tmp/logs/app.log").is_ok());
    assert!(validate_safe_path("/tmp/a_b-c/01.log").is_ok());
    assert!(validate_command("ls", &[]).is_ok());
    assert!(validate_command("custom-read", &["custom-read".into()]).is_ok());
}

#[test]
fn guardrails_reject_traversal_metacharacters_and_denied_commands() {
    assert!(validate_safe_path("../secret").is_err());
    assert!(validate_safe_path("/tmp/a;rm-rf").is_err());
    assert!(validate_command("rm", &[]).is_err());
    assert!(validate_command("python", &["python".into()]).is_err());
}
