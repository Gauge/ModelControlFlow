use super::looping;

#[test]
fn a_repeated_line_is_a_loop() {
    let said = "        # check month and day-of-week rule already handled\n".repeat(30);
    let Some(why) = looping(&said) else {
        panic!("thirty repeats of one line is a loop");
    };
    assert!(why.starts_with("a line "), "{why}");
}

#[test]
fn a_short_cycle_at_the_end_is_a_loop_even_without_newlines() {
    let said = format!(
        "{}{}",
        "some genuine prose first. ",
        "abcabcabcabc".repeat(60)
    );
    assert!(looping(&said).is_some());
}

#[test]
fn ordinary_code_is_not_a_loop() {
    let said = "def parse(text):\n    out = []\n    for line in text.splitlines():\n        if not \
                line:\n            continue\n        out.append(line.strip())\n    return out\n\n\
                def render(rows):\n    return '\\n'.join(str(row) for row in rows)\n";
    assert!(looping(said).is_none(), "{:?}", looping(said));
}

#[test]
fn a_few_repeats_are_not_yet_a_loop() {
    let said = "    return None\n".repeat(5);
    assert!(looping(&said).is_none());
}

#[test]
fn short_output_is_never_called_a_loop() {
    assert!(looping("").is_none());
    assert!(looping("ok").is_none());
}

#[test]
fn a_loop_names_what_repeated_so_the_row_can_say_why() {
    let said = "xxxxxxxxxxxxxxxxxxxxxxxx\n".repeat(20);
    let Some(why) = looping(&said) else {
        panic!("a loop is found");
    };
    assert!(why.contains("times over"), "{why}");
}
