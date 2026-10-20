use super::*;

const RS: &str = "/home/runner/work/shed/shed_rs/src";

fn load(name: &str) -> Snapshot {
    let path = format!("{}/tests/fixtures/lcov/{name}", env!("CARGO_MANIFEST_DIR"));
    parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn one(text: &str) -> FileCoverage {
    let snap = parse(text).unwrap();
    assert_eq!(snap.files.len(), 1);
    snap.files.into_values().next().unwrap()
}

fn func(name: &str, start: LineNo, end: Option<LineNo>, hits: Option<Count>) -> FunctionRecord {
    FunctionRecord {
        name: name.into(),
        start,
        end,
        hits,
    }
}

fn key(line: LineNo, block: &str, branch: &str) -> BranchKey {
    BranchKey {
        line,
        block: block.into(),
        branch: branch.into(),
    }
}

fn find<'a>(fc: &'a FileCoverage, name: &str) -> &'a FunctionRecord {
    fc.functions.iter().find(|f| f.name == name).unwrap()
}

fn lines(pairs: &[(LineNo, Count)]) -> BTreeMap<LineNo, Count> {
    pairs.iter().copied().collect()
}

#[test]
fn coveragepy() {
    let snap = load("coveragepy.lcov");
    assert!(snap.test_names.is_empty());
    assert_eq!(
        snap.files.keys().collect::<Vec<_>>(),
        ["shed/__init__.py", "shed/calc.py"]
    );
    let init = &snap.files["shed/__init__.py"];
    assert!(init.lines.is_empty() && init.branches.is_empty() && init.functions.is_empty());
    let calc = &snap.files["shed/calc.py"];
    assert!(!calc.external);
    assert_eq!(calc.lines.len(), 22);
    assert_eq!((calc.lines[&1], calc.lines[&6], calc.lines[&31]), (1, 0, 1));
    assert!(!calc.lines.contains_key(&3));
    assert_eq!(calc.branches.len(), 10);
    assert_eq!(calc.branches[&key(6, "0", "jump to line 7")], None);
    assert_eq!(calc.branches[&key(12, "0", "jump to line 13")], Some(1));
    assert_eq!(calc.branches[&key(14, "0", "jump to line 15")], Some(0));
    let ret = key(30, "0", "return from function 'note'");
    assert_eq!(calc.branches[&ret], Some(0));
    let names: Vec<_> = calc.functions.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "add",
            "divide",
            "classify",
            "Acc.__init__",
            "Acc.push",
            "note"
        ]
    );
    assert_eq!(*find(calc, "divide"), func("divide", 5, Some(8), Some(0)));
    assert_eq!(
        *find(calc, "Acc.push"),
        func("Acc.push", 23, Some(26), Some(0))
    );
}

#[test]
fn llvm_cov() {
    let snap = load("llvm_cov.lcov");
    let lib_path = format!("{RS}/lib.rs");
    let util_path = format!("{RS}/util.rs");
    assert_eq!(
        snap.files.keys().collect::<Vec<_>>(),
        [&lib_path, &util_path]
    );
    let lib = &snap.files[&lib_path];
    assert!(lib.branches.is_empty());
    assert_eq!(lib.lines.len(), 24);
    assert_eq!((lib.lines[&3], lib.lines[&8], lib.lines[&12]), (0, 2, 0));
    assert_eq!(lib.functions.len(), 6);
    // v0 symbols, demangled.
    assert_eq!(
        *find(lib, "shed::divide"),
        func("shed::divide", 1, None, Some(1))
    );
    assert_eq!(find(lib, "shed::classify").hits, Some(2));
    let closure = "shed::tests::test_divide::{closure#0}";
    assert_eq!(*find(lib, closure), func(closure, 22, None, Some(1)));
    let util = &snap.files[&util_path];
    assert_eq!(
        util.functions,
        [func("shed::util::clamp", 1, None, Some(0))]
    );
    assert!(util.lines.values().all(|&c| c == 0));
}

#[test]
fn tarpaulin() {
    let snap = load("tarpaulin.lcov");
    assert!(snap.test_names.is_empty()); // empty TN: is dropped
    let fc = &snap.files["/builds/shed/shed_rs/src/lib.rs"];
    let want = [
        (1, 2),
        (2, 1),
        (3, 0),
        (5, 2),
        (8, 1),
        (9, 2),
        (10, 1),
        (11, 2),
        (12, 0),
        (14, 1),
    ];
    assert_eq!(fc.lines, lines(&want));
    assert!(fc.branches.is_empty());
    assert_eq!(
        fc.functions,
        [
            func("divide", 1, None, Some(2)),
            func("classify", 8, None, Some(1))
        ]
    );
}

