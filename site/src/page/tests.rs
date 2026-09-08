use super::*;

#[test]
fn what_a_stranger_wrote_cannot_become_markup() {
    let nasty = "<script>alert('x')</script> & \"quotes\"";
    let escaped = safe(nasty);
    assert!(!escaped.contains("<script"), "{escaped}");
    assert!(!escaped.contains('<'), "{escaped}");
    assert!(escaped.contains("&lt;script&gt;"), "{escaped}");
    assert!(escaped.contains("&amp;"), "{escaped}");
}

#[test]
fn a_row_with_markup_in_it_is_shown_as_text() {
    let held = Held {
        digest: "0123456789abcdef0123456789abcdef".to_owned(),
        at: 0,
        body: "comparison · <img src=x onerror=alert(1)> quicker than b.gguf by 1.0%".to_owned(),
    };
    let page = one(&held);
    assert!(!page.contains("<img"), "markup reached the page");
    assert!(page.contains("&lt;img"), "it was not shown either");
}

#[test]
fn even_a_name_is_escaped() {
    let page = kept("\"><script>x</script>");
    assert!(!page.contains("<script>x"), "{page}");
}

#[test]
fn the_page_loads_nothing_from_anywhere() {
    let held = Held {
        digest: "0123456789abcdef0123456789abcdef".to_owned(),
        at: 0,
        body: "absolute · a.gguf took 1 ns".to_owned(),
    };
    for page in [index(&[held.clone()]), one(&held), kept("abc"), refused("no")] {
        for reaching in ["http://", "https://", "//cdn", "<script src"] {
            assert!(!page.contains(reaching), "the page reaches for {reaching}");
        }
    }
}

#[test]
fn an_empty_archive_says_so() {
    let page = index(&[]);
    assert!(page.contains("Nothing has been published yet"), "{page}");
    assert!(page.contains("mcf share"), "it does not say how to publish");
}

#[test]
fn every_page_carries_the_terms() {
    let held = Held {
        digest: "0123456789abcdef0123456789abcdef".to_owned(),
        at: 0,
        body: "absolute · a.gguf took 1 ns".to_owned(),
    };
    for page in [index(&[held.clone()]), one(&held), kept("abc"), refused("no")] {
        assert!(page.contains("cannot be undone"), "a page omits the terms");
        assert!(page.contains("MCF cannot send"), "a page omits how it got here");
    }
}
