# SOUND-OPT-5 (panel2) — phase-conditioned replay of the apply-phase underflow comparator

Branch: `panel2-Phase-conditioned-repl` (base `descend-B` @ `a05c649`, the 2034 SOTA).
Build: `cargo build --release --bin build_circuit --bin eval_circuit`.

## TL;DR

- **Lever:** the apply-phase modular-subtract underflow-correction comparator
  (`dialog_gcd_materialized_special_underflow_clean`) runs **once per GCD apply-replay
  step** (compressed-sidecar route, `compressed.rs:1316`), i.e. ~100-200 times per
  point-add. Each call is a value-exact, phase-clean MEASURED comparator that materializes
  `acc + f + c` (`c = 2^256 - p`) with four 256-wide CCX carry sweeps and reads the carry-out
  into the cleaning flag `acc_ovf`. **Conditioning** that whole block on a fresh
  Hmr-measured bit makes its CCX execute on only the ~50% of shots where the measured bit
  fired — halving the comparator's **avg-EXECUTED Toffoli** (the graded metric), at flat peak.
- **Peak held at the 2034 SOTA** (the conditioned-borrowed variant stacks on descend-B's
  borrow; TRACE_PEAK = 2034 @ `..._underflow_clean`, identical to the borrowed baseline).
- **q×T (fixed GRADER_SEED=1111…, controlled head-to-head, both 0/0/0):**
  - borrowed baseline (`acc_plus_f_measured_borrowed`, 2034q): avg-T **3,398,071** → q×T **6,911,676,414**
  - conditioned-borrowed (this lever, 2034q):                 avg-T **3,192,202** → q×T **6,492,938,868**
  - **−6.06% avg-Toffoli, −6.06% q×T at held peak.**
- **Value/phase: PROVEN byte-identical to the non-conditioned comparator** in isolation over
  8 independent Hmr seeds (selftests `CMP_ACC_PLUS_F_MEASURED_CONDITIONED_SELFTEST` and
  `..._BORROWED_SELFTEST`, both PASS). The conditioned path measures `acc_ovf` (= predicate on
  entry, the cleaning contract) into a classical bit (clears it to |0>) and replays the
  cleaning predicate as a `Z(acc_ext[n])` under `push_condition(measured_bit)`, cancelling the
  Hmr-injected phase exactly.

## 1. Mechanism (mirrors the in-tree clean comparator)

The codebase already ships this exact template for the clean/truncated route:
`cmp_lt_phase_conditioned_with_cin` / `cmp_lt_phase_conditioned_borrowed_carries`
(`compare.rs`), used under `push_condition(measured_bit)`. This lever applies the same
template to the **measured** underflow comparator `cmp_acc_plus_f_ge_p_measured[_borrowed]`:

```
b.hmr(flag, phase);          // measure flag (= predicate pr on entry); clears flag to |0>.
                             //   sim: phase_bit = rng & cond;  global_phase ^= flag & rng & cond
b.push_condition(phase);     // everything below runs only on the rng&cond shots (~50%)
  ... materialize acc + f + c into acc_ext (4 CCX sweeps) ...
  b.cz(acc_ext[n], acc_ext[n]);   // Z on the predicate bit: phase ^= pr & rng & cond  -> cancels Hmr phase
  ... exact MEASURED inverses restore acc, f, acc_ext ...
b.pop_condition();
```

Because the whole arithmetic block runs under one `push_condition(phase)`, every CCX inside
executes only on the ~50% of shots where `phase` fired, so the simulator's
`toffoli_gates += cond.count_ones()` charges ~½ the comparator Toffoli. The per-carry Hmr/cz_if
pairs inside each sweep share that condition, so their phase cancellation is preserved per shot.

**Cleaning contract.** The reference `cmp_acc_plus_f_ge_p_measured` does
`flag ^= (acc+f>=p)`, which cleans `flag` to |0> ONLY when `flag` already held the predicate.
That is exactly the apply usage: `flag = acc_ovf` = the raw-sub borrow, which equals the
predicate on the secp support. The conditioned variant clears `flag` via the measurement, so
it is value-equivalent to the reference EXACTLY on that contract — proven byte-identical by the
differential selftests.

