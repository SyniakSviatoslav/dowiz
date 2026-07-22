export class InterfaceAudio {
  constructor() {
    this.ctx = null;
    this.enabled = true;
    this.volume = 0.3;
  }

  ensure() {
    if (!this.ctx) {
      const Ctor = window.AudioContext || window.webkitAudioContext;
      if (!Ctor) { this.enabled = false; return false; }
      this.ctx = new Ctor();
    }
    return true;
  }

  playTone(freq, duration, type = 'sine') {
    if (!this.enabled || !this.ensure()) return;
    const osc = this.ctx.createOscillator();
    const gain = this.ctx.createGain();
    osc.type = type;
    osc.frequency.value = freq;
    gain.gain.setValueAtTime(this.volume, this.ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, this.ctx.currentTime + duration);
    osc.connect(gain);
    gain.connect(this.ctx.destination);
    osc.start();
    osc.stop(this.ctx.currentTime + duration);
  }

  // ── Navigation ──
  navOpen() { this.playTone(660, 0.08, 'sine'); }
  navClose() { this.playTone(440, 0.06, 'sine'); }
  navSelect() { this.playTone(880, 0.05, 'sine'); }

  // ── Actions ──
  confirm() {
    this.playTone(523, 0.1);
    setTimeout(() => this.playTone(659, 0.1), 80);
    setTimeout(() => this.playTone(784, 0.15), 160);
  }
  cancel() { this.playTone(330, 0.15, 'sawtooth'); }
  error() { this.playTone(200, 0.2, 'sawtooth'); }
  success() {
    this.playTone(784, 0.08);
    setTimeout(() => this.playTone(1047, 0.15), 80);
  }

  // ── Friction feedback ──
  frictionStake(stake) {
    const freq = 220 + stake * 110;
    this.playTone(freq, 0.3, 'triangle');
  }
  frictionConfirm() {
    this.playTone(440, 0.15);
    setTimeout(() => this.playTone(880, 0.2), 150);
  }
  frictionReject() { this.playTone(180, 0.3, 'sawtooth'); }

  // ── Status ──
  orderReceived() {
    this.playTone(600, 0.08);
    setTimeout(() => this.playTone(800, 0.08), 60);
    setTimeout(() => this.playTone(1000, 0.12), 120);
  }
  orderDelivered() { this.success(); }

  // ── Voice cue (short attention tone) ──
  voiceAttention() {
    this.playTone(1200, 0.04, 'sine');
    setTimeout(() => this.playTone(1400, 0.04, 'sine'), 50);
  }
}
