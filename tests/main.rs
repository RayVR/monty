use serde::Deserialize;
use monty::parse_show;


#[derive(Debug, Deserialize)]
struct Case {
    code: String,
    expected: String,
}

#[derive(Debug, Deserialize)]
struct Cases {
    cases: Vec<Case>,
}

#[test]
fn test_syntax() {
    let cases: Cases = toml::from_str(include_str!("syntax-cases.toml")).unwrap();
    dbg!(&cases.cases);
    for case in cases.cases {
        let output = parse_show(&case.code, "test.py").unwrap();
        let expected = case.expected.trim_matches('\n');
        // eprintln!("output: {}", output);
        assert_eq!(output, expected);
    }
}
