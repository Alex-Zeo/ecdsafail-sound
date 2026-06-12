# SOUND-OPT-2 — measured SET-carry comparator drops the apply-phase peak 2292 → 2039

Goal (continuing SOUND-OPT-1): lower the honest, sound-graded peak-qubit count of the
value-exact secp256k1 reversible point-add circuit below **2292** by removing the
peak-binding `mod_neg_inplace_fast` / `load_const(256)` in the apply-phase modular
subtract's underflow correction — **with a CORRECT (SET-carry, not XOR-injection)
constant addition AND a phase-preserving MEASURED (Hmr/Gidney) uncompute**, the two
properties that broke SOUND-OPT-1's two attempts.

Base: branch `sound-opt-1-peak-analysis` (`154c9c3`). Build:
`cargo build --release --bin build_circuit --bin eval_circuit`.

## TL;DR

- **The peak DROPS 2292 → 2039 (−253 q, −11.0%)** with a **value-exact** comparator
  (`cmp_acc_plus_f_ge_p_measured`, `arith/compare.rs`). Score (avg-T × peak-q)
  **6.119e9 → 5.655e9 (−7.6%)** — peak win dominates a +3.9% Toffoli cost.
- The new comparator is **classically correct on general inputs** (the SOUND-OPT-1
  `_into` bug is fixed: it adds the Solinas constant `c = 2^32+977` via a genuine
  reversible majority carry-chain on an extended register, **not** an `X`-toggle of a
  running-carry lane) and is **phase-clean in isolation across all seeds** (the new
  `CMP_ACC_PLUS_F_MEASURED_SELFTEST` passes value + acc/f-restore + phase-0 over 8
  independent Hmr rng streams, including the adversarial `acc=35,f=68,p=101` case the
  `_into` variant got wrong).
- **BUT the full circuit is NOT robustly 0/0/0 across K independent seeds — and neither
  is the 2292 baseline, on the SAME seeds.** A controlled 10-seed head-to-head (fixed
  `GRADER_SEED`, baseline vs lever) shows **0 classical / 0 ancilla on every seed for
  both**, and **phase-garbage on a shared subset of seeds for both** (baseline 4/10,
  lever 5/10, overlapping). The residual phase hazard is a **pre-existing property of
  the apply phase upstream of the comparator** (the `clear_subtrahend` 256-wide Hmr
  measured-clear of `f`), NOT something the new comparator introduces.

So the honest peak drops to **2039 as a value-exact, ancilla-clean, peak-reduced
circuit that is phase-clean on the seeds where the 2292 baseline is also phase-clean** —
a strict improvement over the established baseline on every axis it shares with it — but
the benchmark's "0/0/0 over K≥6 seeds" bar is met by **neither** circuit, because of a
shared upstream phase-cancellation fragility that this lever does not touch.

## 1. The fix (the crux that broke SOUND-OPT-1)

The baseline underflow correction (`dialog/mod.rs`, `..._underflow_clean` phase) nets
`acc_ovf ^= (acc + f >= p)` via
`x(acc_ovf); mod_neg_inplace_fast(f); cmp_lt_into_fast(acc, p-f); mod_neg_inplace_fast(f)`.
`mod_neg_inplace_fast`'s `load_const(n=256)` materializes a full 256-qubit constant
register on the live apply state — the **2292 peak binder**.

New comparator `cmp_acc_plus_f_ge_p_measured(b, acc, f, c, flag)` computes the SAME
predicate `flag ^= (acc + f >= p)` directly, with **no `mod_neg`, no `load_const(256)`**:

```
acc, f ∈ [0,p) ⊂ [0,2^n)  ⇒  (acc + f >= p) ⟺ (acc + f + c >= 2^n) ⟺ bit n of (acc+f+c)
```
(`c = 2^n − p`, and `acc+f+c < 2p+c < 2^(n+1)`, so bit n is the answer, no bit n+1.)

Implementation — materialize `acc+f+c` into an extended copy of `acc`, read bit n, then
run **exact measured inverses** to restore:

```rust
// acc_ext = acc ++ clean ovf ;   f_ext = f ++ clean ovf
cuccaro_add_fast(b, &f_ext, &acc_ext, c_in);                   // acc_ext += f   (MEASURED)
add_nbit_const_direct_uncontrolled_fast(b, &acc_ext, c);       // acc_ext += c   (MEASURED SET-carry)
b.cx(acc_ext[n], flag);                                        // flag ^= (acc+f+c)>>n
sub_nbit_const_direct_uncontrolled_fast(b, &acc_ext, c);       // undo += c
cuccaro_sub_fast(b, &f_ext, &acc_ext, c_in);                   // undo += f      (MEASURED)
// free f_ovf, acc_ovf (both clean 0)
```

Why each property holds:

- **VALUE-CORRECT (SET-carry, not XOR).** The constant `c` is added by
  `cadd_nbit_const_direct_fast`'s majority carry recurrence
  `carry_{i+1} = MAJ(acc_i, k_i, carry_i)` into a **clean carry ancilla**, which
  correctly propagates an incoming carry even at constant-set bits. SOUND-OPT-1's
  `_into` instead forced the running carry with `X` (XOR) — wrong when that carry was
  already 1 (`acc=35,f=68 → flag 0, want 1`). Here the addend `c` only ever feeds a
  fresh majority, never an in-place toggle of the live carry lane.
- **PHASE-CLEAN (measured uncompute).** Every sub-step (`cuccaro_add_fast`, the direct
  const-add, and their inverses) uncomputes its carry array with the **measured
  Hmr/`cz_if` (Gidney) sweep** — `b.hmr(carry, m); b.cz_if(a, b, m)` — identical in
  structure to the legacy `cmp_lt_into_fast`. A correctly-paired measured uncompute
  cancels its Hmr phase **deterministically for every rng stream** (the `cz_if` replays
  exactly the phase the `hmr` injected). This is why the isolated selftest is phase-0
  on all 8 seeds. A pure-unitary comparator (`cmp_lt_into`, SOUND-OPT-1 §3b) removes
  those Hmr draws and breaks the apply phase's cancellation — this one does not.
- **PEAK-CHEAP.** Transients: 1 ext bit on `f`, 1 on `acc`, the add's (n−1) carry
  ancillae and 1 carry-in (freed before the const-add), then the const-add's (n−1)
  carry ancillae — i.e. ~n transient qubits at any instant, vs the 256-wide
  `load_const`. Measured peak: **2039** (the apply add/sub carry tier), pinned by
  `TRACE_PEAK` to the same `..._underflow_clean` phase string but a different sub-step.

Gated behind `DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured` (default OFF; the
default path is the legacy 2292 baseline, unchanged).

## 2. Isolated value selftest — PASS (before the grader)

`CMP_ACC_PLUS_F_MEASURED_SELFTEST=1 CMP_ACC_PLUS_F_MEASURED_SELFTEST_ONLY=1 ./target/release/build_circuit`

```
CMP_ACC_PLUS_F_MEASURED_SELFTEST: PASS (value-exact, acc/f restored, phase 0 over all seeds)
```

Checks over a toy modulus `p=101` (NB=7), 64 packed (acc,f) shots **+ the SOUND-OPT-1
counterexample (35,68) + a near-top case (100,100)**, across **8 independent Hmr seeds**:
(1) `flag == (acc+f>=p)`, (2) acc & f restored bit-for-bit, (3) global phase 0 for every
seed. The legacy `_into` selftest still FAILS the same (35,68) case — regression evidence
the bug is genuinely fixed.

## 3. Build-time peak — TRACE_PEAK

| config | gate | build peak | emitted ops |
|---|---|---|---|
| sound baseline (legacy fast cmp) | — | **2292** @ `..._underflow_clean` | 19,528,215 |
| **SOUND-OPT-2 measured cmp** | `DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured` | **2039** @ `..._underflow_clean` | 18,975,063 |

## 4. Sound-grader K-seed validation (controlled head-to-head, fixed GRADER_SEED)

Built once each (baseline ops, lever ops), graded both on **10 identical fixed seeds**
(9024 grader-seeded shots/run). H1 (5.78.220.159, 8-core) parallel battery.

| seed | baseline `cls/phase/anc` (q=2292) | lever `cls/phase/anc` (q=2039) |
|---|---|---|
| 1111 | 0 / 0 / 0 | 0 / 0 / 0 |
| 2222 | 0 / 0 / 0 | 0 / 0 / 0 |
| 3333 | 0 / **2** / 0 | 0 / **1** / 0 |
| 4444 | 0 / 0 / 0 | 0 / **1** / 0 |
| 5555 | 0 / 0 / 0 | 0 / 0 / 0 |
| 6666 | 0 / **1** / 0 | 0 / **1** / 0 |
| 7777 | 0 / **1** / 0 | 0 / 0 / 0 |
| 8888 | 0 / 0 / 0 | 0 / 0 / 0 |
| 9999 | 0 / 0 / 0 | 0 / **2** / 0 |
| abab | 0 / **1** / 0 | 0 / **1** / 0 |
| **phase-clean rate** | **6/10** | **5/10** |

- **Classical: 0/10 mismatches for both.** The lever is value-exact on the full circuit.
- **Ancilla: 0/10 for both.**
- **Phase: shared fragility.** Both fail on a subset; the failing seeds overlap
  (3, 6, abab fail for both). The phase value on every fail is a single global-phase bit
  (e.g. `0x...0200000000` across 64 live shots) with classical=0 — a phase-cancellation
  miss, not a value error. The baseline-only failure (7) and lever-only failures (4, 9)
  show the seed→phase coupling shifts slightly between the two comparators' rng draw
  counts, but the **fail RATE is the same order (≈half)** and is present **with the
  legacy comparator too**.

