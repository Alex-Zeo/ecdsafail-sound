#!/usr/bin/env python3
"""quantum_runner — SSOT queue + LOCAL worker pool for the SOUND secp256k1 point-add descent.

Runs ON the dedicated CCX33 (no SSH fleet — single box). A local SQLite SSOT + a thread pool;
each worker atomically claims an experiment and runs it as a LOCAL subprocess in its own temp CWD
(isolated ops.bin per worker). Two stages, mining-fleet style:
  - PEAK probe : build_circuit (TRACE_PEAK) -> peak qubits. Sub-SOTA hits auto-promote (depends_on).
  - SOUND check: build once + eval_circuit K=8 times with FRESH grader seeds; valid iff every run
    is 0 classical / 0 phase / 0 ancilla (the sound, ungameable bar).
Every experiment's knobs are appended to the canonical SOTA config (BASE). NO truncation, NO hunt.

  quantum_runner.py --init | --seed cfgs.json | --run [--workers N] | --status
"""
import argparse, json, os, re, shutil, sqlite3, subprocess, tempfile, threading, time
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
DB = os.path.join(HERE, "quantum.sqlite")
BIN = os.environ.get("QR_BIN", "/workspace/sound/target/release")
TMP = os.path.join(HERE, "tmp")
NOW = lambda: datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
SOTA_PEAK = int(os.environ.get("QR_SOTA", "2034"))   # promote/accept only peak < this
SOUND_K = int(os.environ.get("QR_K", "8"))           # fresh-seed evals; all must be 0/0/0
BASE = ("DIALOG_GCD_FOLD_CARRY_TRUNC_W=0 KAL_DOUBLE_CARRY_TRUNC_W=0 KAL_FOLD_CARRY_TRUNC_W=0 "
        "DIALOG_GCD_COMPARE_BITS=256 DIALOG_GCD_PA9024_COMPARE_SCHEDULE=0 SQUARE_ROW_WINDOW_CLEAN_COMPARE_BITS=0 "
        "DIALOG_GCD_RAW_APPLY_TRUNCATED_CLEAN=0 ROUND84_INPLACE_QUOTIENT_CARRY_TRUNC_W=256 "
        "DIALOG_GCD_BODY_CARRY_BAND_TRIMS= DIALOG_GCD_BODY_CARRY_TRUNC_W=0 "
        "DIALOG_GCD_SPECIAL_OVERFLOW_CLEAN_STEP_BITS= DIALOG_GCD_SPECIAL_UNDERFLOW_CLEAN_STEP_BITS= "
        "DIALOG_GCD_BINDER_NOTCH_STEPS= DIALOG_GCD_BINDER_NOTCH_MAP= "
        "DIALOG_GCD_RAW_TOBITVECTOR_VARIABLE_WIDTH=1 DIALOG_GCD_WIDTH_MARGIN=80 DIALOG_GCD_WIDTH_SLOPE_X1000=707 "
        "DIALOG_GCD_ACTIVE_ITERATIONS=402 ROUND84_PROD65=1 ROUND84_SQ_CARRY_HOST=1 ROUND84_SQ_CARRY_HOST_CEILING=1217 "
        "DIALOG_GCD_UNDERFLOW_CLEAN_CMP=acc_plus_f_measured_borrowed")
SCHEMA = """
CREATE TABLE IF NOT EXISTS experiments(
  id INTEGER PRIMARY KEY AUTOINCREMENT, config_id TEXT, knobs TEXT NOT NULL, kind TEXT NOT NULL DEFAULT 'peak',
  priority INTEGER DEFAULT 5, depends_on INTEGER, persona TEXT, why TEXT, status TEXT NOT NULL DEFAULT 'pending',
  worker TEXT, peak INTEGER, avg_t TEXT, sound TEXT, binder TEXT, score INTEGER, abort_reason TEXT,
  claimed_at TEXT, done_at TEXT, UNIQUE(config_id, kind));
CREATE TABLE IF NOT EXISTS worker_status(worker TEXT PRIMARY KEY, status TEXT, current_experiment_id INTEGER, last_heartbeat TEXT, n_done INTEGER DEFAULT 0);
CREATE TABLE IF NOT EXISTS experiment_log(id INTEGER PRIMARY KEY AUTOINCREMENT, experiment_id INTEGER, ts TEXT, event TEXT, message TEXT);
"""

