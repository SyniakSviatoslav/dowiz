# sort: sort [3,1,4,1,5,9,2,6,5,3,5] ascending; fold = Horner hash acc = acc*31 + a[i] over the sorted array
acc = 0
for v in sorted([3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5]):
    acc = acc * 31 + v
print(acc)
