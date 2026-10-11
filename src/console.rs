//! Keeps a Windows console readable after an error (feature 008, research R11).

/// Whether the program should wait for Enter before exiting: only on Windows, after a failure,
/// when standard input is an interactive console (FR-019). A double-click opens such a console;
/// redirected or piped input never waits.
pub fn should_wait(failed: bool, interactive: bool, windows: bool) -> bool {
    failed && interactive && windows
}

/// Prints the prompt and waits for one line on standard input; read errors just end the wait.
pub fn wait_for_enter() {
    eprintln!("Press Enter to close this window.");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_only_after_a_failure_with_interactive_input_on_windows() {
        for failed in [false, true] {
            for interactive in [false, true] {
                for windows in [false, true] {
                    assert_eq!(
                        should_wait(failed, interactive, windows),
                        failed && interactive && windows,
                        "failed={failed} interactive={interactive} windows={windows}"
                    );
                }
            }
        }
    }
}
