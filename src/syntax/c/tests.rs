use super::*;
use crate::syntax::{Language, lex};

fn outline(source: &str) -> Vec<(&'static str, String, usize, usize)> {
    items(source, &lex(Language::C, source))
        .into_iter()
        .map(|i| (i.kind, i.name, i.start, i.end))
        .collect()
}

fn item(
    kind: &'static str,
    name: &str,
    start: usize,
    end: usize,
) -> (&'static str, String, usize, usize) {
    (kind, name.to_owned(), start, end)
}

#[test]
fn functions_types_and_macros() {
    let source = "\
#include <stdio.h>
#define MAX(a, b) \\
    ((a) > (b) ? (a) : (b))
#  define  LIMIT 10
#ifdef __cplusplus
extern \"C\" {
#endif

struct point { int x, y; };
typedef struct node_s {
    struct node_s *next;
} node;
typedef enum { RED, GREEN } color;
union { int i; float f; } u;
int table[] = { 1, 2, 3 };
static const char *names[2] = { \"a\", \"b\" };
int prototype(int a);

__attribute__((noreturn))
static void fail(const char *why)
{
    exit(1);
}

struct point *
make_point(int x, int y) { return 0; }
int (*pick(int which))(int) { return 0; }

#ifdef __cplusplus
}
#endif
";
    assert_eq!(
        outline(source),
        [
            item("macro", "MAX", 2, 3),
            item("macro", "LIMIT", 4, 4),
            item("struct", "point", 9, 9),
            item("struct", "node", 10, 12),
            item("enum", "color", 13, 13),
            item("union", "u", 14, 14),
            item("function", "fail", 19, 23),
            item("function", "make_point", 25, 26),
            item("function", "pick", 27, 27),
        ]
    );
}

#[test]
fn things_that_are_not_items() {
    assert!(outline("#undef X\n#definitely\n#define\nint x;\n{ y; }\n").is_empty());
    assert!(outline("void f(void) {").is_empty());
}