#[test]
fn lcov2_fn() {
    let fc = load("lcov2_fn.lcov").files.into_values().next().unwrap();
    assert_eq!(fc.path, "/home/ci/ringbuf/src/ring.cpp");
    assert_eq!(
        fc.functions,
        [
            func("ring::Ring::push(int)", 6, Some(12), Some(7)),
            func("ring::Ring::pop()", 14, Some(21), Some(0)),
            func("ring::minmax(int, int)", 23, Some(26), Some(2)),
        ]
    );
    assert_eq!(fc.branches[&key(9, "e0", "1")], Some(2));
    assert_eq!(fc.branches[&key(15, "0", "0")], None);
    assert_eq!(fc.branches[&key(7, "0", "1")], Some(5));
    assert_eq!(fc.lines.len(), 18);
}

#[test]
fn duplicate_sf_merged() {
    let snap = load("duplicate_sf.lcov");
    assert_eq!(snap.test_names, ["unit", "integration"]);
    let ring = &snap.files["src/ring.c"];
    let l = &ring.lines;
    assert_eq!((l[&4], l[&6], l[&11], l[&19]), (16, 3, 2, 0));
    let want: BTreeMap<_, _> = [
        (key(5, "0", "0"), Some(3)),
        (key(5, "0", "1"), Some(13)),
        (key(13, "0", "0"), Some(1)), // '-' + 1
        (key(13, "0", "1"), Some(1)),
        (key(20, "0", "0"), None), // '-' + '-'
        (key(20, "0", "1"), None),
    ]
    .into_iter()
    .collect();
    assert_eq!(ring.branches, want);
    assert_eq!(
        ring.functions,
        [
            func("ring_push", 4, Some(9), Some(16)),
            func("ring_pop", 11, Some(17), Some(2)),
            func("ring_len", 19, Some(22), Some(0)),
        ]
    );
    assert_eq!(
        snap.files["src/util.c"].lines,
        lines(&[(1, 5), (2, 5), (3, 5)])
    );
}

#[test]
fn merge_semantics() {
    let text = "TN:a\nSF:x.c\nFN:1,f\nFNDA:1,f\nFN:5,7,g\nDA:1,1\nDA:2,0\n\
                    BRDA:2,0,0,-\nBRDA:2,0,1,-\nBRDA:3,0,0,4\nBRDA:3,0,1,-\nend_of_record\n\
                    TN:b\nSF:x.c\nFN:1,3,f\nFNDA:2,f\nFN:5,g\nFNDA:0,g\nFN:9,h\nDA:2,5\nDA:9,0\n\
                    BRDA:2,0,0,-\nBRDA:2,0,1,2\nBRDA:3,0,0,1\nBRDA:3,0,1,0\nend_of_record\n";
    let fc = one(text);
    assert_eq!(fc.lines, lines(&[(1, 1), (2, 5), (9, 0)]));
    let want: BTreeMap<_, _> = [
        (key(2, "0", "0"), None),
        (key(2, "0", "1"), Some(2)),
        (key(3, "0", "0"), Some(5)),
        (key(3, "0", "1"), Some(0)),
    ]
    .into_iter()
    .collect();
    assert_eq!(fc.branches, want);
    assert_eq!(
        fc.functions,
        [
            func("f", 1, Some(3), Some(3)), // end filled in from the second record
            func("g", 5, Some(7), Some(0)), // no FNDA, then 0
            func("h", 9, None, None),
        ]
    );
}

#[test]
fn repeated_records_within_one_section_are_summed() {
    let fc = one("SF:x\nDA:1,2\nDA:1,3\nFN:1,f\nFNDA:1,f\nFNDA:4,f\nend_of_record\n");
    assert_eq!(fc.lines, lines(&[(1, 5)]));
    assert_eq!(fc.functions, [func("f", 1, None, Some(5))]);
}

#[test]
fn da_checksum_is_ignored() {
    let fc = one("SF:x.py\nDA:1,1,5kX8cZ4xh/H3dUOhjJ9P3w\nDA:2,0,abc\nend_of_record\n");
    assert_eq!(fc.lines, lines(&[(1, 1), (2, 0)]));
}

#[test]
fn names_and_ids_with_commas() {
    let name = "std::swap<int, char>(int&, char&)";
    for (span, end) in [("4", None), ("4,9", Some(9))] {
        let fc = one(&format!(
            "SF:x.cpp\nFN:{span},{name}\nFNDA:3,{name}\nend_of_record\n"
        ));
        assert_eq!(fc.functions, [func(name, 4, end, Some(3))]);
    }
    let fc = one("SF:x.c\nBRDA:3,0,(a, b) == True,7\nBRDA:3,0,(a, b) == True,-\nend_of_record\n");
    assert_eq!(fc.branches[&key(3, "0", "(a, b) == True")], Some(7));
}

#[test]
fn lcov22_fnl_fna() {
    let fc = one("SF:x.cpp\nFNL:0,4,9\nFNA:0,3,f(int)\nFNA:0,1,f(long)\n\
             FNL:1,12\nFNA:1,0,g(int, int)\nend_of_record\n");
    assert_eq!(
        fc.functions,
        [
            func("f(int)", 4, Some(9), Some(3)),
            func("f(long)", 4, Some(9), Some(1)),
            func("g(int, int)", 12, None, Some(0)),
        ]
    );
}

