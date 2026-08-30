# The layouts, as a spec that checks itself

These generate the four console screens and assert that every one is exactly
**80 columns by 24 rows** — the size every terminal has. They were the mockups
the layouts were agreed from, and they are kept here rather than thrown away so
that the shipped screens have something to be checked against.

    python3 build.py        # writes the four fragments and verifies each line

`COLS` and `ROWS` in `screens.py` are one constant each. Retargeting to a
roomier baseline is a two-line change, and everything regenerates verified.

The Rust screens in `../src/screens/` are what actually runs. These are the
drawing they are drawn from, and `checks` compares the two so a layout cannot
drift from its spec without something saying so.
