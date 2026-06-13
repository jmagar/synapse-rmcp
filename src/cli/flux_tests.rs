use super::flux::{run_compose, run_container, run_docker, run_host};
use crate::actions::{ComposeArgs, ContainerArgs, DockerArgs, HostArgs};
use crate::app::SynapseService;
use crate::elicitation_gate::CliStderrWarn;

fn service() -> SynapseService {
    SynapseService::default()
}

fn err_text(error: anyhow::Error) -> String {
    error.to_string()
}

#[tokio::test]
async fn run_host_requires_named_host_for_single_host_ops_before_service_io() {
    let service = service();

    for subaction in ["services", "mounts", "ports", "doctor"] {
        let err = run_host(
            &HostArgs {
                subaction: subaction.to_owned(),
                ..HostArgs::default()
            },
            &service,
        )
        .await
        .map_err(err_text)
        .unwrap_err();

        assert!(
            err.contains("--host"),
            "{subaction} should reject before service IO, got {err}"
        );
    }
}

#[tokio::test]
async fn run_compose_requires_host_and_project_for_project_ops() {
    let service = service();
    let confirmer = CliStderrWarn;

    let missing_host = run_compose(
        &ComposeArgs {
            subaction: "status".to_owned(),
            project: Some("stack".to_owned()),
            ..ComposeArgs::default()
        },
        &service,
        &confirmer,
    )
    .await
    .map_err(err_text)
    .unwrap_err();
    assert!(missing_host.contains("--host"));

    for subaction in [
        "status", "up", "down", "restart", "recreate", "logs", "build", "pull",
    ] {
        let err = run_compose(
            &ComposeArgs {
                subaction: subaction.to_owned(),
                host: Some("missing".to_owned()),
                ..ComposeArgs::default()
            },
            &service,
            &confirmer,
        )
        .await
        .map_err(err_text)
        .unwrap_err();

        assert!(
            err.contains("--project"),
            "{subaction} should reject missing project before host lookup, got {err}"
        );
    }
}

#[tokio::test]
async fn run_container_requires_action_specific_inputs_before_lookup() {
    let service = service();
    let confirmer = CliStderrWarn;

    let cases = [
        (
            ContainerArgs {
                subaction: "search".to_owned(),
                ..ContainerArgs::default()
            },
            "--query",
        ),
        (
            ContainerArgs {
                subaction: "inspect".to_owned(),
                ..ContainerArgs::default()
            },
            "--container-id",
        ),
        (
            ContainerArgs {
                subaction: "top".to_owned(),
                ..ContainerArgs::default()
            },
            "--container-id",
        ),
        (
            ContainerArgs {
                subaction: "logs".to_owned(),
                ..ContainerArgs::default()
            },
            "--container-id",
        ),
        (
            ContainerArgs {
                subaction: "exec".to_owned(),
                container_id: Some("abc".to_owned()),
                ..ContainerArgs::default()
            },
            "--command",
        ),
    ];

    for (args, expected) in cases {
        let subaction = args.subaction.clone();
        let err = run_container(&args, &service, &confirmer)
            .await
            .map_err(err_text)
            .unwrap_err();
        assert!(
            err.contains(expected),
            "{subaction} should mention {expected}, got {err}"
        );
    }
}

#[tokio::test]
async fn run_docker_requires_force_and_required_operands_before_mutating_ops() {
    let service = service();
    let confirmer = CliStderrWarn;

    let pull = run_docker(
        &DockerArgs {
            subaction: "pull".to_owned(),
            host: Some("missing".to_owned()),
            ..DockerArgs::default()
        },
        &service,
        &confirmer,
    )
    .await
    .map_err(err_text)
    .unwrap_err();
    assert!(pull.contains("--image"));

    let build = run_docker(
        &DockerArgs {
            subaction: "build".to_owned(),
            host: Some("missing".to_owned()),
            context: Some("/srv/app".to_owned()),
            ..DockerArgs::default()
        },
        &service,
        &confirmer,
    )
    .await
    .map_err(err_text)
    .unwrap_err();
    assert!(build.contains("--tag"));

    let rmi = run_docker(
        &DockerArgs {
            subaction: "rmi".to_owned(),
            host: Some("missing".to_owned()),
            image: Some("alpine:latest".to_owned()),
            ..DockerArgs::default()
        },
        &service,
        &confirmer,
    )
    .await
    .map_err(err_text)
    .unwrap_err();
    assert!(rmi.contains("--force"));

    let prune = run_docker(
        &DockerArgs {
            subaction: "prune".to_owned(),
            host: Some("missing".to_owned()),
            prune_target: Some("images".to_owned()),
            ..DockerArgs::default()
        },
        &service,
        &confirmer,
    )
    .await
    .map_err(err_text)
    .unwrap_err();
    assert!(prune.contains("--force"));
}
