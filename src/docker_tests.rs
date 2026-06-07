#[tokio::test]
async fn docker_json_reports_unavailable_for_invalid_subcommand() {
    let value = super::docker_json(&["__synapse2_invalid_subcommand__"])
        .await
        .expect("docker_json should convert subprocess failure to JSON");

    assert_eq!(value["available"], false);
    assert_eq!(value["command"], "docker __synapse2_invalid_subcommand__");
}
