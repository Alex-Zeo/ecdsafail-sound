#!/usr/bin/env python3
"""Wire a panel-verified descent lever into the on-box SSOT loop.

Closes the loop: the 11-persona descent workflow produces ACCEPTED levers (sound, q×T-improving) on
git branches. This deploys the lever's branch to the box, rebuilds the grader binary, restarts the
quantum-runner daemon on the new binary, and seeds the lever's knob + a small param-sweep into the
SSOT — so the daemon re-verifies + explores the neighborhood. The SSOT-on-Hetzner analogue of the
mining fleet's enqueue_to_fleet.py.

  panel_to_ssot.py --branch <git-branch> --name <id> --knobs '<EXTRA_ENV_KNOBS>' [--no-restart]

Without --no-restart it makes the lever the daemon's live binary (use once a lever is the new SOTA);
with --no-restart it only seeds a sweep against the current binary (use for knob-space exploration of
a lever already in the deployed binary).
"""
import argparse, json, re, subprocess, tempfile
BOX = "5.78.68.226"
REPO = "/Users/pluto/Documents/ecdsafail-sound"
SSH = ["ssh", "-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null", "root@" + BOX]
def sh(c, **k): return subprocess.run(c, capture_output=True, text=True, **k)

def sweep(name, knobs):
    """lever config + a ±1 sweep over its trailing numeric knob (neighborhood exploration)."""
    cfgs = [{"config_id": name, "knobs": knobs, "kind": "peak", "persona": "panel", "why": "panel-verified lever"}]
    m = re.search(r'([A-Z_][A-Z0-9_]*)=(\d+)\s*$', knobs.strip())
    if m:
        k, v = m.group(1), int(m.group(2))
        for dv in sorted({max(1, v - 1), v + 1} - {v}):
            cfgs.append({"config_id": f"{name}_{k}{dv}", "knobs": re.sub(r'([A-Z_][A-Z0-9_]*)=\d+\s*$', f'{k}={dv}', knobs.strip()),
                         "kind": "peak", "persona": "panel-sweep", "why": f"sweep {k}={dv}"})
    return cfgs

def main():
    a = argparse.ArgumentParser()
    a.add_argument("--branch", required=True); a.add_argument("--name", required=True)
    a.add_argument("--knobs", default=""); a.add_argument("--no-restart", action="store_true")
    o = a.parse_args()
    name = re.sub(r'[^a-zA-Z0-9_]', '_', o.name)[:24]
    dest = "/workspace/sound" if not o.no_restart else "/workspace/sound_panel"
    print(f"deploy {o.branch} -> {dest} on the box ...")
    p = subprocess.Popen(["git", "-C", REPO, "archive", o.branch], stdout=subprocess.PIPE)
    subprocess.run(SSH + [f"rm -rf {dest} && mkdir -p {dest} && tar -x -C {dest}"], stdin=p.stdout)
    print("cargo build:", sh(SSH + [f"cd {dest} && . $HOME/.cargo/env && cargo build --release --bin build_circuit --bin eval_circuit 2>&1 | tail -1"]).stdout.strip())
    if not o.no_restart:
        print("restart daemon on new binary:", sh(SSH + ["systemctl restart quantum-runner && systemctl is-active quantum-runner"]).stdout.strip())
    cfgs = sweep(name, o.knobs)
    tf = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False); json.dump(cfgs, tf); tf.close()
    subprocess.run(["scp", "-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null", tf.name, f"root@{BOX}:/workspace/quantum/panel_seed.json"])
    env = "" if not o.no_restart else f"QR_BIN={dest}/target/release "
    print(sh(SSH + [f"cd /workspace/quantum && {env}python3 quantum_runner.py --seed panel_seed.json"]).stdout.strip())
    print(f"wired: {len(cfgs)} configs ({name} + sweep) seeded; daemon will auto-run them.")

if __name__ == "__main__":
    main()
