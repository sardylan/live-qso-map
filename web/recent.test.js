import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createState, applySnapshot, applyUpsert, applyRemove, setLimit, markers } from '../src/web/assets/map-state.js';
import { recentQsos, headerCells, rowCells, shortName, entityLabel } from '../src/web/assets/recent.js';

function qso(n, overrides = {}) {
  const minute = String(n).padStart(2, '0');
  return {
    id: `Q${minute}`,
    rev: 1,
    sort_key: `2026-10-08T15:${minute}:00Z`,
    datetime: `2026-10-08T15:${minute}:00Z`,
    call: `K${n}AA`,
    band: '20m',
    mode: 'CW',
    submode: null,
    remote: { lat: 40 + n, lon: -70 - n, precision: 'locator4', source: 'logger' },
    station: null,
    ...overrides,
  };
}

function stateOf(qsos, limit = 20) {
  const s = createState({ limit });
  applySnapshot(s, qsos);
  return s;
}

const ids = (list) => list.map((q) => q.id);

test('the five newest by QSO time, newest first', () => {
  const s = stateOf([1, 2, 3, 4, 5, 6, 7].map((n) => qso(n)));
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q07', 'Q06', 'Q05', 'Q04', 'Q03']);
});

test('a late QSO with an older time does not enter the box', () => {
  const s = stateOf([3, 4, 5, 6, 7].map((n) => qso(n)));
  applyUpsert(s, qso(1));
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q07', 'Q06', 'Q05', 'Q04', 'Q03']);
});

test('a correction that changes the time re-orders the box', () => {
  const s = stateOf([3, 4, 5, 6, 7].map((n) => qso(n)));
  applyUpsert(s, qso(3, { rev: 2, sort_key: '2026-10-08T15:59:00Z', datetime: '2026-10-08T15:59:00Z' }));
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q03', 'Q07', 'Q06', 'Q05', 'Q04']);
});

test('a removal moves the next one up', () => {
  const s = stateOf([1, 2, 3, 4, 5, 6, 7].map((n) => qso(n)));
  applyRemove(s, 'Q06');
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q07', 'Q05', 'Q04', 'Q03', 'Q02']);
});

test('fewer than five gives them all; none gives an empty list', () => {
  assert.deepEqual(ids(recentQsos(stateOf([qso(1), qso(2)]), 5)), ['Q02', 'Q01']);
  assert.deepEqual(recentQsos(stateOf([]), 5), []);
});

test('the box follows the limit window', () => {
  const s = stateOf(Array.from({ length: 20 }, (_, i) => qso(i + 1)), 20);
  setLimit(s, 10);
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q20', 'Q19', 'Q18', 'Q17', 'Q16']);
});

test('QSOs without a remote position are listed', () => {
  const s = stateOf([qso(1), qso(2, { remote: null })]);
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q02', 'Q01']);
});

const ALL = ['call', 'band', 'mode', 'date', 'time', 'cqzone', 'country'];
const texts = (cells) => cells.map((c) => c.text);

test('headerCells labels every column, in the configured order', () => {
  assert.deepEqual(
    headerCells(['call', 'band', 'time', 'cqzone', 'country']).map((c) => c.text),
    ['Call', 'Band', 'UTC', 'CQ', 'Country'],
  );
  assert.deepEqual(headerCells(ALL).map((c) => c.text), ['Call', 'Band', 'Mode', 'Date', 'UTC', 'CQ', 'Country']);
  assert.deepEqual(headerCells(['country', 'call']).map((c) => c.column), ['country', 'call']);
});

test('rowCells shows every column in the configured order', () => {
  const q = qso(4, { mode: 'SSB', submode: 'USB', entity: { prefix: 'JA', name: 'Japan', cq_zone: 25 } });
  assert.deepEqual(texts(rowCells(q, ALL)), ['K4AA', '20m', 'SSB/USB', '2026-10-08', '15:04Z', '25', 'JA · Japan']);
  assert.deepEqual(texts(rowCells(q, ['country', 'call', 'mode'])), ['JA · Japan', 'K4AA', 'SSB/USB']);
  assert.deepEqual(rowCells(q, ['call', 'band']).map((c) => c.column), ['call', 'band']);
});

test('rowCells cuts a long country name and keeps the full label as the title', () => {
  const [cell] = rowCells(qso(1, { entity: { prefix: 'TL', name: 'Central African Republic', cq_zone: 36 } }), ['country']);
  assert.equal(cell.text, 'TL · Central African R…');
  assert.equal(cell.title, 'TL · Central African Republic');
});

test('rowCells placeholders: a dash for country and CQ zone, empty otherwise', () => {
  const q = qso(1, { entity: null, mode: null, band: null, datetime: null });
  assert.deepEqual(texts(rowCells(q, ALL)), ['K1AA', '', '', '', '', '—', '—']);
  assert.equal(rowCells(q, ['country'])[0].title, '');
  assert.deepEqual(texts(rowCells(qso(2), ['cqzone', 'country'])), ['—', '—'], 'a QSO without the field');
});

test('rowCells shows different dates either side of midnight UTC', () => {
  const cols = ['date', 'time'];
  assert.deepEqual(texts(rowCells(qso(1, { datetime: '2026-10-09T23:59:00Z' }), cols)), ['2026-10-09', '23:59Z']);
  assert.deepEqual(texts(rowCells(qso(2, { datetime: '2026-10-10T00:01:00Z' }), cols)), ['2026-10-10', '00:01Z']);
});

test("rowCells keeps text verbatim; escaping is the page glue's job", () => {
  assert.equal(rowCells(qso(1, { call: '<b>X</b>' }), ['call'])[0].text, '<b>X</b>');
});

test('shortName keeps names up to 18 characters and cuts longer ones to 18 with an ellipsis', () => {
  assert.equal(shortName('Japan'), 'Japan');
  assert.equal(shortName('Wallis & Futuna Is'), 'Wallis & Futuna Is');
  assert.equal(shortName('Central African Republic'), 'Central African R…');
  assert.equal(shortName('Central African Republic').length, 18);
  // A cut that ends in a space is trimmed before the ellipsis.
  assert.equal(shortName('Tristan da Cunha & Gough'), 'Tristan da Cunha…');
});

test('entityLabel gives prefix · name, or a dash', () => {
  assert.equal(entityLabel({ prefix: 'JA', name: 'Japan' }), 'JA · Japan');
  assert.equal(entityLabel({ prefix: 'TL', name: 'Central African Republic' }), 'TL · Central African Republic');
  assert.equal(entityLabel(null), '—');
  assert.equal(entityLabel(undefined), '—');
});
