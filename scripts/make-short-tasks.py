#!/usr/bin/env python3
"""Write the short-answer corpus.

Every task here has one deterministic answer, and the answer is computed rather than
written down: the generator solves each problem as it poses it, so the key cannot
disagree with the question. Deterministic throughout — the same seed writes the same
corpus, so a reading taken against set 12 means the same thing next month.

Usage: python3 scripts/make-short-tasks.py [--out <dir>] [--per-set 25] [--sets 40]
"""

import argparse, json, math, os, random
from fractions import Fraction

WORDS = ["apple", "bridge", "candle", "domain", "ember", "forest", "garnet", "harbour",
         "ivory", "jasmine", "kettle", "lantern", "marble", "nectar", "obsidian", "pewter"]
NAMES = ["Ana", "Bram", "Cleo", "Dara", "Emil", "Fenn", "Gita", "Hugo"]


def arithmetic(r):
    a, b, c = r.randint(2, 40), r.randint(2, 12), r.randint(2, 30)
    return f"Compute {a} + {b} * {c}.", a + b * c


def precedence(r):
    a, b, c, d = r.randint(10, 90), r.randint(2, 9), r.randint(2, 9), r.randint(2, 20)
    return f"Compute ({a} - {b}) * {c} + {d}.", (a - b) * c + d


def percentage(r):
    whole = r.randrange(100, 9900, 25)
    part = r.choice([2, 4, 5, 8, 10, 12, 15, 16, 20, 24, 25, 30, 35, 40, 45, 50, 60, 64, 75, 80])
    return f"What is {part}% of {whole}?", whole * part // 100


def gcd_lcm(r):
    a, b = r.randint(12, 200), r.randint(12, 200)
    if r.random() < 0.5:
        return f"What is the greatest common divisor of {a} and {b}?", math.gcd(a, b)
    return f"What is the least common multiple of {a} and {b}?", a * b // math.gcd(a, b)


def primes(r):
    n = r.randint(20, 300)
    count = sum(1 for k in range(2, n + 1) if all(k % d for d in range(2, int(k**0.5) + 1)))
    return f"How many prime numbers are there from 1 to {n} inclusive?", count


def modular(r):
    a, m = r.randint(100, 9999), r.randint(3, 97)
    return f"What is the remainder when {a} is divided by {m}?", a % m


def power_mod(r):
    base, exp, m = r.randint(2, 12), r.randint(3, 20), r.randint(5, 97)
    return f"What is {base}^{exp} mod {m}?", pow(base, exp, m)


def base_convert(r):
    n = r.randint(20, 4000)
    which = r.choice(["binary", "hexadecimal", "octal"])
    said = {"binary": bin(n)[2:], "hexadecimal": hex(n)[2:].upper(), "octal": oct(n)[2:]}[which]
    return f"Write {n} in {which}. Give only the digits, no prefix.", said


def from_base(r):
    n = r.randint(20, 2000)
    which = r.choice([("binary", bin(n)[2:], 2), ("hexadecimal", hex(n)[2:].upper(), 16)])
    return f"The {which[0]} number {which[1]} is what in decimal?", n


def sequence_term(r):
    first, step, at = r.randint(1, 30), r.randint(2, 15), r.randint(5, 40)
    return (f"An arithmetic sequence starts at {first} and increases by {step} each term. "
            f"What is term number {at}? (The first term is term number 1.)",
            first + step * (at - 1))


def sequence_sum(r):
    first, step, count = r.randint(1, 20), r.randint(1, 9), r.randint(5, 30)
    total = count * (2 * first + (count - 1) * step) // 2
    return (f"An arithmetic sequence starts at {first} and increases by {step} each term. "
            f"What is the sum of its first {count} terms?", total)


def linear_equation(r):
    x = r.randint(-30, 60)
    a = r.randint(2, 15)
    b = r.randint(-50, 50)
    return f"Solve for x: {a}x + {b} = {a * x + b}. Give x.", x


def combinatorics(r):
    n = r.randint(5, 14)
    k = r.randint(2, min(5, n))
    if r.random() < 0.5:
        return f"In how many ways can {k} items be chosen from {n} distinct items, order not mattering?", math.comb(n, k)
    return f"In how many ways can {k} items be arranged from {n} distinct items, order mattering?", math.perm(n, k)


def rate_problem(r):
    speed, hours = r.randint(20, 120), r.randint(2, 12)
    return f"A vehicle travels at {speed} km/h for {hours} hours. How many kilometres does it cover?", speed * hours


def work_problem(r):
    a, b = r.choice([(4, 6), (3, 6), (2, 3), (6, 12), (5, 20), (10, 15), (8, 24),
                     (9, 18), (12, 24), (7, 42), (10, 40), (14, 35), (6, 30), (15, 10)])
    together = Fraction(1, a) + Fraction(1, b)
    hours = 1 / together
    if hours.denominator != 1:
        return None
    return (f"One pump fills a tank in {a} hours and another fills it in {b} hours. "
            f"Working together, how many hours do they take?", int(hours))


def digit_sum(r):
    n = r.randint(1000, 999999)
    return f"What is the sum of the digits of {n}?", sum(int(d) for d in str(n))


def reverse_number(r):
    n = r.randint(1000, 999999)
    return f"Write the digits of {n} in reverse order.", str(n)[::-1].lstrip("0") or "0"


def bitwise(r):
    a, b = r.randint(1, 255), r.randint(1, 255)
    op = r.choice(["AND", "OR", "XOR"])
    held = {"AND": a & b, "OR": a | b, "XOR": a ^ b}[op]
    return f"What is {a} {op} {b} as a decimal number? (bitwise {op.lower()})", held


