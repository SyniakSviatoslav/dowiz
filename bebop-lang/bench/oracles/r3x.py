# r3x: five emitter regression checks (mul/shift precedence, logical >>, conditional store in loop, str literals, trivial loop) all 1; fold = t1..t5 packed + str_len("abcde")*10 + char("hello",1)
M = (1 << 64) - 1
t1 = int(3 * (5 << 4) == (3 * 5) << 4)
t2 = int(((-16) & M) >> 2 == 4611686018427387900)
t3 = int(max([7, 33, 2, 48]) == 48)
t4 = int(len("hello") * 10000 + ord("h") * 100 + ord("o") == 60511)
t5 = int(sum(1 for _ in range(3)) == 3)
print(t1 * 10**9 + t2 * 10**8 + t3 * 10**7 + t4 * 10**6 + t5 * 10**5 + len("abcde") * 10 + ord("hello"[1]))
