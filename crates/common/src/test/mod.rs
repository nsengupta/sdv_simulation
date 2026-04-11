//! Crate-local tests for `common` (not the `tests/` integration harness).
//!
//! - **`fsm`** — deterministic unit tests for the FSM (`cargo test -p common`).
//! - **`fsm_proptest`** — property tests (`cargo test -p common --features proptest`).
//! - **`scenario_test`** — actor black-box scenarios (`GetStatus`, lifecycle).

#[cfg(test)]
mod fsm;

#[cfg(all(test, feature = "proptest"))]
mod fsm_proptest;

#[cfg(test)]
mod scenario_test;
