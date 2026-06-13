# SOUND-OPT-3 (descend-B) — borrowed-carry apply comparator: peak 2039 → 2034

Branch: `descend-B` (base `sound-opt-2` @ `ec2565f`, the 2039 SOTA).
Approach **B** (per SOUND-OPT-1 §6.2 / SOUND-OPT-2 §7.2): route the apply-phase
underflow-correction comparator through a **borrowed-carries** redesign so its
256-wide carry array draws from idle clean |0> lanes instead of fresh allocation,
adding zero new peak qubits per borrowed lane — while preserving the value and the
MEASURED (Hmr/Gidney) phase structure of `acc_plus_f_measured`.

## TL;DR

- **Peak drops 2039 → 2034 (−5 q)**, value-exact and phase-identical to the 2039
  `acc_plus_f_measured` lever. NOT the ~1783 add/sub carry tier the approach
  targeted — see §3 for the structural reason.
- New comparator `cmp_acc_plus_f_ge_p_measured_borrowed` (`arith/compare.rs`):
  the SAME predicate / gates / measured uncompute as `cmp_acc_plus_f_ge_p_measured`,
  but its four sequential carry arrays (Cuccaro add, SET-carry const-add, const-sub,
  Cuccaro sub) draw their `n` clean lanes from a caller-supplied borrow pool
  (`borrowed_prefix ++ owned_deficit`, the PARTIAL-host pattern of
  `dialog_gcd_ccx_cmp_gt_truncated_into_width_hosted`). Borrowed lanes exit |0>;
  the deficit is freshly allocated.
- New borrowed-carry primitives in `arith/const_arith.rs`:
  `c{add,sub}_nbit_const_direct_fast_borrowed_carries` and their uncontrolled
  wrappers — byte-for-byte the existing direct-fast const add/sub with the carry
  array supplied by the caller.
- **Isolated value+phase selftest PASSES** (`CMP_ACC_PLUS_F_MEASURED_BORROWED_SELFTEST`):
  over 8 independent Hmr seeds + the (35,68) XOR-injection counterexample + (100,100),
  it checks flag value, acc/f restore, **borrow-lane restore to |0>**, phase 0, AND
  byte-identical flag+global-phase vs the non-borrowed `acc_plus_f_measured`. It is a
  proven drop-in equivalence, not merely "correct".

## 1. Lever

Gated behind a NEW value `DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured_borrowed`
(default path and the `acc_plus_f_measured` 2039 path are byte-unchanged). The
borrow pool passed from the apply csub is `clean_scratch` — the current compressed
transcript block's own |0> cells under `DIALOG_GCD_APPLY_REPLAY_SWAP_HOST=1` (set by
`configure_ecdsafail_submission_route`).

```
peak_qubits=2034  phase='dialog_gcd_materialized_special_underflow_clean'
CMP_BORROW need=256 borrowed_avail=5 owned_alloc=251
```

The −5 is exactly the 5 clean idle-ACTIVE cells the compressed block exposes.

## 2. Soundness posture (no over-claim)

This lever is **value- and phase-IDENTICAL** to `acc_plus_f_measured` (selftest
equivalence over all seeds). It therefore inherits exactly that lever's K-seed
robustness — no better, no worse. It does **not** touch the upstream
`clear_subtrahend` 256-wide measured-clear that SOUND-OPT-2 §5 identified as the
real "0/0/0 over K seeds" blocker. Fixed-seed spot-checks (1111/3333/9999) grade
0/0/0 for BOTH this lever and `acc_plus_f_measured` under the canonical config, but
that is a sanity check, not a soundness proof — an independent verifier grades fresh
seeds.

## 3. Why it is 2034, not ~1783 (the honest structural finding)

The peak metric is **max simultaneously-live qubits** (`active_qubits`). Profiling
the 2039 peak (canonical config):

- Peak phase `..._underflow_clean`; comparator climbs 1780 → 2039 by allocating
  acc_ovf + f_ovf + c_in + 256 carry lanes ≈ 259 fresh qubits, draining the free
  pool to 0. So peak = 1780 (apply data base) + ~259 (comparator transient).
- At the comparator instant the free pool is clean but **inactive**; reusing it
  still increments `active`, so it does not lower the peak. Reducing the peak needs
  carry lanes that are **already counted in the 1780 base AND idle AND clean**.

In the canonical **compressed-sidecar** apply, the only such idle-active clean
region is the current block's 5 compressed cells (REPLAY_SWAP_HOST). The
~131-clean-free-pool from SOUND-OPT-1 was the **non-compressed** 2292 mod_neg peak,
where the 804-qubit `dialog_log` future-log was fully active & idle during apply — a
real 256-wide borrow source. Compression frees that log (it becomes the inactive
free pool) and releases `u` to the free pool too, so the apply comparator has no
256-wide idle-active clean register to borrow from. Hence the borrow saturates at
the 5 cells → 2034.

To reach the ~1783 add/sub tier one would have to **keep** a 256-wide register
active-and-idle across the comparator (e.g. retain part of `u` instead of releasing
it), but that lifts the base of the lower apply phases (raw_difference 1911, folds
1926) above 2039 unless those phases also consume the lent lanes — a much larger
refactor that trades the win away elsewhere. That is a different lever, out of
Approach-B scope, and not pursued here.

## Reproduce

```bash
cargo build --release --bin build_circuit --bin eval_circuit
# isolated value+phase selftest (PASS):
CMP_ACC_PLUS_F_MEASURED_BORROWED_SELFTEST=1 CMP_ACC_PLUS_F_MEASURED_BORROWED_SELFTEST_ONLY=1 ./target/release/build_circuit
source ./canon_env.sh                       # canonical sound config
# baseline 2039:
rm -f ops.bin; DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured          TRACE_PEAK=1 ./target/release/build_circuit | grep peak_qubits
# descend-B 2034:
rm -f ops.bin; DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured_borrowed TRACE_PEAK=1 ./target/release/build_circuit | grep peak_qubits
```
