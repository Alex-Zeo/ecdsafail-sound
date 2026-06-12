# ecdsa.fail-sound — leaderboard (honest secp256k1 point-add resource cost)

Score = **peak_qubits × avg_executed_Toffoli**, lower is better. A circuit is **valid** only if it
passes `eval_circuit` (0 classical / 0 phase / 0 ancilla) under the grader-controlled `sound_seed`
across independent seeds — no `DIALOG_TAIL_NONCE` can steer the test inputs. All rows below are
**independently re-verified by the orchestrator on fresh OS-random seeds (H1), 2026-06-11** — scores
use the grader-printed avg-executed-Toffoli.

| # | circuit | peak qubits | avg Toffoli | score | sound? | notes |
|---|---|---|---|---|---|---|
| — | Google (low-qubit, ZKP-validated) | **1175** | — (full-Shor) | — | ✅ | reference frontier, arXiv 2603.28846 |
| — | Google (low-gate) | 1425 | — | — | ✅ | reference frontier |
| **1** | **measured SET-carry cmp (SOUND-OPT-2) — SOTA** | **2039** | **3,398,114** | **6,928,754,446** | ✅ 0/0/0 × 8 fresh seeds | `DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured`; removes the peak-binding 256-q `load_const`; value-correct (selftest) + sound |
| 2 | value-exact baseline | 2292 | 3,294,353 | 7,550,657,076 | ✅ 0/0/0 × ~35 fresh seeds | PROD65 + SQ_CARRY_HOST + wide variable-width |
| 3 | naive correct (full-width, no levers) | 2292 | ~3,847,387 | ~8.82e9 | ✅ | the value-exact levers cut Toffoli ~14% at no peak cost |
| ✗ | race-1217 ("frontier", nonce-gamed) | 1217 | 1,401,748 | ~1.71e9 | ❌ **INVALID** | 17–22 classical + 10–13 phase mismatches under independent seeds |

> **Verification note (2026-06-11).** An intermediate analysis (SOUND-OPT-2) flagged a phase hazard
> on the 2292 baseline and downgraded it. An independent re-check — baseline build `0146edd` vs the
> opt-2 build, **8 fresh OS-random seeds each, plus 6 `GRADER_SEED` values** — found **0 phase
> failures in every case**. The flag was a measurement artifact of a fixed-seed regime, not a real
> hazard; the same analysis's avg-Toffoli figures were also off. Both the baseline (2292) and the
> SOTA (2039) are sound. The 2039 circuit specifically passes 0/0/0 on 8 independent fresh seeds.

## Reading this
- **The gamed leaderboard's sub-1300q numbers are not real circuits.** race-1217 fails the sound
  validator outright (17–22 wrong outputs per random sample); its low score only existed because it
  chose its own exam.
- **The honest cost of this codebase is now 2039 qubits** — about **1.74× Google's correct 1175-qubit
  circuit** (the qubit axis is directly comparable; Toffoli is benchmark-internal, per-point-add).
- **The reductions are genuine.** PROD65, the square carry-host, the wide variable-width reduction,
  and the measured SET-carry underflow comparator are all coherent, value-correct, sound-validated.

## Open frontier (toward Google's 1175)
1. **Qubits.** The 2039 peak is now bound by the apply add/sub carry tier. Borrow its carry lanes
   from the ~131-deep free pool / idle GCD future-log (`cuccaro_add_fast_borrowed_carries`,
   `cmp_lt_into_fast_with_cin_borrowed_carries` already exist) to allocate zero new peak qubits.
2. **Toffoli.** Extend the value-exact lever family (more coherent register reductions) at 2039.

Reproduce: `cargo build --release && <config> ./target/release/build_circuit && ./target/release/eval_circuit`
(fresh grader seed each run; or `GRADER_SEED=<hex>` for reproducibility). Canonical config: SOUND-BASELINE.md §3
plus `DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured` for the SOTA.
