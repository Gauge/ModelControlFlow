import re

_RESERVED = {'let', 'print', 'if', 'else', 'while', 'fn', 'return', 'true', 'false', 'and', 'or', 'not'}
_TOKEN = re.compile(r'\s*(?:(\d+)|("[^"]*")|([A-Za-z_]\w*)|(==|!=|<=|>=|[-+*/%<>=(),{}]))')


def _lex(text):
    out, at = [], 0
    text = text.rstrip()
    while at < len(text):
        match = _TOKEN.match(text, at)
        if not match or match.end() == at:
            raise SyntaxError(text)
        at = match.end()
        number, string, name, op = match.groups()
        if number is not None:
            out.append(('num', int(number)))
        elif string is not None:
            out.append(('str', string[1:-1]))
        elif name is not None:
            out.append(('kw', name) if name in _RESERVED else ('name', name))
        else:
            out.append(('op', op))
    return out


class _Expr:
    def __init__(self, tokens):
        self.tokens, self.at = tokens, 0

    def peek(self, kind=None, value=None):
        if self.at >= len(self.tokens):
            return False
        held = self.tokens[self.at]
        return (kind is None or held[0] == kind) and (value is None or held[1] == value)

    def take(self):
        if self.at >= len(self.tokens):
            raise SyntaxError('end of line')
        self.at += 1
        return self.tokens[self.at - 1]

    def expect(self, kind, value=None):
        if not self.peek(kind, value):
            raise SyntaxError(f'expected {value or kind}')
        return self.take()

    def done(self):
        if self.at != len(self.tokens):
            raise SyntaxError('trailing')

    def parse(self):
        return self.orr()

    def orr(self):
        node = self.andd()
        while self.peek('kw', 'or'):
            self.take()
            node = ('or', node, self.andd())
        return node

    def andd(self):
        node = self.nott()
        while self.peek('kw', 'and'):
            self.take()
            node = ('and', node, self.nott())
        return node

    def nott(self):
        if self.peek('kw', 'not'):
            self.take()
            return ('not', self.nott())
        return self.compare()

    def compare(self):
        node = self.sum()
        ops = ('==', '!=', '<', '<=', '>', '>=')
        if any(self.peek('op', op) for op in ops):
            op = self.take()[1]
            node = ('bin', op, node, self.sum())
            if any(self.peek('op', op) for op in ops):
                raise SyntaxError('chained comparison')
        return node

    def sum(self):
        node = self.product()
        while self.peek('op', '+') or self.peek('op', '-'):
            node = ('bin', self.take()[1], node, self.product())
        return node

    def product(self):
        node = self.unary()
        while self.peek('op', '*') or self.peek('op', '/') or self.peek('op', '%'):
            node = ('bin', self.take()[1], node, self.unary())
        return node

    def unary(self):
        if self.peek('op', '-'):
            self.take()
            return ('neg', self.unary())
        return self.call()

    def call(self):
        node = self.atom()
        while self.peek('op', '('):
            self.take()
            args = []
            if not self.peek('op', ')'):
                args.append(self.parse())
                while self.peek('op', ','):
                    self.take()
                    args.append(self.parse())
            self.expect('op', ')')
            node = ('call', node, args)
        return node

    def atom(self):
        kind, value = self.take()
        if kind == 'num':
            return ('lit', value)
        if kind == 'str':
            return ('lit', value)
        if kind == 'kw' and value in ('true', 'false'):
            return ('lit', value == 'true')
        if kind == 'name':
            return ('var', value)
        if kind == 'op' and value == '(':
            node = self.parse()
            self.expect('op', ')')
            return node
        raise SyntaxError(f'unexpected {value}')


def _expression(tokens):
    parser = _Expr(tokens)
    node = parser.parse()
    parser.done()
    return node


def _parse(lines, at, top):
    body = []
    while at < len(lines):
        tokens = lines[at]
        if tokens == [('op', '}')] or tokens[:2] == [('op', '}'), ('kw', 'else')]:
            if top:
                raise SyntaxError('unmatched brace')
            return body, at
        at += 1
        head = tokens[0]
        if head == ('kw', 'let'):
            if len(tokens) < 4 or tokens[1][0] != 'name' or tokens[2] != ('op', '='):
                raise SyntaxError('let')
            body.append(('let', tokens[1][1], _expression(tokens[3:])))
        elif head == ('kw', 'print'):
            body.append(('print', _expression(tokens[1:])))
        elif head == ('kw', 'return'):
            body.append(('return', _expression(tokens[1:])))
        elif head in (('kw', 'if'), ('kw', 'while')):
            if tokens[-1] != ('op', '{'):
                raise SyntaxError('brace')
            test = _expression(tokens[1:-1])
            block, at = _parse(lines, at, False)
            if at >= len(lines):
                raise SyntaxError('unclosed')
            other = None
            if head[1] == 'if' and lines[at] == [('op', '}'), ('kw', 'else'), ('op', '{')]:
                other, at = _parse(lines, at + 1, False)
                if at >= len(lines) or lines[at] != [('op', '}')]:
                    raise SyntaxError('unclosed')
            elif lines[at] != [('op', '}')]:
                raise SyntaxError('brace')
            at += 1
            body.append((head[1], test, block, other))
        elif head == ('kw', 'fn'):
            if len(tokens) < 5 or tokens[1][0] != 'name' or tokens[2] != ('op', '(') or tokens[-1] != ('op', '{') or tokens[-2] != ('op', ')'):
                raise SyntaxError('fn')
            params = []
            inner = tokens[3:-2]
            for index, token in enumerate(inner):
                if index % 2 == 0:
                    if token[0] != 'name':
                        raise SyntaxError('param')
                    params.append(token[1])
                elif token != ('op', ','):
                    raise SyntaxError('param')
            if inner and len(inner) % 2 == 0:
                raise SyntaxError('param')
            block, at = _parse(lines, at, False)
            if at >= len(lines) or lines[at] != [('op', '}')]:
                raise SyntaxError('unclosed')
            at += 1
            body.append(('fn', tokens[1][1], params, block))
        elif head[0] == 'name' and len(tokens) >= 3 and tokens[1] == ('op', '='):
            body.append(('set', head[1], _expression(tokens[2:])))
        else:
            body.append(('expr', _expression(tokens)))
    if not top:
        raise SyntaxError('unclosed block')
    return body, at


