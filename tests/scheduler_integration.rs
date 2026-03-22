use ketheler::scheduler;

#[test]
fn scheduler_orders_results_by_input() {
    let inputs = vec![3_u64, 1, 2];
    let results = scheduler::run(2, |value| value * 2, inputs);
    assert_eq!(results, vec![(1, 2), (2, 4), (3, 6)]);
}

#[test]
fn scheduler_handles_zero_workers() {
    let inputs = vec![2_u64, 1];
    let results = scheduler::run(0, |value| value + 1, inputs);
    assert_eq!(results, vec![(1, 2), (2, 3)]);
}
