export class WaveSimulation {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this.c = 2.0;
    this.damping = 0.98;
    this.u = new Float32Array(width * height);
    this.uPrev = new Float32Array(width * height);
    this.ripples = [];
  }

  addRipple(x, y, intensity = 1.0) {
    this.ripples.push({ x, y, intensity, age: 0 });
    const idx = Math.floor(y) * this.width + Math.floor(x);
    if (idx >= 0 && idx < this.u.length) {
      this.u[idx] += intensity * 0.5;
    }
  }

  step(dt = 0.016) {
    const { u, uPrev, width, height, c, damping } = this;
    const c2 = c * c;
    const dx2 = 1.0;
    const dt2 = dt * dt;

    for (let y = 1; y < height - 1; y++) {
      for (let x = 1; x < width - 1; x++) {
        const i = y * width + x;
        const laplacian = (
          u[(y - 1) * width + x] + u[(y + 1) * width + x] +
          u[y * width + (x - 1)] + u[y * width + (x + 1)] -
          4 * u[i]
        ) / dx2;
        const acceleration = c2 * laplacian;
        const velocity = u[i] - uPrev[i];
        uPrev[i] = u[i];
        u[i] = damping * (2 * u[i] - uPrev[i] + acceleration * dt2);
      }
    }

    this.ripples = this.ripples.filter(r => {
      r.age++;
      return r.age < 60;
    });
  }

  getField() { return this.u; }

  render(ctx, width, height, palette = 'fire') {
    const { u } = this;
    const imgData = ctx.createImageData(width, height);
    const colors = palette === 'ocean'
      ? [[0,0,50],[0,50,100],[0,100,150],[50,150,200],[100,200,255]]
      : [[50,0,0],[150,30,0],[255,100,0],[255,180,50],[255,255,200]];

    for (let i = 0; i < u.length; i++) {
      const v = (u[i] * 0.5 + 0.5) * (colors.length - 1);
      const idx = Math.floor(Math.max(0, Math.min(v, colors.length - 2)));
      const t = v - idx;
      const c1 = colors[idx], c2 = colors[idx + 1];
      imgData.data[i * 4] = c1[0] + (c2[0] - c1[0]) * t;
      imgData.data[i * 4 + 1] = c1[1] + (c2[1] - c1[1]) * t;
      imgData.data[i * 4 + 2] = c1[2] + (c2[2] - c1[2]) * t;
      imgData.data[i * 4 + 3] = 255;
    }
    ctx.putImageData(imgData, 0, 0);
  }
}

export class EnvironmentEffect {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this.particles = [];
    this.wind = { x: 0.01, y: 0.005 };
    this.density = new Float32Array(width * height);
    for (let i = 0; i < 50; i++) {
      this.particles.push({
        x: Math.random() * width, y: Math.random() * height,
        vx: (Math.random() - 0.5) * 2, vy: (Math.random() - 0.5) * 2,
        life: Math.random() * 100
      });
    }
  }

  step() {
    for (const p of this.particles) {
      p.vx += this.wind.x + (Math.random() - 0.5) * 0.1;
      p.vy += this.wind.y + (Math.random() - 0.5) * 0.1;
      p.x += p.vx; p.y += p.vy;
      p.life -= 0.5;
      if (p.life <= 0 || p.x < 0 || p.x > this.width || p.y < 0 || p.y > this.height) {
        p.x = Math.random() * this.width;
        p.y = Math.random() * this.height;
        p.vx = (Math.random() - 0.5) * 2;
        p.vy = (Math.random() - 0.5) * 2;
        p.life = 100 + Math.random() * 50;
      }
    }
  }

  render(ctx, width, height) {
    for (const p of this.particles) {
      const alpha = Math.max(0, p.life / 100) * 0.3;
      ctx.fillStyle = `rgba(255, 200, 100, ${alpha})`;
      ctx.beginPath();
      ctx.arc(p.x, p.y, 1.5, 0, Math.PI * 2);
      ctx.fill();
    }
  }
}

export class InterfaceRipple {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
    this.wave = new WaveSimulation(64, 64);
    this.env = new EnvironmentEffect(64, 64);
    this.lastTime = 0;
  }

  start() {
    this.canvas.addEventListener('click', (e) => {
      const x = (e.offsetX / this.canvas.width) * this.wave.width;
      const y = (e.offsetY / this.canvas.height) * this.wave.height;
      this.wave.addRipple(x, y);
    });
    this.loop(performance.now());
  }

  loop(time) {
    const dt = Math.min((time - this.lastTime) / 1000, 0.05);
    this.lastTime = time;
    this.wave.step(dt);
    this.env.step();
    this.wave.render(this.ctx, this.canvas.width, this.canvas.height, 'ocean');
    this.env.render(this.ctx, this.canvas.width, this.canvas.height);
    requestAnimationFrame((t) => this.loop(t));
  }
}
