"""Replays the committed requests through the TypeSafe Python SDK against a server and checks that
the SDK's typed parsing takes every response, the way a TypeSafe user's code would see it.

    python sdk-python/run.py http://127.0.0.1:8000 [texts.json]

It needs `typesafe-sdk` 0.7.1 and nothing else, and the SDK is used as published, through its
own environment variables. Each fixture under fixtures/systemone/ is sent with the sync client
and again with the async one, `/v1/models` is listed, and the error cases check that the SDK
raises the class a TypeSafe user catches. With a JSON list of strings, each fixture's questions
are also asked about every text. It exits non-zero on the first kind of failure it counts.
"""

import asyncio
import json
import os
import pathlib
import sys
import time

import typesafe_sdk as t

HERE = pathlib.Path(__file__).resolve().parent
FIXTURES = HERE.parent / "fixtures" / "systemone"
KINDS = {"noul": t.Noul, "choice": t.Choice, "score": t.Score}
ANSWERS = {"noul": t.NoulAnswer, "choice": t.ChoiceAnswer, "score": t.ScoreAnswer}


def questions(raw):
    out = {}
    for qid, q in raw.items():
        fields = {k: v for k, v in q.items() if k != "type"}
        # The SDK's Choice takes a dict, so a list of labels is written as labels with no text.
        if q["type"] == "choice" and isinstance(fields.get("criteria"), list):
            fields["criteria"] = {label: None for label in fields["criteria"]}
        out[qid] = KINDS[q["type"]](**fields)
    return out


def check(r, raw):
    """What the SDK parsed must be what was asked, in the typed form the SDK promises."""
    assert isinstance(r, t.SystemOneResponse), type(r)
    assert list(r.answers) == list(raw), (list(r.answers), list(raw))
    assert r.model and r.model != "jev-latest", r.model
    assert isinstance(r.usage, t.Usage) and r.usage.input_tokens > 0
    for qid, q in raw.items():
        a = r.answers[qid]
        assert isinstance(a, ANSWERS[q["type"]]), (qid, type(a))
        if q["type"] == "noul":
            assert 0.0 <= r.nouls[qid].noul <= 1.0
        elif q["type"] == "choice":
            labels = [str(k) for k in q["criteria"]]
            assert r.choices[qid].choice in labels and sorted(a.probabilities) == sorted(labels)
        else:
            s = r.scores[qid]
            assert 0.0 <= s.score <= len(q["criteria"]) - 1
            assert [s.legend[i] for i in sorted(s.legend)] == list(q["criteria"])
            assert sorted(s.probabilities) == list(range(len(q["criteria"])))
    return r


def errors(client):
    cases = [
        ("unknown model", lambda: client.system_one("x", {"a": t.Noul()}, model="no-such-model"), t.TypeSafeNotFoundError),
        ("empty choice", lambda: client.system_one("x", {"a": t.Choice(criteria={})}), t.TypeSafeError),
        ("no questions", lambda: client.system_one("x", {}), t.TypeSafeError),
    ]
    for name, call, kind in cases:
        try:
            call()
        except kind:
            continue
        except Exception as e:
            raise AssertionError("%s: got %s: %s" % (name, type(e).__name__, e))
        raise AssertionError("%s: no error" % name)
    return len(cases)


async def run_async(requests):
    async with t.AsyncTypeSafeClient() as client:
        got = await asyncio.gather(*(client.system_one(req["state"], questions(req["questions"])) for req in requests))
    return [check(r, req["questions"]) for r, req in zip(got, requests)]


def main():
    url = sys.argv[1]
    os.environ["TYPESAFE_BASE_URL"] = url
    os.environ.setdefault("TYPESAFE_API_KEY", "kime-compat")
    texts = json.loads(pathlib.Path(sys.argv[2]).read_text()) if len(sys.argv) > 2 else []
    requests = [json.loads((d / "request.json").read_text()) for d in sorted(FIXTURES.iterdir()) if d.is_dir()]

    with t.TypeSafeClient() as client:
        models = client.models.list()
        assert isinstance(models, t.ListModelsResponse) and models.models
        assert all(isinstance(m, t.ModelMetadata) for m in models.models)
        sync = [check(client.system_one(req["state"], questions(req["questions"])), req["questions"]) for req in requests]
        n_errors = errors(client)
        took = []
        for text in texts:
            for req in requests:
                start = time.perf_counter()
                r = client.system_one(text, questions(req["questions"]))
                took.append(time.perf_counter() - start)
                check(r, req["questions"])
    aio = asyncio.run(run_async(requests))
    for a, b in zip(sync, aio):
        assert a.model_dump()["answers"] == b.model_dump()["answers"], "sync and async answers differ"

    line = "typesafe-sdk %s: %d models, %d fixtures sync and async, %d error cases" % (
        t.__version__, len(models.models), len(requests), n_errors)
    if took:
        ms = sorted(x * 1e3 for x in took)
        line += ", %d text requests (p50 %.1f ms)" % (len(ms), ms[len(ms) // 2])
    print(line + ", all parsed against " + url)


if __name__ == "__main__":
    main()
