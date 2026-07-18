use super::ActivityLog;

#[test]
fn activity_is_bounded_and_newest_first() {
    let log = ActivityLog::new(2);
    log.record("rest", "one", true, None);
    log.record("mcp", "two", false, Some("failed"));
    log.record("rest", "three", true, None);

    let events = log.snapshot();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].action, "three");
    assert_eq!(events[1].action, "two");
    assert_eq!(events[1].error.as_deref(), Some("execution_failed"));
}

#[test]
fn concurrent_records_remain_strictly_sequence_ordered() {
    let log = ActivityLog::new(128);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(33));
    let mut threads = Vec::new();
    for index in 0..32 {
        let log = log.clone();
        let barrier = std::sync::Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            log.record("rest", &format!("action-{index}"), true, None);
        }));
    }
    barrier.wait();
    for thread in threads {
        thread.join().unwrap();
    }

    let events = log.snapshot();
    assert_eq!(events.len(), 32);
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].sequence > pair[1].sequence)
    );
}

#[test]
fn activity_errors_are_reduced_to_safe_categories() {
    let log = ActivityLog::new(2);
    log.record(
        "mcp",
        "scout.peek",
        false,
        Some("failed reading /secret/operator/path: token=abc"),
    );
    let event = log.snapshot().pop().unwrap();
    assert_eq!(event.error.as_deref(), Some("execution_failed"));
    assert!(!event.error.unwrap().contains("secret"));
}
