# carried.py x.exe: prints the compressed length and writes x.cwasm, the machine code the executable carries
import sys, zlib
data = open(sys.argv[1], 'rb').read()
end = data.rindex(b'WRPCwasm')
length = int.from_bytes(data[end-8:end], 'little')
compressed = data[end-8-length:end-8]
open(sys.argv[1].replace('.exe', '.cwasm'), 'wb').write(zlib.decompress(compressed, -15))
print(sys.argv[1], length)
