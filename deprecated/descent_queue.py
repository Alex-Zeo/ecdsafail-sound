#!/usr/bin/env python3
"""Descent queue for the SOUND secp256k1 point-add benchmark.

Reuses the mining-fleet pattern (experiments + worker_status + experiment_log; atomic claim;
heartbeat). Two-stage: cheap PEAK-PROBE (build_circuit, runs on small boxes) front stage →
auto-promote any peak < 2039 to a SOUND-CHECK (K fresh-seed evals, all 0/0/0; H1 only, RAM-heavy).
Every experiment's knobs are ADDED to the canonical SOTA config (BASE). NO truncation, NO nonce
hunt — the sound grader seeds inputs independently, so each eval is an independent fresh test.

  descent_queue.py --init | --seed cfgs.json | --run | --status
"""
import argparse, json, os, re, sqlite3, subprocess, threading, time
from datetime import datetime, timezone

DB = os.path.expanduser("~/Documents/ecdsafail-sound/descent_queue.db")
NOW = lambda: datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
REMOTE = "/workspace/sound"
SOTA_PEAK = 2039  # promote/accept only configs that beat this
# canonical SOUND config; experiment knobs are appended. (= LEADERBOARD-SOUND.md reproduce line)
BASE = ("DIALOG_GCD_FOLD_CARRY_TRUNC_W=0 KAL_DOUBLE_CARRY_TRUNC_W=0 KAL_FOLD_CARRY_TRUNC_W=0 "
        "DIALOG_GCD_COMPARE_BITS=256 DIALOG_GCD_PA9024_COMPARE_SCHEDULE=0 SQUARE_ROW_WINDOW_CLEAN_COMPARE_BITS=0 "
        "DIALOG_GCD_RAW_APPLY_TRUNCATED_CLEAN=0 ROUND84_INPLACE_QUOTIENT_CARRY_TRUNC_W=256 "
        "DIALOG_GCD_BODY_CARRY_BAND_TRIMS= DIALOG_GCD_BODY_CARRY_TRUNC_W=0 "
        "DIALOG_GCD_SPECIAL_OVERFLOW_CLEAN_STEP_BITS= DIALOG_GCD_SPECIAL_UNDERFLOW_CLEAN_STEP_BITS= "
        "DIALOG_GCD_BINDER_NOTCH_STEPS= DIALOG_GCD_BINDER_NOTCH_MAP= "
        "DIALOG_GCD_RAW_TOBITVECTOR_VARIABLE_WIDTH=1 DIALOG_GCD_WIDTH_MARGIN=80 DIALOG_GCD_WIDTH_SLOPE_X1000=707 "
        "DIALOG_GCD_ACTIVE_ITERATIONS=402 ROUND84_PROD65=1 ROUND84_SQ_CARRY_HOST=1 ROUND84_SQ_CARRY_HOST_CEILING=1217 "
        "DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured")
# (name, ip, can_soundcheck). Set can_soundcheck only on boxes with RAM for the 2039q eval.
# H2-H5 feasibility is probed before the run; this list is the default (edit after the probe).
HOSTS = [("H1", "5.78.220.159", True), ("H2", "5.78.209.92", False),
         ("H3", "5.78.216.161", False), ("H4", "5.78.216.215", False), ("H5", "5.78.216.242", False)]
SSH = ["ssh", "-o", "StrictHostKeyChecking=no", "-o", "ConnectTimeout=10", "-o", "BatchMode=yes"]
SOUND_K = 8  # fresh-seed evals required, all 0/0/0
SCHEMA = """
CREATE TABLE IF NOT EXISTS experiments(
  id INTEGER PRIMARY KEY AUTOINCREMENT, config_id TEXT, knobs TEXT NOT NULL, kind TEXT NOT NULL DEFAULT 'peak',
  priority INTEGER DEFAULT 5, source TEXT, why TEXT, status TEXT NOT NULL DEFAULT 'pending',
  host TEXT, peak INTEGER, avg_t TEXT, sound TEXT, binder TEXT, abort_reason TEXT,
  claimed_at TEXT, done_at TEXT, UNIQUE(config_id, kind));
CREATE TABLE IF NOT EXISTS worker_status(worker_name TEXT PRIMARY KEY, host TEXT, status TEXT DEFAULT 'idle',
  current_experiment_id INTEGER, last_heartbeat TEXT);
CREATE TABLE IF NOT EXISTS experiment_log(id INTEGER PRIMARY KEY AUTOINCREMENT, experiment_id INTEGER, ts TEXT, event TEXT, message TEXT);
"""

