import unittest

from shipping import shipping_cost_cents


class ShippingCostTests(unittest.TestCase):
    def test_return_type(self):
        self.assertIsInstance(shipping_cost_cents("KR", 1000), int)

    def test_negative_subtotal(self):
        with self.assertRaises(ValueError):
            shipping_cost_cents("KR", -1)

    def test_region_rates_below_threshold(self):
        for country, expected in (
            ("KR", 400),
            ("FR", 700),
            ("DE", 700),
            ("ES", 700),
            ("US", 900),
        ):
            with self.subTest(country=country):
                self.assertEqual(shipping_cost_cents(country, 5999), expected)

    def test_free_shipping_at_and_above_threshold(self):
        for subtotal in (6000, 6001):
            for loyal in (False, True):
                with self.subTest(subtotal=subtotal, loyal=loyal):
                    self.assertEqual(shipping_cost_cents("US", subtotal, loyal), 0)

    def test_loyalty_discount_applies_to_regional_rate(self):
        for country, expected in (("KR", 200), ("FR", 500), ("US", 700)):
            with self.subTest(country=country):
                self.assertEqual(shipping_cost_cents(country, 0, loyal=True), expected)


if __name__ == "__main__":
    unittest.main()
