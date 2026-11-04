use super::*;

fn node(committed: i64, subject: &str, body: &str) -> Node {
    Node {
        oid: format!("{committed:040x}"),
        parents: Vec::new(),
        committed,
        author: "Alice".to_owned(),
        email: "alice@example.com".to_owned(),
        subject: subject.to_owned(),
        body: body.to_owned(),
    }
}

#[test]
fn finds_merge_requests() {
    let message = "Merge branch 'x' into 'main'\n\nSee merge request team/sub/project!12\n\
                   See merge request other!3, and See merge request \nnothing!4";
    assert_eq!(
        merge_requests(message),
        [("team/sub/project", 12), ("other", 3)]
    );
    assert_eq!(merge_requests("See merge request !5 or a!b!7"), []);
    assert_eq!(merge_requests("See merge request a!b!7"), [("a!b", 7)]);
}

#[test]
fn finds_hosts() {
    let message = "See https://Example.org/a, http://git.example.com:8080/x and \
                   https://10.0.0.1/ or http://localhost/ or https://example.org.";
    assert_eq!(
        hosts(message),
        ["example.org", "git.example.com", "example.org"]
    );
    assert_eq!(hosts("httpx://a.org ftp://b.org"), Vec::<String>::new());
}

#[test]
fn counts_per_repository() {
    let nodes = [
        node(300, "c", "See merge request team/b!9\nhttps://example.org"),
        node(100, "a", "See merge request team/a!2"),
        node(
            200,
            "b",
            "See merge request team/a!5\nhttps://example.com https://example.org",
        ),
    ];
    let names = names(&nodes);
    let paths: Vec<_> = names
        .merge_requests
        .iter()
        .map(|m| {
            (
                m.path.as_str(),
                m.count,
                m.first,
                m.last,
                m.lowest,
                m.highest,
            )
        })
        .collect();
    assert_eq!(
        paths,
        [("team/a", 2, 100, 200, 2, 5), ("team/b", 1, 300, 300, 9, 9)]
    );
    let hosts: Vec<_> = names
        .hosts
        .iter()
        .map(|h| (h.host.as_str(), h.count))
        .collect();
    assert_eq!(hosts, [("example.org", 2), ("example.com", 1)]);
}
