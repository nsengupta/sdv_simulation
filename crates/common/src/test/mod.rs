//! Crate-local tests for `common` (not the `tests/` integration harness).
//!
//! - **`fsm`** — deterministic unit tests for the FSM (`cargo test -p common`).
//! - **`fsm_proptest`** — property tests (`cargo test -p common --features proptest`).
//! - **`scenario_test`** — actor black-box scenarios (`GetStatus`, lifecycle).

#[cfg(test)]
mod fsm;

#[cfg(test)]
mod fsm_step;

#[cfg(all(test, feature = "proptest"))]
mod fsm_proptest;

#[cfg(test)]
mod scenario_test;

/// A RAII (Resource Acquisition Is Initialization) guard for Ractor tests.
#[allow(dead_code)] // reserved for scenario helpers; not wired yet
pub struct ActorGuard<T: ractor::Message> {
    pub addr: ractor::ActorRef<T>,
    pub handle: ractor::concurrency::JoinHandle<()>,
}

impl<T: ractor::Message> Drop for ActorGuard<T> {
    fn drop(&mut self) {
        // 1. Tell the actor to stop immediately
        self.addr.stop(None);

        // Note: I cannot 'await' inside a synchronous drop() function.
        // However, stopping the actor here is usually enough to
        // clear the mailbox for the next test.
    }
}
