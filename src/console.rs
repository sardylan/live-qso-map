//! Keeps a Windows console readable after an error (feature 008, research R11).

/// Whether the program should wait for Enter before exiting.
pub fn should_wait(_failed: bool, _interactive: bool, _windows: bool) -> bool {
    false
}
