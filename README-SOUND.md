# ecdsa.fail-sound — an ungameable secp256k1 point-add resource benchmark

A fork of the [ecdsa.fail](https://ecdsa.fail) challenge that **corrects a validator-soundness flaw**
so the leaderboard measures the *real* fault-tolerant resource cost of a reversible secp256k1
elliptic-curve point-addition circuit — not a circuit tuned to pass one hand-picked test sample.

## The problem this fixes

The upstream grader (`eval_circuit`) derives its 9024 test inputs from `SHAKE256(op_stream)` — a
Fiat-Shamir hash of **the contestant's own circuit**. Because the op stream is malleable at zero
score cost (`DIALOG_TAIL_NONCE` appends identity gate-pairs — Clifford, uncounted), a contestant
**controls which inputs get tested**. A circuit that is *wrong on a small fraction of reachable
inputs* (lazy-carry / windowed-comparator / width-envelope truncations) can be made to validate
0/0/0 by hunting a nonce whose sample misses its errors.

Consequence: **the frontier's sub-1300-qubit "circuits" are not correct.** A random (un-hunted)
seed leaves a nonzero displacement — proof the errors are on legitimately reachable curve points.
Google deliberately avoided this by validating their circuits with a **zero-knowledge proof of
correctness** (arXiv 2603.28846), which cannot be produced for a wrong circuit.

## The fix (one function)

`eval_circuit::sound_seed()` seeds the test inputs from a **grader-controlled source, independent of
the op stream**:
- **fresh OS randomness per run** by default — ungameable *and* un-hardcodable (the contestant can
  neither predict nor steer the inputs), or
- a fixed `GRADER_SEED` (hex) for reproducible scoring / re-grading.

No `DIALOG_TAIL_NONCE` can change which inputs are tested. A genuinely-correct circuit passes any
seed; a truncation-gamed circuit fails. `run_tests` is otherwise unchanged.

## Rules

1. Only `src/point_add/` may be edited (as upstream). `eval_circuit` is the trusted grader.
2. A submission is **valid** iff it passes `eval_circuit` (0 classical / 0 phase / 0 ancilla) on
   **K ≥ 8 independent grader seeds** (the grader's choice; run with no `GRADER_SEED` for fresh
   randomness, or a published `GRADER_SEED` set for reproducibility).
3. **Score = peak_qubits × avg_executed_Toffoli**, lower is better — measured on a *passing* run.
4. Disclosed, *bounded* approximation is allowed only if declared and counted (as in legitimate
   quantum-arithmetic resource estimation); undisclosed truncation that fails any independent seed
   is not a valid circuit.

## Leaderboard

See [`LEADERBOARD-SOUND.md`](./LEADERBOARD-SOUND.md). The reference frontier is Google's correct
circuits (1175q / 1425q); the open goal is a *correct* circuit below them.

## Reproduce
```bash
cargo build --release --bin build_circuit --bin eval_circuit
<config-env> ./target/release/build_circuit      # writes ops.bin
./target/release/eval_circuit --note mytry        # fresh random grader seed each run
# or reproducibly:  GRADER_SEED=<hex> ./target/release/eval_circuit
```

## Provenance & credit
- Challenge harness + circuit lineage: **ecdsa.fail** (Eigen Labs / `gpsanant`).
- Circuit/resource-estimation methodology: **Google**, "Securing Elliptic Curve Cryptocurrencies
  against Quantum Vulnerabilities" (arXiv 2603.28846) — including the ZKP-validation approach this
  fork's soundness goal is modeled on.
- This fork's contribution: the validator-soundness correction (`sound_seed`) and an honest baseline.
