"""Shipping calculation for checkout totals in integer cents."""


def shipping_cost_cents(country: str, subtotal_cents: int, loyal: bool = False) -> int:
    """Return the shipping charge for one order."""
    if subtotal_cents < 0:
        raise ValueError("subtotal must be non-negative")
    if subtotal_cents >= 6000:
        return 0

    if country == "KR":
        rate = 400
    elif country in {"FR", "DE", "ES"}:
        rate = 700
    else:
        rate = 900

    return max(0, rate - (200 if loyal else 0))
