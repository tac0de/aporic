import importlib.util
import sys
from pathlib import Path
import unittest

module_path = Path(sys.argv[1]).resolve()
spec = importlib.util.spec_from_file_location("shipping_candidate", module_path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
f = module.shipping_cost_cents

class ShippingContract(unittest.TestCase):
    def test_rates(self):
        self.assertEqual([f(code, 1000) for code in ["KR", "FR", "DE", "ES", "US"]], [400,700,700,700,900])
    def test_superseded_threshold(self):
        self.assertEqual(f("KR", 5000), 400)
        self.assertEqual(f("US", 5999), 900)
        self.assertEqual(f("US", 6000), 0)
    def test_loyalty(self):
        self.assertEqual(f("KR", 1000, True), 200)
        self.assertEqual(f("US", 1000, True), 700)
        self.assertEqual(f("DE", 6000, True), 0)
    def test_negative(self):
        with self.assertRaises(ValueError): f("KR", -1)

if __name__ == "__main__": unittest.main(argv=[sys.argv[0]])
