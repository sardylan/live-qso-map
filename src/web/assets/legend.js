// The Legend group (feature 011, FR-011–FR-013): which band groups the viewer shows. Pure; the
// groups come from `hello.band_groups`, already holding only the considered bands (HR-2 order).

const LEGEND_KEY = 'lqm.legend';

/** The groups to offer, in the server's order, each with its checkbox text (FR-012). */
export function offeredGroups(bandGroups) {
  if (!Array.isArray(bandGroups)) return [];
  return bandGroups.map((group) => {
    const n = group.bands.length;
    return { ...group, text: `${group.label} (${n} ${n === 1 ? 'band' : 'bands'})` };
  });
}

/**
 * The bands to show: the union of the chosen groups' bands, in the frequency order of `allBands`
 * (`hello.bands` names), or `null` for every band. A choice naming no known group is `null`.
 */
export function chosenBands(choice, bandGroups, allBands) {
  if (choice === 'all') return null;
  const wanted = new Set();
  for (const group of bandGroups) {
    if (choice.includes(group.name)) for (const band of group.bands) wanted.add(band);
  }
  if (wanted.size === 0) return null;
  const ordered = allBands.filter((band) => wanted.has(band));
  // A group band missing from `allBands` would be a server bug; keep it rather than lose it.
  for (const band of wanted) if (!ordered.includes(band)) ordered.push(band);
  return ordered;
}

/** `all`, or the saved group names still offered; anything else gives `all` (constitution IV). */
export function loadLegend(storage, groupNames) {
  try {
    const raw = storage ? storage.getItem(LEGEND_KEY) : null;
    if (raw === null || raw === 'all') return 'all';
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return 'all';
    const kept = parsed.filter((name) => groupNames.includes(name));
    return kept.length === 0 ? 'all' : kept;
  } catch {
    return 'all';
  }
}

export function saveLegend(storage, choice) {
  try {
    if (storage) storage.setItem(LEGEND_KEY, choice === 'all' ? 'all' : JSON.stringify(choice));
  } catch {
    // A browser that refuses storage keeps the choice for this visit only.
  }
}

/**
 * A click on a checkbox: `all` clears the groups; a group is added or removed, leaving `all`.
 * Unticking the last group goes back to `all`.
 */
export function toggleGroup(choice, name) {
  if (name === 'all') return 'all';
  const current = choice === 'all' ? [] : choice;
  const next = current.includes(name) ? current.filter((n) => n !== name) : [...current, name];
  return next.length === 0 ? 'all' : next;
}
