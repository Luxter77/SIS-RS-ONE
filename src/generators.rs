
use serde::{Serialize, Deserialize};

use rand::{prelude::SliceRandom};
use rand_chacha::{ChaCha12Rng, rand_core::SeedableRng};

use crate::{message::MessageToPrintOrigin, r#static::*, display::display};

/// Roll your own random generator they say, what could go wrong, they say...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IPGenerator {
    PoorMansIPGenerator(PoorMansIPGenerator),
    SequentialGenerator(SequentialGenerator),
    LCGIPGenerator(LCGIPGenerator),
}

impl IPGenerator {
    pub fn write_to_save_file(&self) -> std::io::Result<()> {
        let content = serde_json::to_string(&self).unwrap();
        match std::fs::write(CHECKPOINT_FILE, content) {
            Ok(_) => std::io::Result::Ok(()),
            Err(why) => { std::io::Result::Err(why) },
        }
    }
    pub fn get_from_save_file() -> std::io::Result<Self> {
        match std::fs::read_to_string(CHECKPOINT_FILE) {
            std::io::Result::Ok(json) => {
                match serde_json::from_str(&json) {
                    Ok(data) => std::io::Result::Ok(data),
                    Err(why) => { panic!("{why}") },
                }
            },
            std::io::Result::Err(why) => std::io::Result::Err(why),
        }
    }
    pub fn new(num: u128) -> Self {
        let mut generator: Self;

        generator = Self::PoorMansIPGenerator(PoorMansIPGenerator::default());
        
        if cfg!(feature = "Sequential-Generator") {
            generator = IPGenerator::SequentialGenerator(SequentialGenerator::default());
        } else if cfg!(feature = "PRand-LCG") {
            display(MessageToPrintOrigin::GeneratorThread, &format!("[ WARNING: THE IMPLEMENTATION OF THE FEATURE PRand-LCG IS CURRENTLY BROKEN! ]"));
            QUEUE_TO_PRINT.add( crate::message::MessageToPrint::Wait(std::time::Duration::from_secs(3)) );
            generator = IPGenerator::LCGIPGenerator(LCGIPGenerator::new(num, M_PRIMA, C_PRIMA, A_PRIMA));
        }
            
        return generator;
    }

