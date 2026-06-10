use std::fs;

#[test]
fn workflows_are_hardened_against_common_pr_exploits() {
    let entries = fs::read_dir(".github/workflows").expect("workflow dir");
    let mut seen = 0usize;

    for entry in entries {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        seen += 1;
        let body = fs::read_to_string(&path).expect("workflow read");

        assert!(
            !body.contains("pull_request_target"),
            "workflow {} must not use pull_request_target",
            path.display()
        );
        assert!(
            body.contains("permissions:"),
            "workflow {} must declare explicit permissions",
            path.display()
        );
        assert!(
            body.contains("persist-credentials: false"),
            "workflow {} must disable persisted checkout credentials",
            path.display()
        );
    }

    assert!(
        seen > 0,
        "expected at least one workflow in .github/workflows"
    );
}
