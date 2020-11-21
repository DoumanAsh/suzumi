use core::{fmt, cmp};

const MAX_LEVEL: u8 = 99;
const MAX_EXP: u32 = exp_until_level(MAX_LEVEL);

#[inline]
const fn exp_until_level(level: u8) -> u32 {
    55 * level as u32 * level as u32 * level as u32
}

///Experience calculation for level.
pub trait LevelExpModifier {
    ///Returns experience to add.
    ///
    ///Provides `level` to scale.
    fn calculate(&self, level: u8) -> u32;
}

#[derive(Debug, PartialEq)]
pub enum AddResult {
    Maxed,
    LevelUp,
    Added,
}

pub struct Level {
    pub exp: u32,
    pub level: u8,
}

impl Level {
    pub fn new(exp: u32) -> Self {
        let level = Self::exp_to_level(exp);

        Self {
            exp,
            level,
        }
    }

    pub fn with_max() -> Self {
        Self::new(MAX_EXP - 1)
    }

    ///Calculate cash allowance
    pub const fn cash(&self) -> u32 {
        self.level as u32 * 10
    }

    #[inline]
    pub fn add_for<T: LevelExpModifier>(&mut self, exp: &T) -> AddResult {
        self.add(exp.calculate(self.level))
    }

    ///Adds experience points and returns
    ///whether level up happened.
    pub fn add(&mut self, val: u32) -> AddResult {
        if self.level >= MAX_LEVEL {
            return AddResult::Maxed;
        }

        self.exp = cmp::min(MAX_EXP, self.exp + val);
        let new_level = Self::exp_to_level(self.exp);

        if new_level != self.level {
            self.level = new_level;
            AddResult::LevelUp
        } else {
            AddResult::Added
        }
    }

    fn exp_to_level(exp: u32) -> u8 {
        let exp = cmp::min(exp, MAX_EXP);

        let exp = (exp / 55) as f32;
        let exp = exp.powf(1.0 / 3.0);
        exp as u8
    }
}

impl fmt::Display for Level {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        match self.level {
            MAX_LEVEL => write!(w, "{}", MAX_EXP),
            level => {
                let next_level = level + 1;
                let exp_until_next = exp_until_level(next_level);
                write!(w, "{}/{}", self.exp, exp_until_next)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_level_up() {
        let mut level = Level::new(550);

        assert_eq!(level.add(0), AddResult::Added);
        assert_eq!(level.level, 2);
        assert_eq!(level.add(1), AddResult::Added);
        assert_eq!(level.level, 2);
        assert_eq!(level.add(10), AddResult::Added);
        assert_eq!(level.level, 2);
        let add_exp = 1_485 - level.exp - 1;
        assert_eq!(level.add(add_exp), AddResult::Added);
        assert_eq!(level.level, 2);
        assert_eq!(level.add(1), AddResult::LevelUp);
        assert_eq!(level.exp, 1_485);
        println!("{}", level);
        assert_eq!(level.level, 3);
        assert_eq!(level.add(MAX_EXP), AddResult::LevelUp);
        assert_eq!(level.exp, MAX_EXP);
        assert_eq!(level.level, 99);
        println!("{}", level);
    }

    #[test]
    fn verify_level() {
        assert_eq!(Level::exp_to_level(MAX_EXP), 99);

        assert_eq!(Level::exp_to_level(55 * 5 * 5 * 5), 5);
        assert_eq!(Level::exp_to_level(55 * 5 * 5 * 5 - 1), 4);

        assert_eq!(Level::exp_to_level(0), 0);
        assert_eq!(Level::exp_to_level(50), 0);
        assert_eq!(Level::exp_to_level(55), 1);

        assert_eq!(Level::exp_to_level(550), 2);
    }
}

