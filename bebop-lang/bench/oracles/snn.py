# snn: 16 neurons = bits of state; fire[i] = popcount(state & conn[i]) >= 2; spike[i] = fire - oldbit; fold = sum (spike[i]+1)*4^i + newstate*10^9
conn = [40000, 14881, 3855, 50115, 32256, 8064, 39321, 26214, 21930, 43605, 4080, 61455, 4660, 22136, 39612, 57072]
state = 51238
fold = nstate = 0
for i, c in enumerate(conn):
    fire = int(bin(state & c).count("1") >= 2)
    fold += (fire - (state >> i & 1) + 1) * 4 ** i
    nstate |= fire << i
print(fold + nstate * 10 ** 9)
