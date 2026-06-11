# ecdsa.fail-sound — leaderboard (honest secp256k1 point-add resource cost)

Score = **peak_qubits × avg_executed_Toffoli**, lower is better. A circuit is **valid** only if it
passes `eval_circuit` (0 classical / 0 phase / 0 ancilla) under the **grader-controlled `sound_seed`**
across independent seeds — no `DIALOG_TAIL_NONCE` can steer the test inputs. Verified 2026-06-11.

| # | circuit | peak qubits | avg Toffoli | score | sound? | notes |
|---|---|---|---|---|---|---|
| — | Google (low-qubit, ZKP-validated) | **1175** | — (full-Shor) | — | ✅ | reference frontier, arXiv 2603.28846 |
| — | Google (low-gate) | 1425 | — | — | ✅ | reference frontier |
| **1** | **value-exact (this work) — SOTA** | **2292** | **3,294,335** | **7,550,615,820** | ✅ 0/0/0 × independent seeds | PROD65 + SQ_CARRY_HOST + wide variable-width |
| 2 | naive correct (full-width, no levers) | 2292 | 3,847,387 | 8,820,210,000 | ✅ | levers cut Toffoli −14.4% at no peak cost |
| ✗ | race-1217 ("frontier", nonce-gamed) | 1217 | 1,401,748 | ~1.71e9 | ❌ **INVALID** | 17–22 classical + 10–13 phase mismatches under independent seeds |

## Reading this

- **The gamed leaderboard's sub-1300q numbers are not real circuits.** race-1217 fails the sound
  validator outright (17–22 wrong outputs per random sample). Its low score only existed because it
  chose its own exam.
- **The honest cost of this codebase is ~2292 qubits** — about **2× Google's correct 1175-qubit
  circuit** (the qubit axis is directly comparable; Toffoli is benchmark-internal, per-point-add).
- **The value-exact levers are genuine.** PROD65, the square carry-host, and the wide variable-width
  reduction are coherent, ancilla-clean register reductions; they cut the *correct* circuit's
  Toffoli by 14.4% (3.85M → 3.29M) with no peak change. They earn their place on a sound board.

## Open frontier

Beat the SOTA with a **correct** circuit:
1. **Qubits** — drive 2292 toward Google's 1175. The sound peak is bound by the exact apply
   comparator (`dialog_gcd_materialized_special_underflow_clean`); a coherent (non-truncating)
   redesign of that comparator is the highest-leverage target.
2. **Toffoli** — extend the value-exact lever family (more coherent register reductions) at 2292q.

Submit: `cargo build --release && <config> ./target/release/build_circuit && ./target/release/eval_circuit`
(run several times — each uses a fresh grader seed — or with a published `GRADER_SEED` set).
