# SOUND-OPT-7 (panel2-coset-Zalka-padded-mod) — coset/Zalka padded modular add-sub: NO-GO (research spike)

Branch: `panel2-coset-Zalka-padded-mod` (base `descend-B` @ `a05c649`, the 2034 SOTA / q×T baseline).
Build: `cargo build --release --bin build_circuit`.

## TL;DR

- **VERDICT: NO-GO on this benchmark.** The coset/Zalka padded modular add-sub
  (delete the comparator; Gidney arXiv:1905.08488 / Zalka coset rep, the Google
  arXiv:2603.28846 algorithm family) is **structurally inapplicable** to this
  challenge's simulator. It is committed as a **research spike** with an isolated
  value selftest that PROVES the NO-GO — it is NOT wired into the live circuit
  (doing so would ship a known-wrong approximation, violating README-SOUND rule 4).
- **Peak and q×T are UNCHANGED** at the descend-B base (peak **2034**, the SOTA;
  q×T unchanged) — the lever adds only a selftest, the default/`acc_plus_f_measured*`
  build paths are byte-identical. **No peak win, no q×T win. Honest null.**
- **Root cause (decisive, not a tuning failure):** the Zalka coset trick is a
  **superposition** technique. It keeps the register as an approximate eigenvector
  of `+p` — i.e. `(1/√2^c) Σ_k |v + k·p⟩` — so that a PLAIN add of `x` reduces mod p
  with amplitude error ~`2^-c` per padding qubit. **This benchmark's simulator
  (`src/sim.rs`) is a deterministic computational-basis simulator**: each qubit
  holds ONE classical bit per shot (64 packed in a `u64`), `Hmr`/`R` PROJECT the
  qubit to |0⟩ and accumulate a phase parity bit, and there is no amplitude /
  superposition representation at all. A coset eigenvector is unrepresentable; the
  grader seeds **classical** basis-state inputs (`set_register`) and checks a
  **single classical value** per shot (`get_register == expected`).
- In a computational-basis model a "plain add into a padded register" carries
  `2^n` into the padding, **never `p`**. Since `p` is not a power of two, the coset
  never reduces: the result is exactly `acc + x` (deterministically wrong mod p
  whenever `acc + x ≥ p`), with **probability 1** — not amplitude-suppressed. The
  `2^-c` bound does not exist in this model.

## 1. The isolated value selftest (the brief's mandated phase-0/value GATE)

`coset_modadd_selftest()` (`src/point_add/mod.rs`) instantiates exactly the lever:
a coset-padded accumulator `acc = [data(NB) | c HIGH padding bits]`, seeded as the
only classical "eigenvector of +p" the model admits — a definite offset `k·p` —
then a PLAIN non-modular `cuccaro_add_fast` of `x` into the padded register (NO
comparator, NO fold, NO conditional mod-subtract), checked against `(acc+x) mod p`
over the same kind of computational-basis inputs the SOUND grader uses (including
the wrap cases `p−1+p−1`, `p−1+1`, and the SOUND-OPT-1 adversary `35+68`).

It sweeps **every** achievable classical coset offset `k ∈ [0, 2^c)` for
`c ∈ {3,5,7}` (up to a FULL extra register width — far beyond the proportional
`c~40–50/256` the brief targets, so "too few padding bits" cannot explain the
failure) and PASSES iff some `k` makes the plain add value-correct on all cases.

```
COSET_MODADD_SELFTEST=1 COSET_MODADD_SELFTEST_ONLY=1 ./target/release/build_circuit
COSET_MODADD_SELFTEST: FAIL (EXPECTED — see report): c_pad=3: NO classical coset offset
  k in [0,2^3) makes a PLAIN add value-correct over 63 cases; e.g. acc=35 x=68 ->
  low bits 103, want (acc+x) mod 101 = 2. (Plain add carries 2^n into padding, NOT p;
  p=101 is not a power of two, so the coset never reduces in a deterministic
  computational-basis model — the 2^-c amplitude bound is for SUPERPOSITION cosets,
  which src/sim.rs cannot represent.)
```