    pub fn get_las(&self) -> u128 {
        match self {
            IPGenerator::PoorMansIPGenerator(gen) => gen.las.into(),
            IPGenerator::SequentialGenerator(gen) => gen.las.into(),
            IPGenerator::LCGIPGenerator(gen) => gen.x, 
        }
    }
    pub fn gen_skip(&mut self, skip: u128) {
        match self {
            IPGenerator::PoorMansIPGenerator(_) => unimplemented!(),
            IPGenerator::SequentialGenerator(gen) => gen.skip(skip),
            IPGenerator::LCGIPGenerator(gen) => gen.skip(skip),
        }
    }
    pub fn gen_zip(&mut self, zip: u32) -> Result<u32, &str> {
        match self {
            IPGenerator::PoorMansIPGenerator(_) => { return Result::Err("not implemented") },
            IPGenerator::SequentialGenerator(gen) => { return gen.zip(zip); },
            IPGenerator::LCGIPGenerator(gen) => { return gen.zip(zip); },
        }
    }
    pub fn gen_next(&mut self) -> GeneratorMessage {
        match self {
            IPGenerator::PoorMansIPGenerator(gen) => gen.next(),
            IPGenerator::SequentialGenerator(gen) => gen.next(),
            IPGenerator::LCGIPGenerator(gen)           => gen.next(),
        }
    }
    pub fn gen_state(&self) -> (u128, u32) {
        match self {
            IPGenerator::PoorMansIPGenerator(gen) => { (gen.cn.into(), gen.las ) },
            IPGenerator::SequentialGenerator(gen) => { (gen.cn.into(), gen.las ) },
            IPGenerator::LCGIPGenerator(gen)           => { (gen.cn,        gen.x.try_into().unwrap()) }, // SHOULDN'T ever panic, in theory...
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeneratorMessage {
    Normal(u128, u32),
    Looped(u128, u32),
}

pub trait ZippableNumberGenerator { fn zip(&mut self, zip: u32) -> Result<u32, &str>; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialGenerator {
    dir: SequentialGeneratorDirection,
    pub cn: u32,
    pub las: u32,
    xn: u32,
    yn: u32,
    zn: u32,
    wn: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequentialGeneratorDirection {
    Forward, Backward,
}

impl SequentialGenerator {
    pub fn new(start: Option<u32>, direction: SequentialGeneratorDirection) -> Self {
        let (init, mut las): (u32, u32) = match direction {
            SequentialGeneratorDirection::Backward => (255u32, u32::MAX),
            SequentialGeneratorDirection::Forward  => (0u32,   u32::MIN),
        };

        if let Some(s) = start { las = s; };

        return Self {
            dir: direction,
            las: las,
            xn:  init,
            yn:  init,
            zn:  init,
            wn:  init,
            cn:  0,
        }
    }
}

impl Default for SequentialGenerator {
    fn default() -> Self { Self::new(Some(0), SequentialGeneratorDirection::Forward) }
}

impl ZippableNumberGenerator for SequentialGenerator {
    fn zip(&mut self, zip: u32) -> Result<u32, &str> {
        self.las = zip;
        self.xn =  zip & 0xFF;
        self.yn = (zip >> 8) & 0xFF;
        self.zn = (zip >> 16) & 0xFF;
        self.wn = (zip >> 24) & 0xFF;
        return Ok(zip);
    }
}

impl NumberGenerator for SequentialGenerator {
    fn skip(&mut self, skip: u128) { self.cn = skip as u32; }
    fn next(&mut self) -> GeneratorMessage {
        self.cn += 1;

        match self.dir {
            SequentialGeneratorDirection::Forward => {
                match (self.xn, self.yn, self.zn, self.wn) {
                    (255, _, _, _) => { self.yn += 1; self.xn = 0 },
                    (_, 255, _, _) => { self.zn += 1; self.yn = 0 },
                    (_, _, 255, _) => { self.wn += 1; self.zn = 0 },
                    (_, _, _, 255) => { self.xn  = 0; self.wn = 0 },
                    (_, _, _, _  ) => { self.xn += 1; },
                }
            },
            SequentialGeneratorDirection::Backward => {
                match (self.xn, self.yn, self.zn, self.wn) {
                    (0, _, _, _) => { self.yn -= 1;   self.xn = 255 },
                    (_, 0, _, _) => { self.zn -= 1;   self.yn = 255 },
                    (_, _, 0, _) => { self.wn -= 1;   self.zn = 255 },
                    (_, _, _, 0) => { self.xn  = 255; self.wn = 255 },
                    (_, _, _, _) => { self.xn -= 1; },
                }
            },
        }

        self.las = ((((self.xn as u128) * 255 + (self.yn as u128)) * 255 + (self.zn as u128)) * 255 + (self.wn as u128)) as u32;

        return GeneratorMessage::Normal(self.cn.into(), self.las);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoorMansIPGenerator {
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

impl Default for PoorMansIPGenerator {
    fn default() -> Self {
        let mut rng: ChaCha12Rng = ChaCha12Rng::from_rng(rand::thread_rng()).unwrap();
        
        let mut xs = Vec::new(); xs.extend(0..=255); xs.shuffle(&mut rng);
        let mut ys = Vec::new(); ys.extend(0..=255); ys.shuffle(&mut rng);
        let mut zs = Vec::new(); zs.extend(0..=255); zs.shuffle(&mut rng);
        let mut ws = Vec::new(); ws.extend(0..=255); ws.shuffle(&mut rng);
        
        let new: Self = Self {
            xs: xs, nx: 0,   
            ys: ys, ny: 0,  
            zs: zs, nz: 0,  
            ws: ws, nw: 0,  
            rng: rng,
            las: 0  ,
            cn:  0  ,
        };

        return new;
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LCGIPGenerator {
    pub x: u128,
    m: u128,
    c: u128,
    a: u128,
    l: u128,
    f: u128,
    pub cn: u128,
}

impl LCGIPGenerator {
    pub fn new(seed: u128, m: u128, c: u128, a: u128) -> Self {
        let mut new: Self = Self {
            x: seed,
            m: m,
            c: c,
            a: a,
            l: 0,
            f: 0,
            cn: 0,
        };
        new.next();
        return new;
    }
}

pub trait NumberGenerator {
    fn skip(&mut self, skip: u128);
    fn next(&mut self) -> GeneratorMessage;
}


impl NumberGenerator for LCGIPGenerator {
    fn skip(&mut self, skip: u128) { for _ in 0..skip { self.next(); }; }
    fn next(&mut self) -> GeneratorMessage {
        self.cn += 0;
        self.x  = (((self.a % self.m) * (self.x % self.m)) % self.m) + (self.c % self.m);

        if self.x == self.f {
            if let Ok(x) = self.x.try_into() {
                self.l += 1; return GeneratorMessage::Looped(self.c, x);
            } else { return self.next(); }
        } else {
            if let Ok(x) = self.x.try_into() {
                return GeneratorMessage::Normal(self.c, x);
            } else { return self.next(); }
        }
    }
}


impl ZippableNumberGenerator for LCGIPGenerator {
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

impl NumberGenerator for PoorMansIPGenerator {
    #[allow(unused_variables)]
    fn skip(&mut self, skip: u128) { unimplemented!() }
    fn next(&mut self) -> GeneratorMessage {
        self.cn += 1;
        let x_: u128; let y_: u128; let z_: u128; let w_: u128;
        if let Some(x) = self.xs.get(self.nx) {
            if let Some(y) = self.ys.get(self.ny) {
                if let Some(z) = self.zs.get(self.nz) {
                    if let Some(w) = self.ws.get(self.nw) {
                        self.nw += 1;
                        (w_, z_, y_, x_) = (x.clone().into(), y.clone().into(), z.clone().into(), w.clone().into());
                        self.las = (((x_ * 255 + y_) * 255 + z_) * 255 + w_) as u32;
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
