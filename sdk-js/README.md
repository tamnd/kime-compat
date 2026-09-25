# sdk-js

The TypeSafe JS SDK, `@typesafe-ai/sdk` 0.6.0 from npm and unmodified, run against kime-serve. Each file under `examples/` is a program a TypeSafe user could have written: it imports the SDK, reads `TYPESAFE_BASE_URL` and `TYPESAFE_API_KEY` like any other, and exits non-zero if an answer is not what the SDK's types and the API promise.

```sh
npm ci
KIME_COMPAT_URL=http://127.0.0.1:8000 node run.mjs
KIME_COMPAT_URL=http://127.0.0.1:8000 bun run.mjs
```

`readme.mjs` is the example from the SDK's README exactly as it is there. `every-type.mjs` asks all three question types about text and JSON state and checks the answers the way a caller relies on them. `errors.mjs` checks that a refused request raises the SDK's own error classes.
