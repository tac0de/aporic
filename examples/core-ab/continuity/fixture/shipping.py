"""Shipping calculation for checkout totals in integer cents."""


def shipping_cost_cents(country: str, subtotal_cents: int, loyal: bool = False) -> int:
    """Return the shipping charge for one order."""
    if subtotal_cents < 0:
        raise ValueError("subtotal must be non-negative")
    return 500
