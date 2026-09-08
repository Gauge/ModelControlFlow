#![allow(clippy::panic)]

#[test]
fn nothing_converts_a_degraded_value_back() {
    let source = degradation_source();
    for forbidden in [
        "impl<T> Deref for Degraded",
        "Deref for Degraded",
        "DerefMut",
        "AsRef<T> for Degraded",
        "From<Degraded<T>> for T",
        "pub fn into_inner",
        "pub fn unwrap",
        "pub fn assume_full",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a degraded value be used as an undegraded one (A5)"
        );
    }
}

#[test]
fn the_only_way_out_returns_the_mark_as_well() {
    let source = degradation_source();
    assert!(
        source.contains("pub fn into_parts(self) -> (T, Degradation)"),
        "the escape hatch no longer returns the mark, so this check is passing \
         against a type that has changed shape"
    );
}

#[test]
fn every_combinator_stays_degraded() {
    let source = degradation_source();
    let inside = block(&source, "impl<T> Degraded<T> {");
    for signature in public_functions(&inside) {
        let returns_plain_value = signature.contains("-> T")
            && !signature.contains("-> Degraded")
            && !signature.contains("(T, Degradation)");
        assert!(
            !returns_plain_value,
            "`{signature}` returns an unmarked value from a marked one (A5)"
        );
    }
}

#[test]
fn the_rendering_names_the_degradation() {
    let source = degradation_source();
    let display = block(
        &source,
        "impl<T: fmt::Display> fmt::Display for Degraded<T> {",
    );
    assert!(
        display.contains("self.degradation"),
        "the rendering of a degraded value no longer names its mark (A5)"
    );
}

fn degradation_source() -> String {
    let path = mcf_checks::workspace::root().join("crates/mcf-core/src/degradation/mod.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn block(source: &str, opening: &str) -> String {
    let (_, rest) = source
        .split_once(opening)
        .unwrap_or_else(|| panic!("`{opening}` is no longer where this check looks"));
    let (body, _) = rest
        .split_once("\n}")
        .unwrap_or_else(|| panic!("`{opening}` is never closed"));
    body.to_owned()
}

fn public_functions(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut lines = source.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        if !line.starts_with("pub fn ") && !line.starts_with("pub const fn ") {
            continue;
        }
        let mut signature = line.to_owned();
        while !signature.contains('{') {
            match lines.next() {
                Some(continuation) => {
                    signature.push(' ');
                    signature.push_str(continuation);
                }
                None => break,
            }
        }
        found.push(signature);
    }
    found
}