class _Return(Exception):
    def __init__(self, value):
        self.value = value


class _Scope:
    def __init__(self, parent=None):
        self.names, self.parent = {}, parent

    def find(self, name):
        scope = self
        while scope is not None:
            if name in scope.names:
                return scope
            scope = scope.parent
        raise RuntimeError(f'undefined {name}')


class _Function:
    def __init__(self, params, body, scope):
        self.params, self.body, self.scope = params, body, scope


def _kind(value):
    if isinstance(value, bool):
        return 'bool'
    if isinstance(value, int):
        return 'int'
    if isinstance(value, str):
        return 'str'
    return 'fn'


def _show(value):
    if isinstance(value, bool):
        return 'true' if value else 'false'
    if isinstance(value, _Function):
        return '<fn>'
    return str(value)


class _Machine:
    def __init__(self):
        self.printed = []

    def block(self, body, scope, in_function):
        for statement in body:
            self.statement(statement, scope, in_function)

    def statement(self, statement, scope, in_function):
        kind = statement[0]
        if kind == 'let':
            scope.names[statement[1]] = self.value(statement[2], scope)
        elif kind == 'set':
            held = scope.find(statement[1])
            held.names[statement[1]] = self.value(statement[2], scope)
        elif kind == 'print':
            self.printed.append(_show(self.value(statement[1], scope)))
        elif kind == 'return':
            if not in_function:
                raise RuntimeError('return outside a function')
            raise _Return(self.value(statement[1], scope))
        elif kind == 'if':
            if self.boolean(self.value(statement[1], scope)):
                self.block(statement[2], _Scope(scope), in_function)
            elif statement[3] is not None:
                self.block(statement[3], _Scope(scope), in_function)
        elif kind == 'while':
            while self.boolean(self.value(statement[1], scope)):
                self.block(statement[2], _Scope(scope), in_function)
        elif kind == 'fn':
            scope.names[statement[1]] = _Function(statement[2], statement[3], scope)
        else:
            self.value(statement[1], scope)

    @staticmethod
    def boolean(value):
        if not isinstance(value, bool):
            raise RuntimeError('not a boolean')
        return value

    def value(self, node, scope):
        kind = node[0]
        if kind == 'lit':
            return node[1]
        if kind == 'var':
            return scope.find(node[1]).names[node[1]]
        if kind == 'neg':
            held = self.value(node[1], scope)
            if _kind(held) != 'int':
                raise RuntimeError('negate')
            return -held
        if kind == 'not':
            return not self.boolean(self.value(node[1], scope))
        if kind == 'and':
            return self.boolean(self.value(node[1], scope)) and self.boolean(self.value(node[2], scope))
        if kind == 'or':
            return self.boolean(self.value(node[1], scope)) or self.boolean(self.value(node[2], scope))
        if kind == 'call':
            function = self.value(node[1], scope)
            args = [self.value(arg, scope) for arg in node[2]]
            if not isinstance(function, _Function) or len(args) != len(function.params):
                raise RuntimeError('bad call')
            inner = _Scope(function.scope)
            inner.names.update(zip(function.params, args))
            try:
                self.block(function.body, inner, True)
            except _Return as returned:
                return returned.value
            except RecursionError:
                raise RuntimeError('too deep')
            return 0
        op, left, right = node[1], self.value(node[2], scope), self.value(node[3], scope)
        a, b = _kind(left), _kind(right)
        if op == '==':
            return a == b and left == right
        if op == '!=':
            return not (a == b and left == right)
        if op in ('<', '<=', '>', '>='):
            if a != b or a not in ('int', 'str'):
                raise RuntimeError('compare')
            return {'<': left < right, '<=': left <= right, '>': left > right, '>=': left >= right}[op]
        if op == '+' and a == b == 'str':
            return left + right
        if a != 'int' or b != 'int':
            raise RuntimeError('arithmetic')
        if op == '+':
            return left + right
        if op == '-':
            return left - right
        if op == '*':
            return left * right
        if right == 0:
            raise RuntimeError('division by zero')
        return left // right if op == '/' else left % right


def run(source):
    lines = []
    for line in source.split('\n'):
        stripped = line.strip()
        if not stripped or stripped.startswith('#'):
            continue
        lines.append(_lex(stripped))
    body, _ = _parse(lines, 0, True)
    machine = _Machine()
    machine.block(body, _Scope(), False)
    return machine.printed
