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
        l1: None,
    };
    File {
        schema: SCHEMA.into(),
        path: "m.py".into(),
        language: "python".into(),
        producer: "hand".into(),
        source_sha256: sha256_hex(src),
        newlines: Newlines::Universal,
        levels: Vec::new(),
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
                l1: None,
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

/// IR written by the Python frontend (`src/sundgren/lang/python/ir_frontend.py`) for tricky
/// sources: BOM, CRLF, lone CR, latin-1, non-ASCII columns, decorators in parentheses.
#[test]
fn python_frontend_output_validates() {
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    for name in ["tricky", "latin1_cr"] {
        let source = std::fs::read(data.join(format!("{name}.py.src"))).unwrap();
        let json = std::fs::read_to_string(data.join(format!("{name}.ir.json"))).unwrap();
        let ir = File::from_json(&json).unwrap();
        ir.validate(&source)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let text = |id: &str| {
            let n = ir.node(&NodeId::new(id)).unwrap_or_else(|| panic!("{id}"));
            String::from_utf8_lossy(n.span.text(&source)).into_owned()
        };
        if name == "tricky" {
            assert!(text("tricky.py::decorated").starts_with("@(\r\n    deco\r\n)"));
            assert!(text("tricky.py::C.x#1").starts_with("@x.setter"));
            assert_eq!(text("tricky.py::<lambda>"), "lambda x: x");
        } else {
            // Columns count file bytes (latin-1 here), not UTF-8.
            let t = ir.node(&NodeId::new("latin1_cr.py::<lambda>")).unwrap();
            assert_eq!((t.span.start.line, t.span.start.col), (2, 15));
            assert_eq!(t.span.text(&source), b"lambda: \"\xe9\"");
        }
    }
}

/// SRC's `f` with L1 nodes: the parameter, the docstring statement and `return a`.
fn sample_l1() -> File {
    use sundgren_ir::{L1, Local};
    let src = SRC.as_bytes();
    let ix = LineIndex::new(src, Newlines::Universal);
    let mut ir = sample();
    ir.levels = vec!["L1".into()];
    let label = |l: &str| {
        Some(L1 {
            label: l.into(),
            local: None,
        })
    };
    let f = NodeId::new("m.py::f");
    let local = |name: &str| Local {
        scope: f.clone(),
        name: name.into(),
    };
    for n in &mut ir.nodes {
        n.l1 = label(n.lang["ast"].as_str().unwrap_or("Module"));
    }
    let at = |s: &str| SRC.find(s).unwrap();
    let l1 = |id: NodeId, parent: &NodeId, kind, start: usize, end: usize, l: &str, loc| Node {
        id,
        parent: Some(parent.clone()),
        kind,
        span: ix.span(start, end).unwrap(),
        body_span: None,
        name: None,
        qualified_name: None,
        lang: serde_json::Value::Null,
        l1: Some(L1 {
            label: l.into(),
            local: loc,
        }),
    };
    let dec = l1(NodeId::child(&f, 0), &f, Kind::Expr, 1, 4, "Name:dec", None);
    let arg = l1(
        NodeId::child(&f, 1),
        &f,
        Kind::Other("arg".into()),
        at("a)"),
        at("a)") + 1,
        "arg",
        Some(local("a")),
    );
    let doc_start = at("\"\"\"");
    let doc = l1(
        NodeId::child(&f, 2),
        &f,
        Kind::Expr,
        doc_start,
        doc_start + 10,
        "Expr",
        None,
    );
    let ret = l1(
        NodeId::child(&f, 3),
        &f,
        Kind::Return,
        at("return a"),
        at("return a") + 8,
        "Return",
        None,
    );
    let name = l1(
        NodeId::child(&ret.id, 0),
        &ret.id,
        Kind::Expr,
        at("return a") + 7,
        at("return a") + 8,
        "Name",
        Some(local("a")),
    );
    ir.nodes.splice(2..2, [dec, arg, doc, ret, name]);
    ir
}

#[test]
fn l1_nodes_validate_and_round_trip() {
    let ir = sample_l1();
    ir.validate(SRC.as_bytes()).unwrap();
    assert!(ir.has_l1() && !sample().has_l1());
    assert_eq!(File::from_json(&ir.to_json()).unwrap(), ir);
    // L0 JSON is unchanged: no `levels`, no `l1` keys.
    let l0 = serde_json::to_value(sample()).unwrap();
    assert!(l0.get("levels").is_none() && l0["nodes"][0].get("l1").is_none());
    // Every L1 key is in the schema.
    let schema: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
    let value = serde_json::to_value(&ir).unwrap();
    assert!(schema["properties"].get("levels").is_some());
    let l1_props = schema["$defs"]["l1"]["properties"].as_object().unwrap();
    for key in value["nodes"][3]["l1"].as_object().unwrap().keys() {
        assert!(l1_props.contains_key(key), "l1.{key} missing from schema");
    }
    let local_props = schema["$defs"]["local"]["properties"].as_object().unwrap();
    for key in value["nodes"][3]["l1"]["local"].as_object().unwrap().keys() {
        assert!(
            local_props.contains_key(key),
            "local.{key} missing from schema"
        );
    }
}

#[test]
fn l1_rules_are_checked() {
    let src = SRC.as_bytes();
    let mut ir = sample_l1();
    ir.nodes[3].id = NodeId::new("m.py::f/7");
    let msg = ir.validate(src).unwrap_err().to_string();
    assert!(msg.contains("structural rule"), "{msg}");
    let mut ir = sample_l1();
    ir.nodes[1].l1 = None;
    let msg = ir.validate(src).unwrap_err().to_string();
    assert!(msg.contains("no l1 payload"), "{msg}");
    let mut ir = sample_l1();
    ir.levels.clear();
    let msg = ir.validate(src).unwrap_err().to_string();
    assert!(msg.contains("without a name"), "{msg}");
}
