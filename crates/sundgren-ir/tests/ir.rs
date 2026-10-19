//! Round trips, validation and the checked-in schema.

use sundgren_ir::{
    File, IrError, JSON_SCHEMA, Kind, LineIndex, Newlines, Node, NodeId, NodeIds, SCHEMA, TextEdit,
    apply_edits, sha256_hex,
};

const SRC: &str = "@dec\ndef f(a):\n    \"\"\"Doc.\"\"\"\n    return a\n\ndef g(): return 1\n";

/// What a Python frontend emits for SRC (hand-built).
fn sample() -> File {
    let src = SRC.as_bytes();
    let ix = LineIndex::new(src, Newlines::Universal);
    let mut ids = NodeIds::new("m.py");
    let find = |s: &str| SRC.find(s).unwrap();
    let f_end = find("return a") + "return a".len();
    let g_end = find("return 1") + "return 1".len();
    let node = |id, kind, span, body_span, name: &str| Node {
        id,
        parent: Some(NodeId::root("m.py")),
        kind,
        span,
        body_span,
        name: Some(name.into()),
        qualified_name: Some(name.into()),
        lang: serde_json::json!({"ast": "FunctionDef"}),
    };
    File {
        schema: SCHEMA.into(),
        path: "m.py".into(),
        language: "python".into(),
        producer: "hand".into(),
        source_sha256: sha256_hex(src),
        newlines: Newlines::Universal,
        nodes: vec![
            Node {
                id: NodeId::root("m.py"),
                parent: None,
                kind: Kind::Module,
                span: ix.span(0, src.len()).unwrap(),
                body_span: None,
                name: None,
                qualified_name: None,
                lang: serde_json::Value::Null,
            },
            node(
                ids.named("f"),
                Kind::Function,
                ix.span(0, f_end).unwrap(),
                ix.span(find("return a"), f_end),
                "f",
            ),
            node(
                ids.named("g"),
                Kind::Function,
                ix.span(find("def g"), g_end).unwrap(),
                None,
                "g",
            ),
        ],
    }
}

#[test]
fn sample_is_valid_and_has_prototype_lines() {
    let ir = sample();
    ir.validate(SRC.as_bytes()).unwrap();
    let f = &ir.nodes[1];
    assert_eq!(f.span.lines(), 1..=4);
    assert_eq!(f.body_span.unwrap().start.line, 4);
    assert_eq!(ir.nodes[2].span.lines(), 6..=6);
}

#[test]
fn json_round_trip() {
    let ir = sample();
    let json = ir.to_json();
    assert_eq!(File::from_json(&json).unwrap(), ir);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["nodes"][1]["id"], "m.py::f");
    assert_eq!(value["nodes"][0]["kind"], "Module");
}

#[test]
fn wrong_schema_is_rejected() {
    let json = sample().to_json().replace(SCHEMA, "sundgren.ir/999");
    assert!(matches!(File::from_json(&json), Err(IrError::Schema(s)) if s == "sundgren.ir/999"));
    assert!(matches!(File::from_json("{}"), Err(IrError::Schema(_))));
    assert!(matches!(File::from_json("[1"), Err(IrError::Json(_))));
}

fn invalid(ir: &File) -> Vec<String> {
    match ir.validate(SRC.as_bytes()) {
        Err(IrError::Invalid(errs)) => errs,
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn validation_catches_bad_spans_ids_and_hashes() {
    let mut ir = sample();
    ir.nodes[1].span.start.col = 2;
    assert!(invalid(&ir)[0].contains("line/col disagree"));

    let mut ir = sample();
    ir.nodes[2].id = ir.nodes[1].id.clone();
    let errs = invalid(&ir);
    assert!(errs.iter().any(|e| e.contains("duplicate id")), "{errs:?}");

    let mut ir = sample();
    ir.source_sha256 = sha256_hex(b"other");
    assert!(invalid(&ir)[0].contains("source_sha256"));

    let mut ir = sample();
    ir.nodes[2].body_span = Some(ir.nodes[1].span);
    assert!(invalid(&ir).iter().any(|e| e.contains("body_span outside")));

    let mut ir = sample();
    ir.nodes[2].parent = Some(ir.nodes[1].id.clone());
    assert!(
        invalid(&ir)
            .iter()
            .any(|e| e.contains("outside its parent"))
    );

    let mut ir = sample();
    let ix = LineIndex::new(SRC.as_bytes(), Newlines::Universal);
    ir.nodes[2].span = ix.span(ir.nodes[2].span.start.byte, SRC.len()).unwrap();
    assert!(
        invalid(&ir)
            .iter()
            .any(|e| e.contains("ends with a newline"))
    );
}

#[test]
fn replacing_spans_with_their_own_text_is_identity() {
    let ir = sample();
    let src = SRC.as_bytes();
    let edits: Vec<TextEdit> = ir.nodes[1..]
        .iter()
        .map(|n| TextEdit {
            range: n.span.bytes(),
            text: n.span.text(src).to_vec(),
        })
        .collect();
    assert_eq!(apply_edits(src, &edits).unwrap(), src);
}

#[test]
fn replacing_a_body_keeps_the_signature() {
    let ir = sample();
    let body = ir.nodes[1].body_span.unwrap();
    let edit = TextEdit {
        range: body.bytes(),
        text: b"return a + 1".to_vec(),
    };
    let out = String::from_utf8(apply_edits(SRC.as_bytes(), &[edit]).unwrap()).unwrap();
    assert!(out.starts_with("@dec\ndef f(a):\n    \"\"\"Doc.\"\"\"\n    return a + 1\n"));
}

#[test]
fn schema_file_matches_the_types() {
    let schema: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
    assert_eq!(schema["properties"]["schema"]["const"], SCHEMA);
    assert_eq!(schema["title"], SCHEMA);
    let kinds: Vec<&str> = schema["$defs"]["kind"]["oneOf"][0]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap())
        .collect();
    assert_eq!(kinds, Kind::SHARED);
    for name in Kind::SHARED {
        let kind: Kind = serde_json::from_value(name.into()).unwrap();
        assert_eq!(serde_json::to_value(&kind).unwrap(), name);
    }
    // Every serialized field is listed in the schema.
    let value = serde_json::to_value(sample()).unwrap();
    let top = schema["properties"].as_object().unwrap();
    for key in value.as_object().unwrap().keys() {
        assert!(top.contains_key(key), "{key} missing from schema");
    }
    let node_props = schema["$defs"]["node"]["properties"].as_object().unwrap();
    for key in value["nodes"][0].as_object().unwrap().keys() {
        assert!(
            node_props.contains_key(key),
            "node.{key} missing from schema"
        );
    }
}
