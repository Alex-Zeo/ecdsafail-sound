# SOUND-OPT-4 — windowed comparator carry array: apply peak 2034/2039 → 1926

Branch: `panel-Window-the-comparator-s-` (base `descend-B` @ `a05c649`, the 2034 SOTA).
Lever: **window the comparator's internal carry array** (B-block split of the
apply-phase underflow-correction comparator's Cuccaro add/sub *and* its SET-carry
const add/sub), behind the NEW knob `DIALOG_GCD_APPLY_CMP_WINDOW_BLOCKS`.

## TL;DR

- **Peak drops 2034 (descend-B SOTA) / 2039 (opt-2) → 1926 (−108 / −113 q)** at B=2.
  The apply `..._underflow_clean` comparator collapses BELOW the `..._underflow_fold`
  /`..._overflow_fold` floor (1926), which becomes the new binder (TRACE_PEAK
  confirms the peak phase moves off `..._underflow_clean`). This is exactly the
  brief's predicted collapse onto the fold tier.
- **Value-exact and phase-clean, PROVEN drop-in.** The new comparator
  `cmp_acc_plus_f_ge_p_measured_windowed` (`arith/compare.rs`) is value- AND
  global-phase-IDENTICAL to the non-windowed `cmp_acc_plus_f_ge_p_measured` —
  asserted by `CMP_ACC_PLUS_F_MEASURED_WINDOWED_SELFTEST` over B ∈ {2,3,4,5} ×
  8 independent Hmr seeds × 2 widths (NB=7/p=101 incl. the (35,68) XOR-injection
  counterexample; NB=24 with a sparse secp-like constant + long-carry adversarial
  cases that force the carry across every block boundary up to bit n).
- **HONEST score caveat.** The leaderboard score is **peak_qubits × avg_Toffoli**.
  The four boundary reconstructions cost **+6.06% Toffoli** (3,398,085 → 3,603,922),
  more than the brief's +0.2–1% estimate. Net q×T score **6.912e9 (SOTA) → 6.941e9
  (+0.43% REGRESSION)**. So this is a strict **peak-axis** win (1926, the lowest
  honest peak in this codebase) but a marginal **q×T-product** loss — a space/gate
  tradeoff at the now-exposed 1926 fold floor. B=2 is the optimum window (B=3 → 1955
  peak, more Toffoli; the peak goes back onto `..._underflow_clean`).

## 1. Mechanism (reused, proven primitive + a new const analog)

The comparator materializes `acc + f + c` (`c = 2^n − p`) in an (n+1)-wide register
and reads bit n. Its peak binder is a SINGLE 256-wide live carry array — but there are
FOUR sequential such arrays, each reaching 2034/2039 (TRACE confirmed all four hit the
peak): Cuccaro `+f`, SET-carry const `+c`, const `−c`, Cuccaro `−f`. To lower the peak
ALL FOUR must be windowed (windowing a subset leaves the peak pinned by the rest).

- **Cuccaro `±f`**: reuse the EXISTING, proven `cuccaro_{add,sub}_fast_windowed_
  low_to_ext` (`adder.rs:854/905`, already trusted on the apply `raw_difference`,
  `mod.rs:1894`). B blocks keep ~(n+1)/B carry lanes live + (B−1) boundary couts,
  reconstructed by the MEASURED `cmp_lt_into_fast_with_cin`.
- **Const `±c`**: NEW windowed SET-carry primitives
  `{add,sub}_nbit_const_direct_uncontrolled_fast_windowed` (`const_arith.rs`). Same
  B-block structure; per-block constant masked to the block's data width
  (`mask_low_bits`, so a set const bit never lands on the appended cout lane); blocks
  add `c[lo..hi] + carry_in` via `c{add,sub}_nbit_const_direct_fast_with_cin` (the
  base const-add with its SET-carry chain seeded by the boundary carry-in qubit);
  boundary carries recomputed-and-cleared in reverse by a MEASURED const carry/borrow
  comparison on the post-sum register.

Boundary identities (exact, value-proven by the selftest):
- const-ADD boundary clean: `g = (s < c[..p] + cin)` borrow of the post-sum `s`
  (`cmp_const_borrow_into_fast_with_cin`).
