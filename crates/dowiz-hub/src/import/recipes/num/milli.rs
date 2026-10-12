//! A RECIPE LINE'S QUANTITY IN THOUSANDTHS (W-QTYHUB H4, CONTRACT-QTY §1):
//! what a CSV's `0,0425 kg` or `1,5 pcs` comes to, exactly, or why not.

use super::{Base, Rat};

impl Rat {
    /// THOUSANDTHS of the base unit this is: `0.0425 kg` is 42 500 (42.5 g),
    /// exactly; finer than a thousandth of the base unit is refused, not
    /// rounded, as a fraction of money is.
    pub fn milli(self, base: Base, max: i64) -> Result<i64, String> {
        let n = self.num.checked_mul(1000).ok_or("is too large to compute exactly")?;
        if n % self.den != 0 {
            let tenths = n * 10 / self.den;
            return Err(format!(
                "comes to {}.{:03}{}… {}; finer than a thousandth of a {} is refused, not rounded",
                tenths / 10_000,
                tenths / 10 % 1000,
                tenths % 10,
                base.as_str(),
                base.as_str()
            ));
        }
        let m = n / self.den;
        if m <= 0 {
            return Err("is zero".into());
        }
        if m > i128::from(max) * 1000 {
            return Err(format!("is {}.{:03} {}, over the {max} a line may hold", m / 1000, m % 1000, base.as_str()));
        }
        i64::try_from(m).map_err(|_| "is too large".to_string())
    }
}
