use nous_hsr::{Sdk, SnifferStatus};
#[test]
fn standalone_instance_can_read_subscribe_start_stop_and_restart() {
    let sdk = Sdk::new(None).unwrap();
    assert_eq!(sdk.status(), SnifferStatus::Stopped);
    let events = sdk.subscribe();
    assert_eq!(sdk.read().revision, 0);
    assert_eq!(sdk.read().data.uid, None);
    for _ in 0..2 {
        sdk.start_capture().unwrap();
        sdk.start_capture().unwrap();
        sdk.stop_capture().unwrap();
        assert_eq!(sdk.status(), SnifferStatus::Stopped);
    }
    assert!(events.try_iter().any(|e| e.name == "sniffer-status"));
}
