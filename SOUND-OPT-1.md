# SOUND-OPT-1 — peak-qubit reduction of the sound secp256k1 point-add circuit

Goal: lower the honest (sound-graded) peak-qubit count of the value-exact secp256k1
reversible point-add circuit below the established **2292** baseline, toward Google's
correct **1175**, without nonce-gaming (grader-controlled `sound_seed`).

Commit base: `0146edd`. Build: `cargo build --release --bin build_circuit --bin eval_circuit`.

## TL;DR

- **The honest peak did NOT drop below 2292 with a *valid* circuit.** The only path that
  lowers the build-time peak (2292 → **1911**) eliminates the peak-binding step but is
  **invalid**: it fails the sound grader's PHASE check (and the general-input value check).
- **Root cause of the 2292 peak is now pinpointed exactly** (not the apply *comparator*, as
  the leaderboard note assumed, but the in-place modular **negation** that precedes it):
  `mod_neg_inplace_fast`'s `load_const(n=256)` materializes a full 256-qubit constant
  register on top of the live apply state.
- **New finding that reframes the benchmark:** the established **2292 "sound baseline" is
  itself NOT robustly phase-clean.** Over fresh/published independent grader seeds it fails
  the phase check on a substantial fraction of seeds (e.g. 2/6 fresh seeds; published seed
  `…01` shows 1 phase-garbage batch). Its "0/0/0 across many seeds" status holds only on the
  seeds where it happens to land a phase island — the same gaming pathology, shifted from the
  classical channel to the phase channel via the simulator's per-Hmr rng.

## 1. Peak profile (sound config, `TRACE_PEAK=1` / `TRACE_EACH_PEAK=1`)

Sound config (all truncations OFF, value-exact levers ON) overrides the baked submission-route
truncations:

```
DIALOG_GCD_RAW_APPLY_TRUNCATED_CLEAN=0  DIALOG_GCD_PA9024_COMPARE_SCHEDULE=0
DIALOG_GCD_COMPARE_BITS=256  DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS=256
DIALOG_GCD_ACTIVE_ITERATIONS=402  DIALOG_GCD_FOLD_CARRY_TRUNC_W=4096
KAL_DOUBLE_CARRY_TRUNC_W=4096  KAL_FOLD_CARRY_TRUNC_W=4096
ROUND84_INPLACE_QUOTIENT_CARRY_TRUNC_W=4096  DIALOG_GCD_WIDTH_MARGIN=4096
DIALOG_GCD_BODY_CARRY_BAND_TRIMS=0
DIALOG_GCD_SPECIAL_OVERFLOW_CLEAN_STEP_BITS=  DIALOG_GCD_SPECIAL_UNDERFLOW_CLEAN_STEP_BITS=
DIALOG_TAIL_NONCE=          # cannot nonce-game the grader-controlled seed
```

`TRACE_PEAK` pins the peak to a single sharp spike:

```
peak_qubits=2292  phase='dialog_gcd_materialized_special_underflow_clean'  ops_idx=2307664
```

`TRACE_EACH_PEAK` shows the peak climbing 2036 → 2292 monotonically **inside one call**, i.e.
256 qubits allocated one-by-one at that instant. Splitting the phase string proves the spike is
in the **negation**, not the comparator:

```
PEAK active=2036..2292  phase='dialog_gcd_uflow_clean_modneg'   # the climb
```

Free-pool trace at the spike: `active=1780, free_pool=131, next=1911` — i.e. the live base
just before the negation is 1780, and the negation makes **256 qubits simultaneously live**.

## 2. The comparator / negation code (`src/point_add/rounds/dialog/mod.rs`, sub apply)

With `DIALOG_GCD_RAW_APPLY_TRUNCATED_CLEAN=0` (sound), the exact underflow-clean path was:

```rust
b.x(acc_ovf);
mod_neg_inplace_fast(b, &f, p);     // f -> (p - f) mod p   <-- THE 2292 BINDER
cmp_lt_into_fast(b, acc, &f, acc_ovf);
mod_neg_inplace_fast(b, &f, p);     // restore f
```

`mod_neg_inplace_fast` (`arith/modular.rs`) = `~f ; f += (p+1)`, and `f += (p+1)` calls
`load_const(b, 256, p+1)` (`arith/adder.rs`) — **a fresh 256-qubit constant register**. The
symmetric exact OVERFLOW-clean ADD path uses `cmp_lt_into(acc, f, acc_ovf)` directly (no
negation, 1 ancilla) — only the SUB path pays the negation. `cmp_lt_into_fast` itself
additionally allocates `c_in + n` carries, but it runs *after* the negation frees, so the
negation's 256-wide `load_const` is the true binder.

Correct predicate (derived): the legacy block nets `acc_ovf ^= (acc + f >= p)`, equivalently
`acc_ovf ^= carry_out(acc + f + c)` with `c = 2^256 - p = 2^32 + 977` (sparse).

## 3. Redesign attempted

### 3a. Direct sparse-constant carry comparator — `cmp_acc_plus_f_ge_p_into` (`arith/compare.rs`)
Compute `acc_ovf ^= carry_out(acc + f + c)` with **one** in-place MAJ sweep on `acc`
(restored by the inverse MAJ), one `c_in` ancilla, no `mod_neg`, no `load_const`, no wide
carry array. Build-time peak **2292 → 1911** (binder moves to the modular-add `raw_difference`
floor), op count 19.5M → 16.05M.

