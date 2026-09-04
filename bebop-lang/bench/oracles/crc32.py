# crc32: reflected CRC-32 (poly 0xEDB88320, init ~0, final ~) of the bytes "123456789"
def crc32(data):
    c = 0xFFFFFFFF
    for b in data:
        c ^= b
        for _ in range(8):
            c = (c >> 1) ^ 0xEDB88320 if c & 1 else c >> 1
    return c ^ 0xFFFFFFFF
print(crc32(b"123456789"))
