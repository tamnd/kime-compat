//! The compatibility harness for kime.
//!
//! The design is the API compatibility section of `spec/15-testing.md` in the kime repository. At
//! M0 this crate knows the surfaces and checks the committed fixtures against the response rules
//! in `spec/03-api.md`. From M1, [`live`] checks the same rules on what a running kime-serve
//! answers, and `sdk-js/` runs the TypeSafe JS SDK against it.

pub mod contract;
pub mod live;
