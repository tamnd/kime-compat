# kime-compat

The compatibility harness for [kime](https://github.com/tamnd/kime).

kime promises that code written for TypeSafe's Jev or for Laya works against it with a base URL or an import changed and nothing else. This repository is where that promise becomes a number: the TypeSafe API and SDKs, jev-ultrafast, and Laya's own clients and tests, run against kime.

It is a separate repository for the same reason the benchmarks are: someone who doubts the claim should be able to run the check without trusting the engine's own test suite. The design is the API compatibility section of [`spec/15-testing.md`](https://github.com/tamnd/kime/blob/main/spec/15-testing.md) in the kime repository.

## Where kime is, today

Three things run. The response contract (the rules from `spec/03-api.md` that clients actually break on) is checked on the committed fixtures, and on what a running kime-serve answers to the same requests: answer order matches question order, probability keys are exactly the offered labels, rounded probabilities sum to exactly one, `choice` is an argmax, `score` is the expected level, `legend` echoes the criteria, `model` is never an alias, nothing is ever generated, and `/v1/models` has the shape the TypeSafe SDKs parse. The TypeSafe JS SDK examples in `sdk-js/` run against kime-serve with `@typesafe-ai/sdk` 0.6.0 unmodified, under Node 20 and Bun.

CI builds kime-serve from tamnd/kime main on the laya checkpoint and runs both through `ci/against-kime.sh`, and the kime repository runs the same script on its own changes.

## The surfaces

| Surface | What passes | Starts |
|---|---|---|
| TypeSafe OpenAPI | Every response validates against the committed `/openapi.json` snapshot, and Schemathesis generated requests are all accepted | M1 |
| TypeSafe Python SDK | The SDK's recorded fixtures replay against kime-serve and its typed parsing succeeds | M1 |
| TypeSafe JS SDK | The examples run against kime-serve under Node 20 and Bun | M1 |
| jev-ultrafast | `validate_choice` passes on 10,000 generated agent steps, and the Wikipedia example runs end to end with `jev-latest` unchanged | M1 |
| Laya Python API | Laya's own test suite passes against the kime package, except the documented bugs in `laya_expected_diffs.md` | M1 |
| laya-serve clients | The curl examples give the same response shape, and the same answers within parity tolerance on the compat models | M1 |
| Error bodies | A snapshot for every error status in `spec/03-api.md` | M1 |
| The response contract | Every committed fixture obeys the rules clients depend on | M0 |

The same list is in `surfaces.tsv`, which is what the harness reads.

```sh
cargo run -- surfaces
cargo run -- fixtures
cargo run --release -- live http://127.0.0.1:8000 [texts.json]
ci/against-kime.sh path/to/kime path/to/models node bun
```

`live` sends each committed request to a running server as `jev-latest`, since a request with no model is laya-serve's dialect, and checks the answers with the same rules. With a JSON list of strings it also asks each fixture's questions about every text, which is how the contract gets checked on real inputs and not only the three written by hand.

## Laya's documented bugs

A few of Laya's behaviours are bugs that kime fixes on purpose, like a score level that can never win. Laya's tests that assert those behaviours are listed in `laya_expected_diffs.md` with the reason and the Laya issue, and they are the only tests allowed to fail. The file arrives with the Laya surface at M1.

## Contributing

A new case is a directory under `fixtures/<surface>/` with a `request.json` and a `response.json`. A client that breaks against kime is the most useful report this repository can get: open it in [tamnd/kime](https://github.com/tamnd/kime/issues/new?template=compatibility.yml) with the compatibility template, and the fixture that shows it lands here.

`cargo fmt --all --check`, `cargo clippy --all-targets` and `cargo test` must pass, and the tests include the prose rules: plain English, no em dashes or en dashes, no horizontal rules, and no sentence broken across two lines.

## License

Apache-2.0. See [LICENSE-APACHE](LICENSE-APACHE).
