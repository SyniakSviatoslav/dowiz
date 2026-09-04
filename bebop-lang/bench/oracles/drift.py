# drift oracle: 28 spec checks against the Rust DRIFT GOLDENS
# (bench/vs_rust/spectral_golden/golden.txt), hashed base 131 with i64 wrap.
import os, re
G = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'vs_rust', 'spectral_golden', 'golden.txt')
txt = open(G).read()
sec = txt[txt.index('DRIFT GOLDENS'):txt.index('CSR GOLDENS')]
kv = lambda line: {k: int(v) for k, v in re.findall(r'(\w+)=(-?\d+)', line)}
prof = {m.group(1): kv(m.group(0)) for m in re.finditer(r'== profile (\w+).*', sec)}
drift = {m.group(1): kv(m.group(0)) for m in re.finditer(r'== drift (\S+) .*', sec)}
B = 1 << 20  # rho tolerance band (fp32 units), as in the .bp
checks = []
# profiles: class 0/1/2/2, unstable 0/0/2/1, rho 2^31/2^32/2^33/2^33
for name, cls, un, rho in [('HALF_I', 0, 0, 1 << 31), ('I2', 1, 0, 1 << 32),
                           ('TWO_I', 2, 2, 1 << 33), ('MIX', 2, 1, 1 << 33)]:
    p = prof[name]
    checks += [p['class'] == cls, p['unstable'] == un, p['rho_fp32'] - rho < B]
# drifts: from, to, udelta, rho_delta
for name, fr, to, ud, rd in [('HALF_I->I2', 0, 1, 0, (1 << 31) - 1), ('I2->TWO_I', 1, 2, 2, 1 << 32),
                             ('HALF_I->MIX', 0, 2, 1, 6442450943), ('MIX->MIX', 2, 2, 0, 0)]:
    d = drift[name]
    checks += [d['from'] == fr, d['to'] == to, d['udelta'] == ud,
               d['rho_delta_fp32'] == 0 if rd == 0 else d['rho_delta_fp32'] - rd < B]
assert len(checks) == 28
acc = 0
for c in checks:
    acc = (acc * 131 + int(c)) & ((1 << 64) - 1)
print(acc if acc < 1 << 63 else acc - (1 << 64))