def conn():
    c = sqlite3.connect(DB, timeout=30); c.execute("PRAGMA journal_mode=WAL"); return c
def log(c, eid, ev, msg):
    c.execute("INSERT INTO experiment_log(experiment_id,ts,event,message) VALUES(?,?,?,?)", (eid, NOW(), ev, msg)); c.commit()

def do_init():
    c = conn(); c.executescript(SCHEMA); c.commit(); print("init", DB)
def do_seed(path):
    c = conn(); n = 0
    for x in json.load(open(path)):
        try:
            c.execute("INSERT OR IGNORE INTO experiments(config_id,knobs,kind,priority,source,why) VALUES(?,?,?,?,?,?)",
                      (x.get("config_id"), x["knobs"], x.get("kind", "peak"), x.get("priority", 5), x.get("source", ""), x.get("why", "")))
            n += 1
        except Exception as e: print("skip", x.get("config_id"), e)
    c.commit()
    print(f"seeded {n}; pending={c.execute(chr(83)+'ELECT COUNT(*) FROM experiments WHERE status=?', ('pending',)).fetchone()[0]}")

def claim(c, can_sc):
    cur = c.cursor(); cur.execute("BEGIN IMMEDIATE")
    row = cur.execute("SELECT id,config_id,knobs,kind FROM experiments WHERE status='pending' AND (?=1 OR kind!='sound') ORDER BY priority,id LIMIT 1",
                      (1 if can_sc else 0,)).fetchone()
    if not row: cur.execute("COMMIT"); return None
    cur.execute("UPDATE experiments SET status='running',claimed_at=? WHERE id=?", (NOW(), row[0])); cur.execute("COMMIT"); return row

def run_peak(ip, knobs):
    cmd = SSH + [f"root@{ip}", f"cd {REMOTE} && {BASE} {knobs} TRACE_PEAK=1 ./target/release/build_circuit 2>&1 | "
                 f"grep -m1 'peak_qubits=' ; rm -f {REMOTE}/*.bin 2>/dev/null"]
    out = subprocess.run(cmd, capture_output=True, text=True, timeout=300).stdout
    m = re.search(r"peak_qubits=(\d+) at phase='([^']+)'", out)
    return (int(m.group(1)), m.group(2)) if m else (None, out.strip()[:100])

def run_soundcheck(ip, knobs):
    # build once, then K fresh-seed evals (sound_seed = fresh OS-random per run); require all 0/0/0
    build = SSH + [f"root@{ip}", f"cd {REMOTE} && {BASE} {knobs} ./target/release/build_circuit >/dev/null 2>&1 && echo BUILT"]
    if "BUILT" not in subprocess.run(build, capture_output=True, text=True, timeout=300).stdout:
        return {"sound": False, "peak": None, "avg_t": "?", "passed": "0/0", "note": "build failed"}
    peak = None; avg_t = "?"; passed = 0
    for _ in range(SOUND_K):
        cmd = SSH + [f"root@{ip}", f"cd {REMOTE} && ./target/release/eval_circuit --note dq 2>&1 | "
                     f"grep -E 'qubits *:|classical mismatch|phase-garbage|ancilla-garbage|avg executed'"]
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=600).stdout
        pk = re.search(r"qubits\s*:\s*(\d+)", out); cl = re.search(r"classical mismatches\s*:\s*(\d+)", out)
        ph = re.search(r"phase-garbage batches\s*:\s*(\d+)", out); an = re.search(r"ancilla-garbage batches\s*:\s*(\d+)", out)
        at = re.search(r"avg executed Toffoli\s*:\s*([\d.]+)", out)
        if pk and peak is None: peak = int(pk.group(1))
        if at and avg_t == "?": avg_t = at.group(1)
        if cl and ph and an and cl.group(1) == "0" and ph.group(1) == "0" and an.group(1) == "0":
            passed += 1
        else:
            return {"sound": False, "peak": peak, "avg_t": avg_t, "passed": f"{passed}/{SOUND_K}", "note": "fresh-seed mismatch"}
    subprocess.run(SSH + [f"root@{ip}", f"rm -f {REMOTE}/*.bin 2>/dev/null"], capture_output=True, timeout=30)
    return {"sound": True, "peak": peak, "avg_t": avg_t, "passed": f"{passed}/{SOUND_K}", "note": "verified sound"}