- const-SUB boundary clean: `g = (s + c[..p] + bin ≥ 2^p)` carry of the post-diff `s`
  (`cmp_const_carry_into_fast_with_cin`).

## 2. Phase-cleanliness (the subtle part)

Each boundary clean is a MEASURED (Hmr/cz_if) sweep, identical in structure to the
comparator's own uncompute, so per-carry-pair Hmr phase cancellation is preserved.
The two const compare-only cleans uncompute their carry/borrow lanes with the SAME
acc-negation polarity as their forward majority (carry-compare: acc DIRECT; borrow-
compare: acc NEGATED) — a compare leaves acc unmodified, so unlike the const-ADD's
post-sum uncompute it must NOT inherit the sum-bit negation. Getting this wrong shows
up as global-phase garbage in isolation (it did, mid-development; the selftest caught
it before any grader run). Changing B changes the Hmr draw count/order → re-run the
isolated selftest AND the K-seed grader at each B (done; see below).

## 3. Value selftest — PASS (before the grader)

```
CMP_ACC_PLUS_F_MEASURED_WINDOWED_SELFTEST=1 CMP_ACC_PLUS_F_MEASURED_WINDOWED_SELFTEST_ONLY=1 \
  SKIP_ALT_SEED_CHECKS=1 ./target/release/build_circuit
CMP_ACC_PLUS_F_MEASURED_WINDOWED_SELFTEST: PASS (value-exact, acc/f restored, phase 0,
  matches non-windowed over B in {2,3,4,5} x 8 seeds)
```

## 4. Build-time peak — TRACE_PEAK (canonical config)

| config | knobs | build peak | peak phase | avg Toffoli | score |
|---|---|---|---|---|---|
| opt-2 cmp | `…CMP=acc_plus_f_measured` | 2039 | `…_underflow_clean` | 3,398,101 | 6.929e9 |
| descend-B SOTA | `…CMP=acc_plus_f_measured_borrowed` | 2034 | `…_underflow_clean` | 3,398,085 | 6.912e9 |
| **opt-4 windowed B=2** | `…CMP=acc_plus_f_measured` `+APPLY_CMP_WINDOW_BLOCKS=2` | **1926** | **`…_underflow_fold`** | 3,603,922 | 6.941e9 |
| opt-4 windowed B=3 | …`=3` | 1955 | `…_underflow_clean` | (higher) | (worse) |

## 5. Sound-grader spot-checks (NOT a soundness proof — verifier grades fresh seeds)

B=2 windowed under the canonical config grades **0 classical / 0 phase / 0 ancilla**
on a fresh OS-random seed and on fixed `GRADER_SEED` 1111/3333/9999/abab (including the
3333/9999/abab seeds SOUND-OPT-2 flagged on a different config). Because the windowed
comparator is value-/phase-IDENTICAL to `acc_plus_f_measured` (selftest equivalence),
it inherits exactly that lever's K-seed robustness — no better, no worse. It does NOT
touch the upstream `clear_subtrahend` measured-clear (the K-seed blocker per opt-2 §5).

## 6. Did the SOTA move?

- **Peak axis: YES, 2034 → 1926** — the lowest honest peak in this codebase, value-exact
  and phase-clean, 1.64× Google's 1175 (was 1.74×). The brief's target (collapse the
  comparator onto the 1911/1926 fold tier) is met at 1926.
- **q×T-product score: NO (marginal regression +0.43%).** The boundary-reconstruction
  Toffoli (+6%) slightly outweighs the −108 q under the product metric. Whether opt-4
  is "the new #1" depends on which axis the leaderboard ranks: peak (yes) vs q×T (no).

## Reproduce

```bash
cargo build --release --bin build_circuit --bin eval_circuit
CMP_ACC_PLUS_F_MEASURED_WINDOWED_SELFTEST=1 CMP_ACC_PLUS_F_MEASURED_WINDOWED_SELFTEST_ONLY=1 \
  SKIP_ALT_SEED_CHECKS=1 ./target/release/build_circuit      # PASS
source ./canon_env.sh
DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured DIALOG_GCD_APPLY_CMP_WINDOW_BLOCKS=2 \
  TRACE_PEAK=1 ./target/release/build_circuit | grep peak_qubits   # 1926
./target/release/eval_circuit                                # 0/0/0
```
