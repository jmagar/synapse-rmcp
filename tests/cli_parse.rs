use synapse2::cli::{parse_args_from, Command, SetupCommand};

#[test]
fn flux_docker_info_parsed() {
    assert_eq!(
        parse_args_from(["flux", "docker", "info"]).unwrap(),
        Some(Command::FluxDocker {
            subaction: "info".into()
        })
    );
}

#[test]
fn flux_container_logs_parsed() {
    assert_eq!(
        parse_args_from([
            "flux",
            "container",
            "logs",
            "--container-id",
            "abc",
            "--lines",
            "20"
        ])
        .unwrap(),
        Some(Command::FluxContainer {
            subaction: "logs".into(),
            container_id: Some("abc".into()),
            lines: Some(20),
        })
    );
}

#[test]
fn scout_commands_parse() {
    assert_eq!(
        parse_args_from(["scout", "nodes"]).unwrap(),
        Some(Command::ScoutNodes)
    );
    assert_eq!(
        parse_args_from(["scout", "peek", "--host", "local", "--path", "/tmp"]).unwrap(),
        Some(Command::ScoutPeek {
            host: "local".into(),
            path: "/tmp".into(),
        })
    );
    assert_eq!(
        parse_args_from([
            "scout",
            "exec",
            "--host",
            "local",
            "--path",
            "/tmp",
            "--command",
            "ls"
        ])
        .unwrap(),
        Some(Command::ScoutExec {
            host: "local".into(),
            path: "/tmp".into(),
            command: "ls".into(),
        })
    );
}

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
