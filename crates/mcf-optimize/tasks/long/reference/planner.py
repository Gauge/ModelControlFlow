import heapq


class Project:
    def __init__(self):
        self.tasks = {}

    def add(self, name, duration, after=()):
        if name in self.tasks:
            raise ValueError(name)
        if not isinstance(duration, int) or isinstance(duration, bool) or duration <= 0:
            raise ValueError(duration)
        self.tasks[name] = (duration, list(after))

    def order(self):
        for name, (_, after) in self.tasks.items():
            for need in after:
                if need not in self.tasks:
                    raise ValueError(need)
        waiting = {name: len(set(after)) for name, (_, after) in self.tasks.items()}
        users = {name: [] for name in self.tasks}
        for name, (_, after) in self.tasks.items():
            for need in set(after):
                users[need].append(name)
        ready = [name for name, count in waiting.items() if count == 0]
        heapq.heapify(ready)
        out = []
        while ready:
            name = heapq.heappop(ready)
            out.append(name)
            for user in users[name]:
                waiting[user] -= 1
                if waiting[user] == 0:
                    heapq.heappush(ready, user)
        if len(out) != len(self.tasks):
            raise ValueError('cycle')
        return out

    def _times(self):
        order = self.order()
        start = {}
        for name in order:
            _, after = self.tasks[name]
            start[name] = max((start[need] + self.tasks[need][0] for need in after), default=0)
        end = max((start[name] + self.tasks[name][0] for name in order), default=0)
        latest = {}
        for name in reversed(order):
            users = [other for other in order if name in self.tasks[other][1]]
            finish_by = min((latest[user] for user in users), default=end)
            latest[name] = finish_by - self.tasks[name][0]
        return order, start, latest, end

    def _known(self, name):
        if name not in self.tasks:
            raise KeyError(name)

    def start(self, name):
        _, start, _, _ = self._times()
        self._known(name)
        return start[name]

    def finish(self, name):
        return self.start(name) + self.tasks[name][0]

    def length(self):
        return self._times()[3]

    def slack(self, name):
        _, start, latest, _ = self._times()
        self._known(name)
        return latest[name] - start[name]

    def critical(self):
        order, start, latest, _ = self._times()
        return [name for name in order if latest[name] == start[name]]
