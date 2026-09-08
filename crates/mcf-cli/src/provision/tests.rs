use super::{Came, came_from, report};
use mcf_core::component::COMPONENTS;
use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
use mcf_record::json::Value;

#[test]
fn a_daemon_built_line_reports_like_a_local_build() {
    let component = &COMPONENTS[0];
    let body = Value::map([
        ("provisioning", Value::text(component.name)),
        ("already", Value::Bool(false)),
        ("prefix", Value::text("/somewhere/prefix")),
        ("log", Value::text("/somewhere/prefix/provision.log")),
        ("toolchain", Value::text("gcc 1.0\ncmake 2.0")),
        ("recorded", Value::text("/somewhere/record.jsonl")),
        ("usable_engine", Value::Bool(true)),
        ("done", Value::Bool(true)),
    ]);
    let response = report(component, "", "by the daemon", came_from(&body));
    assert!(response.served);
    for expected in [
        "  into    /somewhere/prefix",
        "    gcc 1.0",
        "    cmake 2.0",
        "  log kept at /somewhere/prefix/provision.log",
        "  recorded in /somewhere/record.jsonl",
        "  built by the daemon",
        "  the daemon now reaches it as an engine",
    ] {
        assert!(
            response.text.contains(expected),
            "{expected}\n{}",
            response.text
        );
    }
}

#[test]
fn a_failed_record_is_said() {
    let failure = mcf_core::Failure::new(
        Category::ResourceDiskReadonly,
        Attribution::Machine,
        Disposition::Refused,
        Subsystem::new("mcf-cli::provision::tests"),
        "the record could not be opened",
    );
    let body = Value::map([
        ("already", Value::Bool(false)),
        ("prefix", Value::text("/p")),
        ("recorded", mcf_record::encode::failure(&failure)),
        ("done", Value::Bool(true)),
    ]);
    match came_from(&body) {
        Came::Built { recorded, .. } => {
            let why = recorded.expect_err("a failure map is not a path");
            assert!(why.contains("could not be opened"), "{why}");
        }
        Came::Already { .. } | Came::Refused(_) => panic!("a finished build"),
    }
}

#[test]
fn an_already_line_is_already() {
    let body = Value::map([
        ("already", Value::Bool(true)),
        ("prefix", Value::text("/p")),
        ("done", Value::Bool(true)),
    ]);
    assert!(matches!(came_from(&body), Came::Already { .. }));
    assert!(matches!(
        came_from(&Value::map([("done", Value::Bool(true))])),
        Came::Refused(_)
    ));
}
