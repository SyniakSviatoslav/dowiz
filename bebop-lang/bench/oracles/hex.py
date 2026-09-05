# hex: byte -> packed lowercase hex ASCII pair hi*256+lo; fold = enc(171)*2^32 + enc(205)*2^16 + enc(239)
def enc(b):
    s = '%02x' % b
    return ord(s[0]) * 256 + ord(s[1])
# T123: * 131 + decoded bytes 171 + 205*256 + 239*65536 + hex_val('g') == -1
print((enc(171) * 2**32 + enc(205) * 2**16 + enc(239)) * 131 + 171 + 205 * 256 + 239 * 65536 - 1)
