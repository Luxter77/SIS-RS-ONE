
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
#[allow(clippy::large_enum_variant)] // TODO: SEE IF THIS OPTIMIZATION ACTUALLY MAKES SENSE OR NOT
pub enum IPGenerator {
    PoorMans(PoorMansIPGenerator),
    Sequential(SequentialGenerator),
    LCGIP(LCGIPGenerator),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GeneratorDirection {
    Forward, Backward, Random
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum NumberGenerators {
    PoorMansGen,
    Sequential,
    LCG,
}

impl IPGenerator {
    pub fn can_last(&self) -> bool {
        match self {
            IPGenerator::PoorMans(_) => false,
            IPGenerator::Sequential(_) => true,
            IPGenerator::LCGIP(_)      => true,
        }
    }
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
    pub fn new(seed: u128, strategy: NumberGenerators, no_continue: bool) -> Self {
        if !no_continue {
            if let Ok(gen) = IPGenerator::get_from_save_file() {
                return gen
            } else {
                return Self::new(seed, strategy, true);
            };
        } else {
            return match strategy {
                NumberGenerators::PoorMansGen => IPGenerator::PoorMans(PoorMansIPGenerator::default()),
                NumberGenerators::Sequential  => IPGenerator::Sequential(SequentialGenerator::default()),
                NumberGenerators::LCG => {
                    display(MessageToPrintOrigin::Generator, "[ WARNING: THE IMPLEMENTATION OF THE FEATURE PRand-LCG IS CURRENTLY BROKEN! ]");
                    QUEUE_TO_PRINT.add( crate::message::MessageToPrint::Wait(std::time::Duration::from_secs(3)) );
                    IPGenerator::LCGIP(LCGIPGenerator::new(seed, M_PRIMA, C_PRIMA, A_PRIMA))
                },
            };
        };
    }
    pub fn get_las(&self) -> u128 {
        match self {
            IPGenerator::PoorMans(gen) => gen.las.into(),
            IPGenerator::Sequential(gen) => gen.las.into(),
            IPGenerator::LCGIP(gen)           => gen.x, 
        }
    }
    pub fn gen_skip(&mut self, skip: u32) {
        match self {
            IPGenerator::PoorMans(_) => unimplemented!(),
            IPGenerator::Sequential(gen) => gen.skip(skip),
            IPGenerator::LCGIP(gen)           => gen.skip(skip),
        }
    }
    pub fn gen_zip(&mut self, zip: u32) -> Result<u32, &str> {
        match self {
            IPGenerator::PoorMans(_) => { return Result::Err("not implemented") },
            IPGenerator::Sequential(gen) => { return gen.zip(zip); },
            IPGenerator::LCGIP(gen) => { return gen.zip(zip); },
        }
    }
    pub fn gen_next(&mut self) -> GeneratorMessage {
        match self {
            IPGenerator::PoorMans(gen) => gen.next(),
            IPGenerator::Sequential(gen) => gen.next(),
            IPGenerator::LCGIP(gen)           => gen.next(),
        }
    }
    pub fn gen_state(&self) -> (u128, u32) {
        match self {
            IPGenerator::PoorMans(gen) => { (gen.cn.into(), gen.las ) },
            IPGenerator::Sequential(gen) => { (gen.cn.into(), gen.las ) },
            IPGenerator::LCGIP(gen)           => { (gen.cn,        gen.x.try_into().unwrap()) }, // SHOULDN'T ever panic, in theory...
        }
    }
    pub fn gen_dir(&self) -> GeneratorDirection {
        match self {
            IPGenerator::PoorMans(_)                         => { GeneratorDirection::Random },
            IPGenerator::LCGIP(_)                              => { GeneratorDirection::Random },
            IPGenerator::Sequential(gen) => { gen.dir },
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

