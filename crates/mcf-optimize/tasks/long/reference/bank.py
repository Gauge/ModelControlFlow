class InsufficientFunds(Exception):
    pass


class Bank:
    def __init__(self, rates):
        self.rates = dict(rates)
        self.accounts = {}

    def open(self, account, currency):
        if account in self.accounts or currency not in self.rates:
            raise ValueError(account)
        self.accounts[account] = {'currency': currency, 'balance': 0, 'history': []}

    def _get(self, account):
        if account not in self.accounts:
            raise KeyError(account)
        return self.accounts[account]

    @staticmethod
    def _amount(amount):
        if not isinstance(amount, int) or isinstance(amount, bool) or amount <= 0:
            raise ValueError(amount)

    def deposit(self, account, amount):
        held = self._get(account)
        self._amount(amount)
        held['balance'] += amount
        held['history'].append(('deposit', amount))

    def withdraw(self, account, amount):
        held = self._get(account)
        self._amount(amount)
        if amount > held['balance']:
            raise InsufficientFunds(account)
        held['balance'] -= amount
        held['history'].append(('withdraw', amount))

    def transfer(self, source, target, amount):
        out = self._get(source)
        into = self._get(target)
        if source == target:
            raise ValueError(source)
        self._amount(amount)
        if amount > out['balance']:
            raise InsufficientFunds(source)
        moved = amount * self.rates[out['currency']] // self.rates[into['currency']]
        out['balance'] -= amount
        into['balance'] += moved
        out['history'].append(('transfer out', amount))
        into['history'].append(('transfer in', moved))

    def balance(self, account):
        return self._get(account)['balance']

    def history(self, account):
        return list(self._get(account)['history'])

    def total(self, currency):
        return sum(held['balance'] * self.rates[held['currency']] // self.rates[currency] for held in self.accounts.values())

    def statement(self, account):
        held = self._get(account)
        show = lambda cents: f"{cents // 100}.{cents % 100:02d}"
        lines = [f"Account {account} ({held['currency']})"]
        lines += [f"{kind}: {show(amount)}" for kind, amount in held['history']]
        lines.append(f"Balance: {show(held['balance'])}")
        return '\n'.join(lines)
