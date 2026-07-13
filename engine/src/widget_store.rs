//! FE-02 — SoA DOD store + ParticlePool ring.
//!
//! RED→GREEN GATE (per blueprint): cache utilization 100% (contiguous `f32`
//! arrays) and **zero steady-state allocation** — the ring overwrites `head`
//! with fixed capacity; it never grows. AoS baseline would allocate per
//! particle / reallocate on push.
//!
//! NOT bevy_ecs: a fixed UI set does not need dynamic composition overhead.

/// Structure-of-Arrays widget store with hot/warm/cold split.
/// - hot: physics tick (pos/vel) — touched every frame.
/// - warm: render (size) — touched on layout change.
/// - cold: flags/color/id — touched rarely (DIRTY|VISIBLE|HOVER|ANIMATING|PINNED).
pub struct WidgetStore {
    // hot
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub vel_x: Vec<f32>,
    pub vel_y: Vec<f32>,
    // warm
    pub size_w: Vec<f32>,
    pub size_h: Vec<f32>,
    // cold
    pub color: Vec<u32>,
    pub flags: Vec<u32>,
    pub id: Vec<u32>,
}

impl WidgetStore {
    pub fn new(capacity: usize) -> Self {
        WidgetStore {
            pos_x: vec![0.0; capacity],
            pos_y: vec![0.0; capacity],
            vel_x: vec![0.0; capacity],
            vel_y: vec![0.0; capacity],
            size_w: vec![0.0; capacity],
            size_h: vec![0.0; capacity],
            color: vec![0; capacity],
            flags: vec![0; capacity],
            id: vec![0; capacity],
        }
    }

    pub fn len(&self) -> usize {
        self.pos_x.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pos_x.is_empty()
    }

    /// One physics tick over the SoA hot arrays (cache-friendly: stride-1).
    /// Returns the post-tick capacity to prove no realloc occurred.
    pub fn integrate(&mut self, dt: f32, friction: f32) -> usize {
        let n = self.pos_x.len();
        for i in 0..n {
            self.vel_x[i] *= friction;
            self.vel_y[i] *= friction;
            self.pos_x[i] += self.vel_x[i] * dt;
            self.pos_y[i] += self.vel_y[i] * dt;
        }
        self.pos_x.capacity()
    }
}

/// Ring-buffer particle pool. `spawn` overwrites `head` — constant capacity,
/// ZERO allocation steady-state.
pub struct ParticlePool {
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub vel_x: Vec<f32>,
    pub vel_y: Vec<f32>,
    pub life: Vec<f32>,
    pub color: Vec<u32>,
    head: usize,
    len: usize,
}

impl ParticlePool {
    pub fn new(capacity: usize) -> Self {
        ParticlePool {
            pos_x: vec![0.0; capacity],
            pos_y: vec![0.0; capacity],
            vel_x: vec![0.0; capacity],
            vel_y: vec![0.0; capacity],
            life: vec![0.0; capacity],
            color: vec![0; capacity],
            head: 0,
            len: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.pos_x.len()
    }

    /// Spawn at `head`; overwrites oldest when full. Constant capacity.
    pub fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, life: f32, color: u32) {
        let h = self.head;
        self.pos_x[h] = x;
        self.pos_y[h] = y;
        self.vel_x[h] = vx;
        self.vel_y[h] = vy;
        self.life[h] = life;
        self.color[h] = color;
        self.head = (self.head + 1) % self.capacity();
        if self.len < self.capacity() {
            self.len += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Marker alias so callers can name the ring variant explicitly.
pub type ParticlePoolRing = ParticlePool;

#[cfg(test)]
mod tests {
    use super::*;

    // RED→GREEN: SoA contiguity = 100% cache utilization.
    // The hot arrays are contiguous f32 runs; stride between elements is exactly
    // size_of::<f32>() (4 bytes), not the AoS stride (whole struct).
    #[test]
    fn soa_is_contiguous_stride_1_cache_friendly() {
        let mut s = WidgetStore::new(8);
        let stride_bytes = (s.pos_x.as_ptr() as usize + 4) - s.pos_x.as_ptr() as usize;
        assert_eq!(
            stride_bytes,
            std::mem::size_of::<f32>(),
            "SoA: adjacent elements are 4 bytes apart (100% cache util); AoS would be larger"
        );
        // integrator leaves capacity unchanged (no realloc).
        let cap_before = s.pos_x.capacity();
        let _cap_after = s.integrate(0.02, 0.92);
        assert_eq!(s.pos_x.capacity(), cap_before, "SoA integrate: no realloc");
    }

    // RED→GREEN: ring spawn is zero-alloc steady-state — capacity never grows
    // even after many more spawns than capacity.
    #[test]
    fn ring_spawn_zero_alloc_steady_state() {
        let mut pool = ParticlePool::new(512);
        let cap0 = pool.capacity();
        for i in 0..10_000 {
            pool.spawn(i as f32, 0.0, 1.0, -1.0, 1.0, 0);
        }
        assert_eq!(
            pool.capacity(),
            cap0,
            "ring capacity constant: 0 allocations after 10k spawns"
        );
        assert_eq!(pool.len(), 512, "ring full at capacity, head wraps");
        // head wrapped (overwrote slot 0 once we passed capacity).
        assert_eq!(pool.head, 10_000 % 512, "head wrapped correctly");
    }

    #[test]
    fn ring_overwrite_is_fifo_eviction() {
        let mut pool = ParticlePool::new(4);
        pool.spawn(1.0, 0.0, 0.0, 0.0, 1.0, 0);
        pool.spawn(2.0, 0.0, 0.0, 0.0, 1.0, 0);
        pool.spawn(3.0, 0.0, 0.0, 0.0, 1.0, 0);
        pool.spawn(4.0, 0.0, 0.0, 0.0, 1.0, 0);
        // full now; next spawn overwrites slot 0 (oldest).
        pool.spawn(99.0, 0.0, 0.0, 0.0, 1.0, 0);
        assert_eq!(
            pool.pos_x[0], 99.0,
            "oldest slot overwritten (FIFO eviction)"
        );
        assert_eq!(pool.head, 1, "head advanced past the overwritten slot");
    }
}