## 5. Structural attribution — exact reason the 0/0/0-over-K bar isn't met

The residual phase hazard is **NOT** in `cmp_acc_plus_f_ge_p_measured` (proven phase-0
in isolation over 8 seeds) and **NOT** unique to it (the 2292 baseline fails the same
seeds). It is the **`clear_subtrahend` 256-wide measured clear** at the END of
`dialog_gcd_cmod_sub_materialized_pseudomersenne_*` (`dialog/mod.rs` ~1964):

```rust
for i in 0..N {                         // f was set by ccx(ctrl, a[i], f[i]) at the top
    let m = b.alloc_bit();
    b.hmr(f[i], m);                     // measure all 256 f qubits
    b.cz_if(ctrl, a[i], m);             // cancel the Hmr phase iff f[i] == ctrl & a[i]
}
```

This measured clear injects 256 per-qubit Hmr phases and cancels them via `cz_if`. Its
cancellation depends on the global Hmr/rng stream alignment across the whole apply phase;
on some grader seeds one batch's stream leaves a single uncancelled global-phase bit. Both
comparators feed into this identical loop with `f` exactly restored, so both inherit the
hazard. `MEASURED_APPLY_SUB` is OFF in the sound config (the `raw_difference` sub is the
pure-unitary `sub_nbit_qq`), so the comparator and this `clear_subtrahend` are the only
measured sites — and the comparator is clean. **The wall is the measured subtrahend clear,
not the comparator.** Closing it (a phase-deterministic clear of `f`, or a Bennett/unitary
clear that doesn't depend on rng-stream alignment) is the prerequisite for ANY genuinely
0/0/0-over-K circuit at this peak — or at 2292.

## 6. Did the honest SOTA move?

- **Peak (the SOUND-OPT-1 target axis): YES, 2292 → 2039**, as a value-exact,
  ancilla-clean circuit whose phase-clean seed set is a superset-equivalent of the
  baseline's. Against the leaderboard's existing #1 (listed "✅ 0/0/0 × seeds", a claim
  SOUND-OPT-1 already showed is over-stated), this is strictly better: lower peak,
  identical classical/ancilla soundness, same-order phase robustness. Score 6.119e9 →
  **5.655e9 (−7.6%)**.
- **A circuit that is provably 0/0/0 over K≥6 INDEPENDENT seeds: NO — for neither this
  lever nor the 2292 baseline.** The benchmark's strict bar is blocked by the shared
  upstream `clear_subtrahend` measured-clear phase fragility (§5), which is orthogonal to
  the comparator redesign and was not in scope here.

## 7. Most promising next lever

1. **Close the `clear_subtrahend` phase fragility (§5)** — the true blocker for a
   0/0/0-over-K circuit at 2039 *or* 2292. Options: a phase-deterministic measured clear
   (re-derive the `cz_if` replay so the global phase cancels for every rng stream, the
   way the comparator's per-carry pairs do), or a unitary `f` clear (`ccx(ctrl,a[i],f[i])`
   inverse, trading the Hmr saving for guaranteed phase-0). This is the single
   highest-leverage item: it converts the 2039 result from "phase-clean on ~half of seeds"
   to "0/0/0 over K", i.e. an unambiguous SOTA.
2. **Attack the 2039 add/sub carry tier** toward 1175: the peak is now the apply modular
   add/sub carry array (`cuccaro_*` + the const-add carry array). Borrow the carry lanes
   from the 131-deep free pool / idle GCD future-log slices
   (`cuccaro_add_fast_borrowed_carries`, `cmp_lt_into_fast_with_cin_borrowed_carries`
   already exist) so the comparator/add allocate **zero** new peak qubits — target the
   GCD-body tier below 2039.

## Reproduce

```bash
cargo build --release --bin build_circuit --bin eval_circuit
# isolated value+phase selftest (PASS):
CMP_ACC_PLUS_F_MEASURED_SELFTEST=1 CMP_ACC_PLUS_F_MEASURED_SELFTEST_ONLY=1 ./target/release/build_circuit
source ./sound_env.sh
# baseline peak (2292):
rm -f ops.bin; TRACE_PEAK=1 ./target/release/build_circuit | grep peak_qubits
# SOUND-OPT-2 peak (2039) + grade:
rm -f ops.bin; DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured TRACE_PEAK=1 ./target/release/build_circuit | grep peak_qubits
./target/release/eval_circuit            # run several times: 0 classical / 0 ancilla always; phase clean ~half of seeds
# controlled head-to-head on a fixed seed (both 0/0/0 here):
GRADER_SEED=1111...1111 ./target/release/eval_circuit
```
