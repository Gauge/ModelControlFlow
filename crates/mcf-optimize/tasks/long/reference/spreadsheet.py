import re


class _Error(Exception):
    def __init__(self, code):
        self.code = code


def _is_number(text):
    return re.fullmatch(r"-?(\d+(\.\d*)?|\.\d+)", text) is not None


def _name(col, row):
    return f"{chr(ord('A') + col)}{row}"


class Sheet:
    def __init__(self):
        self.cells = {}

    def set(self, cell, text):
        self.cells[cell] = text

    def get(self, cell):
        return self._value(cell, set())

    def _value(self, cell, visiting):
        text = self.cells.get(cell, "")
        if text == "":
            return ""
        if not text.startswith("="):
            if _is_number(text):
                return float(text) if "." in text else int(text)
            return text
        if cell in visiting:
            return "#CYCLE!"
        visiting = visiting | {cell}
        try:
            tokens = _tokens(text[1:])
            parser = _Parser(tokens)
            tree = parser.expression()
            if parser.at < len(tokens):
                raise _Error("#ERROR!")
        except _Error as error:
            return error.code
        try:
            return self._eval(tree, visiting)
        except _Error as error:
            return error.code

    def _cell_value(self, name, visiting):
        match = re.fullmatch(r"([A-Z])(\d+)", name)
        if not match or not 1 <= int(match.group(2)) <= 99:
            raise _Error("#REF!")
        value = self._value(name, visiting)
        if isinstance(value, str) and value.startswith("#") and value.endswith(("!", "?")):
            raise _Error(value)
        return value

    def _number(self, value):
        if value == "":
            return 0
        if isinstance(value, str):
            raise _Error("#VALUE!")
        return value

    def _eval(self, node, visiting):
        kind = node[0]
        if kind == "num":
            return node[1]
        if kind == "ref":
            return self._number(self._cell_value(node[1], visiting))
        if kind == "neg":
            return -self._eval(node[1], visiting)
        if kind == "bin":
            left = self._eval(node[2], visiting)
            right = self._eval(node[3], visiting)
            op = node[1]
            if op == "+":
                return left + right
            if op == "-":
                return left - right
            if op == "*":
                return left * right
            if right == 0:
                raise _Error("#DIV/0!")
            return left / right
        if kind == "call":
            name, args = node[1], node[2]
            if name not in ("SUM", "MIN", "MAX", "AVERAGE"):
                raise _Error("#NAME?")
            numbers = []
            for arg in args:
                if arg[0] == "range":
                    for name_in in self._range(arg[1], arg[2]):
                        value = self._cell_value(name_in, visiting)
                        if value == "":
                            continue
                        if isinstance(value, str):
                            raise _Error("#VALUE!")
                        numbers.append(value)
                elif arg[0] == "ref":
                    value = self._cell_value(arg[1], visiting)
                    if value == "":
                        continue
                    if isinstance(value, str):
                        raise _Error("#VALUE!")
                    numbers.append(value)
                else:
                    numbers.append(self._eval(arg, visiting))
            if name == "SUM":
                return sum(numbers)
            if name == "MIN":
                return min(numbers) if numbers else 0
            if name == "MAX":
                return max(numbers) if numbers else 0
            if not numbers:
                raise _Error("#DIV/0!")
            return sum(numbers) / len(numbers)
        raise _Error("#ERROR!")

    def _range(self, first, last):
        a = re.fullmatch(r"([A-Z])(\d+)", first)
        b = re.fullmatch(r"([A-Z])(\d+)", last)
        if not a or not b:
            raise _Error("#REF!")
        rows = sorted((int(a.group(2)), int(b.group(2))))
        cols = sorted((ord(a.group(1)) - 65, ord(b.group(1)) - 65))
        if rows[0] < 1 or rows[1] > 99:
            raise _Error("#REF!")
        return [_name(col, row) for row in range(rows[0], rows[1] + 1) for col in range(cols[0], cols[1] + 1)]


def _tokens(text):
    text = text.replace(" ", "")
    out = []
    at = 0
    while at < len(text):
        match = re.match(r"\d+(\.\d*)?|\.\d+|[A-Za-z]+\d*|[-+*/(),:]", text[at:])
        if not match:
            raise _Error("#ERROR!")
        out.append(match.group(0))
        at += len(match.group(0))
    return out


class _Parser:
    def __init__(self, tokens):
        self.tokens = tokens
        self.at = 0

    def peek(self):
        return self.tokens[self.at] if self.at < len(self.tokens) else None

    def take(self):
        token = self.peek()
        if token is None:
            raise _Error("#ERROR!")
        self.at += 1
        return token

    def expression(self):
        node = self.term()
        while self.peek() in ("+", "-"):
            node = ("bin", self.take(), node, self.term())
        return node

    def term(self):
        node = self.unary()
        while self.peek() in ("*", "/"):
            node = ("bin", self.take(), node, self.unary())
        return node

    def unary(self):
        if self.peek() == "-":
            self.take()
            return ("neg", self.unary())
        return self.atom()

    def atom(self):
        token = self.take()
        if token == "(":
            node = self.expression()
            if self.take() != ")":
                raise _Error("#ERROR!")
            return node
        if re.fullmatch(r"\d+(\.\d*)?|\.\d+", token):
            return ("num", float(token) if "." in token else int(token))
        if re.fullmatch(r"[A-Za-z]+", token) and self.peek() == "(":
            self.take()
            args = []
            if self.peek() != ")":
                while True:
                    args.append(self.argument())
                    if self.peek() == ",":
                        self.take()
                        continue
                    break
            if self.take() != ")":
                raise _Error("#ERROR!")
            return ("call", token, args)
        if re.fullmatch(r"[A-Za-z]+\d+", token):
            return ("ref", token)
        raise _Error("#ERROR!")

    def argument(self):
        token = self.peek()
        following = self.tokens[self.at + 1] if self.at + 1 < len(self.tokens) else None
        if token and re.fullmatch(r"[A-Za-z]+\d+", token) and following == ":":
            self.take()
            self.take()
            last = self.take()
            return ("range", token, last)
        return self.expression()
