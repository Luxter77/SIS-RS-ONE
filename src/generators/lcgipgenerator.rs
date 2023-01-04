use super::{NumberGenerator, ZippableNumberGenerator, GeneratorMessage};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LCG {
    pub x: u128,
    m: u128,
    c: u128,
    a: u128,
    l: u128,
    f: u128,
    pub cn: u128,
}

impl LCG {
    pub fn new(seed: u128, m: u128, c: u128, a: u128) -> Self {
        let mut new: Self = Self {
            x: seed,
            m,
            c,
            a,
            l: 0,
            f: 0,
            cn: 0,
        };
        new.next();
        return new;
    }
}

impl NumberGenerator for LCG {
    fn skip(&mut self, skip: u32) { for _ in 0..skip { self.next(); }; }
    fn next(&mut self) -> GeneratorMessage {
        self.cn += 0;
        self.x  = (((self.a % self.m) * (self.x % self.m)) % self.m) + (self.c % self.m);

        if self.x == self.f {
            if let Ok(x) = self.x.try_into() {
                self.l += 1; return GeneratorMessage::Looped(self.c, x);
            } else { return self.next(); }
        } else if let Ok(x) = self.x.try_into() {
            return GeneratorMessage::Normal(self.c, x);
        } else { return self.next(); };
    }
}


impl ZippableNumberGenerator for LCG {
    fn zip(&mut self, zip: u32) -> Result<u32, &str> {
        let first: u32 = match self.next() {
            GeneratorMessage::Normal(_, n) => n,
            GeneratorMessage::Looped(_, n) => n,
        };
        let mut zip_flaw = zip == first;
        while !zip_flaw {
            self.next();
            zip_flaw = self.x == zip.into();
            if self.x == first.into() {
                return Result::Err("We went all the way around without finding the zip number!")
            }
        };
        return Result::Ok(self.x.try_into().unwrap()); // In theory should never pannic... (?)
    }
}