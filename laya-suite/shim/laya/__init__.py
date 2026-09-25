"""`laya` as kime: the public Laya 0.3.20 API, answered by the kime package.

Laya's tests put their parent folder first on sys.path and `import laya`, so a copy of them next
to this folder runs against kime unchanged. Only the modules Laya documents for users are here:
the package, `agent`, `router`, `lang`, `shortlist`, `structured`, `email` and `presets`. Laya's
torch internals (most of `common`, `hooks`, `fast`, `serve`, `mcp`, `cli`, `integrations`)
are left out on purpose, and a test that imports one fails. expected.tsv says which and why.
"""

import json
import os
import sys
import types

import kime
from kime import *  # noqa: F401,F403
from kime import router as router
from kime import shortlist as shortlist
from kime import structured as structured

__version__ = "0.3.20"

# Laya's __all__, less what kime does not have.
__all__ = [n for n in (
    "Agent", "BaseHook", "DEFAULT_MODELS", "DecisionResult", "Hook", "LayaEvaluator", "LayaGuardrail",
    "LayaGuardrailError", "LayaRouter", "LayaTriage", "PredictContext", "PredictHook", "QTYPES",
    "QTYPE_NAMES", "RLAgent", "RouteDecision", "Router", "__version__", "answer_confidence",
    "clean_email_body", "confidence_from_probs", "decide", "detect_language", "detect_script",
    "ece_score", "email_questions", "email_state", "embed_fn_from_agent", "guard_questions",
    "is_english", "load", "moderation_questions", "predict_shortlist", "proper_reward",
    "render_options", "router_questions", "shortlist_choice", "td_lambda_targets", "triage_questions",
) if n == "__version__" or hasattr(kime, n)]


def _module(name, **names):
    m = types.ModuleType("laya." + name)
    m.__dict__.update(names)
    sys.modules[m.__name__] = m
    return m


# KIME_IDENTIFIER=0 makes every Router route on Laya's word lists alone, as Laya does. By default
# the language identifier sits on top, as it does in kime.
if os.environ.get("KIME_IDENTIFIER") == "0":
    router.Router.__init__.__kwdefaults__["identifier"] = False

for _name, _mod in (("router", router), ("shortlist", shortlist), ("structured", structured)):
    sys.modules["laya." + _name] = _mod

def _analyse(state):
    return json.loads(kime._native.analyse(json.dumps(state)))


def _clamp_temperature(t, lo=0.5, hi=5.0):
    try:
        t = float(t)
    except (TypeError, ValueError):
        return 1.0
    if t != t or t in (float("inf"), float("-inf")):
        return 1.0
    return min(hi, max(lo, t))


def _temp_bucket(qtype, k):
    size = "2" if k <= 2 else "3-5" if k <= 5 else "6-10" if k <= 10 else "11+"
    return "%s:%s" % (("choice", "score", "noul")[int(qtype)], size)


# `laya.agent` is the kime package itself, so a test that patches `laya.agent.Agent` patches the
# class `kime.Router` builds.
agent = sys.modules["laya.agent"] = kime
lang = _module(
    "lang",
    detect_script=kime.detect_script,
    is_english=kime.is_english,
    analyse=_analyse,
    guess_latin_language=router.guess_latin_language,
    state_text=router.state_text,
    _STOP={lg: set(words) for lg, words in kime._native.stop_words()},
)
# Only the constants and the two temperature helpers of laya.common, which test_router imports
# next to the routing it checks. kime applies the same clamp and buckets in Rust.
common = _module(
    "common",
    QTYPES={"choice": 0, "score": 1, "noul": 2},
    QTYPE_NAMES={0: "choice", 1: "score", 2: "noul"},
    TEMP_MIN=0.5,
    TEMP_MAX=5.0,
    clamp_temperature=_clamp_temperature,
    temp_bucket=_temp_bucket,
)
email = _module(
    "email",
    clean_email_body=kime.clean_email_body,
    email_state=kime.email_state,
    email_questions=kime.email_questions,
)
# Laya's ONNX Runtime agent runs the same checkpoint without torch. kime is that too, so it is the
# same Agent.
onnx_agent = _module("onnx_agent", ONNXAgent=kime.Agent)
presets = _module(
    "presets",
    email_questions=kime.email_questions,
    guard_questions=kime.guard_questions,
    moderation_questions=kime.moderation_questions,
    router_questions=kime.router_questions,
    triage_questions=kime.triage_questions,
)
