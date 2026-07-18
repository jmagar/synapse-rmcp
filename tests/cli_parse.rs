use synapse2::cli::{Command, SetupCommand, parse_args_from};

#[test]
fn setup_and_doctor_still_parse() {
    assert_eq!(
        parse_args_from(["setup", "plugin-hook", "--no-repair"]).unwrap(),
        Some(Command::Setup(SetupCommand::PluginHook { no_repair: true }))
    );
    assert_eq!(
        parse_args_from(["doctor", "--json"]).unwrap(),
        Some(Command::Doctor { json: true })
    );
}

#[test]
fn malformed_args_are_rejected() {
    for args in [
        &["flux", "container", "logs", "--container-id"][..],
        &["scout", "exec", "--host", "local", "--path", "/tmp"],
        &["watch", "--interval", "0"],
        &["setup", "plugin-hook", "--no-reapir"],
    ] {
        assert!(
            parse_args_from(args.iter().copied()).is_err(),
            "{args:?} should be rejected"
        );
    }
}

#[test]
fn scout_rejects_unknown_duplicate_and_invalid_numeric_options() {
    for args in [
        &[
            "scout",
            "find",
            "--host",
            "local",
            "--path",
            "/tmp",
            "--pattern",
            "*",
            "--limt",
            "2",
        ][..],
        &["scout", "ps", "--host", "local", "--host", "other"][..],
        &[
            "scout", "logs", "syslog", "--host", "local", "--lines", "many",
        ][..],
        &["flux", "host", "info", "--host", "local", "--typo", "value"][..],
    ] {
        assert!(parse_args_from(args.iter().copied()).is_err(), "{args:?}");
    }
}

#[test]
fn scout_exec_and_emit_preserve_variadic_argv_and_timeout() {
    let exec = parse_args_from([
        "scout",
        "exec",
        "--host",
        "local",
        "--command",
        "cat",
        "--timeout",
        "7",
        "--args",
        "-n",
        "/tmp/input",
    ])
    .unwrap()
    .unwrap();
    match exec {
        Command::ScoutExec(args) => {
            assert_eq!(args.args, ["-n", "/tmp/input"]);
            assert_eq!(args.timeout_secs, Some(7));
        }
        other => panic!("expected scout exec, got {other:?}"),
    }

    let emit = parse_args_from([
        "scout",
        "emit",
        "--target",
        "local",
        "--command",
        "cat",
        "--args",
        "/etc/hostname",
    ])
    .unwrap()
    .unwrap();
    match emit {
        Command::ScoutEmit(args) => assert_eq!(args.args, ["/etc/hostname"]),
        other => panic!("expected scout emit, got {other:?}"),
    }
}
