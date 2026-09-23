//! The response rules clients depend on, from `spec/03-api.md`, checked against a request and the
//! response kime gave for it.
//!
//! These are the rules a client breaks on, not a schema. A schema says `probabilities` is an object
//! of numbers. jev-ultrafast says the numbers sum to one within 0.02, and it is the second one that
//! decides whether an agent run works.

use serde_json::{Map, Value};

/// Checks one response against its request. Returns every violation, not only the first, because
/// a fixture that is wrong in three ways should say so once.
pub fn check(request: &Value, response: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(questions) = request.get("questions").and_then(Value::as_object) else {
        return vec!["the request has no questions object".into()];
    };
    let Some(answers) = response.get("answers").and_then(Value::as_object) else {
        return vec!["the response has no answers object".into()];
    };

    let asked: Vec<&String> = questions.keys().collect();
    let answered: Vec<&String> = answers.keys().collect();
    if asked != answered {
        problems.push(format!(
            "answers are {answered:?} but the questions were {asked:?}, in that order"
        ));
    }

    match response.get("model").and_then(Value::as_str) {
        None => problems.push("the response has no model".into()),
        Some(m) if m.ends_with("-latest") => {
            problems.push(format!("model {m} is an alias, not the resolved id"))
        }
        Some(_) => {}
    }

    match response.pointer("/usage/output_tokens").and_then(Value::as_u64) {
        Some(0) => {}
        other => problems
            .push(format!("usage.output_tokens is {other:?}, and nothing is ever generated")),
    }

    for (id, question) in questions {
        let Some(answer) = answers.get(id) else { continue };
        let kind = question.get("type").and_then(Value::as_str).unwrap_or("");
        if answer.get("type").and_then(Value::as_str) != Some(kind) {
            problems.push(format!("{id}: the answer type is not {kind}"));
        }
        let mut say = |p: String| problems.push(format!("{id}: {p}"));
        match kind {
            "choice" => check_choice(question, answer, &mut say),
            "score" => check_score(question, answer, &mut say),
            "noul" => check_noul(answer, &mut say),
            other => say(format!("unknown question type {other:?}")),
        }
    }
    problems
}

fn labels(question: &Value) -> Vec<String> {
    match question.get("criteria") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => Vec::new(),
    }
}

fn probabilities(answer: &Value, say: &mut impl FnMut(String)) -> Option<Map<String, Value>> {
    let Some(p) = answer.get("probabilities").and_then(Value::as_object) else {
        say("no probabilities".into());
        return None;
    };
    let mut hundredths = 0i64;
    for (k, v) in p {
        match v.as_f64() {
            Some(x) if x.is_finite() && (0.0..=1.0).contains(&x) => {
                hundredths += (x * 100.0).round() as i64
            }
            _ => say(format!("probability for {k} is {v}, not a finite value in [0, 1]")),
        }
    }
    // Rounded to two decimals by largest remainder, so the hundredths sum to exactly one hundred.
    // That is stricter than jev-ultrafast's 0.02 and it is what spec/03-api.md promises.
    if hundredths != 100 {
        say(format!("probabilities sum to {hundredths} hundredths, not exactly 100"));
    }
    Some(p.clone())
}

fn check_choice(question: &Value, answer: &Value, say: &mut impl FnMut(String)) {
    let want = labels(question);
    let Some(p) = probabilities(answer, say) else { return };
    let got: Vec<&String> = p.keys().collect();
    if got != want.iter().collect::<Vec<_>>() {
        say(format!("probability keys are {got:?}, the offered labels are {want:?}"));
    }
    let top = p.values().filter_map(Value::as_f64).fold(f64::MIN, f64::max);
    match answer.get("choice").and_then(Value::as_str) {
        Some(c) if p.get(c).and_then(Value::as_f64) == Some(top) => {}
        other => say(format!("choice {other:?} is not an argmax of the probabilities")),
    }
    check_confidence(answer, say);
}

fn check_score(question: &Value, answer: &Value, say: &mut impl FnMut(String)) {
    let n = question.get("criteria").and_then(Value::as_array).map_or(0, Vec::len);
    let want: Vec<String> = (0..n).map(|i| i.to_string()).collect();
    let Some(p) = probabilities(answer, say) else { return };
    let got: Vec<&String> = p.keys().collect();
    if got != want.iter().collect::<Vec<_>>() {
        say(format!("probability keys are {got:?}, the levels are {want:?}"));
    }
    // The score is computed from unrounded probabilities, so the rounded ones only bound it.
    let expected: f64 =
        p.iter().filter_map(|(k, v)| Some(k.parse::<f64>().ok()? * v.as_f64()?)).sum();
    match answer.get("score").and_then(Value::as_f64) {
        Some(s) if (s - expected).abs() <= 0.005 * n as f64 + 0.01 => {}
        other => say(format!("score {other:?} is not the expected level {expected:.3}")),
    }
    let criteria = question.get("criteria").and_then(Value::as_array);
    for (i, c) in criteria.into_iter().flatten().enumerate() {
        if answer.pointer(&format!("/legend/{i}")) != Some(c) {
            say(format!("legend {i} does not echo the criterion exactly as sent"));
        }
    }
    check_confidence(answer, say);
}

fn check_noul(answer: &Value, say: &mut impl FnMut(String)) {
    match answer.get("noul").and_then(Value::as_f64) {
        Some(x) if (0.0..=1.0).contains(&x) => {}
        other => say(format!("noul {other:?} is not in [0, 1]")),
    }
}

fn check_confidence(answer: &Value, say: &mut impl FnMut(String)) {
    match answer.get("confidence").and_then(Value::as_f64) {
        Some(x) if (0.0..=1.0).contains(&x) => {}
        other => say(format!("confidence {other:?} is not in [0, 1]")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> Value {
        json!({"questions": {
            "team": {"type": "choice", "criteria": {"billing": "Payments", "technical": null}},
            "churn": {"type": "noul"}
        }})
    }

    fn response() -> Value {
        json!({"model": "kime-v1-s-en-0.0.1", "answers": {
            "team": {"type": "choice", "choice": "billing", "confidence": 0.8, "probabilities": {"billing": 0.9, "technical": 0.1}},
            "churn": {"type": "noul", "noul": 0.7}
        }, "usage": {"input_tokens": 40, "output_tokens": 0}})
    }

    #[test]
    fn a_good_response_passes() {
        assert_eq!(check(&request(), &response()), Vec::<String>::new());
    }

    #[test]
    fn answers_out_of_order_fail() {
        let mut r = response();
        let a = r["answers"].as_object_mut().unwrap();
        let team = a.shift_remove("team").unwrap();
        a.insert("team".into(), team);
        assert!(check(&request(), &r)[0].contains("in that order"));
    }

    #[test]
    fn a_sum_that_is_only_close_fails() {
        let mut r = response();
        r["answers"]["team"]["probabilities"]["technical"] = json!(0.11);
        assert!(check(&request(), &r).iter().any(|p| p.contains("101 hundredths")));
    }

    #[test]
    fn a_choice_that_is_not_the_argmax_fails() {
        let mut r = response();
        r["answers"]["team"]["choice"] = json!("technical");
        assert!(check(&request(), &r).iter().any(|p| p.contains("argmax")));
    }

    #[test]
    fn an_alias_as_the_model_fails() {
        let mut r = response();
        r["model"] = json!("jev-latest");
        assert!(check(&request(), &r).iter().any(|p| p.contains("alias")));
    }
}
