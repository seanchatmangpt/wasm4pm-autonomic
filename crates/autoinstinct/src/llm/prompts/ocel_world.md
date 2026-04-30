Return ONLY valid JSON.

Do not wrap the JSON in markdown.
Do not include prose before or after the JSON.
Do not include comments.
Do not include trailing commas.

Generate an ontology-aligned OCEL world.

Profile: {{PROFILE}}
Scenario: {{SCENARIO}}

The JSON must match this exact shape (camelCase keys):

{
  "version": "30.1.1",
  "profile": "{{PROFILE}}",
  "scenario": "{{SCENARIO}}",
  "objects": [
    {
      "id": "string",
      "type": "string",
      "label": "string",
      "ontologyType": "string",
      "attributes": {}
    }
  ],
  "events": [
    {
      "id": "string",
      "type": "string",
      "time": "ISO-8601 string",
      "ontologyType": "string",
      "objects": ["object id"],
      "attributes": {}
    }
  ],
  "counterfactuals": [
    {
      "id": "string",
      "description": "string",
      "removeObjects": ["object id"],
      "removeEvents": ["event id"],
      "expectedResponse": "Settle"
    }
  ],
  "expectedInstincts": [
    {
      "condition": "string",
      "response": "Ask",
      "forbidden": ["string"]
    }
  ]
}

Allowed response values (canonical lattice — no other strings permitted):
Settle, Retrieve, Inspect, Ask, Refuse, Escalate, Ignore.

Use public ontology terms only. Prefer schema.org, PROV-O, SOSA/SSN,
SKOS, OWL-Time, GeoSPARQL, QUDT, SHACL, ODRL.

Do not emit PII-bearing IRIs. For opaque tokens use `urn:blake3:`.

Every event must reference at least one declared object id.
There must be at least one object and at least one event.
