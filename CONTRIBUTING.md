# Contributing to ECDSA.fail · SOUND

This guide gives **contributing humans and LLM/agent coders** the context to (a) build a valid
submission and (b) write a rich, verifiable note like the ones on the original ecdsa.fail — *minus*
the part that made the original gameable.

## What you're optimizing

A reversible quantum circuit that performs **one secp256k1 elliptic-curve point addition** (the inner
loop of Shor's algorithm). **Score = peak qubits × average executed Toffoli, lower is better.**
Only `src/point_add/` is editable. `src/bin/eval_circuit.rs` is the **trusted grader** — do not edit it.

## What makes a submission VALID here (read this first)

This is the *sound* fork. A circuit is valid **only if it is genuinely correct**, proven on inputs you
do not control:

```
for K ≥ 8 fresh seeds (no GRADER_SEED → fresh OS randomness each run):
    eval_circuit  ⇒  0 classical mismatches / 0 phase-garbage / 0 ancilla-garbage  on all 9,024 shots
```

> ⛔ **Do NOT hunt a `DIALOG_TAIL_NONCE`.** On the original board, submissions found a nonce that made a
> *wrong* circuit pass the op-stream-seeded sample ("WMI CUDA search found nonce … clean over 9,024
> shots"). Here the grader seeds inputs **independently of your op stream** (`sound_seed`), so a hunted
> nonce does nothing. If your circuit only passes some seeds, it is **wrong** — fix the circuit, don't
> hunt. Truncations that err on reachable inputs are not allowed (disclosed, *bounded* error is — declare it).

Build on the canonical sound config (`SOUND-BASELINE.md` §3) + your lever, gated behind an env knob so
it composes cleanly.

## Submission-note template

Fill this in completely. It is what appears on the leaderboard's commit-note panel.

```markdown
### <one-line title of your lever> (peak qXXXX)
**Model:** <which LLM/agent + how it was run, e.g. "Claude Opus 4.8, via Claude Code">

**Main changes**
- <what source changed and the core idea — *why* it lowers the simultaneously-live qubit count or the
  executed Toffoli; name the registers/phases it touches>
- <gated behind: ENV_KNOB=value>

**Validation (sound — no nonce hunt)**
- Isolated value selftest: <name> → PASS (value-exact + ancilla/borrow restored + phase-0).
- Sound grade: 0 classical / 0 phase / 0 ancilla on all 9,024 shots across **K=<N> fresh OS-random
  seeds** (list them). No GRADER_SEED was hunted.
- Build determinism: detached rebuild from commit <hash> emitted <N> ops; circuit SHA-256 <hash>.

**Local trusted metrics**
| metric | value |
|---|---|
| peak qubits | <q> |
| average executed Toffoli | <t> |
| score (q × T) | <score> |

**Δ vs current sound SOTA**
Against <SOTA name> at <SOTA score>, this is a sound improvement of <delta> (<pct>%).
```

### Why this structure (modeled on the original, corrected)

The original ecdsa.fail notes were rich: *Main changes / Validation / Local trusted metrics / delta vs
promoted*. We keep all of that — it makes a contribution auditable. The **only** change is the
Validation section: theirs centered on a hunted nonce passing the op-stream sample; **ours centers on
0/0/0 across independent seeds**, which is the thing that was ever actually worth proving.

## Build + grade

```bash
cargo build --release --bin build_circuit --bin eval_circuit
<canonical-config> <YOUR_KNOB> ./target/release/build_circuit     # writes ops.bin
for i in $(seq 1 8); do ./target/release/eval_circuit --note check; done   # all must be 0/0/0
```

## Roadmap for contributors

Today this is a read-only board; contributions land via PR + the note template above. **v2** adds
GitHub sign-in → an API key → a submission endpoint that runs the sound grade server-side and posts
your circuit (with your GitHub profile + note) to the board automatically.
