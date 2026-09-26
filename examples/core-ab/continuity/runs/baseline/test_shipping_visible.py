import unittest

from shipping import shipping_cost_cents


class ShippingCostTests(unittest.TestCase):
    def test_return_type(self):
        self.assertIsInstance(shipping_cost_cents("KR", 1000), int)

    def test_negative_subtotal(self):
        with self.assertRaises(ValueError):
            shipping_cost_cents("KR", -1)

    def test_free_shipping_uses_subtotal_before_shipping(self):
        self.assertEqual(shipping_cost_cents("KR", 5999), 400)
        self.assertEqual(shipping_cost_cents("KR", 6000), 0)
        self.assertEqual(shipping_cost_cents("US", 6000, loyal=True), 0)

    def test_regional_rates_below_threshold(self):
        self.assertEqual(shipping_cost_cents("KR", 0), 400)
        for country in ("FR", "DE", "ES"):
            with self.subTest(country=country):
                self.assertEqual(shipping_cost_cents(country, 1000), 700)
        self.assertEqual(shipping_cost_cents("US", 1000), 900)

    def test_loyalty_discount_applies_to_shipping_charge(self):
        self.assertEqual(shipping_cost_cents("KR", 1000, loyal=True), 200)
        self.assertEqual(shipping_cost_cents("FR", 1000, loyal=True), 500)
        self.assertEqual(shipping_cost_cents("US", 1000, loyal=True), 700)


if __name__ == "__main__":
    unittest.main()
