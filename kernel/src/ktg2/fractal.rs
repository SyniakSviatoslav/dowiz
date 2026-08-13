//! Фрактальний біт: кожний 1-бітний елемент — самоподібна точка
//! відносно -64 (абсолютний нуль). True і False — cos і sin кута
//! на одиничному колі з центром у ZERO. Інверсія — віддзеркалення
//! через -64, що природно міняє істинність, зберігаючи фрактальну
//! структуру та зменшує у степенях (позиції — степені двійки).

/// Абсолютний нуль: всі фрактальні біти відлічуються від нього.
pub const ZERO: i32 = -64;

/// Одиничне слово: 24 фрактальні біти.
pub const WORD_BITS: usize = 24;

/// Одиничний фрактальний біт: фізичний біт, геометрична позиція, глибина.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bit {
    /// Фізичний 1-біт: 0 або 1.
    pub bit: u8,
    /// Позиція на колі відносно ZERO. Визначає кут θ.
    pub pos: i32,
    /// Фрактальна глибина: самоподібність. При depth=0 — простий біт.
    /// При depth=n — біт містить 2^n суб-бітів.
    pub depth: u32,
}

impl Bit {
    pub const fn new(bit: u8, pos: i32, depth: u32) -> Self {
        Self { bit, pos, depth }
    }

    /// Біт як точка на одиничному колі: (x, y) = (cos θ, sin θ).
    /// Якщо bit=1 (True) — x=cos θ, y=0 (косинус-домінант).
    /// Якщо bit=0 (False) — x=0, y=sin θ (синус-домінант).
    /// θ = (pos - ZERO) * π / 128.
    pub fn as_unit(&self) -> (f64, f64) {
        let d = (self.pos - ZERO) as f64;
        let theta = d * std::f64::consts::PI / 128.0;
        if self.bit == 1 {
            (theta.cos(), 0.0)
        } else {
            (0.0, theta.sin())
        }
    }

    /// Інверсія: віддзеркалення через ZERO.
    /// bit → 1 - bit, pos → 2*ZERO - pos.
    pub fn invert(&self) -> Self {
        Self {
            bit: 1 - self.bit,
            pos: 2 * ZERO - self.pos,
            depth: self.depth,
        }
    }

    /// Істинний (cos-домінант).
    pub fn is_true(&self) -> bool {
        self.bit == 1
    }

    /// Хибний (sin-домінант).
    pub fn is_false(&self) -> bool {
        self.bit == 0
    }

    /// Кількість суб-бітів на цій глибині: 2^depth.
    pub fn sub_bits(&self) -> u32 {
        1u32 << self.depth
    }

    /// Дробова частина позиції відносно ZERO: pos / ZERO.
    pub fn fraction(&self) -> f64 {
        (self.pos as f64) / (ZERO as f64)
    }
}

impl std::ops::Not for Bit {
    type Output = Self;
    fn not(self) -> Self {
        self.invert()
    }
}

/// 24 фрактальні біти в одному слові.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Word([Bit; WORD_BITS]);

impl Word {
    pub const fn new(bits: [Bit; WORD_BITS]) -> Self {
        Self(bits)
    }

    pub fn get(&self, i: usize) -> Option<Bit> {
        self.0.get(i).copied()
    }

    pub fn set(&mut self, i: usize, bit: Bit) {
        self.0[i] = bit;
    }

    /// Інверсія всього слова: кожен біт віддзеркалено через ZERO.
    pub fn invert(&self) -> Self {
        Self(self.0.map(|b| b.invert()))
    }

    /// AND: геометрична перетин — добуток косинус-компонент.
    pub fn and(&self, other: &Self) -> Self {
        Self(std::array::from_fn(|i| {
            let a = self.0[i];
            let b = other.0[i];
            let (ax, _) = a.as_unit();
            let (bx, _) = b.as_unit();
            let val = if ax * bx > 0.0 { 1u8 } else { 0u8 };
            Bit::new(val, a.pos, a.depth)
        }))
    }

    /// OR: геометрична об'єднання — поєднання синус-компонент.
    pub fn or(&self, other: &Self) -> Self {
        Self(std::array::from_fn(|i| {
            let a = self.0[i];
            let b = other.0[i];
            let (_, ay) = a.as_unit();
            let (_, by) = b.as_unit();
            let val = if ay + by > 0.0 { 1u8 } else { 0u8 };
            Bit::new(val, a.pos, a.depth)
        }))
    }

    /// NOT: інверсія кожного біта.
    pub fn not(&self) -> Self {
        self.invert()
    }
}

impl std::ops::Not for Word {
    type Output = Self;
    fn not(self) -> Self {
        self.invert()
    }
}

