# Fixtures

One directory per case under `fixtures/<surface>/<case>/`, each with the `request.json` a client sends and the `response.json` it gets back. `cargo run -- fixtures` checks every pair against the response rules in `spec/03-api.md`.

The three cases under `systemone/` are written by hand from the examples in the kime specification and README. They pin the contract before there is a server to record from. From M1 on, recorded responses from kime-serve, Jev and laya-serve go next to them, and a recorded response that breaks a rule is a bug in whichever server sent it.

`systemone/jev-quickstart` is the request from TypeSafe's quickstart, with the response kime 0.0.26 gave for it on the Laya checkpoint.

`usage.tsv` has the `usage.input_tokens` Laya 0.3.20 reports for each fixture request, and `live` fails a server that counts differently. Laya counts every token the model reads, and since it reads the state once per question, a request with three questions counts the state three times. kime counts the same way for the Laya checkpoint. Jev's count is there only for the quickstart, the one request TypeSafe published a count for, and it is 392 where Laya counts 205 for the same request. The two models tokenize and lay out the input differently, so the counts cannot match and cost per million input tokens is not the same unit across the two. Comparing cost per request avoids the problem.
