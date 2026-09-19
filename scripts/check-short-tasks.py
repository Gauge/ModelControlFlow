#!/usr/bin/env python3
"""Re-derive the short corpus's answers from the questions, a second way.

The generator solves each problem as it poses it, which makes the key right by
construction — and a key that is wrong by construction would be wrong silently. This
reads the question text back and works each answer out again with different code, so a
mistake would have to be made twice in two different ways to survive.

It re-derives most of the corpus. What it cannot read back it counts and says so, rather
than passing over it quietly.

Usage: python3 scripts/check-short-tasks.py
"""
import json, glob, re, math, sys
tasks=[]
for p in sorted(glob.glob('crates/mcf-optimize/tasks/short/*.json')):
    tasks += json.load(open(p))
checked=0; skipped=0; bad=[]
def ok(t, got):
    global checked
    checked+=1
    if str(got) != t['a']: bad.append((t['n'], t['p'][:70].replace('\n',' / '), t['a'], got))
def grab(pat, text, *groups):
    m = re.search(pat, text)
    if not m: raise LookupError
    return [m.group(g) for g in groups] if groups else m
for t in tasks:
    p,a = t['p'],t['a']; k=t['n'].split('-')[0]
    try:
        if k in ('arith','prec'):
            ok(t, eval(grab(r'Compute (.+)\.$', p).group(1)))
        elif k=='pct':
            x,y = grab(r'What is (\d+)% of (\d+)\?', p, 1, 2); ok(t, int(y)*int(x)//100)
        elif k=='gcd':
            w,x,y = grab(r'(greatest common divisor|least common multiple) of (\d+) and (\d+)', p, 1,2,3)
            x,y=int(x),int(y); ok(t, math.gcd(x,y) if 'greatest' in w else x*y//math.gcd(x,y))
        elif k=='mod':
            x,y = grab(r'when (\d+) is divided by (\d+)', p, 1,2); ok(t, int(x)%int(y))
        elif k=='powmod':
            b,e,m_ = grab(r'What is (\d+)\^(\d+) mod (\d+)', p, 1,2,3); ok(t, pow(int(b),int(e),int(m_)))
        elif k=='base':
            v,w = grab(r'Write (\d+) in (\w+)', p, 1,2); v=int(v)
            ok(t, {'binary':bin(v)[2:], 'hexadecimal':hex(v)[2:].upper(), 'octal':oct(v)[2:]}[w])
        elif k=='frombase':
            w,v = grab(r'The (\w+) number (\w+) is what in decimal', p, 1,2)
            ok(t, int(v, 2 if w=='binary' else 16))
        elif k=='digits':
            ok(t, sum(int(d) for d in grab(r'digits of (\d+)', p, 1)[0]))
        elif k=='revnum':
            v = grab(r'digits of (\d+) in reverse', p, 1)[0]; ok(t, v[::-1].lstrip('0') or '0')
        elif k=='bits':
            x,op,y = grab(r'What is (\d+) (AND|OR|XOR) (\d+)', p, 1,2,3); x,y=int(x),int(y)
            ok(t, {'AND':x&y,'OR':x|y,'XOR':x^y}[op])
        elif k=='sets':
            one=set(map(int, grab(r'Set A is \{([^}]*)\}', p, 1)[0].split(', ')))
            two=set(map(int, grab(r'set B is \{([^}]*)\}', p, 1)[0].split(', ')))
            ok(t, len(one&two))
        elif k=='list':
            held=eval(grab(r'(\[[^\]]*\])', p, 1)[0])
            if p.startswith('What is the largest'): ok(t, max(held))
            elif p.startswith('What is the sum'): ok(t, sum(held))
            else: ok(t, sorted(held)[2])
        elif k=='units':
            if 'minutes' in p: ok(t, int(grab(r'in (\d+) hours', p,1)[0])*60)
            elif 'metres' in p: ok(t, int(grab(r'in (\d+) kilometres', p,1)[0])*1000)
            else: ok(t, int(grab(r'in (\d+) mebibytes', p,1)[0])*1024)
        elif k=='weeks':
            d=int(grab(r'of (\d+) days', p,1)[0])
            ok(t, d//7 if p.startswith('How many whole weeks') else d%7)
        elif k=='rate':
            s,h = grab(r'at (\d+) km/h for (\d+) hours', p,1,2); ok(t, int(s)*int(h))
        elif k=='solve':
            A,B,C = grab(r'Solve for x: (\d+)x \+ (-?\d+) = (-?\d+)', p,1,2,3)
            ok(t, (int(C)-int(B))//int(A))
        elif k=='code':
            body=p.split('\n\n',1)[1]; arg=grab(r'What does f\((.+?)\) return', p,1)[0]
            env={}; exec(body, env); ok(t, env['f'](eval(arg)))
        elif k=='geom':
            if p.startswith('What is the area'):
                w,h = grab(r'rectangle (\d+) by (\d+)', p,1,2); ok(t, int(w)*int(h))
            else:
                x,y = grab(r'legs of (\d+) and (\d+)', p,1,2); ok(t, int(math.hypot(int(x),int(y))))
        elif k=='seqterm':
            f_,s_,n_ = grab(r'starts at (\d+) and increases by (\d+) each term\. What is term number (\d+)', p.replace('\n',' '),1,2,3)
            ok(t, int(f_)+int(s_)*(int(n_)-1))
        elif k=='seqsum':
            f_,s_,n_ = grab(r'starts at (\d+) and increases by (\d+) each term\. What is the sum of its first (\d+)', p.replace('\n',' '),1,2,3)
            f_,s_,n_=int(f_),int(s_),int(n_); ok(t, n_*(2*f_+(n_-1)*s_)//2)
        elif k=='comb':
            n_,k_ = grab(r'can (\d+) items be (?:chosen|arranged) from (\d+)', p,1,2)
            n_,k_=int(n_),int(k_)
            ok(t, math.comb(k_,n_) if 'not mattering' in p else math.perm(k_,n_))
        elif k=='prime':
            n_=int(grab(r'from 1 to (\d+)', p,1)[0])
            ok(t, sum(1 for v in range(2,n_+1) if all(v%d for d in range(2,int(v**0.5)+1))))
        elif k=='text':
            w = grab(r'"([a-z]+)"', p,1)[0]
            if p.startswith('How many letters'): ok(t, len(w))
            elif 'backwards' in p: ok(t, w[::-1])
            else: ok(t, w[int(grab(r'letter number (\d+)', p,1)[0])-1])
        else:
            skipped+=1
    except LookupError:
        skipped+=1
print(f"re-derived {checked} of {len(tasks)}; not re-derivable by this checker: {skipped}; mismatches: {len(bad)}")
for b in bad[:8]: print("  ", b)
sys.exit(1 if bad else 0)
