// The sun's elevation at a place (feature 011, research R3), for the "sun at station" theme,
// and the subsolar point (feature 013) for the globe's glow and the grayline.
// NOAA Solar Calculator equations, in UTC (HR-1), without atmospheric refraction: accurate to
// about half a degree, enough for the 0° and −6° thresholds. Pure.

const RAD = Math.PI / 180;

/** The solar declination (degrees) and the equation of time (minutes) at `date`. */
function solarPosition(date) {
  const julianDay = date.getTime() / 86_400_000 + 2_440_587.5;
  const t = (julianDay - 2_451_545) / 36_525; // Julian centuries since J2000.0

  const meanLongitude = (280.46646 + t * (36_000.76983 + t * 0.0003032)) % 360;
  const meanAnomaly = 357.52911 + t * (35_999.05029 - 0.0001537 * t);
  const eccentricity = 0.016708634 - t * (0.000042037 + 0.0000001267 * t);
  const center =
    Math.sin(meanAnomaly * RAD) * (1.914602 - t * (0.004817 + 0.000014 * t)) +
    Math.sin(2 * meanAnomaly * RAD) * (0.019993 - 0.000101 * t) +
    Math.sin(3 * meanAnomaly * RAD) * 0.000289;
  const trueLongitude = meanLongitude + center;
  const omega = 125.04 - 1934.136 * t;
  const apparentLongitude = trueLongitude - 0.00569 - 0.00478 * Math.sin(omega * RAD);
  const meanObliquity = 23 + (26 + (21.448 - t * (46.815 + t * (0.00059 - t * 0.001813))) / 60) / 60;
  const obliquity = meanObliquity + 0.00256 * Math.cos(omega * RAD);
  const declination = Math.asin(Math.sin(obliquity * RAD) * Math.sin(apparentLongitude * RAD)) / RAD;

  // Equation of time, in minutes.
  const y = Math.tan((obliquity / 2) * RAD) ** 2;
  const equationOfTime =
    4 *
    (y * Math.sin(2 * meanLongitude * RAD) -
      2 * eccentricity * Math.sin(meanAnomaly * RAD) +
      4 * eccentricity * y * Math.sin(meanAnomaly * RAD) * Math.cos(2 * meanLongitude * RAD) -
      0.5 * y * y * Math.sin(4 * meanLongitude * RAD) -
      1.25 * eccentricity * eccentricity * Math.sin(2 * meanAnomaly * RAD)) /
    RAD;

  return { declination, equationOfTime };
}

function minutesUtc(date) {
  return date.getUTCHours() * 60 + date.getUTCMinutes() + date.getUTCSeconds() / 60;
}

/** Degrees above (positive) or below (negative) the horizon, at `date`, for `lat`/`lon`. */
export function solarElevation(date, lat, lon) {
  const { declination, equationOfTime } = solarPosition(date);
  const trueSolarTime = (((minutesUtc(date) + equationOfTime + 4 * lon) % 1440) + 1440) % 1440;
  const hourAngle = trueSolarTime / 4 < 0 ? trueSolarTime / 4 + 180 : trueSolarTime / 4 - 180;

  const cosZenith =
    Math.sin(lat * RAD) * Math.sin(declination * RAD) +
    Math.cos(lat * RAD) * Math.cos(declination * RAD) * Math.cos(hourAngle * RAD);
  const zenith = Math.acos(Math.min(1, Math.max(-1, cosZenith))) / RAD;
  return 90 - zenith;
}

/**
 * The point where the Sun is overhead at `date` (UTC), `{ lat, lon }` in degrees, `lon` in
 * [−180, 180) (feature 013, research R3). Same equations as `solarElevation`, so the globe's
 * glow, the grayline and the sun-based theme agree.
 */
export function subsolarPoint(date) {
  const { declination, equationOfTime } = solarPosition(date);
  const lon = -(minutesUtc(date) + equationOfTime - 720) / 4;
  return { lat: declination, lon: ((((lon + 180) % 360) + 360) % 360) - 180 };
}
