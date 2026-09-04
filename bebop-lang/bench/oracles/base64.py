# base64: encode "Man" and "fox"; each 4-char group packed big-endian as a 32-bit int; fold = v1*1e9 + v2
# (hand-rolled: this file's name shadows the stdlib base64 module)
ALPH = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
def enc3(s):
    x = int.from_bytes(s, "big")
    return int.from_bytes(bytes(ALPH[(x >> sh) & 63] for sh in (18, 12, 6, 0)), "big")
print(enc3(b"Man") * 10**9 + enc3(b"fox"))
