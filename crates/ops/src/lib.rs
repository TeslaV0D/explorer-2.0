//! File operations: copy / move / trash / delete jobs with progress, conflicts
//! and undo; rename and new folder; system clipboard (Windows CF_HDROP).

pub mod clipboard;
pub mod exec;
pub mod names;
pub mod simple;
pub mod undo;
#[cfg(windows)]
mod win_copy;

pub use exec::{
    spawn, Choice, Conflict, Control, Done, Event, JobHandle, JobSpec, Kind, Progress, Report,
};
