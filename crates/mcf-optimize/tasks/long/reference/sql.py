import re

_KEYWORDS = {'SELECT', 'FROM', 'WHERE', 'ORDER', 'BY', 'ASC', 'DESC', 'LIMIT', 'AND', 'OR', 'NOT', 'COUNT'}
_TOKEN = re.compile(r"\s*(?:(\d+\.\d+|\d+)|('(?:[^']|'')*')|([A-Za-z_]\w*)|(<=|>=|!=|[=<>(),*]))")


def _lex(sql):
    out, at = [], 0
    sql = sql.strip()
    while at < len(sql):
        match = _TOKEN.match(sql, at)
        if not match or match.end() == at:
            raise SyntaxError(sql[at:])
        at = match.end()
        number, string, word, op = match.groups()
        if number is not None:
            out.append(('lit', float(number) if '.' in number else int(number)))
        elif string is not None:
            out.append(('lit', string[1:-1].replace("''", "'")))
        elif word is not None:
            out.append(('kw', word.upper()) if word.upper() in _KEYWORDS else ('name', word))
        else:
            out.append(('op', op))
    return out


class _Parser:
    def __init__(self, tokens):
        self.tokens, self.at = tokens, 0

    def peek(self, kind, value=None):
        if self.at >= len(self.tokens):
            return False
        held = self.tokens[self.at]
        return held[0] == kind and (value is None or held[1] == value)

    def take(self, kind, value=None):
        if not self.peek(kind, value):
            raise SyntaxError(f'expected {value or kind}')
        self.at += 1
        return self.tokens[self.at - 1][1]

    def condition(self):
        node = self.conjunction()
        while self.peek('kw', 'OR'):
            self.take('kw')
            node = ('or', node, self.conjunction())
        return node

    def conjunction(self):
        node = self.negation()
        while self.peek('kw', 'AND'):
            self.take('kw')
            node = ('and', node, self.negation())
        return node

    def negation(self):
        if self.peek('kw', 'NOT'):
            self.take('kw')
            return ('not', self.negation())
        if self.peek('op', '('):
            self.take('op')
            node = self.condition()
            self.take('op', ')')
            return node
        left = self.operand()
        for op in ('<=', '>=', '!=', '=', '<', '>'):
            if self.peek('op', op):
                self.take('op')
                return ('cmp', op, left, self.operand())
        raise SyntaxError('comparison')

    def operand(self):
        if self.peek('lit'):
            return ('lit', self.take('lit'))
        return ('col', self.take('name'))


def _compare(op, left, right):
    number = lambda v: isinstance(v, (int, float)) and not isinstance(v, bool)
    if number(left) != number(right):
        return op == '!='
    return {'=': left == right, '!=': left != right, '<': left < right, '<=': left <= right, '>': left > right, '>=': left >= right}[op]


def _truth(node, row):
    kind = node[0]
    if kind == 'or':
        return _truth(node[1], row) or _truth(node[2], row)
    if kind == 'and':
        return _truth(node[1], row) and _truth(node[2], row)
    if kind == 'not':
        return not _truth(node[1], row)
    value = lambda operand: row[operand[1]] if operand[0] == 'col' else operand[1]
    return _compare(node[1], value(node[2]), value(node[3]))


def _columns_in(node):
    if node[0] in ('or', 'and'):
        return _columns_in(node[1]) + _columns_in(node[2])
    if node[0] == 'not':
        return _columns_in(node[1])
    return [operand[1] for operand in node[2:] if operand[0] == 'col']


def query(tables, sql):
    parser = _Parser(_lex(sql))
    parser.take('kw', 'SELECT')
    counting = False
    columns = None
    if parser.peek('op', '*'):
        parser.take('op')
    elif parser.peek('kw', 'COUNT'):
        parser.take('kw')
        parser.take('op', '(')
        parser.take('op', '*')
        parser.take('op', ')')
        counting = True
    else:
        columns = [parser.take('name')]
        while parser.peek('op', ','):
            parser.take('op')
            columns.append(parser.take('name'))
    parser.take('kw', 'FROM')
    table = parser.take('name')
    where = None
    if parser.peek('kw', 'WHERE'):
        parser.take('kw')
        where = parser.condition()
    order = []
    if parser.peek('kw', 'ORDER'):
        parser.take('kw')
        parser.take('kw', 'BY')
        while True:
            name = parser.take('name')
            descending = False
            if parser.peek('kw', 'ASC'):
                parser.take('kw')
            elif parser.peek('kw', 'DESC'):
                parser.take('kw')
                descending = True
            order.append((name, descending))
            if not parser.peek('op', ','):
                break
            parser.take('op')
    limit = None
    if parser.peek('kw', 'LIMIT'):
        parser.take('kw')
        limit = parser.take('lit')
        if not isinstance(limit, int):
            raise SyntaxError('limit')
    if parser.at != len(parser.tokens):
        raise SyntaxError('trailing')
    if table not in tables:
        raise KeyError(table)
    rows = tables[table]
    known = list(rows[0].keys()) if rows else []
    wanted = (columns or []) + ([] if where is None else _columns_in(where)) + [name for name, _ in order]
    for name in wanted:
        if name not in known:
            raise KeyError(name)
    kept = [row for row in rows if where is None or _truth(where, row)]
    if counting:
        return [(len(kept),)]
    for name, descending in reversed(order):
        kept = sorted(kept, key=lambda row: row[name], reverse=descending)
    if limit is not None:
        kept = kept[:limit]
    names = columns if columns is not None else known
    return [tuple(row[name] for name in names) for row in kept]
