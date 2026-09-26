from shipping import shipping_cost_cents


def test_return_type():
    assert isinstance(shipping_cost_cents("KR", 1000), int)


def test_negative_subtotal():
    try:
        shipping_cost_cents("KR", -1)
    except ValueError:
        pass
    else:
        raise AssertionError("negative subtotal must fail")
