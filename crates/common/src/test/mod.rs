//! Crate-local tests for `common` (not the `tests/` integration harness).
//!
//! Focused test modules by contract boundary:
//! - `fsm_engine_contract` — deterministic unit tests for transition/output rules
//! - `fsm_step_contract` — step boundary contract tests
//! - `fsm_properties` — property tests behind the `proptest` feature
//! - `actor_contract` — actor and transition-sink behavior contracts
//! - `scenarios_smoke` — lightweight end-to-end behavior smoke tests

#[cfg(test)]
mod actor_contract;

#[cfg(test)]
mod controller_api_contract;

#[cfg(test)]
mod engine_namespace_contract;

#[cfg(test)]
mod fsm_engine_contract;

#[cfg(all(test, feature = "proptest"))]
mod fsm_properties;

#[cfg(test)]
mod fsm_step_contract;

#[cfg(test)]
mod lighting_step_contract;

#[cfg(test)]
mod op_strategy_contract;

#[cfg(test)]
mod projection_contract;

#[cfg(test)]
mod scenarios_smoke;

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
