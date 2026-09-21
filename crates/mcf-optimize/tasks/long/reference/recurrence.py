import datetime

_DAYS = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU']


def _positive(text):
    if not text.isdigit() or int(text) <= 0:
        raise ValueError(text)
    return int(text)


def _month_date(year, month, day):
    try:
        return datetime.date(year, month, day)
    except ValueError:
        return None


def expand(start, rule, until):
    first = datetime.date.fromisoformat(start)
    last = datetime.date.fromisoformat(until)
    parts = {}
    for part in rule.split(';'):
        if '=' not in part:
            raise ValueError(part)
        key, value = part.split('=', 1)
        if key not in ('FREQ', 'INTERVAL', 'COUNT', 'BYDAY', 'BYMONTHDAY') or key in parts:
            raise ValueError(key)
        parts[key] = value
    freq = parts.get('FREQ')
    if freq not in ('DAILY', 'WEEKLY', 'MONTHLY', 'YEARLY'):
        raise ValueError(freq)
    interval = _positive(parts['INTERVAL']) if 'INTERVAL' in parts else 1
    count = _positive(parts['COUNT']) if 'COUNT' in parts else None
    if 'BYDAY' in parts and freq != 'WEEKLY':
        raise ValueError('BYDAY')
    if 'BYMONTHDAY' in parts and freq != 'MONTHLY':
        raise ValueError('BYMONTHDAY')
    days = None
    if 'BYDAY' in parts:
        names = parts['BYDAY'].split(',')
        if not names or any(name not in _DAYS for name in names):
            raise ValueError(parts['BYDAY'])
        days = sorted({_DAYS.index(name) for name in names})
    monthday = first.day
    if 'BYMONTHDAY' in parts:
        monthday = _positive(parts['BYMONTHDAY'])
        if monthday > 31:
            raise ValueError(monthday)

    def candidates():
        step = 0
        while True:
            if freq == 'DAILY':
                yield [first + datetime.timedelta(days=step * interval)]
            elif freq == 'WEEKLY':
                monday = first - datetime.timedelta(days=first.weekday()) + datetime.timedelta(weeks=step * interval)
                wanted = days if days is not None else [first.weekday()]
                yield [monday + datetime.timedelta(days=day) for day in wanted]
            elif freq == 'MONTHLY':
                index = first.month - 1 + step * interval
                held = _month_date(first.year + index // 12, index % 12 + 1, monthday)
                yield [held] if held else []
            else:
                held = _month_date(first.year + step * interval, first.month, first.day)
                yield [held] if held else []
            step += 1

    out = []
    for group in candidates():
        if any(day > last for day in group) and all(day > last for day in group):
            break
        for day in group:
            if first <= day <= last:
                out.append(day.isoformat())
                if count is not None and len(out) == count:
                    return out
        if group and min(group) > last:
            break
        if not group and freq in ('MONTHLY', 'YEARLY'):
            continue
    return out
