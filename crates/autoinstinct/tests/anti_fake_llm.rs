//! Anti-fake LLM admission gate.
//!
//! Treats `.response` as adversarial. None of these tests touch a live
//! Gemini binary; all fixtures are canned strings — the suite stays
//! hermetic and fast.

use autoinstinct::llm::{admit, LlmAdmissionError};

const PROFILE: &str = "supply-chain";

fn good_world_json() -> String {
    r#"{
        "version":"30.1.1",
        "profile":"supply-chain",
        "scenario":"dock-obstruction",
        "objects":[
          {"id":"pallet-1","type":"pallet","label":"Pallet 1",
           "ontologyType":"https://schema.org/Product","attributes":{}}
        ],
        "events":[
          {"id":"urn:blake3:e1","type":"scan","time":"2026-04-30T08:00:00Z",
           "ontologyType":"https://schema.org/Action","objects":["pallet-1"],
           "attributes":{}}
        ],
        "counterfactuals":[
          {"id":"cf1","description":"remove pallet",
           "removeObjects":["pallet-1"],"removeEvents":[],
           "expectedResponse":"Ask"}
        ],
        "expectedInstincts":[
          {"condition":"pallet present","response":"Settle","forbidden":["fake-completion"]}
        ]
    }"#
    .to_string()
}

#[test]
fn good_world_admits() {
    let w = admit(&good_world_json(), PROFILE).expect("genuine world admits");
    assert_eq!(w.objects.len(), 1);
    assert_eq!(w.events.len(), 1);
}

#[test]
fn llm_rejects_markdown_wrapped_json() {
    let wrapped = format!("```json\n{}\n```", good_world_json());
    let err = admit(&wrapped, PROFILE).unwrap_err();
    assert!(matches!(err, LlmAdmissionError::Shape(_)),
            "markdown fence must fail shape gate, got {err:?}");
}

#[test]
fn llm_rejects_response_outside_lattice() {
    // "Bark" is not in the canonical 7-class response lattice. Closed enum
    // makes serde reject it at parse time.
    let bad = good_world_json().replace(r#""expectedResponse":"Ask""#,
                                        r#""expectedResponse":"Bark""#);
    let err = admit(&bad, PROFILE).unwrap_err();
    assert!(matches!(err, LlmAdmissionError::Shape(_)),
            "out-of-lattice response must fail shape gate, got {err:?}");
}

#[test]
fn llm_rejects_private_ontology_namespace() {
    let bad = good_world_json().replace(
        r#""ontologyType":"https://schema.org/Product""#,
        r#""ontologyType":"https://acme.internal/Pallet""#,
    );
    let err = admit(&bad, PROFILE).unwrap_err();
    assert!(matches!(err, LlmAdmissionError::NonPublicOntology(_)),
            "private namespace must be rejected, got {err:?}");
}

#[test]
fn llm_rejects_pii_bearing_event_id() {
    // Looks like an IRI, but not on the public allowlist → PII-suspect.
    let bad = good_world_json().replace(
        r#""id":"urn:blake3:e1""#,
        r#""id":"https://acme.internal/users/alice/events/42""#,
    );
    let err = admit(&bad, PROFILE).unwrap_err();
    assert!(matches!(err, LlmAdmissionError::PiiSuspected(_)),
            "PII-bearing IRI must be rejected, got {err:?}");
}

#[test]
fn llm_rejects_dangling_event_object_reference() {
    let bad = good_world_json()
        .replace(r#""objects":["pallet-1"]"#, r#""objects":["pallet-DOES-NOT-EXIST"]"#);
    let err = admit(&bad, PROFILE).unwrap_err();
    assert!(matches!(err, LlmAdmissionError::DanglingReference { .. }),
            "dangling reference must be rejected, got {err:?}");
}

#[test]
fn llm_rejects_zero_objects_or_zero_events() {
    let bad = good_world_json().replace(
        r#""objects":[
          {"id":"pallet-1","type":"pallet","label":"Pallet 1",
           "ontologyType":"https://schema.org/Product","attributes":{}}
        ]"#,
        r#""objects":[]"#,
    );
    let err = admit(&bad, PROFILE).unwrap_err();
    assert!(matches!(err, LlmAdmissionError::Structural(_)),
            "zero-object world must be rejected, got {err:?}");
}

#[test]
fn llm_rejects_profile_mismatch() {
    let err = admit(&good_world_json(), "lifestyle").unwrap_err();
    assert!(matches!(err, LlmAdmissionError::ProfileMismatch { .. }),
            "wrong profile must be rejected, got {err:?}");
}

#[test]
fn llm_rejects_event_with_zero_object_links() {
    let bad = good_world_json().replace(r#""objects":["pallet-1"]"#, r#""objects":[]"#);
    let err = admit(&bad, PROFILE).unwrap_err();
    assert!(matches!(err, LlmAdmissionError::Structural(_)),
            "event with zero object links must be rejected, got {err:?}");
}

#[test]
fn llm_rejects_prose_around_json() {
    let bad = format!("Here is your world:\n{}\nHope that helps!", good_world_json());
    let err = admit(&bad, PROFILE).unwrap_err();
    assert!(matches!(err, LlmAdmissionError::Shape(_)),
            "prose around JSON must be rejected, got {err:?}");
}