VERDICT: **INVALID.**
- **Value (general inputs): WRONG.** The sparse-constant injection forces a running carry to 1
  with `X`, which is XOR not SET — when the carry is already 1 it flips to 0. The isolated
  selftest `CMP_ACC_PLUS_F_SELFTEST=1` catches it: `acc=35, f=68, p=101 → flag 0, want 1`.
  (On the secp reachable support it happens to be 0-classical, which is why a naive single-seed
  run looks clean — exactly the trap this benchmark exists to expose.)
- **Phase: FAILS** the grader's phase check, 141/141 batches, on every seed.

### 3b. Value-correct low-peak path — `mod_neg` + slow `cmp_lt_into`
Keep the proven `mod_neg` but uncompute the comparator carry with the slow pure-unitary
1-ancilla `cmp_lt_into` (`DIALOG_GCD_UNDERFLOW_CLEAN_CMP=slow`). VALUE-EXACT (0 classical /
0 ancilla, all seeds). But the negation is kept, so the build-time **peak stays 2292** (the
`load_const` still binds) — no peak win — and it ALSO fails the phase check (~141/141).

### Why phase fails (the real wall)
Swapping the apply comparator's **measured (Hmr) uncompute** (`cmp_lt_into_fast`) for ANY
pure-unitary comparator turns the grader's phase result non-zero, independent of the comparator's
correctness and independent of mod_neg. Padding the Hmr/rng stream (self-cancelling dummy Hmr)
does not restore cleanliness. The apply phase's measured-uncompute phase cancellation depends on
the Hmr/rng-stream structure that the legacy `mod_neg + cmp_lt_into_fast` provides; a low-peak
comparator that removes those Hmr draws (or the negation that supplies the const-register the
fast comparator's carries reuse) leaves an uncancelled global phase. This is the same
fragility as the seed-dependence of the baseline (§ TL;DR).

## 4. Sound-validation results

| config | gate | build peak | avg-T | sound grader (independent seeds) |
|---|---|---|---|---|
| **DEFAULT (legacy fast)** | — | **2292** | ~2,669,730 | **NOT robust**: 0/0/0 on ~4/6 fresh + 2/3 published; 1 phase-garbage batch on the rest |
| value-correct slow | `…CMP=slow` | 2292 | ~2,772,650 | 0 classical / 0 ancilla all seeds; phase fails ~½ of seeds |
| direct low-peak | `…CMP=acc_plus_f` | **1911** | — | INVALID: general-input value bug (selftest) + 141/141 phase fail |

Isolated unit evidence: `CMP_ACC_PLUS_F_SELFTEST=1 ./target/release/build_circuit` (toy modulus,
packed shots) checks flag value, acc/f restoration, and phase=0; it FAILS on the value channel,
documenting the 3a bug precisely.

## 5. Did the honest SOTA move? No.

The honest sound peak remains **2292** (and is, strictly, not even robustly phase-clean at 2292).
No valid circuit below 2292 was produced. The "1911" is a real *structural* peak floor (the
modular add/sub carry tier) but is only reachable by an invalid comparator.

## 6. Most promising next lever toward 1175

The peak is the **256-qubit `load_const` inside the apply-phase modular negation**, and beneath
it the modular add/sub carry tier sits at **1911**. To move the honest SOTA you must, *with a
phase-clean measured uncompute*:

1. **Replace the apply-phase modular subtract's underflow correction with a measured (Hmr)
   sparse-constant carry-out** of `acc + f + c` — i.e. fix 3a's constant injection to use a
   correct (SET, not XOR) carry via the proven `cadd_nbit_const_direct_fast`-style majority
   recurrence, AND uncompute its carries with the *measured* (Gidney) sweep the apply phase
   already relies on, so the phase cancellation structure is preserved. That removes the
   `load_const(256)` while keeping the Hmr/rng structure the grader's phase check needs. This is
   the single highest-leverage target: it directly attacks the 256-qubit binder. Target peak ≈
   the 1911 add/sub tier.
2. Then attack the **1911 modular add/sub carry tier** (`raw_difference`/`raw_sum`) with the
   borrowed-clean-carry adders already in the codebase (`cuccaro_*_borrowed_carries`, sourcing
   the 131-deep free pool + the idle GCD future-log slices the truncated route borrows from),
   to push toward the GCD-body tier.
3. **Separately, audit the baseline's phase non-robustness** (the apply fused-fold /
   measured-uncompute levers `FUSED_*_MEASURED`, `MEASURED_APPLY_SUB`) for the per-input phase
   hazard that surfaces ~1 batch/9024 even on the legacy 2292 path — a genuinely sound 2292 (let
   alone <2292) requires closing it, since the benchmark's own rule is 0/0/0 over K≥8 independent
   seeds.

## Reproduce

```bash
cargo build --release --bin build_circuit --bin eval_circuit
source ./sound_env.sh   # the §1 env set; or export inline
# baseline (2292):
rm -f ops.bin; TRACE_PEAK=1 ./target/release/build_circuit      # peak 2292 @ underflow_clean
./target/release/eval_circuit                                   # run several times: phase not robust
# experimental low-peak (1911, INVALID):
rm -f ops.bin; DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f TRACE_PEAK=1 ./target/release/build_circuit
./target/release/eval_circuit                                   # 0 classical/0 ancilla, 141 phase-garbage
# isolated comparator selftest (documents the value bug):
CMP_ACC_PLUS_F_SELFTEST=1 CMP_ACC_PLUS_F_SELFTEST_ONLY=1 ./target/release/build_circuit
```
