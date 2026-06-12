# ecdsa.fail-sound — leaderboard (honest secp256k1 point-add resource cost)

Score = **peak_qubits × avg_executed_Toffoli**, lower is better. A circuit is **valid** only if it
passes `eval_circuit` (0 classical / 0 phase / 0 ancilla) under the **grader-controlled `sound_seed`**
across independent seeds — no `DIALOG_TAIL_NONCE` can steer the test inputs. Verified 2026-06-11.
SOUND-OPT-2 (2026-06-11) corrects the score basis to score.json (avg-T × peak-q, both grader-measured).

| # | circuit | peak qubits | avg Toffoli | score | sound? | notes |
|---|---|---|---|---|---|---|
| — | Google (low-qubit, ZKP-validated) | **1175** | — (full-Shor) | — | ✅ | reference frontier, arXiv 2603.28846 |
| — | Google (low-gate) | 1425 | — | — | ✅ | reference frontier |
| **1** | **measured SET-carry cmp (SOUND-OPT-2) — peak SOTA** | **2039** | **2,773,440** | **5,655,044,160** | ⚠ 0 classical / 0 ancilla × all seeds; phase-clean ~½ of seeds (SAME as baseline #2) | `DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured`; removes the 256-q `load_const`; value-exact (selftest + grader); see SOUND-OPT-2.md |
| 2 | value-exact baseline (SOUND-OPT-1 #1) | 2292 | 2,669,713 | 6,118,982,196 | ⚠ 0 classical / 0 ancilla × all seeds; phase-clean ~6/10 seeds (NOT 0/0/0-robust) | PROD65 + SQ_CARRY_HOST + wide variable-width |
| 3 | naive correct (full-width, no levers) | 2292 | 3,847,387 | 8,820,210,000 | ⚠ as #2 | levers cut Toffoli −14.4% at no peak cost |
| ✗ | race-1217 ("frontier", nonce-gamed) | 1217 | 1,401,748 | ~1.71e9 | ❌ **INVALID** | 17–22 classical + 10–13 phase mismatches under independent seeds |

> **Honest caveat (corrected in SOUND-OPT-2).** The earlier "✅ 0/0/0 × independent seeds" tag on the
> 2292 baseline was over-stated (SOUND-OPT-1 flagged it; SOUND-OPT-2 quantified it): in a controlled
> 10-seed head-to-head the **2292 baseline itself fails the phase channel on ~4/10 seeds** (0 classical /
> 0 ancilla always). The 2039 SOUND-OPT-2 circuit is **value-exact (0 classical / 0 ancilla on every
> seed)** and phase-clean on the same-order seed set — a strict peak/score improvement over the baseline —
> but **neither circuit is yet 0/0/0-robust over K≥6 seeds.** The blocker is a *shared, upstream*
> `clear_subtrahend` 256-wide measured-clear phase fragility (SOUND-OPT-2 §5), orthogonal to the
> comparator. No circuit here is "fully valid" by the strict bar; #1 is the honest **peak/score SOTA**
> among them.

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
1. **Close the shared phase fragility** — the `clear_subtrahend` 256-wide measured-clear of `f`
   (SOUND-OPT-2 §5) leaves a single uncancelled global-phase bit on ~½ of grader seeds, for BOTH
   the 2039 and 2292 circuits. A phase-deterministic (or unitary) `f` clear converts the 2039 peak
   from "phase-clean on ~½ of seeds" into a genuine **0/0/0-over-K SOTA**. Highest leverage.
2. **Qubits** — drive 2039 toward Google's 1175. The peak is now the apply add/sub carry tier;
   borrow the carry lanes from the 131-deep free pool / idle GCD future-log
   (`cuccaro_add_fast_borrowed_carries`, `cmp_lt_into_fast_with_cin_borrowed_carries` exist) so the
   comparator/add allocate zero new peak qubits.
3. **Toffoli** — extend the value-exact lever family (more coherent register reductions).

Submit: `cargo build --release && <config> ./target/release/build_circuit && ./target/release/eval_circuit`
(run several times — each uses a fresh grader seed — or with a published `GRADER_SEED` set).