def conn():
    c = sqlite3.connect(DB, timeout=60); c.execute("PRAGMA journal_mode=WAL"); c.execute("PRAGMA busy_timeout=10000"); return c
def log(c, eid, ev, msg):
    c.execute("INSERT INTO experiment_log(experiment_id,ts,event,message) VALUES(?,?,?,?)", (eid, NOW(), ev, msg)); c.commit()

def envmap(knobs):
    e = dict(os.environ)
    for tok in (BASE + " " + (knobs or "")).split():
        if "=" in tok:
            k, v = tok.split("=", 1); e[k] = v
    return e

def run_peak(knobs):
    os.makedirs(TMP, exist_ok=True); d = tempfile.mkdtemp(dir=TMP)
    try:
        e = envmap(knobs); e["TRACE_PEAK"] = "1"
        out = subprocess.run([BIN + "/build_circuit"], cwd=d, env=e, capture_output=True, text=True, timeout=400).stdout
        m = re.search(r"peak_qubits=(\d+) at phase='([^']+)'", out)
        return (int(m.group(1)), m.group(2)) if m else (None, out.strip()[:120])
    finally:
        shutil.rmtree(d, ignore_errors=True)

def run_sound(knobs):
    os.makedirs(TMP, exist_ok=True); d = tempfile.mkdtemp(dir=TMP)
    try:
        e = envmap(knobs)
        if subprocess.run([BIN + "/build_circuit"], cwd=d, env=e, capture_output=True, timeout=400).returncode != 0:
            return {"sound": False, "peak": None, "avg_t": "?", "passed": "0/%d" % SOUND_K, "note": "build failed"}
        peak = None; avg_t = "?"; passed = 0
        for _ in range(SOUND_K):
            out = subprocess.run([BIN + "/eval_circuit", "--note", "qr"], cwd=d, capture_output=True, text=True, timeout=900).stdout
            pk = re.search(r"qubits\s*:\s*(\d+)", out); cl = re.search(r"classical mismatches\s*:\s*(\d+)", out)
            ph = re.search(r"phase-garbage batches\s*:\s*(\d+)", out); an = re.search(r"ancilla-garbage batches\s*:\s*(\d+)", out)
            at = re.search(r"avg executed Toffoli\s*:\s*([\d.]+)", out)
            if pk and peak is None: peak = int(pk.group(1))
            if at and avg_t == "?": avg_t = at.group(1)
            if cl and ph and an and cl.group(1) == "0" and ph.group(1) == "0" and an.group(1) == "0":
                passed += 1
            else:
                return {"sound": False, "peak": peak, "avg_t": avg_t, "passed": "%d/%d" % (passed, SOUND_K), "note": "fresh-seed mismatch"}
        return {"sound": True, "peak": peak, "avg_t": avg_t, "passed": "%d/%d" % (passed, SOUND_K), "note": "verified sound"}
    finally:
        shutil.rmtree(d, ignore_errors=True)

def claim(c):
    cur = c.cursor(); cur.execute("BEGIN IMMEDIATE")
    row = cur.execute("""SELECT id,config_id,knobs,kind FROM experiments WHERE status='pending'
        AND (depends_on IS NULL OR depends_on IN (SELECT id FROM experiments WHERE status='completed'))
        ORDER BY priority, id LIMIT 1""").fetchone()
    if not row: cur.execute("COMMIT"); return None
    cur.execute("UPDATE experiments SET status='running',claimed_at=? WHERE id=?", (NOW(), row[0])); cur.execute("COMMIT")
    return row

