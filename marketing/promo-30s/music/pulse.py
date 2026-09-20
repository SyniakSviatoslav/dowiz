# A 30-second keynote pulse at exactly 120 BPM, synthesised from nothing, so the cut owns its
# soundtrack and every beat is on a known frame: beat = 0.5 s, bar = 2 s, downbeats at 0, 4, 16,
# 20, 28 s match shots 1, 3, 7, 9, 13. Stereo 48 kHz WAV, the plan's UI taps baked in.
import numpy as np, wave, sys, json
SR = 48000; BPM = 120; BEAT = 60 / BPM; LEN = 30.0
N = int(SR * LEN); t = np.arange(N) / SR
rng = np.random.default_rng(7)
L = np.zeros(N); R = np.zeros(N)

def at(sig, start, gain=1.0, pan=0.0):
    i = int(start * SR); n = min(len(sig), N - i)
    if n <= 0: return
    L[i:i+n] += sig[:n] * gain * (1 - max(0, pan)); R[i:i+n] += sig[:n] * gain * (1 + min(0, pan))
def env(n, a, d, s=0.0, r=0.0):
    e = np.ones(n); A = int(a*SR); D = int(d*SR); Rr = int(r*SR)
    if A: e[:A] = np.linspace(0, 1, A)
    if D: e[A:A+D] = np.linspace(1, s, min(D, max(0, n-A)))[:max(0, min(D, n-A))]
    if A + D < n: e[A+D:] = s
    if Rr and Rr < n: e[-Rr:] *= np.linspace(1, 0, Rr)
    return e
def lowpass_fft(x, cutoff, order=2):
    X = np.fft.rfft(x); f = np.fft.rfftfreq(len(x), 1/SR)
    H = 1 / np.sqrt(1 + (f / max(cutoff, 20)) ** (2*order)); return np.fft.irfft(X * H, n=len(x))
def note(semi, base=55.0): return base * 2 ** (semi/12)

# --- harmony: A minor cinematic, one chord per two bars from 4 s; a drone before ---
CHORDS = [(0, [0, 3, 7, 12]), (4, [0, 3, 7, 12]), (8, [-4, 0, 3, 8]), (12, [-9, -5, 0, 3]), (16, [0, 3, 7, 12]), (20, [-2, 2, 5, 10]), (24, [-4, 0, 3, 8]), (28, [0, 3, 7, 12])]  # Am, Am, F, C, Am, G, F, Am
def chord_at(s):
    c = CHORDS[0][1]
    for st, ch in CHORDS:
        if s >= st: c = ch
    return c

# --- drone / pad: chord tones as soft sines with slow attack, low-passed ---
pad = np.zeros(N)
for st, ch in CHORDS:
    en = min(LEN, st + 4.0 + 0.4)
    n = int((en - st) * SR); tt = np.arange(n) / SR
    seg = np.zeros(n)
    for semi in ch:
        for oct_, g in ((1, 1.0), (2, 0.5), (4, 0.18)):
            f = note(semi) * oct_
            seg += g * np.sin(2*np.pi*f*tt + rng.uniform(0, 6.28)) * (1 + 0.02*np.sin(2*np.pi*0.3*tt))
    seg *= env(n, 0.6 if st else 2.5, 0.2, 0.9, 0.4)
    at_i = int(st*SR); seg = seg[:N-at_i]; pad[at_i:at_i+len(seg)] += seg
pad = lowpass_fft(pad, 900, 2)
pad *= 0.11
# the drone before the drop is darker and swells into 4 s
sw = np.ones(N); i4 = int(4*SR); sw[:i4] = np.linspace(0.35, 1.05, i4)
pad *= sw

# --- sub bass: root of the chord, sidechained on every beat from 4 s ---
bass = np.zeros(N)
for st, ch in CHORDS:
    if st < 4: continue
    en = min(LEN, st + 4.0); n = int((en - st) * SR); tt = np.arange(n) / SR
    f = note(ch[0] if ch[0] >= -6 else ch[0] + 12) / 2  # one octave under the pad root
    seg = np.sin(2*np.pi*f*tt) + 0.25*np.sin(2*np.pi*2*f*tt)
    seg *= env(n, 0.02, 0.05, 1.0, 0.05)
    bass[int(st*SR):int(st*SR)+n] += seg
duck = np.ones(N)
for b in np.arange(4, LEN, BEAT):
    i = int(b*SR); n = int(0.22*SR); duck[i:i+n] *= np.concatenate([np.linspace(0.25, 1.0, n)])[:min(n, N-i)]
bass *= duck * 0.28

# --- kick on every beat from 4 s; the two downbeat hits (4 s, 28 s) heavier ---
def kick(strength=1.0, boom=False):
    n = int(0.45*SR); tt = np.arange(n)/SR
    f = 42 + 130*np.exp(-tt*40); ph = 2*np.pi*np.cumsum(f)/SR
    k = np.sin(ph) * np.exp(-tt*9) * strength
    click = rng.normal(0, 1, n) * np.exp(-tt*400) * 0.35
    if boom:
        n2 = int(1.8*SR); tt2 = np.arange(n2)/SR
        k2 = np.sin(2*np.pi*(38 + 20*np.exp(-tt2*6))*tt2) * np.exp(-tt2*2.2) * 0.9
        k = np.concatenate([k + 0, np.zeros(n2 - n)]); k += k2
        click = np.concatenate([click, np.zeros(n2 - n)])
    return k + click
