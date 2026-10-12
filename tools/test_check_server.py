import unittest

from check_server import objects_with_defaults


class MapComparisonTests(unittest.TestCase):
    def test_tiled_object_defaults_preserve_authored_values(self):
        source = {"id": 1, "name": "tree", "type": "block", "x": 24, "y": 208,
                  "point": True, "visible": False, "width": 8, "custom": "keep"}
        expected = {**source, "height": 0, "rotation": 0, "properties": []}
        normalized = objects_with_defaults([source])
        self.assertEqual(normalized, [expected])
        self.assertNotIn("height", source)
        self.assertNotEqual(normalized, [{**expected, "x": 25}])
        self.assertNotEqual(normalized, [{**expected, "custom": "changed"}])