def worker(name):
    c = conn(); c.execute("INSERT OR REPLACE INTO worker_status(worker,status,last_heartbeat,n_done) VALUES(?,?,?,COALESCE((SELECT n_done FROM worker_status WHERE worker=?),0))", (name, "idle", NOW(), name)); c.commit()
    while True:
        row = claim(c)
        if not row:
            c.execute("UPDATE worker_status SET status='idle',last_heartbeat=? WHERE worker=?", (NOW(), name)); c.commit(); time.sleep(6)
            if c.execute("SELECT COUNT(*) FROM experiments WHERE status='pending'").fetchone()[0] == 0: break
            continue
        eid, cid, knobs, kind = row
        c.execute("UPDATE worker_status SET status='busy',current_experiment_id=?,last_heartbeat=? WHERE worker=?", (eid, NOW(), name)); c.commit()
        log(c, eid, "claimed", "%s %s %s" % (name, kind, knobs))
        try:
            if kind == "sound":
                r = run_sound(knobs); score = (r["peak"] * round(float(r["avg_t"]))) if (r["sound"] and r["avg_t"] not in ("?", None)) else None
                c.execute("UPDATE experiments SET status='completed',worker=?,peak=?,avg_t=?,sound=?,score=?,abort_reason=?,done_at=? WHERE id=?",
                          (name, r["peak"], r["avg_t"], "1" if r["sound"] else "0", score, "%s %s" % (r["passed"], r["note"]), NOW(), eid))
                log(c, eid, "sound", "SOUND=%s peak=%s avgT=%s score=%s %s" % (r["sound"], r["peak"], r["avg_t"], score, r["passed"]))
            else:
                peak, binder = run_peak(knobs)
                c.execute("UPDATE experiments SET status='completed',worker=?,peak=?,binder=?,done_at=? WHERE id=?", (name, peak, binder, NOW(), eid))
                log(c, eid, "peak", "peak=%s (%s)" % (peak, binder))
                if peak and peak < SOTA_PEAK:
                    c.execute("INSERT OR IGNORE INTO experiments(config_id,knobs,kind,priority,persona,why) VALUES(?,?,?,?,?,?)",
                              (cid, knobs, "sound", 1, "auto", "peak %d<%d -> sound-verify" % (peak, SOTA_PEAK)))
                    log(c, eid, "promote", "peak %d<%d -> queued sound-check" % (peak, SOTA_PEAK))
            c.execute("UPDATE worker_status SET n_done=n_done+1,last_heartbeat=? WHERE worker=?", (NOW(), name)); c.commit()
        except Exception as ex:
            c.execute("UPDATE experiments SET status='failed',worker=?,abort_reason=?,done_at=? WHERE id=?", (name, str(ex)[:200], NOW(), eid)); log(c, eid, "failed", str(ex)[:200]); c.commit()
    print("[%s] drained" % name)

def do_init():
    c = conn(); c.executescript(SCHEMA); c.commit(); print("init", DB)
def do_seed(path):
    c = conn(); n = 0
    for x in json.load(open(path)):
        try:
            c.execute("INSERT OR IGNORE INTO experiments(config_id,knobs,kind,priority,persona,why) VALUES(?,?,?,?,?,?)",
                      (x.get("config_id"), x["knobs"], x.get("kind", "peak"), x.get("priority", 5), x.get("persona", x.get("source", "")), x.get("why", ""))); n += 1
        except Exception as ex: print("skip", x.get("config_id"), ex)
    c.commit(); print("seeded %d; pending=%d" % (n, c.execute("SELECT COUNT(*) FROM experiments WHERE status=?", ("pending",)).fetchone()[0]))
def do_run(nw):
    ts = [threading.Thread(target=worker, args=("w%d" % i,), daemon=True) for i in range(nw)]
    for t in ts: t.start()
    for t in ts: t.join()
def do_status():
    c = conn(); print("QUEUE:", dict(c.execute("SELECT status,COUNT(*) FROM experiments GROUP BY status").fetchall()))
    for w, st, eid, hb, n in c.execute("SELECT worker,status,current_experiment_id,last_heartbeat,n_done FROM worker_status ORDER BY worker"):
        print("  %s: %s exp=%s done=%s hb=%s" % (w, st, eid, n, hb))
    print("SUB-%d PEAK HITS:" % SOTA_PEAK)
    for cid, pk, b in c.execute("SELECT config_id,peak,binder FROM experiments WHERE kind='peak' AND status='completed' AND peak<? ORDER BY peak", (SOTA_PEAK,)):
        print("  %s: peak=%s (%s)" % (cid, pk, b))
    print("SOUND-VERIFIED:")
    for cid, pk, at, sc in c.execute("SELECT config_id,peak,avg_t,score FROM experiments WHERE kind='sound' AND status='completed' AND sound='1' ORDER BY score"):
        print("  %s: peak=%s avgT=%s score=%s" % (cid, pk, at, sc))

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--init", action="store_true"); ap.add_argument("--seed"); ap.add_argument("--run", action="store_true"); ap.add_argument("--workers", type=int, default=7); ap.add_argument("--status", action="store_true")
    a = ap.parse_args()
    if a.init: do_init()
    elif a.seed: do_seed(a.seed)
    elif a.run: do_run(a.workers)
    elif a.status: do_status()
    else: ap.print_help()
