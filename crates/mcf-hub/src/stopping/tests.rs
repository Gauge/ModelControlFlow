use super::Stopping;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn a_transfer_nobody_can_stop_is_never_asked_to() {
    assert!(!Stopping::never().asked());
}

#[test]
fn a_transfer_reads_the_flag_it_was_given() {
    let flag = Arc::new(AtomicBool::new(false));
    let stopping = Stopping::on(Arc::clone(&flag));
    assert!(!stopping.asked());
    flag.store(true, Ordering::Relaxed);
    assert!(stopping.asked(), "asking must be seen where the bytes land");
}

#[test]
fn every_copy_of_one_asking_sees_it() {
    let flag = Arc::new(AtomicBool::new(false));
    let stopping = Stopping::on(Arc::clone(&flag));
    let copy = stopping.clone();
    flag.store(true, Ordering::Relaxed);
    assert!(
        copy.asked(),
        "a copy handed to a worker reads the same flag"
    );
}
