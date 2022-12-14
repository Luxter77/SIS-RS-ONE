use super::{NumberGenerator, ZippableNumberGenerator, GeneratorMessage, GeneratorDirection, GeneratorLimit, LimitedNumberGenerator};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequential {
    pub dir: GeneratorDirection,
    pub cn:  u32,
    pub las: u32,
    #[serde(default = "GeneratorLimit::default")] lim: GeneratorLimit,
    xn: u8,
    yn: u8,
    zn: u8,
    wn: u8,
}

impl Sequential {
    pub fn new(start: Option<u32>, direction: GeneratorDirection, limit: Option<u32>) -> Self {
        let mut limited: GeneratorLimit = GeneratorLimit::Unlimited;
        let (init, mut las): (u8, u32) = match direction {
            GeneratorDirection::Backward => (255u8, u8::MAX as u32),
            GeneratorDirection::Forward  => (0u8,   u8::MIN as u32),
            GeneratorDirection::Random   => unimplemented!(),
        };

        if let Some(s) = start { las = s; };
        if let Some(lim) = limit {
            limited = GeneratorLimit::Limited(lim);
        };

        return Self {
            dir: direction,
            lim: limited,
            las,
            xn:  init,
            yn:  init,
            zn:  init,
            wn:  init,
            cn:  0,
        };
    }
    fn reg_from_n(&mut self) {
        self.las = ((self.xn as u32) << 00) + ((self.yn as u32) << 08) + ((self.zn as u32) << 16) + ((self.wn as u32) << 24);
    }
    fn reg_from_las(&mut self) {
        self.xn = ((self.las >> 24) & 0xFF) as u8;
        self.yn = ((self.las >> 16) & 0xFF) as u8;
        self.zn = ((self.las >> 08) & 0xFF) as u8;
        self.wn = ((self.las >> 00) & 0xFF) as u8;
    }
}

impl Default for Sequential {
    fn default() -> Self { Self::new(Some(0), GeneratorDirection::Forward, Option::None) }
}

impl ZippableNumberGenerator for Sequential {
    fn zip(&mut self, zip: u32) -> Result<u32, &str> {
        self.las = zip;
        self.reg_from_las();
        return Ok(zip);
    }
}

impl NumberGenerator for Sequential {
    fn skip(&mut self, skip: u32) {
        (self.cn,  _) = self.cn.overflowing_add(skip);
        (self.las, _) = match self.dir {
            GeneratorDirection::Backward => self.las.overflowing_add(skip),
            GeneratorDirection::Forward  => self.las.overflowing_add(skip),
            GeneratorDirection::Random   => unimplemented!(),
        };
        self.reg_from_n();
    }
    fn next(&mut self) -> GeneratorMessage {
        let looped: bool;
        (self.cn, _) = self.cn.overflowing_add(1);
        (self.las, looped) = match self.dir {
            GeneratorDirection::Forward  => self.las.overflowing_add(1),
            GeneratorDirection::Backward => self.las.overflowing_sub(1),
            GeneratorDirection::Random =>   unimplemented!(),
        };
        self.reg_from_las();
        match looped {
            true  => { return GeneratorMessage::Looped(self.cn as u128, self.las); },
            false => { return GeneratorMessage::Normal(self.cn as u128, self.las); },
        };
    }
}

impl LimitedNumberGenerator for Sequential {
    fn has_passed_limit(&self) -> bool {
        match self.lim {
            GeneratorLimit::Limited(lim) => {
                match self.dir {
                    GeneratorDirection::Forward  => { lim <= self.las },
                    GeneratorDirection::Backward => { lim >= self.las },
                    GeneratorDirection::Random   => unimplemented!(),
                }
            },
            GeneratorLimit::Unlimited => false,
        }
    }
}