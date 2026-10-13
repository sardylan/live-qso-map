# Feature Specification: Automatic Theme, Atmosphere, Settings Panel and Legend Selection

**Feature Branch**: `feature/011-theme-settings-panel`
**Created**: 2026-10-13
**Status**: Draft
**Input**: User description: "New spec: frontend theme (including background) should have the right
behavior based on the light sensors (if any), current time or any other way to switch between dark
or light theme, or any other intermediate theme between dark and light, if exists or are a best
practice. Evaluate also the possibility to display the atmosphere, selectable by user on the
frontend, saved on browser local storage, and enabled as default. Since settings are too much to
stay on the page, group them in something that let the user choose what to do. Choose one between
most used and suggested solutions for the UX (settings button with modal, collapsed tool, or
else). In settings add also something to choose if the legend should display only a restricted set
of items (like HF only, contest only, etc...). The user should choose only between combination of
server-enabled items. Set the default configuration of the server to support only hf-context bands,
and the legend displaying only the 6 colors."

## Context

**The page today:**
- **Appearance:** always light, whatever the time or the room. On a wall screen in a dark shack,
  at night, the bright page dazzles.
- **Viewer settings:** "Map: last N" and the recent-QSO box always take a corner of the map. More
  settings would crowd it further.
- **Legend:** it lists every band the server considers. With no band setting (today's default),
  that's every band, far more colours than a contest uses.

**Options evaluated for switching between light and dark:**
- **The device's light sensor:** browsers don't expose it by default (the ambient-light API is
  experimental, off by default, and limited for privacy). So it isn't used directly.
- **The device's light/dark preference:** every current browser exposes it. Phones, tablets and
  many laptops already switch it from their light sensor or the time of day.
- **The time of day at the station:** the page knows the station's position, so it can tell
  day, twilight and night there.
- **An intermediate theme:** a "dim" theme between light and dark is common in map and media
  apps for twilight or low light.

**Atmosphere:** on the globe, a thin glow around the Earth's limb and a sky colour behind it make
the globe read as a planet. It has no meaning on the flat map.

## Clarifications

### Session 2026-10-13

- Q: What does the "automatic" theme follow, and is there an intermediate theme? → A: Both. The
  viewer picks one of five choices:
  - **Automatic, device** (the default): follows the device's light/dark preference;
  - **Automatic, sun at station**: light by day, Dim at twilight, Dark at night;
  - **Light**, **Dim** or **Dark**, fixed.
- Q: Does restricting the legend also hide the QSOs of the other bands on the map? → A: Legend and
  map. QSOs on bands outside the chosen groups are hidden from the map and the recent box. The
  server still keeps them, and they come back when the choice changes. The legend always
  describes exactly what is drawn.
- Q: What can the viewer pick in the Legend group: groups, single bands, or both? → A: Groups
  only: "All considered bands", or any combination of the named groups that include considered
  bands. Single bands can't be picked one by one.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The page matches the light around it (Priority: P1)

As an operator, the map is light during the day and dark at night, including its background and
panels, without my doing anything. When I prefer, I can fix it to a theme, and the page remembers
my choice.

**Why this priority**: a dazzling wall screen in a dark shack is the most visible problem, and it
affects every session.

**Independent Test**: with the theme on automatic, switch the condition the automatic mode
follows (device preference, or a simulated time of day at the station). Check that page,
background, panels and base map change within a minute. Then pick a fixed theme, reload, and check
it is kept.

**Acceptance Scenarios**:

1. **Given** the theme on "Automatic, device", **When** the device switches its preference from
   light to dark, **Then** the page switches to the Dark theme within 60 seconds, without a reload,
   and keeps showing every QSO.
2. **Given** the dark theme, **When** the page is shown, **Then** the base map, the page
   background, the panels, the legend, the recent box and the score are all dark, with text that
   stays readable.
3. **Given** a fixed theme chosen by the viewer, **When** the browser is closed and reopened,
   **Then** the same theme is shown (constitution IV).
4. **Given** the theme changes, **When** the map redraws, **Then** the view (centre, zoom,
   projection) and the QSO markers stay as they were.
5. **Given** the theme on "Automatic, sun at station", **When** the sun at the station goes from
   above the horizon to civil twilight, then lower, **Then** the page goes from Light to Dim to
   Dark, each within 60 seconds of the change.

---

### User Story 2 - All viewer settings in one place (Priority: P1)

As a viewer, the map shows no settings until I ask for them. One settings button opens a panel
with every viewer setting, grouped:
- **Map:** "last N" QSOs, atmosphere;
- **Last QSOs box:** shown, size, order;
- **Appearance:** theme;
- **Legend:** which bands it lists.

The map stays visible behind the panel. Closing the panel returns the map to full view.

**Why this priority**: it frees the map from the settings corner, and every new setting in this
feature needs a home.

**Independent Test**: open the panel, change a setting in each group, close it, reload, and check
every change is kept. Check that no setting is visible on the map while the panel is closed.

**Acceptance Scenarios**:

1. **Given** the page, **When** it loads, **Then** no setting is visible, only a settings
   button in a corner.
2. **Given** the settings button, **When** it is pressed (mouse, touch or keyboard), **Then** a
   side panel opens with the groups Map, Last QSOs box, Appearance and Legend, and the map stays
   visible beside it.
3. **Given** the open panel, **When** the viewer presses the close button, presses Escape, or
   presses the settings button again, **Then** the panel closes.
4. **Given** a setting changed in the panel, **When** it changes, **Then** it applies
   immediately (no "apply" button) and is saved in the browser (constitution IV).
5. **Given** the existing settings ("last N", box shown, size and order), **When** the feature
   ships, **Then** they move into the panel, keeping their saved values.
6. **Given** a keyboard or a screen reader, **When** the panel is used, **Then** every control is
   reachable, labelled, and focus returns to the settings button when the panel closes.

---

### User Story 3 - Atmosphere around the globe (Priority: P2)

As a viewer, the globe shows the atmosphere (a glow along the Earth's edge and a sky colour
behind it) by default. I can switch it off in the settings, and the page remembers.

**Why this priority**: a visual improvement that's on by default, with a simple switch.

**Independent Test**: on the globe, check the glow is there; switch it off in the panel, reload,
and check it stays off. Switch to the flat map, and check there's no atmosphere.

**Acceptance Scenarios**:

1. **Given** a first visit, **When** the globe is shown, **Then** the atmosphere is on.
2. **Given** the atmosphere switched off, **When** the browser is reopened, **Then** it is still
   off.
3. **Given** the flat projection, **When** it is shown, **Then** no atmosphere is drawn, whatever
   the setting.
4. **Given** the dark theme, **When** the globe is shown with the atmosphere on, **Then** the
   atmosphere and the space behind the globe use night colours, not daylight blue.

---

### User Story 4 - Show only chosen band groups (Priority: P2)

As a viewer, I can choose which band groups the page shows, for example "HF contest" or "WARC",
among the groups the server actually considers. The legend lists only those bands, and the map
and the recent box show only QSOs on them. A group with none of the server's bands isn't offered.

**Why this priority**: it makes the legend match the operating, for example contest bands only,
without touching the server.

**Independent Test**: with the server considering all HF bands and QSOs on 40 m and 30 m, choose
"HF contest" in the panel. Check that the legend lists 160, 80, 40, 20, 15 and 10 m in frequency
order, and that the 30 m QSO disappears from the map and the box. Choose "All considered bands",
and check it comes back. Reload and check the choice is kept.

**Acceptance Scenarios**:

1. **Given** the server's considered bands, **When** the Legend group opens, **Then** it offers
   "All considered bands", plus each named band group with at least one considered band, each
   showing how many of its bands are considered.
2. **Given** one or more groups chosen, **When** the legend is shown, **Then** it lists the union
   of their considered bands, in frequency order (HR-2).
3. **Given** a saved choice whose groups no longer have any considered band (the server
   configuration changed), **When** the page loads, **Then** the legend falls back to "All
   considered bands" (constitution IV: a stale value falls back to the default).
4. **Given** no choice yet, **When** the page loads, **Then** the legend lists all considered
   bands, and every QSO is shown.
5. **Given** a choice that excludes some bands, **When** QSOs arrive or are corrected on those
   bands, **Then** they are not drawn on the map nor listed in the recent box. "Last N" and
   auto-zoom count only the QSOs that are shown. The contest score is unchanged, because it comes
   from the logger.
6. **Given** a QSO hidden by the choice, **When** the choice changes to include its band, **Then**
   it appears without a reload.

---

### User Story 5 - Contest bands by default on the server (Priority: P3)

As an operator installing or upgrading the service without a band setting, it considers the six
contest HF bands (160, 80, 40, 20, 15 and 10 m) by default, so the legend shows six colours. I
can still choose any bands, including all of them, in the configuration.

**Why this priority**: a default change, quick to make, but it affects existing installations
without a band setting.

**Independent Test**: start the service with no band setting, and check the startup log and the
page legend list exactly the six contest bands. Then configure all bands, and check every band is
considered.

**Acceptance Scenarios**:

1. **Given** no band setting, **When** the service starts, **Then** it considers 160, 80, 40, 20,
   15 and 10 m, logs them in frequency order, and the legend shows those six colours.
2. **Given** the configuration asks for every band explicitly, **When** the service starts,
   **Then** every band is considered, as the old default did.
3. **Given** an existing band setting, **When** the service is upgraded, **Then** that setting
   is used unchanged.
4. **Given** the configuration export (feature 008), **When** it runs with no band setting,
   **Then** it writes the new default explicitly.

---

### Edge Cases

- **Browser storage refused or cleared**: the theme, atmosphere and legend choices fall back to
  their defaults, and the page works (constitution IV).
- **No station position known**: with "Automatic, sun at station", when neither a configured
  position nor a QSO with a station position exists, the page follows the device preference
  instead, and the Appearance group says why. As soon as a QSO brings a station position, the sun
  mode takes over within 60 seconds.
- **Operator-configured base map** (a custom style or raster tiles): the dark theme can't
  switch to a dark version of an unknown map. The map is shown dimmed instead, and the rest of
  the page goes dark.
- **Polar day and night at the station**: with a sun-based theme, the sun may not rise or set for
  days. The theme follows the sun's actual height, not sunrise and sunset times.
- **Wall screen with nobody touching it**: the panel never opens by itself, and an open panel
  closes after 5 minutes without interaction, so a kiosk returns to the full map.
- **Very small screens**: on narrow screens (phones), the panel takes the full width and closes
  with the same controls.
- **Theme change during an animation** (auto-zoom, fly-to): the animation finishes normally.

## Requirements *(mandatory)*

### Functional Requirements

**Theme (User Story 1)**

- **FR-001**: The page MUST offer three themes, **Light**, **Dim** (between the two) and **Dark**,
  and five theme choices in the Appearance group:
  - **Automatic, device** (the default): Light or Dark, following the device's light/dark
    preference;
  - **Automatic, sun at station**: Light while the sun is above the horizon at the station; Dim in
    civil twilight (the sun between 0° and 6° below the horizon); Dark when it's lower. The
    station is where the map's house marker is (feature 004): the configured position, else the
    station position of the newest QSO;
  - **Light**, **Dim**, **Dark**: fixed.
- **FR-002**: The theme MUST apply to the whole page: base map, page background (including the
  space behind the globe), settings panel, legend, recent box, score, banners and map buttons.
  Text MUST keep a contrast ratio of at least 4.5:1 in every theme.
- **FR-003**: With the automatic theme, a change of the condition it follows MUST be applied
  within 60 seconds, without a reload, and without changing the view or losing QSOs.
- **FR-004**: The device's light sensor MUST NOT be required. The page relies on information every
  current browser provides (see the Context section).

**Settings panel (User Story 2)**

- **FR-005**: All viewer settings MUST be in one panel, opened and closed by a single settings
  button on the map. The panel is a side panel that leaves the map visible and usable. On screens
  narrower than 600 px, it takes the full width.
- **FR-006**: The panel MUST group the settings as Map, Last QSOs box, Appearance and Legend. A
  change MUST apply immediately and be saved in the browser.
- **FR-007**: The panel MUST close with its close button, the Escape key, or the settings button.
  It MUST close by itself after 5 minutes without interaction. It MUST always start closed.
- **FR-008**: The panel MUST be usable with keyboard and screen reader: labelled controls, a
  logical focus order, and focus back on the settings button after closing.
- **FR-009**: The existing settings ("last N" QSOs; box shown, size, order) MUST move into the
  panel with their saved values and behaviour unchanged. The quick map buttons (zoom, globe/flat,
  auto-zoom) stay on the map.

**Atmosphere (User Story 3)**

- **FR-010**: On the globe, the page MUST draw an atmosphere: a glow along the limb and a sky
  colour behind the Earth. It is on by default, can be switched off in the Map group, and the
  choice is saved in the browser.
- **FR-011**: The atmosphere MUST NOT be drawn on the flat map, and MUST use night colours in the
  dark theme.

**Legend selection (User Story 4)**

- **FR-012**: The Legend group MUST offer "All considered bands" (the default), plus every named
  band group (`docs/ham-radio.md` and the server's band groups: LF/MF, HF, HF contest, WARC, VHF,
  UHF, SHF, EHF) that contains at least one band the server considers. Each group shows how many
  of its bands are considered. The viewer can choose one or more groups. Single bands can't be
  picked one by one.
- Q: Which position does "Automatic, sun at station" use? → A: The same position as the map's
  house marker (feature 004): the configured station position, else the station position of the
  newest QSO. It falls back to the device preference only when neither exists.
- **FR-013**: The legend MUST list the union of the chosen groups' considered bands, always in
  frequency order (HR-2), with the server's colours. The map and the recent box MUST show only
  QSOs on those bands. "Last N" and auto-zoom MUST count only the shown QSOs. A change of choice
  MUST apply immediately, without a reload. Hidden QSOs stay on the server, and the contest score
  is not affected.
- **FR-014**: The choice MUST be saved in the browser. A saved choice that no longer matches any
  considered band, or an unknown group, MUST fall back to "All considered bands".

**Server default bands (User Story 5)**

- **FR-015**: With no band setting, the service MUST consider the HF contest group: 160, 80, 40,
  20, 15 and 10 m.
- **FR-016**: The configuration MUST offer an explicit way to consider every band (for example
  the group name `all`), documented next to the band setting.
- **FR-017**: An existing band setting MUST keep working unchanged. The configuration export
  (feature 008) MUST write the effective bands, including the new default.
- **FR-018**: The README, the example configuration and the changelog MUST say that the default
  changed from every band to the HF contest bands, and how to get every band back.

### Key Entities

- **Theme**: the colour scheme in use: Light, Dim or Dark.
- **Theme choice**: the viewer setting, saved in the browser. It is one of "Automatic, device"
  (default), "Automatic, sun at station", Light, Dim or Dark.
- **Atmosphere choice**: on (default) or off, saved in the browser.
- **Legend choice**: "All considered bands", or a set of band group names, saved in the browser.
  It is valid only against the bands the server currently considers. It decides both the legend
  and which QSOs are shown.
- **Band group**: a named, fixed list of bands in frequency order (the server's existing groups).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the automatic theme, the page reaches the right theme within 60 seconds of
  the condition changing, in 100% of tested switches, with no QSO lost and the view unchanged.
- **SC-002**: In every theme, all text on the page meets a contrast ratio of at least 4.5:1.
- **SC-003**: With the panel closed, no setting control covers the map: only the settings button
  and the quick map buttons remain.
- **SC-004**: A first-time viewer finds and changes the "last N" setting within 30 seconds,
  starting from the closed panel.
- **SC-005**: Every viewer setting (the existing four plus theme, atmosphere and legend)
  survives closing and reopening the browser (constitution IV).
- **SC-006**: With no band setting, a fresh installation's legend shows exactly six bands: 160,
  80, 40, 20, 15 and 10 m, in that order.
- **SC-007**: The legend never offers a group with no considered band, and never lists a band the
  server doesn't consider.
- **SC-008**: With a restricted choice, no marker or recent-box row shows a band the legend doesn't
  list, in 100% of tested cases.

## Assumptions

- **Settings UX**: a settings button on the map opening a side panel (a drawer) was chosen over a
  modal dialog and a collapsible toolbar. Map apps use it for layers and options: the map stays
  visible, so a change can be seen as it's made. A modal would hide the map; a collapsible
  toolbar fits only a handful of controls.
- **No light sensor**: browsers don't expose it by default. Devices that have one already use it
  to switch their light/dark preference, which the page can follow.
- **Dark base map**: for the default base map, a dark style from the same provider is used. For
  an operator-configured map, the page dims it (Edge Cases).
- **Atmosphere colours** are part of each theme; no separate colour setting.
- **The panel's open/closed state is not a setting**: it always starts closed, which suits a
  kiosk.
- **Band groups** are the server's existing named groups (feature 003). No new group is defined,
  apart from the explicit "every band" value (FR-016).
- **Upgrade impact**: installations without a band setting will see only the six contest bands
  after upgrading. This is intended (the request asks for it), and the changelog and README state
  it with the way back (FR-018).
