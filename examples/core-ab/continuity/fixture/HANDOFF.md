# Interrupted shipping revision

The prior worker was asked to replace the fixed shipping charge in `shipping.py`. Decisions, in chronological order:

- Initial draft: free shipping at 5,000 cents. This was later superseded.
- Current decision: free shipping at 6,000 cents, using the order subtotal before shipping.
- Current regional rates below that threshold: KR 400 cents; FR, DE and ES 700 cents; all other country codes 900 cents.
- Current loyalty rule: subtract 200 cents from the shipping charge after choosing the regional rate, never below zero. Free shipping stays zero.
- Negative subtotals raise `ValueError`; all money uses integer cents.
- Unrelated design note: the checkout banner color should eventually be navy.

Finish the revision, update or add focused checks, and report what was verified.
