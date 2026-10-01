# Oracle for gate `dl_bound` (row DG8, A-6): p(Y) :- p(X), two(T), mul(X,T,Y) from p = {1}, two = {2}
# derives 2^k for every k -- an infinite relation; with p's declared domain of 8 rows the least fixpoint
# does not exist inside it, so the only correct outcome is the loud refusal. Prints `exit:125 <trap line>`.
cap, p, k = 8, {1}, 0
while True:
    new = {x * 2 for x in p} - p
    if not new:
        break
    p |= new
    k += 1
    if len(p) > cap:
        print("exit:125 trap 125: datalog fixpoint bound exceeded")
        break
else:
    print("exit:0 %d" % len(p))
