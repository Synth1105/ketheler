#![cfg(feature = "agent")]

ketheler::agent!(String);

#[test]
fn string_agent_roundtrip() {
    let handle = Agent::start_link();

    let value = Agent::get(&handle, |v| v);
    assert_eq!(value, "");

    let updated = Agent::get_and_update(&handle, |v| format!("{v}hi"));
    assert_eq!(updated, "hi");

    let set = Agent::update(&handle, "hello".to_string());
    assert_eq!(set, "hello");

    let value = Agent::get(&handle, |v| v);
    assert_eq!(value, "hello");
}
