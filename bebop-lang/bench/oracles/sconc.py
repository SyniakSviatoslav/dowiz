# G6 sconc oracle (T115): 4 writers x 10^4 increments through the single-writer lock,
# 4 readers x 10^4 consistent reads -> the gate prints 40000 (every counter 10^4, sum
# == generation - 1, zero inconsistent reads); anything else encodes the failure.
print(40000)
