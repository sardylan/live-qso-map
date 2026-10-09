// Pure auto-zoom controller: a state machine fed by page events (feature 004, research R7).
// No DOM, no MapLibre: everything here is tested with `node --test web/`.

const MIN_FIT_INTERVAL_MS = 1000;

/**
 * The auto-zoom controller (data-model "Auto-zoom controller"). Every method returns a list of
 * actions for the glue: `{type: 'save', enabled}` stores the choice, `{type: 'fit'}` refits now.
 * Only user camera moves may call `userMove()`; moves the page makes itself are never fed in, so
 * they never switch auto-zoom off (FR-011).
 */
export function createAutoZoom({ enabled = true } = {}) {
  const state = { enabled, popupOpen: false, pending: false, lastFitAt: -Infinity, autoFlat: false };

  function fitAt(now) {
    state.pending = false;
    state.lastFitAt = now;
    return [{ type: 'fit' }];
  }

  return {
    state,

    /** The button: on refits at once (FR-012); off changes nothing else (FR-017). */
    toggle(now) {
      state.enabled = !state.enabled;
      const save = { type: 'save', enabled: state.enabled };
      if (!state.enabled) {
        state.pending = false;
        return [save];
      }
      return [save, ...fitAt(now)];
    },

    /** Something the frame depends on changed: data, limit, QTH, panels or window size. */
    change() {
      if (state.enabled) state.pending = true;
      return [];
    },

    /** The viewer moved the camera by hand (FR-011). */
    userMove() {
      if (!state.enabled) return [];
      state.enabled = false;
      state.pending = false;
      return [{ type: 'save', enabled: false }];
    },

    popupOpened() {
      state.popupOpen = true;
      return [];
    },

    /** One catch-up refit after a popup closes (FR-012a). */
    popupClosed() {
      state.popupOpen = false;
      if (state.enabled) state.pending = true;
      return [];
    },

    /**
     * After a fit is computed: the projection to show (FR-015–FR-017). Only a globe preference can
     * become an automatic flat view, and only when the frame does not fit on the globe.
     */
    fitted(fitsGlobe, preference) {
      state.autoFlat = preference === 'globe' && !fitsGlobe;
      return state.autoFlat || preference !== 'globe' ? 'mercator' : 'globe';
    },

    /**
     * The viewer chose a projection with the button (FR-018): the choice wins, so any automatic
     * flat view ends and auto-zoom switches off, as after a manual move.
     */
    projectionChosen() {
      state.autoFlat = false;
      if (!state.enabled) return [];
      state.enabled = false;
      state.pending = false;
      return [{ type: 'save', enabled: false }];
    },

    /** Called on a timer: at most one fit per second, never while a popup is open (FR-014). */
    tick(now) {
      if (!state.enabled || !state.pending || state.popupOpen) return [];
      if (now - state.lastFitAt < MIN_FIT_INTERVAL_MS) return [];
      return fitAt(now);
    },
  };
}
