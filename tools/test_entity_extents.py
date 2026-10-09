"""Tests for the dev-only entity extents generator (feature 004, T016).

Run with: python3 -m unittest tools/test_entity_extents.py
"""

import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import entity_extents as ee  # noqa: E402


def feature(props, coords):
    return {"type": "Feature", "properties": props, "geometry": {"type": "Polygon", "coordinates": [coords]}}


def square(lon, lat, size=1.0):
    return [[lon, lat], [lon + size, lat], [lon + size, lat + size], [lon, lat + size], [lon, lat]]


SUBUNITS = [
    feature({"SU_A3": "ITA"}, square(10, 42)),
    feature({"SU_A3": "FJI"}, [[177, -18], [179.9, -18], [179.9, -17], [177, -17], [177, -18]]),
    feature({"SU_A3": "FJE"}, [[-180, -18], [-178.5, -18], [-178.5, -17], [-180, -17], [-180, -18]]),
]
ADMIN1 = [
    feature({"iso_3166_2": "IT-88", "adm0_a3": "ITA"}, square(8.2, 39, 1.5)),
    feature({"iso_3166_2": "IT-82", "adm0_a3": "ITA"}, square(12.5, 36.7, 2.5)),
    feature({"iso_3166_2": "IT-62", "adm0_a3": "ITA"}, square(12, 41.5, 1)),
    feature({"iso_3166_2": "IT-25", "adm0_a3": "ITA"}, square(9, 45, 1.5)),
]


class MinLonSpan(unittest.TestCase):
    def test_ordinary_span(self):
        self.assertEqual(ee.min_lon_span([10.0, 12.0, 11.0]), (10.0, 12.0))

    def test_span_across_the_antimeridian(self):
        w, e = ee.min_lon_span([177.0, 179.9, -180.0, -178.5])
        self.assertGreater(w, e)
        self.assertEqual((w, e), (177.0, -178.5))

    def test_single_longitude(self):
        self.assertEqual(ee.min_lon_span([5.0]), (5.0, 5.0))


class Select(unittest.TestCase):
    def test_admin1(self):
        self.assertEqual(ee.extent(ee.select(SUBUNITS, ADMIN1, {"admin1": ["IT-88"]})), [8.2, 39.0, 9.7, 40.5])

    def test_admin0(self):
        self.assertEqual(ee.extent(ee.select(SUBUNITS, ADMIN1, {"admin0": ["ITA"]})), [10.0, 42.0, 11.0, 43.0])

    def test_admin0_across_the_antimeridian(self):
        box = ee.extent(ee.select(SUBUNITS, ADMIN1, {"admin0": ["FJI", "FJE"]}))
        self.assertEqual(box, [177.0, -18.0, -178.5, -17.0])

    def test_exclude_admin1_uses_the_remaining_units(self):
        sel = {"admin1_of": ["ITA"], "exclude_admin1": ["IT-88", "IT-82"]}
        self.assertEqual(ee.extent(ee.select(SUBUNITS, ADMIN1, sel)), [9.0, 41.5, 13.0, 46.5])

    def test_clip(self):
        sel = {"admin1_of": ["ITA"], "clip": [8.0, 41.0, 11.0, 47.0]}
        self.assertEqual(ee.extent(ee.select(SUBUNITS, ADMIN1, sel)), [9.0, 45.0, 10.5, 46.5])

    def test_unknown_selector_key_is_an_error(self):
        with self.assertRaises(ValueError):
            ee.select(SUBUNITS, ADMIN1, {"admin2": ["X"]})


class Output(unittest.TestCase):
    def test_deterministic_json(self):
        a = ee.render({"I": [6.6266, 37.9, 18.52, 47.09], "IS": [8.13, 38.86, 9.83, 41.31]})
        b = ee.render({"IS": [8.13, 38.86, 9.83, 41.31], "I": [6.6266, 37.9, 18.52, 47.09]})
        self.assertEqual(a, b)
        doc = json.loads(a)
        self.assertEqual(list(doc["extents"]), ["I", "IS"])
        self.assertEqual(doc["extents"]["I"], [6.62, 37.9, 18.52, 47.09])
        self.assertNotIn("generated", doc)
        self.assertIn("Natural Earth", doc["source"])


class Outward(unittest.TestCase):
    def test_rounding_never_shrinks_the_box(self):
        self.assertEqual(ee.outward([6.6266, 37.901, 18.5201, 47.0899]), [6.62, 37.9, 18.53, 47.09])

    def test_a_point_keeps_south_below_north(self):
        w, s, e, n = ee.outward([12.4528, 41.9002, 12.4528, 41.9002])
        self.assertLess(s, n)
        self.assertLessEqual(w, 12.4528)
        self.assertGreaterEqual(e, 12.4528)


class Coverage(unittest.TestCase):
    def test_every_entity_is_mapped_or_listed(self):
        ee.check_coverage(["I", "IS"], {"I": {}, "IS": {}}, {})
        ee.check_coverage(["I", "IS"], {"I": {}}, {"IS": "reason"})

    def test_missing_entity_is_named(self):
        with self.assertRaises(ValueError) as ctx:
            ee.check_coverage(["I", "IS", "KH6"], {"I": {}}, {"IS": "reason"})
        self.assertIn("KH6", str(ctx.exception))

    def test_cty_primary_prefixes(self):
        text = "Sardinia:  15:  28:  EU:   40.15:    -9.27:    -1.0:  IS:\n    IS,IM0;\nSicily: 15: 28: EU: 37.50: -14.00: -1.0: *IT9:\n    IT9;\n"
        self.assertEqual(ee.cty_prefixes(text), ["IS", "IT9"])


if __name__ == "__main__":
    unittest.main()