## 2. Knobs (NEW, default OFF)

`DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured_conditioned`
  — phase-conditioned, non-borrowed. Peak 2039 (no borrow), avg-T halved comparator.
`DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured_conditioned_borrowed`
  — phase-conditioned + descend-B borrow. **Peak 2034 (SOTA held)**, avg-T halved comparator.
  This is the recommended value (stacks the peak win and the Toffoli win).

Wired at the `dialog/mod.rs` underflow_clean dispatch (the same branch ladder as the
`acc_plus_f_measured[_borrowed]` levers). The default path and all prior knob values are
byte-unchanged (purely additive: +562 lines, 0 deletions).

## 3. Selftests (run BEFORE any grade)

```bash
CMP_ACC_PLUS_F_MEASURED_CONDITIONED_SELFTEST=1 CMP_ACC_PLUS_F_MEASURED_CONDITIONED_SELFTEST_ONLY=1 ./target/release/build_circuit
CMP_ACC_PLUS_F_MEASURED_CONDITIONED_BORROWED_SELFTEST=1 CMP_ACC_PLUS_F_MEASURED_CONDITIONED_BORROWED_SELFTEST_ONLY=1 ./target/release/build_circuit
```

Both PASS. Each builds the conditioned circuit AND the non-conditioned reference on the same
toy modulus (p=101, NB=7, 64 packed (acc,f) shots + the (35,68) XOR-injection counterexample +
(100,100)), seeds `flag = predicate` on entry (the cleaning contract), reuses the SAME Hmr seed
for both, and asserts over 8 independent seeds: (1) conditioned `flag` exits |0>, acc & f
restored bit-for-bit (borrow lanes too, in the borrowed test); (2) conditioned global phase 0;
(3) byte-identical flag/acc/f outputs AND global phase vs the reference.

## 4. Sound-grader validation

Canonical config (`canon_env.sh` + the baked submission route).

| config | knob | peak | avg-T (seed 1111…) | q×T | grade (seed 1111…) |
|---|---|---|---|---|---|
| borrowed baseline (2034 SOTA) | `acc_plus_f_measured_borrowed` | 2034 | 3,398,071 | 6.9117e9 | 0/0/0 |
| **conditioned-borrowed (this lever)** | `acc_plus_f_measured_conditioned_borrowed` | **2034** | **3,192,202** | **6.4929e9** | **0/0/0** |

Fresh OS-random-seed runs: see §5 below (the K-seed phase channel is the real check; the
fixed-seed grade is a sanity head-to-head, not a soundness proof).

## 5. Phase-cleanliness posture (the dominant risk — NO over-claim)

SOUND-OPT-2 §5 established that the residual phase hazard at this peak is **NOT** in the
comparator (proven phase-0 in isolation over 8 seeds) but in the shared upstream
`clear_subtrahend` 256-wide measured-clear (`dialog/mod.rs` ~1981), which fails the grader's
phase check on a subset of independent seeds for the BASELINE comparator too. Conditioning the
comparator changes the global Hmr/rng draw COUNT and ORDER (it adds one predicate Hmr per call
and reorders the comparator's internal Hmrs under `push_condition`), which is precisely what
can shift that shared cancellation. The isolated selftest cannot catch a cross-talk regression;
only a fresh-seed K-grade can. This lever inherits — at best — the baseline's K-seed phase
robustness, and the burden is to show it does not WORSEN it. The fresh-seed grades in this run
are the evidence; an independent verifier grades fresh OS-random seeds and recomputes q×T.

## Reproduce

```bash
cargo build --release --bin build_circuit --bin eval_circuit
# selftests (PASS) — see §3
source ./canon_env.sh
# peak (held at 2034):
rm -f ops.bin; DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured_conditioned_borrowed TRACE_PEAK=1 ./target/release/build_circuit | grep peak_qubits
# grade (fresh OS-random seed each run):
./target/release/eval_circuit
# controlled head-to-head on a fixed seed:
GRADER_SEED=1111111111111111111111111111111111111111111111111111111111111111 ./target/release/eval_circuit
```