def geometry(r):
    a, b = r.choice([(3, 4), (5, 12), (8, 15), (7, 24), (20, 21), (9, 40)])
    if r.random() < 0.5:
        return f"A right triangle has legs of {a} and {b}. What is the length of its hypotenuse?", int(math.hypot(a, b))
    w, h = r.randint(3, 40), r.randint(3, 40)
    return f"What is the area of a rectangle {w} by {h}?", w * h


def set_ops(r):
    one = sorted(r.sample(range(1, 40), r.randint(6, 12)))
    two = sorted(r.sample(range(1, 40), r.randint(6, 12)))
    return (f"Set A is {{{', '.join(map(str, one))}}} and set B is {{{', '.join(map(str, two))}}}. "
            f"How many numbers are in both A and B?", len(set(one) & set(two)))


def string_ops(r):
    word = r.choice(WORDS)
    which = r.random()
    if which < 0.34:
        return f"How many letters are in the word \"{word}\"?", len(word)
    if which < 0.67:
        return f"Write the word \"{word}\" backwards.", word[::-1]
    at = r.randint(1, len(word))
    return f"What is letter number {at} of the word \"{word}\"? (The first letter is number 1.)", word[at - 1]


def list_ops(r):
    held = r.sample(range(1, 100), r.randint(5, 9))
    which = r.random()
    if which < 0.34:
        return f"What is the largest number in this list: {held}?", max(held)
    if which < 0.67:
        return f"What is the sum of this list: {held}?", sum(held)
    return f"Sorted from smallest to largest, what is the third number in this list: {held}?", sorted(held)[2]


def code_reading(r):
    a, b = r.randint(2, 20), r.randint(2, 20)
    shape = r.random()
    if shape < 0.34:
        body = f"def f(x):\n    total = 0\n    for i in range(x):\n        total += i * {a}\n    return total"
        answer = sum(i * a for i in range(b))
        return f"What does f({b}) return?\n\n{body}", answer
    if shape < 0.67:
        body = f"def f(n):\n    while n > {a}:\n        n -= {b}\n    return n"
        n = a * r.randint(2, 6) + r.randint(0, b)
        held = n
        while held > a:
            held -= b
        return f"What does f({n}) return?\n\n{body}", held
    held = r.sample(range(1, 50), 6)
    body = f"def f(xs):\n    return sum(x for x in xs if x % 2 == 0)"
    return f"What does f({held}) return?\n\n{body}", sum(x for x in held if x % 2 == 0)


def ordering(r):
    one, two, three = r.sample(NAMES, 3)
    trait, most, least = r.choice([
        ("taller", "tallest", "shortest"), ("older", "oldest", "youngest"),
        ("faster", "fastest", "slowest"), ("heavier", "heaviest", "lightest"),
    ])
    asking = r.choice([most, least])
    answer = one if asking == most else three
    return (f"{one} is {trait} than {two}. {two} is {trait} than {three}. "
            f"Who is the {asking}?", answer)


def counting_days(r):
    days = r.randint(10, 400)
    weeks = days // 7
    if r.random() < 0.5:
        return f"How many whole weeks are there in {days} days?", weeks
    return f"How many days are left over after taking whole weeks out of {days} days?", days % 7


def unit_convert(r):
    which = r.random()
    if which < 0.34:
        hours = r.randint(2, 480)
        return f"How many minutes are there in {hours} hours?", hours * 60
    if which < 0.67:
        km = r.randint(2, 90)
        return f"How many metres are there in {km} kilometres?", km * 1000
    mib = r.randint(2, 512)
    return f"How many kibibytes are there in {mib} mebibytes?", mib * 1024


MAKERS = [
    ("arith", arithmetic), ("prec", precedence), ("pct", percentage), ("gcd", gcd_lcm),
    ("prime", primes), ("mod", modular), ("powmod", power_mod), ("base", base_convert),
    ("frombase", from_base), ("seqterm", sequence_term), ("seqsum", sequence_sum),
    ("solve", linear_equation), ("comb", combinatorics), ("rate", rate_problem),
    ("work", work_problem), ("digits", digit_sum), ("revnum", reverse_number),
    ("bits", bitwise), ("geom", geometry), ("sets", set_ops), ("text", string_ops),
    ("list", list_ops), ("code", code_reading), ("order", ordering),
    ("weeks", counting_days), ("units", unit_convert),
]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="crates/mcf-optimize/tasks/short")
    ap.add_argument("--per-set", type=int, default=25)
    ap.add_argument("--sets", type=int, default=40)
    ap.add_argument("--seed", type=int, default=20260919)
    args = ap.parse_args()

    r = random.Random(args.seed)
    wanted = args.per_set * args.sets
    tasks, seen = [], set()
    guard = 0
    while len(tasks) < wanted and guard < wanted * 200:
        guard += 1
        # Cycled on attempts rather than on acceptances: keyed on the number accepted, a
        # maker whose questions had all been asked before would be asked again forever
        # while every other maker waited its turn.
        name, maker = MAKERS[guard % len(MAKERS)]
        made = maker(r)
        if made is None:
            continue
        asked, answer = made
        answer = str(answer)
        if asked in seen:
            continue
        seen.add(asked)
        tasks.append({"n": f"{name}-{len(tasks) + 1:04d}", "p": asked, "a": answer})

    os.makedirs(args.out, exist_ok=True)
    for at in range(args.sets):
        held = tasks[at * args.per_set:(at + 1) * args.per_set]
        path = os.path.join(args.out, f"set{at + 1}.json")
        with open(path, "w") as file:
            json.dump(held, file, indent=1)
            file.write("\n")
    print(f"wrote {len(tasks)} tasks into {args.sets} set(s) under {args.out}")


if __name__ == "__main__":
    main()
