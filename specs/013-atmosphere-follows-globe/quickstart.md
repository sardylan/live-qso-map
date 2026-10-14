# Quickstart: check the atmosphere and the grayline by eye

Start the program as usual and open the map. Settings are in the gear panel, Map group.

## Globe glow (User Stories 1 and 2)

1. **Set up:** Globe, "Atmosphere on the globe" checked, Dark theme (the glow shows best there).
   Auto-zoom off.
2. **Hand move:** the glow is brightest on the daylit side. Drag the globe half a turn: the bright
   side moves with the Earth and stays on the daylit side. Rotate and tilt (right-drag or
   Ctrl-drag): the glow turns too.
3. **Zoom:** zoom in until the limb leaves the window, then out. The glow is back along the limb
   at once, in the same place on the Earth.
4. **Auto-zoom:** switch auto-zoom on, and log a QSO from the other side of the Earth (from the
   logger, or by replaying a capture). While the globe flies, the glow moves with the Earth.

## Grayline (User Story 3)

5. **Flat map:** with "Grayline (day and night)" checked (the default), the night side is
   shaded. The shade fades smoothly through twilight to nothing on the day side, with no hard
   edge. Compare it with any day/night map for the current UTC time: the start of the fade
   lines up with sunrise and sunset.
6. **Pan and zoom:** pan across the date line, and zoom out until two copies of the world show.
   The shading stays over the same places, on every copy, with no seam. Near a solstice, check
   that the pole in polar night is shaded up to the top (or bottom) of the map.
7. **Labels and markers:** on the night side, city and country names, borders, QSO markers,
   paths and the house marker are as readable as in day. Clicking a night-side QSO marker opens
   its popup.
8. **Globe:** switch to Globe. The same places are shaded, on the Earth's surface, under the glow.
   The glow's bright side is over the unshaded side.
9. **Themes:** in Light, Dim and Dark, night and twilight are both visible against day. The
   colours in `themes.js` (`night`, `nightOpacity`) are tuned here.
10. **Choices:** uncheck Grayline: no shading on either view. Check "Sun position": a small Sun
    symbol at the point where the Sun is overhead, under any QSO marker, not clickable, and
    hidden on the far side of the globe. Reload: both choices are kept.
11. **Time:** leave the page open for an hour. The shading and the Sun symbol move about 15° west,
    with no visible jump.

## Automated checks

- `node --test 'web/*.test.js'`: on every change.
- `node tests/browser/run.mjs`: on releases. It includes step 10 `atmosphere` and step 11
  `grayline`.

## Known side effects

- 3D buildings (street zoom, Light theme) take their shading from the same light, so it may come
  from another direction than before (research R6).
- The shading stops at about 85° of latitude, where MapLibre stops drawing the map (research R8).
