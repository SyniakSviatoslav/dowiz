# tb: tokenbox self-test; fold = c1*10^6 + cz*10^5 + lh*10^4 + it*10^3 where c1: crc32("123456789")==3421780262, cz: crc32("")==0, lh: "456" in "123456789", it: itoa(12345) is 5 digits '1'..'5'
import zlib
c1 = int(zlib.crc32(b"123456789") == 3421780262)
cz = int(zlib.crc32(b"") == 0)
lh = int(b"456" in b"123456789")
s = str(12345)
it = int(len(s) == 5 and s[0] == "1" and s[4] == "5")
print(c1 * 1000000 + cz * 100000 + lh * 10000 + it * 1000)
