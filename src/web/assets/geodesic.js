// Great-circle interpolation with longitudes unwrapped for continuity (research R15).

const DEG = Math.PI / 180;

function toVector([lat, lon]) {
  const φ = lat * DEG;
  const λ = lon * DEG;
  return [Math.cos(φ) * Math.cos(λ), Math.cos(φ) * Math.sin(λ), Math.sin(φ)];
}

/**
 * Points along the great circle from `a` to `b` (both `[lat, lon]`), `segments + 1` points.
 * Longitudes are unwrapped so a line crossing the antimeridian stays continuous.
 */
export function greatCircle(a, b, segments = 64) {
  const start = [a[0], a[1]];
  if (a[0] === b[0] && a[1] === b[1]) {
    return [start, [b[0], b[1]]];
  }
  const va = toVector(a);
  const vb = toVector(b);
  const cos = Math.min(1, Math.max(-1, va[0] * vb[0] + va[1] * vb[1] + va[2] * vb[2]));
  const d = Math.acos(cos);
  const sinD = Math.sin(d);
  if (sinD < 1e-12) {
    return [start, [b[0], b[1]]];
  }

  const points = [start];
  for (let i = 1; i <= segments; i++) {
    const f = i / segments;
    const ka = Math.sin((1 - f) * d) / sinD;
    const kb = Math.sin(f * d) / sinD;
    const x = ka * va[0] + kb * vb[0];
    const y = ka * va[1] + kb * vb[1];
    const z = ka * va[2] + kb * vb[2];
    const lat = Math.atan2(z, Math.hypot(x, y)) / DEG;
    let lon = Math.atan2(y, x) / DEG;
    const prev = points[points.length - 1][1];
    while (lon - prev > 180) lon -= 360;
    while (lon - prev < -180) lon += 360;
    points.push([lat, lon]);
  }
  return points;
}

/**
 * The point `distanceDeg` along the great circle from `[lat, lon]` at `bearingDeg` (clockwise
 * from north), as `[lat, lon]` (feature 013, research R7: the grayline's caps).
 */
export function destination([lat, lon], bearingDeg, distanceDeg) {
  const φ1 = lat * DEG;
  const θ = bearingDeg * DEG;
  const δ = distanceDeg * DEG;
  const sinφ2 = Math.sin(φ1) * Math.cos(δ) + Math.cos(φ1) * Math.sin(δ) * Math.cos(θ);
  const φ2 = Math.asin(Math.min(1, Math.max(-1, sinφ2)));
  const λ2 = lon * DEG + Math.atan2(Math.sin(θ) * Math.sin(δ) * Math.cos(φ1), Math.cos(δ) - Math.sin(φ1) * sinφ2);
  return [φ2 / DEG, λ2 / DEG];
}
