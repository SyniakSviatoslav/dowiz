# Oracle for gate `dp` (T38): closed-form-free Fibonacci via a generator.
M62 = (1 << 62) - 1
def mix(h, x): return ((h * 1000003) + x) & M62
def fib(n):
    a, b = 0, 1
    for _ in range(n): a, b = b, a + b
    return a
h = 37
for n in range(91): h = mix(h, fib(n))
for n in range(21): h = mix(h, fib(n))
print(h)
