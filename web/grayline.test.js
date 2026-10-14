import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shade, elevationAt, tileLatLon, tilePixels, graylineBeforeId, graylineTiles, parseGraylineUrl, rgbOf } from '../src/web/assets/grayline.js';
import { solarElevation, subsolarPoint } from '../src/web/assets/sun.js';

// The grayline (feature 013, research R7–R9, revised: raster tiles drawn in the page; T016).

const DATES = ['2026-03-20', '2026-06-21', '2026-12-21'].flatMap((d) =>
  ['00:00', '06:00', '12:00', '18:00'].map((t) => new Date(`${d}T${t}:00Z`)),
);

test('the shade is nothing in day, full from −12°, and grows smoothly between', () => {
  assert.equal(shade(5), 0);
  assert.equal(shade(0), 0);
  assert.equal(shade(-12), 1);
  assert.equal(shade(-40), 1);
  let previous = 0;
  for (let e = -0.25; e >= -12; e -= 0.25) {
    const v = shade(e);
    assert.ok(v > previous && v - previous < 0.05, `${e}: ${v} after ${previous}`);
    previous = v;
  }
  assert.ok(shade(-6) > 0.2 && shade(-6) < 0.8, 'civil twilight is the lighter part of the fade');
});

test('the Sun height from the subsolar point agrees with sun.js', () => {
  for (const date of DATES) {
    const sub = subsolarPoint(date);
    for (let lat = -80; lat <= 80; lat += 20) {
      for (let lon = -180; lon < 180; lon += 30) {
        const e = elevationAt(sub, lat, lon);
        assert.ok(Math.abs(e - solarElevation(date, lat, lon)) < 0.1, `${date.toISOString()} ${lat},${lon}: ${e}`);
      }
    }
  }
});

test('tile pixels map to Web Mercator latitudes and longitudes', () => {
  const [lat0, lon0] = tileLatLon(0, 0, 0, 256, 0, 0);
  assert.ok(Math.abs(lat0 - 85.0511) < 0.01 && Math.abs(lon0 + 180) < 1e-9, `${lat0},${lon0}`);
  const [latC, lonC] = tileLatLon(0, 0, 0, 256, 128, 128);
  assert.ok(Math.abs(latC) < 1e-9 && Math.abs(lonC) < 1e-9);
  const [latT, lonT] = tileLatLon(1, 1, 0, 256, 0, 256);
  assert.ok(Math.abs(latT) < 1e-9 && Math.abs(lonT) < 1e-9, 'tile 1/1/0 starts at the equator and Greenwich');
});

test('every tile pixel carries the night colour at the shade of its Sun height', () => {
  for (const date of DATES.slice(0, 6)) {
    const sub = subsolarPoint(date);
    for (const [z, x, y] of [[0, 0, 0], [2, 1, 2], [3, 7, 3]]) {
      const data = tilePixels(z, x, y, 64, sub, [11, 26, 60], 0.4);
      assert.equal(data.length, 64 * 64 * 4);
      for (const [px, py] of [[0, 0], [10, 50], [32, 32], [63, 7]]) {
        const i = (py * 64 + px) * 4;
        assert.deepEqual([...data.slice(i, i + 3)], [11, 26, 60]);
        const [lat, lon] = tileLatLon(z, x, y, 64, px + 0.5, py + 0.5);
        const expected = Math.round(shade(elevationAt(sub, lat, lon)) * 0.4 * 255);
        assert.ok(Math.abs(data[i + 3] - expected) <= 1, `${z}/${x}/${y} ${px},${py}: alpha ${data[i + 3]} vs ${expected}`);
      }
    }
  }
});

test('deep night is at full opacity, full day is clear', () => {
  const sub = { lat: 0, lon: 0 };
  // Tile 1/1/0 (north-east): mostly day around Greenwich... tile 1/0/0 holds 180°W–0, so its far west is night.
  const data = tilePixels(1, 0, 0, 64, sub, [0, 0, 0], 0.5);
  const alpha = (px, py) => data[(py * 64 + px) * 4 + 3];
  assert.equal(alpha(0, 63), Math.round(0.5 * 255), 'near 180°W on the equator: deep night');
  assert.equal(alpha(63, 63), 0, 'near Greenwich on the equator: full day');
});

// The OpenFreeMap style layers around the first boundary (research R9), reduced to id, type and
// source-layer.
const LIBERTY = [
  { id: 'background', type: 'background' },
  { id: 'water', type: 'fill', 'source-layer': 'water' },
  { id: 'road_one_way_arrow', type: 'symbol', 'source-layer': 'transportation' },
  { id: 'building-3d', type: 'fill-extrusion', 'source-layer': 'building' },
  { id: 'boundary_3', type: 'line', 'source-layer': 'boundary' },
  { id: 'boundary_2', type: 'line', 'source-layer': 'boundary' },
  { id: 'label_city', type: 'symbol', 'source-layer': 'place' },
];
const FIORD = [
  { id: 'background', type: 'background' },
  { id: 'water', type: 'fill', 'source-layer': 'water' },
  { id: 'water_name', type: 'symbol', 'source-layer': 'water_name' },
  { id: 'highway_ref', type: 'symbol', 'source-layer': 'transportation_name' },
  { id: 'boundary_state', type: 'line', 'source-layer': 'boundary' },
  { id: 'place_city', type: 'symbol', 'source-layer': 'place' },
];
const DARK = [
  { id: 'background', type: 'background' },
  { id: 'water', type: 'fill', 'source-layer': 'water' },
  { id: 'water_name', type: 'symbol', 'source-layer': 'water_name' },
  { id: 'building', type: 'fill', 'source-layer': 'building' },
  { id: 'highway_name_motorway', type: 'symbol', 'source-layer': 'transportation_name' },
  { id: 'boundary_state', type: 'line', 'source-layer': 'boundary' },
  { id: 'place_country_major', type: 'symbol', 'source-layer': 'place' },
];

test('the grayline goes just before the first boundary layer', () => {
  assert.equal(graylineBeforeId(LIBERTY), 'boundary_3');
  assert.equal(graylineBeforeId(FIORD), 'boundary_state');
  assert.equal(graylineBeforeId(DARK), 'boundary_state');
});

test('without boundaries it goes before the first symbol layer, else on top', () => {
  assert.equal(graylineBeforeId(LIBERTY.filter((l) => l['source-layer'] !== 'boundary')), 'road_one_way_arrow');
  assert.equal(graylineBeforeId([{ id: 'background', type: 'background' }]), undefined);
  assert.equal(graylineBeforeId(undefined), undefined);
});

test('graylineTiles names the tiles by theme and minute, so either change reloads them', () => {
  const a = graylineTiles('light', new Date('2026-06-21T12:00:10Z'));
  assert.deepEqual(a, ['grayline://light/29700720/{z}/{x}/{y}']);
  assert.deepEqual(graylineTiles('light', new Date('2026-06-21T12:00:50Z')), a, 'same minute');
  assert.notDeepEqual(graylineTiles('dark', new Date('2026-06-21T12:00:10Z')), a);
  assert.deepEqual(parseGraylineUrl('grayline://dark/29696160/3/5/2'), { theme: 'dark', minute: 29696160, z: 3, x: 5, y: 2 });
  assert.equal(parseGraylineUrl('https://example.org/3/5/2'), null);
});

test('rgbOf reads a hex colour', () => {
  assert.deepEqual(rgbOf('#0b1a3c'), [11, 26, 60]);
});
