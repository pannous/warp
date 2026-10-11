#!/usr/bin/env python3
# card reason-beep: how much of beep's energy is aliasing, frequencies that are no harmonic of its 880 Hz (a naive
# square wave's harmonics above the Nyquist frequency fold back into the audible range as inharmonic buzz).
# usage: probes/sound/beep_spectrum.py <warp binary> ["program rendering into SONG"]
import cmath, math, os, struct, subprocess, sys

WARP = os.path.abspath(sys.argv[1])
PROGRAM = sys.argv[2] if len(sys.argv) > 2 else "use sound\nbeep()\nrender_sound(\"SONG\")"
FUNDAMENTAL = 880
BIN_HERTZ = 10
NEAR_HARMONIC_HERTZ = 30
WAV_HEADER = 44
FOLDER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scratch", "beep")


def rendered_samples():
	os.makedirs(FOLDER, exist_ok=True)
	path = os.path.join(FOLDER, "beep.wav")
	run = subprocess.run([WARP, "--no-ask", "eval", PROGRAM.replace("SONG", path)], capture_output=True, text=True, cwd=FOLDER,
		env={**os.environ, "WARP_NO_WINDOW": "1"})
	if run.returncode:
		sys.exit(run.stdout + run.stderr)
	data = open(path, "rb").read()
	rate = struct.unpack("<I", data[24:28])[0]
	return rate, struct.unpack(f"<{(len(data) - WAV_HEADER) // 2}h", data[WAV_HEADER:])


def power_at(samples, rate, frequency):
	turn = -2j * math.pi * frequency / rate
	return abs(sum(sample * cmath.exp(turn * i) for i, sample in enumerate(samples))) ** 2


def near_harmonic(frequency):
	nearest = round(frequency / FUNDAMENTAL) * FUNDAMENTAL
	return nearest > 0 and abs(frequency - nearest) <= NEAR_HARMONIC_HERTZ


rate, samples = rendered_samples()
spectrum = [(frequency, power_at(samples, rate, frequency)) for frequency in range(BIN_HERTZ, rate // 2, BIN_HERTZ)]
total = sum(power for _, power in spectrum)
aliased = sum(power for frequency, power in spectrum if not near_harmonic(frequency))
loudest = max(abs(sample) for sample in samples)
print(f"rate {rate}, {len(samples)} samples, peak {loudest / 32768:.2f} of full scale")
print(f"aliasing: {100 * aliased / total:.1f} % of the energy off the harmonics of {FUNDAMENTAL} Hz")
for frequency, power in sorted(spectrum, key=lambda entry: -entry[1])[:8]:
	print(f"  {frequency:5d} Hz {10 * math.log10(power / total):6.1f} dB{'' if near_harmonic(frequency) else '  alias'}")
