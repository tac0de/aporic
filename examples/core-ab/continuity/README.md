# Cross-session continuity pilot

Two fresh `gpt-6-sol` medium agents started from shipping fixture commit
`30da00fec21cf4ff241a704377fdc75192663e46`. Both were asked to continue
an interrupted revision. The baseline read an ordinary local `HANDOFF.md`;
the treatment used isolated Aporic `resume`, `open`, and `task_list` calls.
The handoff and Aporic records contained the same shipping decisions, including
an obsolete 5,000-cent threshold superseded by 6,000 cents and an irrelevant
banner-color note. Neither agent requested user clarification.

| Condition | Hidden checks | Current threshold | Agent-reported UTC elapsed |
| --- | ---: | --- | ---: |
| [Without Aporic](runs/baseline/shipping.py) | [4/4](runs/baseline/hidden-result.txt) | 6,000 cents | 65 seconds |
| [With Aporic](runs/treatment/shipping.py) | [4/4](runs/treatment/hidden-result.txt) | 6,000 cents | 83 seconds |

Both passed five visible `unittest` checks as reported by the agents. Their
code was evaluated unchanged with the same [hidden suite](evaluator/hidden_test_shipping.py).
The treatment took 18 seconds longer by agent-observed clock times. There is
no correctness difference in this pair and no basis for an average speed claim.
Per-run token and tool-call counts were unavailable.

The [fixture](fixture/) and [operator seed scripts](operator/) preserve the
scenario. The scripts contain local paths from this run and need path changes
for a new machine; they do not represent product logic. Aporic's isolated
SQLite test database remains outside Git.
