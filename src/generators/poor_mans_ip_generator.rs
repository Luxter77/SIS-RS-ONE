use super::{NumberGenerator, GeneratorMessage};

use rand_chacha::{ChaCha12Rng, rand_core::SeedableRng};
use serde::{Deserialize, Serialize};
use rand::{prelude::SliceRandom};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoorMans {
    xs: Vec<u8>,
    ys: Vec<u8>,
    zs: Vec<u8>,
    ws: Vec<u8>,
    nx: usize,
    ny: usize,
    nz: usize,
    nw: usize,
    pub cn: u32,
    rng: ChaCha12Rng,
    pub las: u32,
}

impl Default for PoorMans {
    fn default() -> Self {
        let mut rng: ChaCha12Rng = ChaCha12Rng::from_rng(rand::thread_rng()).unwrap();
        
        let mut xs: Vec<u8> = Vec::new(); xs.extend(0..=255); xs.shuffle(&mut rng);
        let mut ys: Vec<u8> = Vec::new(); ys.extend(0..=255); ys.shuffle(&mut rng);
        let mut zs: Vec<u8> = Vec::new(); zs.extend(0..=255); zs.shuffle(&mut rng);
        let mut ws: Vec<u8> = Vec::new(); ws.extend(0..=255); ws.shuffle(&mut rng);
        
        let new: Self = Self {
            xs, nx: 0,
            ys, ny: 0,
            zs, nz: 0,
            ws, nw: 0,
            rng,
            las: 0,
            cn:  0,
        };

        return new;
    }
}

impl NumberGenerator for PoorMans {
    #[allow(unused_variables)]
    fn skip(&mut self, skip: u32) { unimplemented!() }
    fn next(&mut self) -> GeneratorMessage {
        self.cn += 1;
        if let Some(x) = self.xs.get(self.nx) {
            if let Some(y) = self.ys.get(self.ny) {
                if let Some(z) = self.zs.get(self.nz) {
                    if let Some(w) = self.ws.get(self.nw) {
                        self.nw += 1;
                        self.las = ((*x as u32) << 00) + ((*y as u32) << 08) + ((*z as u32) << 16) + ((*w as u32) << 24);
                        return GeneratorMessage::Normal(self.cn.into(), self.las);
                    } else {
                        self.nw = 0;
                        self.nz += 1;
                        self.ws.shuffle(&mut self.rng);
                        return self.next();
                    };
                } else {
                    self.nz = 0;
                    self.ny += 1;
                    self.zs.shuffle(&mut self.rng);
                    return self.next();
                };
            } else {
                self.ny = 0;
                self.nx += 1;
                self.ys.shuffle(&mut self.rng);
                return self.next();
            };
        } else {
            self.nx = 0;
            self.xs.shuffle(&mut self.rng);
            return match self.next() {
                GeneratorMessage::Normal(c, n) => GeneratorMessage::Looped(c, n),
                GeneratorMessage::Looped(c, _) => panic!("NumberGenerator entered an infinte Loop @ {c} iterations!\nWe dont allow those kind of loops here!"),
            };
        };
    }
}