for b in np.arange(4, LEN, BEAT):
    beat_in_bar = round((b % 2) / BEAT)
    g = 0.95 if beat_in_bar == 0 else 0.62 if beat_in_bar == 2 else 0.5
    if b >= 28.4: break
    at(kick(g, boom=(b in (4.0, 28.0))), b, 0.9)
at(kick(1.0, boom=True), 28.0, 0.35)

# --- hats: ticking tension from 0, driving 8ths after 4 s, 16ths after 16 s ---
def hat(dur=0.05, g=1.0):
    n = int(dur*SR); x = rng.normal(0, 1, n) * np.exp(-np.arange(n)/SR*90)
    X = np.fft.rfft(x); f = np.fft.rfftfreq(n, 1/SR); X[f < 6000] *= 0.05; return np.fft.irfft(X, n=n) * g
for b in np.arange(0, LEN, BEAT/2):
    if b >= 29.5: break
    off = abs((b % BEAT) - BEAT/2) < 1e-6
    if b < 4: g = 0.12 if not off else 0.05
    else: g = 0.16 if not off else 0.09
    at(hat(0.04 if off else 0.06, g), b, 1.0, 0.25 if off else -0.15)
for b in np.arange(16, 28, BEAT/4):
    if abs((b % (BEAT/2))) > 1e-6: at(hat(0.03, 0.06), b, 1.0, 0.4)

# --- clap on 2 and 4 from 16 s ---
def clap():
    n = int(0.25*SR); x = np.zeros(n)
    for d in (0, 0.011, 0.022, 0.034):
        i = int(d*SR); m = n - i; x[i:] += rng.normal(0, 1, m) * np.exp(-np.arange(m)/SR*(30 if d > 0.03 else 140))
    X = np.fft.rfft(x); f = np.fft.rfftfreq(n, 1/SR); X[(f < 900) | (f > 9000)] *= 0.15; return np.fft.irfft(X, n=n) * 0.35
for b in np.arange(16 + BEAT, 28, BEAT*2): at(clap(), b, 1.0)

# --- plucks: 16th-note arpeggio on chord tones from 7 s (the phone rises) ---
def pluck(f):
    n = int(0.35*SR); tt = np.arange(n)/SR
    x = np.sin(2*np.pi*f*tt) + 0.4*np.sin(2*np.pi*2*f*tt) + 0.15*np.sin(2*np.pi*3*f*tt)
    return x * np.exp(-tt*11) * 0.13
k = 0
for b in np.arange(7, 28, BEAT/4):
    ch = chord_at(b); pattern = [0, 2, 1, 3, 2, 1, 0, 2]
    semi = ch[pattern[k % 8]]; f = note(semi) * 4 * (2 if (k % 16) in (3, 11) else 1)
    at(pluck(f), b, 1.0, (-0.5 if k % 2 else 0.5) * 0.6); k += 1

# --- risers into the two big downbeats: 4 s (the zero) and 28 s (the mark) ---
def riser(dur):
    n = int(dur*SR); x = rng.normal(0, 1, n); X = np.fft.rfft(x); f = np.fft.rfftfreq(n, 1/SR)
    X[f > 3500] *= 0.1; y = np.fft.irfft(X, n=n); return y * (np.linspace(0, 1, n) ** 2.2) * 0.22
at(riser(3.4), 0.6); at(riser(2.0), 26.0, 0.6)

# --- the plan's spot effects, -16 dB under the track ---
def tap():
    n = int(0.06*SR); tt = np.arange(n)/SR; return (np.sin(2*np.pi*1800*tt) * np.exp(-tt*120) + rng.normal(0, .3, n)*np.exp(-tt*400)) * 0.16
def hit():
    n = int(0.9*SR); tt = np.arange(n)/SR; x = rng.normal(0, 1, n) * np.exp(-tt*5)
    return lowpass_fft(x, 1200) * 0.5 + np.sin(2*np.pi*55*tt)*np.exp(-tt*4)*0.4
def stamp():
    n = int(0.5*SR); tt = np.arange(n)/SR; return (np.sin(2*np.pi*(70+60*np.exp(-tt*30))*tt) * np.exp(-tt*10) * 0.5 + rng.normal(0, 1, n)*np.exp(-tt*200)*0.2) * 0.7
at(hit(), 0.9, 0.16 * 3); at(tap(), 10.6); at(tap(), 18.9); at(tap(), 24.8); at(stamp(), 28.2, 0.5)

# --- master: gentle pump on the pad from the kick, soft clip, fade, normalise ---
pad *= np.minimum(1, duck + 0.35)
L += pad * 1.0; R += pad * 1.0
L += bass; R += bass
mix = np.stack([L, R])
mix = np.tanh(mix * 1.15) / np.tanh(1.15)
fade = np.ones(N); nf = int(0.6*SR); fade[-nf:] = np.linspace(1, 0, nf); mix *= fade
mix *= 0.70 / np.max(np.abs(mix))
out = sys.argv[1]
with wave.open(out, 'wb') as w:
    w.setnchannels(2); w.setsampwidth(2); w.setframerate(SR)
    w.writeframes((mix.T * 32767).astype(np.int16).tobytes())
json.dump({"bpm": BPM, "beat_s": BEAT, "bar_s": BEAT*4, "beats": [round(x, 3) for x in np.arange(0, LEN, BEAT)], "downbeats": [round(x, 3) for x in np.arange(0, LEN, BEAT*4)], "drops": [4.0, 28.0]}, open(out.replace('.wav', '-beat-grid.json'), 'w'))
print("wrote", out, "peak", float(np.max(np.abs(mix))))
