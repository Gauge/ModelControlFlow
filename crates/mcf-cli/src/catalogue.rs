//! The coding catalogue: challenges with a tier, a category, a statement
//! in words, and cases in a form no language owns, run in every language
//! MCF has an image for (B-563, D56).
//!
//! A challenge names a function, its parameters' kinds and its return
//! kind; a case gives the arguments and the result as literals. Each
//! language renders those in its own words. Every case is held by a
//! reference solution in Python that a test runs in the container, so no
//! model is graded against a case nobody can pass.

/// How hard a challenge is meant to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tier {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl Tier {
    /// The tier's name, as the rows' dimension and the flag's word.
    #[must_use]
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
            Self::Expert => "expert",
        }
    }

    /// The tier of a name.
    #[must_use]
    pub(crate) fn named(name: &str) -> Option<Self> {
        match name {
            "easy" => Some(Self::Easy),
            "medium" => Some(Self::Medium),
            "hard" => Some(Self::Hard),
            "expert" => Some(Self::Expert),
            _ => None,
        }
    }
}

/// The kinds a parameter or a result can be, small enough that every
/// language renders them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Int,
    Bool,
    Text,
    Ints,
    Texts,
    OptInt,
}

/// A literal of one of the kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lit {
    Int(i64),
    Bool(bool),
    Text(&'static str),
    Ints(&'static [i64]),
    Texts(&'static [&'static str]),
    None,
}

/// One case: the arguments, in the parameters' order, and the result.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Case {
    pub args: &'static [Lit],
    pub expects: Lit,
}

/// One challenge.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Challenge {
    pub name: &'static str,
    pub tier: Tier,
    pub category: &'static str,
    pub statement: &'static str,
    pub function: &'static str,
    pub params: &'static [(&'static str, Kind)],
    pub returns: Kind,
    pub cases: &'static [Case],
}

macro_rules! case {
    ([$($arg:expr),*] => $expects:expr) => {
        Case { args: &[$($arg),*], expects: $expects }
    };
}

use Kind::{Bool, Int, Ints, OptInt, Text, Texts};
use Lit::{Int as I, Ints as IS, None as NONE, Text as T, Texts as TS};

