//! The compatibility harness for kime.
//!
//! The design is the API compatibility section of `spec/15-testing.md` in the kime repository. At
//! M0 this crate knows the surfaces and checks the committed fixtures against the response rules
//! in `spec/03-api.md`. Replaying the SDKs and Laya's tests against a running kime-serve starts at
//! M1.

pub mod contract;
