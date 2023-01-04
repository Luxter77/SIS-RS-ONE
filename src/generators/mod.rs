
use serde::{Serialize, Deserialize};

use crate::{message::MessageToPrintOrigin, r#static::*, display::display};

pub mod generator;

mod poor_mans_ip_generator;
mod sequential_generator;
mod lcgipgenerator;

use sequential_generator::SequentialGenerator;
use poor_mans_ip_generator::PoorMansIPGenerator;
use lcgipgenerator::LCGIPGenerator;

/// Roll your own random generator they say, what could go wrong, they say...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IPGenerator {
    PoorMansIPGenerator(PoorMansIPGenerator),
    SequentialGenerator(SequentialGenerator),
    LCGIPGenerator(LCGIPGenerator),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GeneratorDirection {
    Forward, Backward, Random
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GeneratorLimit {
    Limited(u32),
    Unlimited,
}

impl Default for GeneratorLimit { fn default() -> Self { GeneratorLimit::Unlimited }}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum NumberGenerators {
    PoorMansGen,
    Sequential,
    LCG,
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
    pub fn new(seed: u128, strategy: NumberGenerators, no_continue: bool, last: u128) -> Self {
        if !no_continue {
            if let Ok(gen) = IPGenerator::get_from_save_file() {
                return gen
            } else {
                return Self::new(seed, strategy, true, last);
            };
        } else {
            return match strategy {
                NumberGenerators::PoorMansGen => IPGenerator::PoorMansIPGenerator(PoorMansIPGenerator::default()),
                NumberGenerators::Sequential  => { IPGenerator::SequentialGenerator(SequentialGenerator::new(seed.try_into().ok().or(Some(0)), GeneratorDirection::Forward, TryInto::<u32>::try_into(last).ok())) },
                NumberGenerators::LCG => {
                    display(MessageToPrintOrigin::GeneratorThread, &format!("[ WARNING: THE IMPLEMENTATION OF THE FEATURE PRand-LCG IS CURRENTLY BROKEN! ]"));
                    QUEUE_TO_PRINT.add( crate::message::MessageToPrint::Wait(std::time::Duration::from_secs(3)) );
                    IPGenerator::LCGIPGenerator(LCGIPGenerator::new(seed, M_PRIMA, C_PRIMA, A_PRIMA))
                },
            };
        };
    }
    pub fn gen_skip(&mut self, skip: u32) {
        match self {
            IPGenerator::PoorMansIPGenerator(_) => unimplemented!(),
            IPGenerator::SequentialGenerator(gen) => gen.skip(skip),
            IPGenerator::LCGIPGenerator(gen)           => gen.skip(skip),
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
    pub fn gen_dir(&self) -> GeneratorDirection {
        match self {
            IPGenerator::PoorMansIPGenerator(_)                         => { GeneratorDirection::Random },
            IPGenerator::LCGIPGenerator(_)                              => { GeneratorDirection::Random },
            IPGenerator::SequentialGenerator(gen) => { gen.dir },
        }
    }
    pub fn las_passed(&self) -> bool {
        match self {
            IPGenerator::PoorMansIPGenerator(_)                         => { false },
            IPGenerator::LCGIPGenerator(_)                              => { false },
            IPGenerator::SequentialGenerator(gen) => { gen.has_passed_limit() },
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeneratorMessage {
    Normal(u128, u32),
    Looped(u128, u32),
}

pub trait ZippableNumberGenerator { fn zip(&mut self, zip: u32) -> Result<u32, &str>; }

pub trait NumberGenerator {
    fn skip(&mut self, skip: u32);
    fn next(&mut self) -> GeneratorMessage;
}

pub trait LimitedNumberGenerator {
    fn has_passed_limit(&self) -> bool;
}