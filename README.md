# ECDSA.fail · SOUND

A **sound (ungameable)** fork of [ecdsa.fail](https://www.ecdsa.fail) — the benchmark that scores how
*cheap* a quantum computer breaking secp256k1 ECDSA can be, on a reversible elliptic-curve
**point-addition** circuit (the inner loop of Shor's algorithm). Metric: **peak qubits × executed
Toffoli**, lower is better.

**Maintained by [Alejandro Gutierrez (@Alex-Zeo)](https://github.com/Alex-Zeo).** Forked from Eigen
Labs' ecdsa.fail; circuit lineage + the ZKP-validation standard from Google
([arXiv 2603.28846](https://arxiv.org/abs/2603.28846)). Leaderboard: **https://ecdsa.redcorsair.ai**

## Why this fork exists

A benchmark is only as honest as its grader. Upstream `eval_circuit` seeds its 9,024 test inputs from
a **Fiat-Shamir hash of the contestant's own op stream**. The *intent* is anti-hardcoding — every
circuit gets different test points, so you can't pre-bake a fixed answer table. The *flaw*: the
contestant **controls the op stream**, so appending cost-free no-op gates (`DIALOG_TAIL_NONCE`)
re-rolls *which* inputs are tested until the circuit's errors fall outside the sample. The circuit is
still **wrong on reachable inputs** — it just chose its own exam. (Verified: a circuit near the top of
the upstream board fails **17–22 of a fresh random sample**.)

> **The fix is one function.** `eval_circuit::sound_seed()` draws the test inputs from a
> **grader-controlled source independent of the op stream** (fresh OS randomness, or a fixed
> `GRADER_SEED` for reproducible scoring). That kills both hardcoding *and* nonce-hunting: the
> contestant can neither predict nor steer the inputs. Only a genuinely-correct circuit
> (0 classical / 0 phase / 0 ancilla across independent seeds — the bar Google met with a
> zero-knowledge proof) scores.

## Current frontier (sound, correct circuits only)

| circuit | peak qubits | avg Toffoli | score (q×T) | validation |
|---|---|---|---|---|
| Google (low-qubit, ZKP) | **1175** | — | — | reference / target |
| **phase-conditioned comparator (SOUND-OPT-5)** | **2034** | **3,192,240** | **6,493,016,160** | 0/0/0 × 8 fresh seeds |
| borrowed-carry comparator (SOUND-OPT-3) | 2034 | 3,398,102 | 6,911,739,468 | 0/0/0 × 8 |
| measured SET-carry comparator (SOUND-OPT-2) | 2039 | 3,398,114 | 6,928,754,446 | 0/0/0 × 8 |
| windowed comparator (SOUND-OPT-4) — *lowest peak* | **1926** | 3,603,901 | 6,941,113,326 | 0/0/0 × 8 (q×T regression) |
| value-exact baseline (SOUND-OPT-1) | 2292 | 3,294,353 | 7,550,657,076 | 0/0/0 × ~35 |
| ~~race-1217 (upstream "frontier")~~ | ~~1217~~ | ~~1,401,748~~ | ~~1,705,927,316~~ | **INVALID** — fails under independent seeds |

q×T SOTA: **6,493,016,160** (SOUND-OPT-5 — phase-conditioned comparator replay, −6.06% via halving the
per-iteration underflow comparator's *executed* Toffoli at a held 2034 peak). Lowest *peak* is **1926**
(SOUND-OPT-4, but a q×T regression). The open problem is closing the gap to Google's **1175** *honestly*.
See `SOUND-OPT-{1,2,3,4,5}.md` and `SOUND-BASELINE.md`.

## Reproduce

```bash
cargo build --release --bin build_circuit --bin eval_circuit
<canonical-config> DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured_borrowed ./target/release/build_circuit
./target/release/eval_circuit                # fresh grader seed each run
GRADER_SEED=<hex> ./target/release/eval_circuit   # reproducible
```
(canonical config = `SOUND-BASELINE.md` §3.) Only `src/point_add/` is contestant-editable;
`eval_circuit` is the trusted grader.

## Contributing

Read **[CONTRIBUTING.md](CONTRIBUTING.md)** — it gives contributing humans *and LLM agents* the project
context and a **submission-note template** so every contribution carries a rich, verifiable writeup
(the change, the sound-validation evidence, the metrics, the delta vs SOTA).

## Roadmap
- **v1 (now):** open repo + deployed read-only leaderboard.
- **v2:** GitHub sign-in → API keys → submission endpoint → server-side sound-grading, so anyone can
  contribute correct circuits (with their own GitHub profile + rich note on the board).

## Credits
Forked from **ECDSA.fail** by **Eigen Labs**. Reversible secp256k1 point-add circuit lineage and the
zero-knowledge-proof validation standard from **Google** (arXiv 2603.28846). The soundness correction
(`sound_seed`) and the value-exact / borrowed-carry levers are this fork's contribution.