def worker(name, ip, can_sc):
    c = conn(); c.execute("INSERT OR REPLACE INTO worker_status VALUES(?,?,?,?,?)", (name, ip, "idle", None, NOW())); c.commit()
    while True:
        row = claim(c, can_sc)
        if not row:
            c.execute("UPDATE worker_status SET status='idle',last_heartbeat=? WHERE worker_name=?", (NOW(), name)); c.commit(); time.sleep(8)
            if c.execute("SELECT COUNT(*) FROM experiments WHERE status='pending'").fetchone()[0] == 0: break
            continue
        eid, cid, knobs, kind = row
        c.execute("UPDATE worker_status SET status='busy',current_experiment_id=?,last_heartbeat=? WHERE worker_name=?", (eid, NOW(), name)); c.commit()
        log(c, eid, "claimed", f"{name} {kind} {knobs}")
        try:
            if kind == "sound":
                r = run_soundcheck(ip, knobs)
                c.execute("UPDATE experiments SET status='completed',host=?,peak=?,avg_t=?,sound=?,abort_reason=?,done_at=? WHERE id=?",
                          (name, r["peak"], r["avg_t"], "1" if r["sound"] else "0", f"{r['passed']} {r['note']}", NOW(), eid))
                log(c, eid, "sound", f"SOUND={r['sound']} peak={r['peak']} avgT={r['avg_t']} {r['passed']}")
            else:
                peak, binder = run_peak(ip, knobs)
                c.execute("UPDATE experiments SET status='completed',host=?,peak=?,binder=?,done_at=? WHERE id=?", (name, peak, binder, NOW(), eid))
                log(c, eid, "peak", f"peak={peak} ({binder})")
                if peak and peak < SOTA_PEAK:  # promote sub-2039 hit to a sound-check
                    c.execute("INSERT OR IGNORE INTO experiments(config_id,knobs,kind,priority,source,why) VALUES(?,?,?,?,?,?)",
                              (cid, knobs, "sound", 1, "auto", f"peak {peak}<{SOTA_PEAK} -> verify sound"))
                    log(c, eid, "promote", f"peak {peak}<{SOTA_PEAK} -> queued sound-check")
            c.commit()
        except Exception as e:
            c.execute("UPDATE experiments SET status='failed',host=?,abort_reason=?,done_at=? WHERE id=?", (name, str(e)[:200], NOW(), eid)); log(c, eid, "failed", str(e)[:200]); c.commit()
        c.execute("UPDATE worker_status SET last_heartbeat=? WHERE worker_name=?", (NOW(), name)); c.commit()
    print(f"[{name}] drained")

def do_run():
    ts = [threading.Thread(target=worker, args=h, daemon=True) for h in HOSTS]
    for t in ts: t.start()
    for t in ts: t.join()

def do_status():
    c = conn(); print("QUEUE:", dict(c.execute("SELECT status,COUNT(*) FROM experiments GROUP BY status").fetchall()))
    print("WORKERS:")
    for n, h, st, eid, hb in c.execute("SELECT worker_name,host,status,current_experiment_id,last_heartbeat FROM worker_status ORDER BY worker_name"):
        print(f"  {n} ({h}): {st} exp={eid} hb={hb}")
    print(f"SUB-{SOTA_PEAK} PEAK HITS:")
    for cid, pk, b in c.execute(f"SELECT config_id,peak,binder FROM experiments WHERE kind='peak' AND status='completed' AND peak<{SOTA_PEAK} ORDER BY peak"):
        print(f"  {cid}: peak={pk} ({b})")
    print("SOUND-VERIFIED:")
    for cid, pk, at, snd in c.execute("SELECT config_id,peak,avg_t,sound FROM experiments WHERE kind='sound' AND status='completed' ORDER BY peak"):
        print(f"  {cid}: peak={pk} avgT={at} sound={'YES' if snd=='1' else 'NO'}")

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--init", action="store_true"); ap.add_argument("--seed"); ap.add_argument("--run", action="store_true"); ap.add_argument("--status", action="store_true")
    a = ap.parse_args()
    if a.init: do_init()
    elif a.seed: do_seed(a.seed)
    elif a.run: do_run()
    elif a.status: do_status()
    else: ap.print_help()