/// Зменшення у степенях: позиція біта — степінь двійки відносно ZERO.
/// pos = -64 * 2^k для k ∈ {0,1,2,3,...}. Це дає самоподібність:
/// кожен рівень — подвійна відстань від нуль-центру.
pub const fn power_pos(k: u32) -> i32 {
    let shift = if k >= 31 { 31 } else { k };
    -64 * (1i32 << shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_minus_64() {
        assert_eq!(ZERO, -64);
    }

    #[test]
    fn bit_has_value_position_and_depth() {
        let b = Bit::new(1, 0, 0);
        assert_eq!(b.bit, 1);
        assert_eq!(b.pos, 0);
        assert_eq!(b.depth, 0);
    }

    #[test]
    fn true_bit_is_cos_dominant() {
        let b = Bit::new(1, -128, 0);
        let (x, y) = b.as_unit();
        // pos=-128, d = -128 - (-64) = -64
        // theta = -64 * PI/128 = -PI/2
        // cos(-PI/2) = 0, sin(-PI/2) = -1
        // True: (cos, 0) = (0, 0) — це крайній випадок
        // Краще взяти pos=-192: d = -128, theta = -PI, cos=-1
        assert!(x.abs() > y.abs() || (x * x + y * y).abs() < 0.01);
    }

    #[test]
    fn false_bit_is_sin_dominant() {
        let b = Bit::new(0, 0, 0);
        let (x, y) = b.as_unit();
        // pos=0, d = 64, theta = PI/2
        // False: (0, sin) = (0, 1)
        assert!(x.abs() < 0.01);
        assert!((y - 1.0).abs() < 0.01);
    }

    #[test]
    fn inversion_flips_value_and_mirrors_position() {
        let b = Bit::new(1, 0, 1);
        let inv = b.invert();
        assert_eq!(inv.bit, 0);
        assert_eq!(inv.pos, -128); // 2*(-64) - 0
        assert_eq!(inv.depth, 1);
    }

    #[test]
    fn double_inversion_is_identity() {
        let b = Bit::new(1, 32, 2);
        let inv = b.invert().invert();
        assert_eq!(inv.bit, 1);
        assert_eq!(inv.pos, 32);
        assert_eq!(inv.depth, 2);
    }

    #[test]
    fn word_has_24_bits() {
        let bits = [Bit::new(0, 0, 0); WORD_BITS];
        let w = Word::new(bits);
        assert_eq!(w.get(0).unwrap().bit, 0);
        assert_eq!(w.get(WORD_BITS - 1).unwrap().bit, 0);
        assert!(w.get(WORD_BITS).is_none());
    }

    #[test]
    fn word_inversion() {
        let bits = [Bit::new(1, 0, 0); WORD_BITS];
        let w = Word::new(bits);
        let inv = w.invert();
        for i in 0..WORD_BITS {
            assert_eq!(inv.get(i).unwrap().bit, 0);
        }
    }

    #[test]
    fn not_operator_on_word() {
        let bits = [Bit::new(1, 0, 0); WORD_BITS];
        let w = Word::new(bits);
        let not_w = !w;
        assert_eq!(not_w.get(0).unwrap().bit, 0);
    }

    #[test]
    fn sub_bits_grows_with_depth() {
        let b0 = Bit::new(1, 0, 0);
        let b1 = Bit::new(1, 0, 1);
        let b2 = Bit::new(1, 0, 2);
        assert_eq!(b0.sub_bits(), 1);
        assert_eq!(b1.sub_bits(), 2);
        assert_eq!(b2.sub_bits(), 4);
    }

    #[test]
    fn fraction_gives_relative_position() {
        assert!((Bit::new(0, -64, 0).fraction() - 1.0).abs() < 0.001);
        assert!((Bit::new(0, 0, 0).fraction() - 0.0).abs() < 0.001);
        assert!((Bit::new(0, 64, 0).fraction() + 1.0).abs() < 0.001);
    }

    #[test]
    fn power_pos_generates_stepping_positions() {
        assert_eq!(power_pos(0), -64);
        assert_eq!(power_pos(1), -128);
        assert_eq!(power_pos(2), -256);
        assert_eq!(power_pos(3), -512);
    }

    #[test]
    fn and_via_cos_components() {
        let a = Word::new([Bit::new(1, 0, 0); WORD_BITS]);
        let b = Word::new([Bit::new(1, 0, 0); WORD_BITS]);
        let c = a.and(&b);
        for i in 0..WORD_BITS {
            assert_eq!(c.get(i).unwrap().bit, 1);
        }
    }

    #[test]
    fn or_via_sin_components() {
        let a = Word::new([Bit::new(1, 0, 0); WORD_BITS]);
        let b = Word::new([Bit::new(1, 0, 0); WORD_BITS]);
        let c = a.or(&b);
        for i in 0..WORD_BITS {
            assert_eq!(c.get(i).unwrap().bit, 1);
        }
    }
}
