"""Elaborate generated proofs through the real judge."""
import sys, os, json
# Run this from the Stage 2 checkout, so `judge` and `pipeline` are importable,
# with this directory on the path:
#   PYTHONPATH=/path/to/prover python3 judgecheck.py problems.json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ".")
import drive as D
from judge.verify import verify_answer, _find_banned_token
from pipeline.proxy import DEFAULT_PROOF_POLICY


def wrap(body):
    return ("import JudgeProblem\n\ndef submission : Goal := by\n  intro G _ h\n"
            + "\n".join("  " + l for l in body.split("\n")) + "\n")


def check(p, budget=25.0, **kw):
    path, dt, err = D.run_complete(p, budget=budget, **kw)
    if path is None:
        path, dt2, err = D.run(p, budget=budget)
        dt += dt2
    if path is None:
        return "no-derivation", dt, None
    if err:
        return f"UNSOUND {err}", dt, None
    L, R, GL, GR, hv, gv = D.prep(p)
    code = wrap(D.emit(path, GL, GR, hv, gv))
    ban = _find_banned_token(code)
    if ban:
        return f"BANNED {ban}", dt, code
    spec = dict(p); spec["proof_policy"] = DEFAULT_PROOF_POLICY
    r = verify_answer(spec, json.dumps({"verdict": "true", "code": code}))
    return r.get("status"), dt, (code if r.get("status") != "accepted" else None), r


if __name__ == "__main__":
    probs = json.load(open(sys.argv[1]))
    pref = sys.argv[2] if len(sys.argv) > 2 else ""
    budget = float(sys.argv[3]) if len(sys.argv) > 3 else 25.0
    ok = 0
    for p in probs:
        if pref and not p["id"].startswith(pref):
            continue
        out = check(p, budget=budget)
        status, dt = out[0], out[1]
        if status == "accepted":
            ok += 1
        print(f"  {p['id']:22} {p['eq1_id']:>6}->{p['eq2_id']:<6} {status:<16} {dt:5.1f}s")
        if status not in ("accepted", "no-derivation") and len(out) > 3:
            print("     ", (out[3].get("stderr") or out[3].get("message") or "")[:300].replace("\n", " "))
    print(f"\n  judge-accepted: {ok}")