This is a NO-GATE harness: it reports the verdict (does not `panic`) and documents
the model mismatch. Per README-SOUND rule 4, a disclosed/bounded/counted
approximation is allowed only if it survives the grader; this one is value-wrong on
**every** reachable input with `acc+x ≥ p` (not a rare tail), so it cannot.

## 2. Why this is a category result, not a "needs bigger c" result

- **Eigenvector argument.** `+p` is a shift operator on the integer register; its
  eigenvectors are Fourier-type **superpositions**. A computational-basis state is
  never an eigenvector of a non-trivial shift. So the lever's premise ("seed the
  register as an approximate eigenvector of +p") cannot be met by any classical
  seeding — independent of `c`.
- **The `2^-c` bound is amplitude.** Gidney/Zalka's worst-case `2^-c`-type bound
  bounds the probability that a measurement of the *superposed* coset gives the
  wrong residue. With one classical value per shot there is no superposition and
  no amplitude: the error is deterministic per input, probability 1, not `2^-c`.
- **This is exactly the README-SOUND distinction.** MSB-truncation / `COMPARE_BITS<256`
  / `ACTIVE_ITERATIONS<402` were rejected because they are wrong on a structured
  near-`p` tail (CENSUS). The coset add is worse — wrong on the *bulk* of inputs
  (any `acc+x ≥ p`) in this model — so it fails the bar a fortiori.

## 3. Independent confirmation against the prior wall (phase)

Even setting the value blocker aside, SOUND-OPT-1 §3 already PROVED that deleting
the apply comparator's MEASURED (Hmr/cz_if) uncompute for ANY pure-unitary path
breaks the apply-phase cancellation (141/141 phase-garbage). A plain coset adder
removes those Hmr draws entirely. So the lever faces TWO independent walls (value
in §1; phase in SOUND-OPT-1 §3) — the value wall is hit first and is decisive.

## 4. Scope / soundness posture (no over-claim)

- The default circuit and all prior `DIALOG_GCD_UNDERFLOW_CLEAN_CMP` values are
  **byte-unchanged** (this change is purely additive: one selftest fn + one
  dispatch block, no edit to any build path).
- **No `DIALOG_GCD_COSET_MODADD` knob is wired into `point_add::build()`** — there
  is no value-correct coset construction to gate, and shipping a known-wrong knob
  (even default-OFF) would be a back-fill of the brief's deliverable, not the
  method. The honest deliverable is the proof-of-NO-GO selftest + this writeup.
- Regression: `CMP_ACC_PLUS_F_MEASURED_SELFTEST` and
  `CMP_ACC_PLUS_F_MEASURED_BORROWED_SELFTEST` still PASS.

## 5. Did the SOTA move?

**No — honest null.** Peak stays at the descend-B SOTA (2034) and q×T is unchanged;
the only lever in the set that could structurally close the 1.73× qubit gap toward
Google's 1175 relies on a superposition coset that this benchmark's deterministic
computational-basis simulator cannot represent. The descent toward 1175 on THIS
benchmark must come from levers that are value-exact in a computational-basis model
(the SOUND-OPT-2…6 family: low-peak measured comparators, borrowed carries,
windowing, conditioned replay), not from the Zalka algorithm family.

## Reproduce

```bash
cargo build --release --bin build_circuit
COSET_MODADD_SELFTEST=1 COSET_MODADD_SELFTEST_ONLY=1 ./target/release/build_circuit  # FAIL (expected; proves NO-GO)
# default build path unchanged (peak 2034 @ descend-B base):
CMP_ACC_PLUS_F_MEASURED_SELFTEST=1 CMP_ACC_PLUS_F_MEASURED_SELFTEST_ONLY=1 ./target/release/build_circuit  # PASS
```
