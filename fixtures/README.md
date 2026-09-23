# Fixtures

One directory per case under `fixtures/<surface>/<case>/`, each with the `request.json` a client sends and the `response.json` it gets back. `cargo run -- fixtures` checks every pair against the response rules in `spec/03-api.md`.

The three cases under `systemone/` are written by hand from the examples in the kime specification and README. They pin the contract before there is a server to record from. From M1 on, recorded responses from kime-serve, Jev and laya-serve go next to them, and a recorded response that breaks a rule is a bug in whichever server sent it.
