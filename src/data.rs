//! Data types and serialization

use core::time;

pub trait Serialize {
    const SIZE: usize;
    //Generally should be static array;
    type Output: AsRef<[u8]>;

    fn serialize(&self) -> Self::Output;
}

pub trait Deserialize: Serialize {
    fn deserialize(ser: &Self::Output) -> Self;
}

#[derive(Debug, Default)]
pub struct Server {
    pub welcome_ch: u64,
    pub music_ch: u64,
    pub dev_ch: u64,
    pub spam_ch: u64,
}

impl Server {
    #[inline]
    pub const fn from_bytes(data: &<Self as Serialize>::Output) -> Self {
        Self {
            welcome_ch: u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]),
            music_ch: u64::from_le_bytes([data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15]]),
            dev_ch: u64::from_le_bytes([data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23]]),
            spam_ch: u64::from_le_bytes([data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31]]),
        }
    }

    #[inline]
    pub const fn to_bytes(&self) -> <Self as Serialize>::Output {
        let welcome_ch = self.welcome_ch.to_le_bytes();
        let music_ch = self.music_ch.to_le_bytes();
        let dev_ch = self.dev_ch.to_le_bytes();
        let spam_ch = self.spam_ch.to_le_bytes();

        [
            welcome_ch[0],
            welcome_ch[1],
            welcome_ch[2],
            welcome_ch[3],
            welcome_ch[4],
            welcome_ch[5],
            welcome_ch[6],
            welcome_ch[7],
            music_ch[0],
            music_ch[1],
            music_ch[2],
            music_ch[3],
            music_ch[4],
            music_ch[5],
            music_ch[6],
            music_ch[7],
            dev_ch[0],
            dev_ch[1],
            dev_ch[2],
            dev_ch[3],
            dev_ch[4],
            dev_ch[5],
            dev_ch[6],
            dev_ch[7],
            spam_ch[0],
            spam_ch[1],
            spam_ch[2],
            spam_ch[3],
            spam_ch[4],
            spam_ch[5],
            spam_ch[6],
            spam_ch[7],
        ]
    }
}

impl Serialize for Server {
    const SIZE: usize = 32;
    type Output = [u8; 32];

    #[inline]
    fn serialize(&self) -> Self::Output {
        self.to_bytes()
    }
}

impl Deserialize for Server {
    #[inline]
    fn deserialize(data: &Self::Output) -> Self {
        Self::from_bytes(data)
    }
}

#[derive(Debug)]
pub struct User {
    pub cash: u32,
    pub exp: u32,
    //since epoch
    pub last_allowance: time::Duration,
}

impl User {
    #[inline]
    pub const fn new() -> Self {
        Self {
            cash: 100,
            exp: 0,
            last_allowance: time::Duration::from_secs(0),
        }
    }

    #[inline]
    pub const fn from_bytes(data: &<Self as Serialize>::Output) -> Self {
        Self {
            cash: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            exp: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            last_allowance: time::Duration::from_secs(u64::from_le_bytes([
                    data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15]
            ])),
        }
    }

    #[inline]
    pub const fn to_bytes(&self) -> <Self as Serialize>::Output {
        let cash = self.cash.to_le_bytes();
        let exp = self.exp.to_le_bytes();
        let last_allowance = self.last_allowance.as_secs().to_le_bytes();

        [
            cash[0],
            cash[1],
            cash[2],
            cash[3],
            exp[0],
            exp[1],
            exp[2],
            exp[3],
            last_allowance[0],
            last_allowance[1],
            last_allowance[2],
            last_allowance[3],
            last_allowance[4],
            last_allowance[5],
            last_allowance[6],
            last_allowance[7],
        ]
    }
}

impl Serialize for User {
    const SIZE: usize = 16;
    type Output = [u8; 16];

    #[inline]
    fn serialize(&self) -> Self::Output {
        self.to_bytes()
    }
}

impl Deserialize for User {
    #[inline]
    fn deserialize(data: &Self::Output) -> Self {
        Self::from_bytes(data)
    }
}

impl Default for User {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
