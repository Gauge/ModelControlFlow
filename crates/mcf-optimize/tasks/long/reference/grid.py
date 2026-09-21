import heapq


def _checked(grid):
    if not grid or any(len(row) != len(grid[0]) for row in grid):
        raise ValueError('rows')
    starts = [(r, c) for r, row in enumerate(grid) for c, cell in enumerate(row) if cell == 'S']
    ends = [(r, c) for r, row in enumerate(grid) for c, cell in enumerate(row) if cell == 'E']
    if len(starts) != 1 or len(ends) != 1:
        raise ValueError('S and E')
    return starts[0], ends[0]


def _search(grid):
    start, end = _checked(grid)
    best = {start: 0}
    came = {}
    queue = [(0, start)]
    while queue:
        cost, (r, c) = heapq.heappop(queue)
        if (r, c) == end:
            path = [end]
            while path[-1] != start:
                path.append(came[path[-1]])
            return cost, path[::-1]
        if cost > best[(r, c)]:
            continue
        for dr, dc in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nr, nc = r + dr, c + dc
            if not (0 <= nr < len(grid) and 0 <= nc < len(grid[0])):
                continue
            cell = grid[nr][nc]
            if cell == '#' or cell == 'S':
                continue
            step = int(cell) if cell.isdigit() else 1
            if cost + step < best.get((nr, nc), float('inf')):
                best[(nr, nc)] = cost + step
                came[(nr, nc)] = (r, c)
                heapq.heappush(queue, (cost + step, (nr, nc)))
    return -1, []


def cheapest(grid):
    return _search(grid)[0]


def route(grid):
    return _search(grid)[1]


def draw(grid):
    rows = [list(row) for row in grid]
    for r, c in _search(grid)[1]:
        if rows[r][c] not in ('S', 'E'):
            rows[r][c] = '*'
    return [''.join(row) for row in rows]