#[test]
fn summary_and_unknown_records_are_ignored() {
    let plain = parse("SF:x\nDA:1,1\nend_of_record\n").unwrap();
    let summary = "SF:x\nDA:1,1\nLF:99\nLH:42\nFNF:7\nFNH:1\nBRF:3\nBRH:0\nend_of_record\n";
    assert_eq!(parse(summary).unwrap(), plain);
    let unknown = "SF:x\nVER:abc\nMCDC:3,2,t,1,0,a && b\nDA:1,1\nend_of_record\nDA:2,1\nweird\n";
    assert_eq!(parse(unknown).unwrap(), plain);
}

#[test]
fn fnda_without_fn_is_dropped() {
    assert!(
        one("SF:x\nFNDA:1,ghost\nend_of_record\n")
            .functions
            .is_empty()
    );
}

#[test]
fn missing_end_of_record_crlf_and_bom() {
    let snap = parse("TN:t\r\nSF:a\r\nDA:1,1\r\nSF:b\r\nDA:2,0\r\n\r\n").unwrap();
    assert_eq!(snap.test_names, ["t"]);
    assert_eq!(snap.files["a"].lines, lines(&[(1, 1)]));
    assert_eq!(snap.files["b"].lines, lines(&[(2, 0)]));
    let snap = parse("\u{feff}TN:t\nSF:a.py\nDA:1,1\nend_of_record\nSF:b.py\nend_of_record\n");
    let snap = snap.unwrap();
    assert_eq!(snap.test_names, ["t"]);
    assert_eq!(snap.files.keys().collect::<Vec<_>>(), ["a.py", "b.py"]);
}

#[test]
fn only_newline_ends_a_record() {
    for sep in ["\x0b", "\x0c", "\x1c", "\u{85}", "\u{2028}", "\u{2029}"] {
        let path = format!("src/a{sep}b.py");
        let snap = parse(&format!("SF:{path}\nFN:1,f{sep}g\nDA:1,1\nend_of_record\n")).unwrap();
        let fc = &snap.files[&path];
        assert_eq!(fc.functions, [func(&format!("f{sep}g"), 1, None, None)]);
    }
}

#[test]
fn test_names_deduplicated_in_first_seen_order() {
    let text = "TN:z\nSF:a\nend_of_record\nTN:\nTN:a\nTN:z\nSF:a\nend_of_record\n";
    assert_eq!(parse(text).unwrap().test_names, ["z", "a"]);
}

#[test]
fn windows_sf_path_kept_raw() {
    assert_eq!(
        one("SF:C:\\ci\\repo\\src\\lib.rs\nDA:1,1\n").path,
        "C:\\ci\\repo\\src\\lib.rs"
    );
}

#[test]
fn huge_and_negative_counts() {
    let fc = one("SF:a\nDA:1,18446744073709551615\nDA:2,-1\nend_of_record\n");
    assert_eq!(fc.lines, lines(&[(1, 18446744073709551615), (2, -1)]));
}

#[test]
fn malformed_records() {
    for rec in [
        "DA:1",
        "DA:x,1",
        "BRDA:1,0,1",
        "BRDA:1,0,0,x",
        "FN:x,f",
        "FN:4",
        "FNDA:1",
        "FNA:9,1,f",
        "FNL:1",
        "FNL:1,2,3,4",
    ] {
        let err = parse(&format!("SF:x\n{rec}\nend_of_record\n")).unwrap_err();
        assert!(err.starts_with("lcov line 2: malformed"), "{err}");
    }
}

#[test]
fn empty_input() {
    assert_eq!(parse("").unwrap(), Snapshot::default());
}

#[test]
fn rust_symbols_are_demangled() {
    let fc = one("SF:a\n\
         FN:1,_RNvCsc61jNJ0enoW_7mycrate6divide\n\
         FNDA:2,_RNvCsc61jNJ0enoW_7mycrate6divide\n\
         FN:5,_RNvNtCsc61jNJ0enoW_7mycrate5testss_11test_divide\n\
         FN:9,_ZN7mycrate8classify17h0123456789abcdefE\n\
         FNDA:3,_ZN7mycrate8classify17h0123456789abcdefE\n\
         FN:12,_ZN7mycrate8classify17hfedcba9876543210E\n\
         FNDA:1,_ZN7mycrate8classify17hfedcba9876543210E\n\
         FN:20,_Rnot_a_symbol\n\
         FN:21,divide\n\
         end_of_record\n");
    let got: Vec<(&str, LineNo, Option<Count>)> = fc
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.start, f.hits))
        .collect();
    // Two instances of one function merge: first span, summed hits.
    assert_eq!(
        got,
        [
            ("mycrate::divide", 1, Some(2)),
            ("mycrate::tests::test_divide", 5, None),
            ("mycrate::classify", 9, Some(4)),
            ("_Rnot_a_symbol", 20, None),
            ("divide", 21, None),
        ]
    );
}