/// The catalogue, easiest first.
pub(crate) const CHALLENGES: &[Challenge] = &[
    Challenge {
        name: "merge-sorted",
        tier: Tier::Easy,
        category: "arrays",
        function: "merge",
        statement: "Merge two lists of integers, each already sorted ascending, into one sorted list containing every element of both, without calling a sort.",
        params: &[("a", Ints), ("b", Ints)],
        returns: Ints,
        cases: &[
            case!([IS(&[1,3,5]), IS(&[2,4,6])] => IS(&[1,2,3,4,5,6])),
            case!([IS(&[]), IS(&[1])] => IS(&[1])),
            case!([IS(&[2,2]), IS(&[2])] => IS(&[2,2,2])),
        ],
    },
    Challenge {
        name: "balanced-brackets",
        tier: Tier::Easy,
        category: "strings",
        function: "balanced",
        statement: "Return whether every bracket in the string is closed in the right order; the brackets are (), [] and {}.",
        params: &[("s", Text)],
        returns: Bool,
        cases: &[
            case!([T("([]{})")] => Lit::Bool(true)),
            case!([T("([)]")] => Lit::Bool(false)),
            case!([T("")] => Lit::Bool(true)),
            case!([T("{[}")] => Lit::Bool(false)),
        ],
    },
    Challenge {
        name: "run-length",
        tier: Tier::Easy,
        category: "strings",
        function: "encode",
        statement: "Return the run-length encoding of the string as a list of strings, each a character followed by how many times it repeats in a row, in order.",
        params: &[("s", Text)],
        returns: Texts,
        cases: &[
            case!([T("aaabbc")] => TS(&["a3","b2","c1"])),
            case!([T("")] => TS(&[])),
            case!([T("ab")] => TS(&["a1","b1"])),
            case!([T("zzzzzzzzzzzz")] => TS(&["z12"])),
        ],
    },
    Challenge {
        name: "duration-seconds",
        tier: Tier::Easy,
        category: "parsing",
        function: "seconds",
        statement: "Turn a duration into a whole number of seconds. The text is a run of number-unit pairs where the unit is h, m or s, for example 1h30m; the pairs may come in any order and a unit may repeat, in which case they add up; empty text is 0.",
        params: &[("text", Text)],
        returns: Int,
        cases: &[
            case!([T("1h30m")] => I(5400)),
            case!([T("30m1h")] => I(5400)),
            case!([T("1h1h")] => I(7200)),
            case!([T("")] => I(0)),
            case!([T("45s")] => I(45)),
        ],
    },
    Challenge {
        name: "rle-decode",
        tier: Tier::Easy,
        category: "strings",
        function: "decode",
        statement: "Expand a run-length encoding: the text is a run of items, each one character followed by an optional decimal count that may have more than one digit; a missing count means one; a count of zero means the character does not appear.",
        params: &[("text", Text)],
        returns: Text,
        cases: &[
            case!([T("a12b")] => T("aaaaaaaaaaaab")),
            case!([T("ab")] => T("ab")),
            case!([T("a0b2")] => T("bb")),
            case!([T("")] => T("")),
        ],
    },
    Challenge {
        name: "roman-to-integer",
        tier: Tier::Easy,
        category: "parsing",
        function: "roman",
        statement: "Convert a Roman numeral written with the letters I, V, X, L, C, D and M, including the subtractive forms IV, IX, XL, XC, CD and CM, to an integer.",
        params: &[("s", Text)],
        returns: Int,
        cases: &[
            case!([T("XIV")] => I(14)),
            case!([T("MCMXCIV")] => I(1994)),
            case!([T("III")] => I(3)),
            case!([T("XLII")] => I(42)),
        ],
    },
    Challenge {
        name: "median-doubled",
        tier: Tier::Easy,
        category: "math",
        function: "doubled_median",
        statement: "Return twice the median of a non-empty list of integers, so the result is a whole number: twice the middle value when the count is odd, the sum of the two middle values when it is even.",
        params: &[("xs", Ints)],
        returns: Int,
        cases: &[
            case!([IS(&[3,1,2])] => I(4)),
            case!([IS(&[4,1,3,2])] => I(5)),
            case!([IS(&[7])] => I(14)),
            case!([IS(&[1,2,3,4,5,6])] => I(7)),
        ],
    },
    Challenge {
        name: "two-sum-indices",
        tier: Tier::Easy,
        category: "arrays",
        function: "two_sum",
        statement: "Given a list of integers and a target, return the indices i and j, with i < j, of the first pair (smallest i, then smallest j) whose values add up to the target, as a two-element list.",
        params: &[("xs", Ints), ("target", Int)],
        returns: Ints,
        cases: &[
            case!([IS(&[2,7,11,15]), I(9)] => IS(&[0,1])),
            case!([IS(&[3,2,4]), I(6)] => IS(&[1,2])),
            case!([IS(&[1,5,5]), I(10)] => IS(&[1,2])),
        ],
    },
    Challenge {
        name: "primes-below",
        tier: Tier::Easy,
        category: "math",
        function: "primes_below",
        statement: "Return how many prime numbers are strictly less than n.",
        params: &[("n", Int)],
        returns: Int,
        cases: &[
            case!([I(10)] => I(4)),
            case!([I(100)] => I(25)),
            case!([I(2)] => I(0)),
            case!([I(1000)] => I(168)),
        ],
    },
    Challenge {
        name: "fizzbuzz-sum",
        tier: Tier::Easy,
        category: "math",
        function: "fizzbuzz_sum",
        statement: "Return the sum of every integer from 1 to n inclusive that is divisible by 3 or by 5.",
        params: &[("n", Int)],
        returns: Int,
        cases: &[
            case!([I(10)] => I(33)),
            case!([I(15)] => I(60)),
            case!([I(0)] => I(0)),
            case!([I(999)] => I(233_168)),
        ],
    },
    Challenge {
        name: "binary-search-insert",
        tier: Tier::Easy,
        category: "arrays",
        function: "insert_position",
        statement: "Given a sorted list of distinct integers and a target, return the index where the target is found, or the index where it would be inserted to keep the list sorted.",
        params: &[("xs", Ints), ("target", Int)],
        returns: Int,
        cases: &[
            case!([IS(&[1,3,5,6]), I(5)] => I(2)),
            case!([IS(&[1,3,5,6]), I(2)] => I(1)),
            case!([IS(&[1,3,5,6]), I(7)] => I(4)),
            case!([IS(&[]), I(3)] => I(0)),
        ],
    },
    Challenge {
        name: "caesar-shift",
        tier: Tier::Easy,
        category: "strings",
        function: "shift",
        statement: "Shift every letter of the text forward by k places in the alphabet, wrapping from z to a and keeping each letter's case; every other character stays as it is.",
        params: &[("text", Text), ("k", Int)],
        returns: Text,
        cases: &[
            case!([T("abc xyz"), I(3)] => T("def abc")),
            case!([T("Hello, World!"), I(13)] => T("Uryyb, Jbeyq!")),
            case!([T(""), I(5)] => T("")),
            case!([T("Zz"), I(1)] => T("Aa")),
        ],
    },
    Challenge {
        name: "lcm",
        tier: Tier::Easy,
        category: "math",
        function: "lcm",
        statement: "Return the least common multiple of two non-negative integers; the least common multiple of anything with 0 is 0.",
        params: &[("a", Int), ("b", Int)],
        returns: Int,
        cases: &[
            case!([I(4), I(6)] => I(12)),
            case!([I(7), I(3)] => I(21)),
            case!([I(0), I(5)] => I(0)),
            case!([I(21), I(6)] => I(42)),
        ],
    },
    Challenge {
        name: "kth-largest",
        tier: Tier::Easy,
        category: "arrays",
        function: "kth_largest",
        statement: "Return the k-th largest element of a list of integers, counting from 1 and counting repeated values each time they occur.",
        params: &[("xs", Ints), ("k", Int)],
        returns: Int,
        cases: &[
            case!([IS(&[3,2,1,5,6,4]), I(2)] => I(5)),
            case!([IS(&[3,2,3,1,2,4,5,5,6]), I(4)] => I(4)),
            case!([IS(&[1]), I(1)] => I(1)),
        ],
    },
    Challenge {
        name: "edit-distance",
        tier: Tier::Medium,
        category: "strings",
        function: "distance",
        statement: "Return the Levenshtein edit distance between two strings: the fewest single-character insertions, deletions or substitutions that turn the first into the second.",
        params: &[("a", Text), ("b", Text)],
        returns: Int,
        cases: &[
            case!([T("kitten"), T("sitting")] => I(3)),
            case!([T(""), T("abc")] => I(3)),
            case!([T("same"), T("same")] => I(0)),
            case!([T("flaw"), T("lawn")] => I(2)),
        ],
    },
    Challenge {
        name: "spiral-order",
        tier: Tier::Medium,
        category: "arrays",
        function: "spiral",
        statement: "Given a rectangular grid of integers as one flat list in row-major order and its width, return its elements in clockwise spiral order starting at the top-left and going right first.",
        params: &[("cells", Ints), ("width", Int)],
        returns: Ints,
        cases: &[
            case!([IS(&[1,2,3,4,5,6,7,8,9]), I(3)] => IS(&[1,2,3,6,9,8,7,4,5])),
            case!([IS(&[1,2,3,4,5,6]), I(2)] => IS(&[1,2,4,6,5,3])),
            case!([IS(&[1]), I(1)] => IS(&[1])),
            case!([IS(&[1,2,3,4]), I(4)] => IS(&[1,2,3,4])),
        ],
    },
    Challenge {
        name: "glob-match",
        tier: Tier::Medium,
        category: "strings",
        function: "matches",
        statement: "Return whether the whole text matches the pattern, where in the pattern ? matches exactly one character and * matches any run of characters including none, and every other character matches itself; do not use a regular-expression library.",
        params: &[("pattern", Text), ("text", Text)],
        returns: Bool,
        cases: &[
            case!([T("*a*b"), T("xaxbx")] => Lit::Bool(false)),
            case!([T("a?c"), T("abc")] => Lit::Bool(true)),
            case!([T("*"), T("")] => Lit::Bool(true)),
            case!([T("a*b*c"), T("axxbyyc")] => Lit::Bool(true)),
            case!([T("a*"), T("ba")] => Lit::Bool(false)),
        ],
    },
    Challenge {
        name: "days-between",
        tier: Tier::Medium,
        category: "dates",
        function: "between",
        statement: "Given two dates as YYYY-MM-DD strings, return the number of whole days between them as a non-negative integer, using the Gregorian leap-year rules and no date library.",
        params: &[("a", Text), ("b", Text)],
        returns: Int,
        cases: &[
            case!([T("2024-02-28"), T("2024-03-01")] => I(2)),
            case!([T("1900-02-28"), T("1900-03-01")] => I(1)),
            case!([T("2020-01-01"), T("2020-01-01")] => I(0)),
            case!([T("2023-12-31"), T("2025-01-01")] => I(367)),
        ],
    },
    Challenge {
        name: "intervals-merge",
        tier: Tier::Medium,
        category: "arrays",
        function: "merge_intervals",
        statement: "Given intervals as one flat list of start, end pairs, return the merged intervals sorted by start as a flat list of pairs; intervals are half-open, so [1,2] and [2,3] do not overlap and are not merged.",
        params: &[("xs", Ints)],
        returns: Ints,
        cases: &[
            case!([IS(&[1,2,2,3])] => IS(&[1,2,2,3])),
            case!([IS(&[1,5,2,3,6,8])] => IS(&[1,5,6,8])),
            case!([IS(&[])] => IS(&[])),
            case!([IS(&[5,7,1,3,2,6])] => IS(&[1,7])),
        ],
    },
    Challenge {
        name: "brackets-in-quotes",
        tier: Tier::Medium,
        category: "strings",
        function: "balanced_quoted",
        statement: "Return whether the brackets (), [] and {} in the text are balanced, where a single quote starts a quoted stretch that the next single quote ends, brackets inside a quoted stretch do not count, and text ending inside a quoted stretch is not balanced.",
        params: &[("text", Text)],
        returns: Bool,
        cases: &[
            case!([T("(a['b'])")] => Lit::Bool(true)),
            case!([T("('(')")] => Lit::Bool(true)),
            case!([T("(']')")] => Lit::Bool(true)),
            case!([T("('abc")] => Lit::Bool(false)),
            case!([T("(]")] => Lit::Bool(false)),
        ],
    },
    Challenge {
        name: "parse-duration",
        tier: Tier::Medium,
        category: "parsing",
        function: "seconds_or_none",
        statement: "Parse a duration written as hours and minutes, like 1h30m, 2h or 45m, and return the total number of seconds; return none (null) for anything not in that form, such as an empty string, 90 or 1h30.",
        params: &[("text", Text)],
        returns: OptInt,
        cases: &[
            case!([T("1h30m")] => I(5400)),
            case!([T("45m")] => I(2700)),
            case!([T("2h")] => I(7200)),
            case!([T("1h30")] => NONE),
            case!([T("")] => NONE),
            case!([T("90")] => NONE),
        ],
    },
    Challenge {
        name: "word-frequency",
        tier: Tier::Medium,
        category: "strings",
        function: "top_words",
        statement: "Split the text on whitespace, lowercase the words, strip the characters .,;:!? from each end of each word, and return the k most frequent words as strings of the form word space count, most frequent first, words with the same count in alphabetical order.",
        params: &[("text", Text), ("k", Int)],
        returns: Texts,
        cases: &[
            case!([T("the cat and the hat. The end!"), I(2)] => TS(&["the 3","and 1"])),
            case!([T("b a b a c"), I(3)] => TS(&["a 2","b 2","c 1"])),
            case!([T(""), I(2)] => TS(&[])),
        ],
    },
    Challenge {
        name: "anagram-groups",
        tier: Tier::Medium,
        category: "strings",
        function: "anagram_groups",
        statement: "Return how many groups the words form when two words are in the same group exactly when they are anagrams of each other.",
        params: &[("words", Texts)],
        returns: Int,
        cases: &[
            case!([TS(&["eat","tea","tan","ate","nat","bat"])] => I(3)),
            case!([TS(&[])] => I(0)),
            case!([TS(&["a","a"])] => I(1)),
            case!([TS(&["ab","ba","abc"])] => I(2)),
        ],
    },
    Challenge {
        name: "matrix-rotate",
        tier: Tier::Medium,
        category: "arrays",
        function: "rotate",
        statement: "Given an n by n grid of integers as one flat list in row-major order, return the grid rotated a quarter turn clockwise, as a flat list in row-major order.",
        params: &[("cells", Ints), ("n", Int)],
        returns: Ints,
        cases: &[
            case!([IS(&[1,2,3,4]), I(2)] => IS(&[3,1,4,2])),
            case!([IS(&[1,2,3,4,5,6,7,8,9]), I(3)] => IS(&[7,4,1,8,5,2,9,6,3])),
            case!([IS(&[5]), I(1)] => IS(&[5])),
        ],
    },
    Challenge {
        name: "bits-in-range",
        tier: Tier::Medium,
        category: "bits",
        function: "total_bits",
        statement: "Return the total number of 1 bits in the binary representations of every integer from 0 to n inclusive.",
        params: &[("n", Int)],
        returns: Int,
        cases: &[
            case!([I(5)] => I(7)),
            case!([I(0)] => I(0)),
            case!([I(15)] => I(32)),
            case!([I(1000)] => I(4938)),
        ],
    },
    Challenge {
        name: "power-mod",
        tier: Tier::Medium,
        category: "math",
        function: "power_mod",
        statement: "Return base raised to the power exp, modulo m, for non-negative exp and m greater than 1, without overflowing 64-bit integers on the way.",
        params: &[("base", Int), ("exp", Int), ("m", Int)],
        returns: Int,
        cases: &[
            case!([I(2), I(10), I(1000)] => I(24)),
            case!([I(3), I(0), I(7)] => I(1)),
            case!([I(5), I(3), I(13)] => I(8)),
            case!([I(2), I(30), I(1000)] => I(824)),
            case!([I(7), I(100), I(13)] => I(9)),
        ],
    },
    Challenge {
        name: "subset-sum",
        tier: Tier::Medium,
        category: "dynamic programming",
        function: "can_sum",
        statement: "Return whether some subset of the non-negative integers in the list, each used at most once, adds up exactly to the target; the empty subset sums to 0.",
        params: &[("xs", Ints), ("target", Int)],
        returns: Bool,
        cases: &[
            case!([IS(&[3,34,4,12,5,2]), I(9)] => Lit::Bool(true)),
            case!([IS(&[3,34,4,12,5,2]), I(30)] => Lit::Bool(false)),
            case!([IS(&[]), I(0)] => Lit::Bool(true)),
            case!([IS(&[1,2]), I(4)] => Lit::Bool(false)),
        ],
    },
    Challenge {
        name: "valid-ipv4",
        tier: Tier::Medium,
        category: "parsing",
        function: "is_ipv4",
        statement: "Return whether the text is a valid dotted IPv4 address: exactly four decimal numbers from 0 to 255 separated by single dots, with no leading zeros except the number 0 itself and nothing else in the text.",
        params: &[("text", Text)],
        returns: Bool,
        cases: &[
            case!([T("192.168.1.1")] => Lit::Bool(true)),
            case!([T("256.1.1.1")] => Lit::Bool(false)),
            case!([T("01.1.1.1")] => Lit::Bool(false)),
            case!([T("1.1.1")] => Lit::Bool(false)),
            case!([T("0.0.0.0")] => Lit::Bool(true)),
            case!([T("1.1.1.1.")] => Lit::Bool(false)),
        ],
    },
    Challenge {
        name: "min-add-parentheses",
        tier: Tier::Medium,
        category: "strings",
        function: "min_add",
        statement: "Given a string of only ( and ), return the fewest parentheses that must be added anywhere to make it balanced.",
        params: &[("s", Text)],
        returns: Int,
        cases: &[
            case!([T("())")] => I(1)),
            case!([T("(((")] => I(3)),
            case!([T("()")] => I(0)),
            case!([T("()))((")] => I(4)),
            case!([T("")] => I(0)),
        ],
    },
    Challenge {
        name: "josephus",
        tier: Tier::Medium,
        category: "simulation",
        function: "josephus",
        statement: "n people stand in a circle numbered 1 to n; counting from person 1, every k-th person is removed and counting resumes from the next; return the number of the last person left.",
        params: &[("n", Int), ("k", Int)],
        returns: Int,
        cases: &[
            case!([I(7), I(3)] => I(4)),
            case!([I(1), I(1)] => I(1)),
            case!([I(5), I(2)] => I(3)),
            case!([I(10), I(4)] => I(5)),
        ],
    },
    Challenge {
        name: "expression-value",
        tier: Tier::Hard,
        category: "parsing",
        function: "value",
        statement: "Evaluate an arithmetic expression given as a string and return an integer: the expression holds non-negative integers, the operators + - * / and parentheses; multiplication and division bind tighter than addition and subtraction, division truncates toward zero, and no expression evaluator of the language may be used.",
        params: &[("s", Text)],
        returns: Int,
        cases: &[
            case!([T("2+3*4")] => I(14)),
            case!([T("(2+3)*4")] => I(20)),
            case!([T("7/2")] => I(3)),
            case!([T("10-4-3")] => I(3)),
            case!([T("2*(3+(4-1))*2")] => I(24)),
        ],
    },
    Challenge {
        name: "shortest-path",
        tier: Tier::Hard,
        category: "graphs",
        function: "shortest",
        statement: "Given a directed graph as one flat list of triples u, v, w with non-negative integer weights, return the total weight of the cheapest path from start to goal, or -1 if there is none; a path from a node to itself costs 0.",
        params: &[("edges", Ints), ("start", Int), ("goal", Int)],
        returns: Int,
        cases: &[
            case!([IS(&[1,2,4,1,3,1,3,2,1]), I(1), I(2)] => I(2)),
            case!([IS(&[1,2,1]), I(2), I(1)] => I(-1)),
            case!([IS(&[]), I(1), I(1)] => I(0)),
            case!([IS(&[1,2,5,2,3,5,1,3,20,3,4,1]), I(1), I(4)] => I(11)),
        ],
    },
    Challenge {
        name: "topological-order",
        tier: Tier::Hard,
        category: "graphs",
        function: "order",
        statement: "The vertices are 1 to n and the edges are given as one flat list of pairs u, v meaning u must come before v; return the lexicographically smallest ordering of all vertices that satisfies every edge, or an empty list if none exists.",
        params: &[("n", Int), ("edges", Ints)],
        returns: Ints,
        cases: &[
            case!([I(4), IS(&[1,2,1,3,3,4])] => IS(&[1,2,3,4])),
            case!([I(2), IS(&[1,2,2,1])] => IS(&[])),
            case!([I(3), IS(&[])] => IS(&[1,2,3])),
            case!([I(4), IS(&[4,1,3,1])] => IS(&[2,3,4,1])),
        ],
    },
    Challenge {
        name: "n-queens",
        tier: Tier::Hard,
        category: "search",
        function: "count_queens",
        statement: "Return how many ways n queens can be placed on an n by n chessboard so that no two attack each other.",
        params: &[("n", Int)],
        returns: Int,
        cases: &[
            case!([I(6)] => I(4)),
            case!([I(8)] => I(92)),
            case!([I(1)] => I(1)),
            case!([I(4)] => I(2)),
        ],
    },
    Challenge {
        name: "lru-cache",
        tier: Tier::Hard,
        category: "data structures",
        function: "run_cache",
        statement: "Simulate a least-recently-used cache of the given capacity over a list of operations, each a string of the form put KEY VALUE or get KEY with whole-number keys and values; return the results of the get operations in order, -1 when a key is absent; both put and get count as a use.",
        params: &[("capacity", Int), ("ops", Texts)],
        returns: Ints,
        cases: &[
            case!([I(2), TS(&["put 1 1","put 2 2","get 1","put 3 3","get 2"])] => IS(&[1,-1])),
            case!([I(1), TS(&["put 1 1","put 2 2","get 1","get 2"])] => IS(&[-1,2])),
            case!([I(2), TS(&["get 9"])] => IS(&[-1])),
            case!([I(2), TS(&["put 1 1","put 1 5","put 2 2","put 3 3","get 1"])] => IS(&[-1])),
        ],
    },
    Challenge {
        name: "longest-increasing-subsequence",
        tier: Tier::Hard,
        category: "dynamic programming",
        function: "lis",
        statement: "Return the length of the longest strictly increasing subsequence of the list; a subsequence keeps the order but need not be contiguous.",
        params: &[("xs", Ints)],
        returns: Int,
        cases: &[
            case!([IS(&[10,9,2,5,3,7,101,18])] => I(4)),
            case!([IS(&[0,1,0,3,2,3])] => I(4)),
            case!([IS(&[7,7,7])] => I(1)),
            case!([IS(&[])] => I(0)),
        ],
    },
    Challenge {
        name: "coin-change",
        tier: Tier::Hard,
        category: "dynamic programming",
        function: "fewest_coins",
        statement: "Given coin denominations and an amount, return the fewest coins that make the amount exactly, using each denomination any number of times, or -1 if it cannot be made; 0 needs no coins.",
        params: &[("coins", Ints), ("amount", Int)],
        returns: Int,
        cases: &[
            case!([IS(&[1,2,5]), I(11)] => I(3)),
            case!([IS(&[2]), I(3)] => I(-1)),
            case!([IS(&[1]), I(0)] => I(0)),
            case!([IS(&[3,7]), I(20)] => I(4)),
        ],
    },
    Challenge {
        name: "min-path-sum",
        tier: Tier::Hard,
        category: "graphs",
        function: "min_path_sum",
        statement: "Given a grid of non-negative integers as one flat list in row-major order and its width, return the smallest possible sum of the cells on a path from the top-left cell to the bottom-right cell moving only right or down.",
        params: &[("cells", Ints), ("width", Int)],
        returns: Int,
        cases: &[
            case!([IS(&[1,3,1,1,5,1,4,2,1]), I(3)] => I(7)),
            case!([IS(&[1,2,3,4]), I(2)] => I(7)),
            case!([IS(&[5]), I(1)] => I(5)),
            case!([IS(&[1,2,5,3,2,1]), I(3)] => I(6)),
        ],
    },
    Challenge {
        name: "median-of-two-sorted",
        tier: Tier::Hard,
        category: "arrays",
        function: "doubled_median_of_two",
        statement: "Given two sorted lists of integers, at least one non-empty, return twice the median of all their elements together, so that the result is a whole number.",
        params: &[("a", Ints), ("b", Ints)],
        returns: Int,
        cases: &[
            case!([IS(&[1,3]), IS(&[2])] => I(4)),
            case!([IS(&[1,2]), IS(&[3,4])] => I(5)),
            case!([IS(&[]), IS(&[1])] => I(2)),
            case!([IS(&[1,1,1]), IS(&[1,1])] => I(2)),
        ],
    },
    Challenge {
        name: "trapped-rain",
        tier: Tier::Hard,
        category: "arrays",
        function: "trapped",
        statement: "Given an elevation map as a list of non-negative bar heights of width 1, return how much water it can trap after raining.",
        params: &[("heights", Ints)],
        returns: Int,
        cases: &[
            case!([IS(&[0,1,0,2,1,0,1,3,2,1,2,1])] => I(6)),
            case!([IS(&[4,2,0,3,2,5])] => I(9)),
            case!([IS(&[1])] => I(0)),
            case!([IS(&[])] => I(0)),
        ],
    },
    Challenge {
        name: "word-break",
        tier: Tier::Hard,
        category: "dynamic programming",
        function: "can_segment",
        statement: "Return whether the string can be split into a sequence of one or more words, each taken from the given list, reusing words as needed.",
        params: &[("s", Text), ("words", Texts)],
        returns: Bool,
        cases: &[
            case!([T("leetcode"), TS(&["leet","code"])] => Lit::Bool(true)),
            case!([T("applepenapple"), TS(&["apple","pen"])] => Lit::Bool(true)),
            case!([T("catsandog"), TS(&["cats","dog","sand","and","cat"])] => Lit::Bool(false)),
            case!([T(""), TS(&["a"])] => Lit::Bool(false)),
        ],
    },
    Challenge {
        name: "regex-lite",
        tier: Tier::Expert,
        category: "parsing",
        function: "matches_pattern",
        statement: "Return whether the whole text matches the pattern, where in the pattern . matches any single character and * means zero or more of the preceding element, and every other character matches itself; implement the matching yourself.",
        params: &[("pattern", Text), ("text", Text)],
        returns: Bool,
        cases: &[
            case!([T("a*b"), T("aaab")] => Lit::Bool(true)),
            case!([T(".*"), T("")] => Lit::Bool(true)),
            case!([T("ab*c"), T("ac")] => Lit::Bool(true)),
            case!([T("a.c"), T("abd")] => Lit::Bool(false)),
            case!([T("c*a*b"), T("aab")] => Lit::Bool(true)),
            case!([T("mis*is*p*."), T("mississippi")] => Lit::Bool(false)),
        ],
    },
    Challenge {
        name: "nine-grid-valid",
        tier: Tier::Expert,
        category: "arrays",
        function: "valid_grid",
        statement: "Given a 9 by 9 number-placement grid as one flat list of 81 integers in row-major order, with 0 for an empty cell, return whether the filled cells break no rule: no digit repeats within a row, a column or one of the nine 3 by 3 boxes; the board need not be solvable.",
        params: &[("cells", Ints)],
        returns: Bool,
        cases: &[
            case!([IS(&[0;81])] => Lit::Bool(true)),
            case!([IS(&[5,5,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0])] => Lit::Bool(false)),
            case!([IS(&[5,0,0,0,0,0,0,0,0, 0,5,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0])] => Lit::Bool(false)),
            case!([IS(&[5,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,5,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0])] => Lit::Bool(true)),
            case!([IS(&[5,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0, 5,0,0,0,0,0,0,0,0])] => Lit::Bool(false)),
        ],
    },
    Challenge {
        name: "knapsack",
        tier: Tier::Expert,
        category: "dynamic programming",
        function: "knapsack",
        statement: "Given item weights and values as two lists of the same length and a capacity, return the greatest total value of a set of items whose total weight does not exceed the capacity, each item used at most once.",
        params: &[("weights", Ints), ("values", Ints), ("capacity", Int)],
        returns: Int,
        cases: &[
            case!([IS(&[1,3,4,5]), IS(&[1,4,5,7]), I(7)] => I(9)),
            case!([IS(&[10]), IS(&[5]), I(5)] => I(0)),
            case!([IS(&[]), IS(&[]), I(10)] => I(0)),
            case!([IS(&[2,3,4,5]), IS(&[3,4,5,6]), I(5)] => I(7)),
        ],
    },
    Challenge {
        name: "interval-scheduling",
        tier: Tier::Expert,
        category: "arrays",
        function: "most_meetings",
        statement: "Given meetings as one flat list of start, end pairs with half-open times, return the largest number of meetings one room can host without any two overlapping.",
        params: &[("meetings", Ints)],
        returns: Int,
        cases: &[
            case!([IS(&[1,3,2,4,3,5])] => I(2)),
            case!([IS(&[])] => I(0)),
            case!([IS(&[1,10,2,3,3,4,4,5])] => I(3)),
            case!([IS(&[0,1,0,1,0,1])] => I(1)),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::{CHALLENGES, Kind, Lit, Tier};

    #[test]
    fn every_challenge_is_whole() {
        let mut names = std::collections::BTreeSet::new();
        for challenge in CHALLENGES {
            assert!(names.insert(challenge.name), "{} twice", challenge.name);
            assert!(!challenge.cases.is_empty(), "{}", challenge.name);
            for case in challenge.cases {
                assert_eq!(
                    case.args.len(),
                    challenge.params.len(),
                    "{}",
                    challenge.name
                );
                for (arg, (_, kind)) in case.args.iter().zip(challenge.params) {
                    assert!(
                        fits(arg, *kind),
                        "{}: {arg:?} is not a {kind:?}",
                        challenge.name
                    );
                }
                assert!(
                    fits(&case.expects, challenge.returns),
                    "{}: {:?}",
                    challenge.name,
                    case.expects
                );
            }
        }
        assert!(CHALLENGES.iter().filter(|c| c.tier == Tier::Expert).count() >= 3);
        assert!(CHALLENGES.len() >= 40);
    }

    fn fits(lit: &Lit, kind: Kind) -> bool {
        matches!(
            (lit, kind),
            (Lit::Int(_), Kind::Int)
                | (Lit::Bool(_), Kind::Bool)
                | (Lit::Text(_), Kind::Text)
                | (Lit::Ints(_), Kind::Ints)
                | (Lit::Texts(_), Kind::Texts)
                | (Lit::Int(_) | Lit::None, Kind::OptInt)
        )
    }

    /// Every case of every challenge held by a reference solution in
    /// Python, run in the container: a case nobody can pass grades every
    /// model wrong. Needs podman.
    #[test]
    #[ignore = "needs podman and the pinned image; run with --ignored"]
    fn every_case_is_held_by_a_reference_solution() {
        let podman = std::path::Path::new("/usr/bin/podman");
        let scratch = std::env::temp_dir().join(format!("mcf-catalogue-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let mut wrong = Vec::new();
        for challenge in CHALLENGES {
            let program =
                crate::challenges::checker(&crate::challenges::PYTHON, challenge, REFERENCES);
            let said = crate::languages::run_program(
                podman,
                &scratch,
                &crate::challenges::PYTHON,
                &program,
            )
            .unwrap();
            let (_, ran, cases) =
                crate::challenges::read_harness(&said, challenge.cases.len(), false);
            if !ran || cases.iter().any(|(held, _)| !held) {
                wrong.push(format!("{}: {said}", challenge.name));
            }
        }
        let _gone = std::fs::remove_dir_all(&scratch);
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// The reference solutions, one a function.
    const REFERENCES: &str = r#"
def merge(a, b):
    i = j = 0; out = []
    while i < len(a) and j < len(b):
        if a[i] <= b[j]: out.append(a[i]); i += 1
        else: out.append(b[j]); j += 1
    return out + a[i:] + b[j:]

def balanced(s):
    pairs = {')': '(', ']': '[', '}': '{'}; st = []
    for c in s:
        if c in '([{': st.append(c)
        elif c in pairs:
            if not st or st.pop() != pairs[c]: return False
    return not st

def encode(s):
    out = []
    for c in s:
        if out and out[-1][0] == c: out[-1][1] += 1
        else: out.append([c, 1])
    return [c + str(n) for c, n in out]

def seconds(text):
    total = n = 0
    for c in text:
        if c.isdigit(): n = n * 10 + int(c)
        else: total += n * {'h': 3600, 'm': 60, 's': 1}[c]; n = 0
    return total

def decode(text):
    out = ''; i = 0
    while i < len(text):
        c = text[i]; i += 1; d = ''
        while i < len(text) and text[i].isdigit(): d += text[i]; i += 1
        out += c * (int(d) if d else 1)
    return out

def roman(s):
    v = {'I': 1, 'V': 5, 'X': 10, 'L': 50, 'C': 100, 'D': 500, 'M': 1000}; t = 0
    for i, c in enumerate(s):
        if i + 1 < len(s) and v[c] < v[s[i + 1]]: t -= v[c]
        else: t += v[c]
    return t

def doubled_median(xs):
    xs = sorted(xs); n = len(xs)
    return 2 * xs[n // 2] if n % 2 else xs[n // 2 - 1] + xs[n // 2]

def two_sum(xs, target):
    for i in range(len(xs)):
        for j in range(i + 1, len(xs)):
            if xs[i] + xs[j] == target: return [i, j]
    return []

def primes_below(n):
    if n < 3: return 0
    sieve = [True] * n; sieve[0] = sieve[1] = False
    for i in range(2, int(n ** 0.5) + 1):
        if sieve[i]:
            for j in range(i * i, n, i): sieve[j] = False
    return sum(sieve)

def fizzbuzz_sum(n):
    return sum(i for i in range(1, n + 1) if i % 3 == 0 or i % 5 == 0)

def insert_position(xs, target):
    lo, hi = 0, len(xs)
    while lo < hi:
        mid = (lo + hi) // 2
        if xs[mid] < target: lo = mid + 1
        else: hi = mid
    return lo

def shift(text, k):
    out = ''
    for c in text:
        if 'a' <= c <= 'z': out += chr((ord(c) - 97 + k) % 26 + 97)
        elif 'A' <= c <= 'Z': out += chr((ord(c) - 65 + k) % 26 + 65)
        else: out += c
    return out

def lcm(a, b):
    from math import gcd
    return 0 if a == 0 or b == 0 else a * b // gcd(a, b)

def kth_largest(xs, k):
    return sorted(xs, reverse=True)[k - 1]

def distance(a, b):
    prev = list(range(len(b) + 1))
    for i in range(1, len(a) + 1):
        cur = [i]
        for j in range(1, len(b) + 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (a[i - 1] != b[j - 1])))
        prev = cur
    return prev[len(b)]

def spiral(cells, width):
    h = len(cells) // width; m = [cells[r * width:(r + 1) * width] for r in range(h)]
    out = []; top, bottom, left, right = 0, h - 1, 0, width - 1
    while top <= bottom and left <= right:
        for j in range(left, right + 1): out.append(m[top][j])
        top += 1
        for i in range(top, bottom + 1): out.append(m[i][right])
        right -= 1
        if top <= bottom:
            for j in range(right, left - 1, -1): out.append(m[bottom][j])
            bottom -= 1
        if left <= right:
            for i in range(bottom, top - 1, -1): out.append(m[i][left])
            left += 1
    return out

def matches(pattern, text):
    m, n = len(pattern), len(text)
    dp = [[False] * (n + 1) for _ in range(m + 1)]; dp[0][0] = True
    for i in range(1, m + 1):
        if pattern[i - 1] == '*': dp[i][0] = dp[i - 1][0]
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            p = pattern[i - 1]
            dp[i][j] = (dp[i - 1][j] or dp[i][j - 1]) if p == '*' else (dp[i - 1][j - 1] and (p == '?' or p == text[j - 1]))
    return dp[m][n]

def between(a, b):
    def days(s):
        y, m, d = map(int, s.split('-'))
        leap = lambda yy: yy % 4 == 0 and (yy % 100 != 0 or yy % 400 == 0)
        ml = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        t = sum(366 if leap(yy) else 365 for yy in range(1, y))
        t += sum(ml[mm - 1] + (1 if mm == 2 and leap(y) else 0) for mm in range(1, m))
        return t + d
    return abs(days(a) - days(b))

def merge_intervals(xs):
    pairs = sorted((xs[i], xs[i + 1]) for i in range(0, len(xs), 2)); out = []
    for a, b in pairs:
        if out and a < out[-1][1]: out[-1][1] = max(out[-1][1], b)
        else: out.append([a, b])
    return [v for p in out for v in p]

def balanced_quoted(text):
    pairs = {')': '(', ']': '[', '}': '{'}; st = []; q = False
    for c in text:
        if c == "'": q = not q; continue
        if q: continue
        if c in '([{': st.append(c)
        elif c in pairs:
            if not st or st.pop() != pairs[c]: return False
    return not q and not st

def seconds_or_none(text):
    import re
    m = re.fullmatch(r'(?:(\d+)h)?(?:(\d+)m)?', text)
    if not m or text == '': return None
    return (int(m.group(1)) * 3600 if m.group(1) else 0) + (int(m.group(2)) * 60 if m.group(2) else 0)

def top_words(text, k):
    counts = {}
    for w in text.split():
        w = w.lower().strip('.,;:!?')
        if w: counts[w] = counts.get(w, 0) + 1
    ordered = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))[:k]
    return [f"{w} {c}" for w, c in ordered]

def anagram_groups(words):
    return len({''.join(sorted(w)) for w in words})

def rotate(cells, n):
    m = [cells[r * n:(r + 1) * n] for r in range(n)]
    return [m[n - 1 - i][j] for j in range(n) for i in range(n)]

def total_bits(n):
    return sum(bin(i).count('1') for i in range(n + 1))

def power_mod(base, exp, m):
    r = 1; base %= m
    while exp:
        if exp & 1: r = r * base % m
        base = base * base % m; exp >>= 1
    return r

def can_sum(xs, target):
    reach = {0}
    for x in xs: reach |= {r + x for r in reach if r + x <= target}
    return target in reach

def is_ipv4(text):
    parts = text.split('.')
    if len(parts) != 4: return False
    for p in parts:
        if not p.isdigit() or (len(p) > 1 and p[0] == '0') or int(p) > 255: return False
    return True

def min_add(s):
    opened = need = 0
    for c in s:
        if c == '(': opened += 1
        elif opened: opened -= 1
        else: need += 1
    return need + opened

def josephus(n, k):
    people = list(range(1, n + 1)); i = 0
    while len(people) > 1:
        i = (i + k - 1) % len(people); people.pop(i)
    return people[0]

def value(s):
    pos = [0]
    def num():
        if s[pos[0]] == '(':
            pos[0] += 1; v = expr(); pos[0] += 1; return v
        n = 0
        while pos[0] < len(s) and s[pos[0]].isdigit(): n = n * 10 + int(s[pos[0]]); pos[0] += 1
        return n
    def term():
        v = num()
        while pos[0] < len(s) and s[pos[0]] in '*/':
            op = s[pos[0]]; pos[0] += 1; r = num()
            v = v * r if op == '*' else int(v / r)
        return v
    def expr():
        v = term()
        while pos[0] < len(s) and s[pos[0]] in '+-':
            op = s[pos[0]]; pos[0] += 1; r = term()
            v = v + r if op == '+' else v - r
        return v
    return expr()

def shortest(edges, start, goal):
    import heapq
    adj = {}
    for i in range(0, len(edges), 3): adj.setdefault(edges[i], []).append((edges[i + 1], edges[i + 2]))
    dist = {start: 0}; heap = [(0, start)]
    while heap:
        d, u = heapq.heappop(heap)
        if u == goal: return d
        if d > dist.get(u, float('inf')): continue
        for v, w in adj.get(u, []):
            if d + w < dist.get(v, float('inf')): dist[v] = d + w; heapq.heappush(heap, (d + w, v))
    return -1

def order(n, edges):
    import heapq
    indeg = [0] * (n + 1); adj = [[] for _ in range(n + 1)]
    for i in range(0, len(edges), 2): adj[edges[i]].append(edges[i + 1]); indeg[edges[i + 1]] += 1
    ready = [v for v in range(1, n + 1) if indeg[v] == 0]; heapq.heapify(ready); out = []
    while ready:
        u = heapq.heappop(ready); out.append(u)
        for v in adj[u]:
            indeg[v] -= 1
            if indeg[v] == 0: heapq.heappush(ready, v)
    return out if len(out) == n else []

def count_queens(n):
    def place(row, cols, d1, d2):
        if row == n: return 1
        t = 0
        for c in range(n):
            if c in cols or row - c in d1 or row + c in d2: continue
            t += place(row + 1, cols | {c}, d1 | {row - c}, d2 | {row + c})
        return t
    return place(0, set(), set(), set())

def run_cache(capacity, ops):
    from collections import OrderedDict
    d = OrderedDict(); out = []
    for op in ops:
        parts = op.split()
        if parts[0] == 'put':
            k, v = int(parts[1]), int(parts[2])
            if k in d: d.pop(k)
            d[k] = v
            if len(d) > capacity: d.popitem(last=False)
        else:
            k = int(parts[1])
            if k in d: v = d.pop(k); d[k] = v; out.append(v)
            else: out.append(-1)
    return out

def lis(xs):
    import bisect
    tails = []
    for x in xs:
        i = bisect.bisect_left(tails, x)
        if i == len(tails): tails.append(x)
        else: tails[i] = x
    return len(tails)

def fewest_coins(coins, amount):
    best = [0] + [float('inf')] * amount
    for a in range(1, amount + 1):
        for c in coins:
            if c <= a and best[a - c] + 1 < best[a]: best[a] = best[a - c] + 1
    return -1 if best[amount] == float('inf') else best[amount]

def min_path_sum(cells, width):
    h = len(cells) // width; best = [[0] * width for _ in range(h)]
    for i in range(h):
        for j in range(width):
            v = cells[i * width + j]
            if i == 0 and j == 0: best[i][j] = v
            elif i == 0: best[i][j] = best[i][j - 1] + v
            elif j == 0: best[i][j] = best[i - 1][j] + v
            else: best[i][j] = min(best[i - 1][j], best[i][j - 1]) + v
    return best[h - 1][width - 1]

def doubled_median_of_two(a, b):
    return doubled_median(sorted(a + b))

def trapped(heights):
    n = len(heights)
    if n == 0: return 0
    left = [0] * n; right = [0] * n; m = 0
    for i in range(n): m = max(m, heights[i]); left[i] = m
    m = 0
    for i in range(n - 1, -1, -1): m = max(m, heights[i]); right[i] = m
    return sum(min(left[i], right[i]) - heights[i] for i in range(n))

def can_segment(s, words):
    if not s: return False
    ok = [True] + [False] * len(s)
    for i in range(1, len(s) + 1):
        ok[i] = any(ok[i - len(w)] and s[i - len(w):i] == w for w in words if len(w) <= i)
    return ok[len(s)]

def matches_pattern(pattern, text):
    from functools import lru_cache
    @lru_cache(None)
    def go(i, j):
        if i == len(pattern): return j == len(text)
        first = j < len(text) and pattern[i] in (text[j], '.')
        if i + 1 < len(pattern) and pattern[i + 1] == '*':
            return go(i + 2, j) or (first and go(i, j + 1))
        return first and go(i + 1, j + 1)
    return go(0, 0)

def valid_grid(cells):
    rows = [set() for _ in range(9)]; cols = [set() for _ in range(9)]; boxes = [set() for _ in range(9)]
    for i in range(81):
        v = cells[i]
        if v == 0: continue
        r, c = divmod(i, 9); b = (r // 3) * 3 + c // 3
        if v in rows[r] or v in cols[c] or v in boxes[b]: return False
        rows[r].add(v); cols[c].add(v); boxes[b].add(v)
    return True

def knapsack(weights, values, capacity):
    best = [0] * (capacity + 1)
    for w, v in zip(weights, values):
        for c in range(capacity, w - 1, -1): best[c] = max(best[c], best[c - w] + v)
    return best[capacity]

def most_meetings(meetings):
    pairs = sorted((meetings[i + 1], meetings[i]) for i in range(0, len(meetings), 2))
    count = 0; last_end = -10**18
    for end, start in pairs:
        if start >= last_end: count += 1; last_end = end
    return count
"#;
}
