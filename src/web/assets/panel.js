// The settings panel's open/close logic (feature 011, research R6). Pure, with the time passed
// in, so the 5-minute auto-close is testable; app.js wires it to the DOM.

/** An open panel closes by itself after this long without input in it (FR-007). */
export const AUTO_CLOSE_MS = 5 * 60_000;

/** The panel always starts closed: its open state is not a viewer setting (FR-007). */
export function createPanel() {
  return { open: false, lastActivity: 0 };
}

function closed(panel) {
  const wasOpen = panel.open;
  panel.open = false;
  // Focus goes back to the gear only when something actually closed (FR-008).
  return { open: false, focusGear: wasOpen };
}

/** The gear button: open a closed panel, close an open one. */
export function toggle(panel, now) {
  if (panel.open) return closed(panel);
  panel.open = true;
  panel.lastActivity = now;
  return { open: true, focusGear: false };
}

/** The ✕ button or Escape. */
export function close(panel) {
  return closed(panel);
}

/** Any pointer, key or focus event inside the panel. */
export function activity(panel, now) {
  if (panel.open) panel.lastActivity = now;
}

/** Called regularly: closes an idle panel, else returns `null`. */
export function tick(panel, now) {
  if (!panel.open || now - panel.lastActivity < AUTO_CLOSE_MS) return null;
  return closed(panel);
}
