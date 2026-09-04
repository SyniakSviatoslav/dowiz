# genarena: 100 generations x 10000 bump-allocs of size (k%5)+1 from the gen mark, reset to mark; fold = frag*10^12 + reset_ok*10^11 + mono*10^10 + hw*10^4 + gens
frag = reset_ok = mono = 1
cursor = hw = 0
for g in range(100):
    mark = cursor
    last = -1
    for k in range(10000):
        ptr = cursor
        mono &= int(ptr > last)
        last = ptr
        cursor += k % 5 + 1
        hw = max(hw, cursor)
    frag &= int(cursor == mark + sum(k % 5 + 1 for k in range(10000)))
    cursor = mark
    reset_ok &= int(cursor == mark)
print(frag * 10**12 + reset_ok * 10**11 + mono * 10**10 + hw * 10**4 + 100)
