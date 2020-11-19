//! Data types and serialization

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
    welcome_ch: u64,
    music_ch: u64,
}

impl Server {
    pub const fn new() -> Self {
        Self {
            welcome_ch: 0,
            music_ch: 0,
        }
    }


    #[inline]
    pub const fn from_bytes(data: &<Self as Serialize>::Output) -> Self {
        Self {
            welcome_ch: u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]),
            music_ch: u64::from_le_bytes([data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15]]),
        }
    }

    #[inline]
    pub const fn to_bytes(&self) -> <Self as Serialize>::Output {
        let welcome_ch = self.welcome_ch.to_le_bytes();
        let music_ch = self.music_ch.to_le_bytes();

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
        ]
    }
}

impl Serialize for Server {
    const SIZE: usize = 16;
    type Output = [u8; 16];

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
    cash: u32,
    exp: u32,
}

impl User {
    #[inline]
    pub const fn new() -> Self {
        Self {
            cash: 100,
            exp: 0,
        }
    }

    #[inline]
    pub const fn from_bytes(data: &<Self as Serialize>::Output) -> Self {
        Self {
            cash: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            exp: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        }
    }

    #[inline]
    pub const fn to_bytes(&self) -> <Self as Serialize>::Output {
        let cash = self.cash.to_le_bytes();
        let exp = self.exp.to_le_bytes();

        [
            cash[0],
            cash[1],
            cash[2],
            cash[3],
            exp[0],
            exp[1],
            exp[2],
            exp[3],
        ]
    }
}

impl Serialize for User {
    const SIZE: usize = 8;
    type Output = [u8; 8];

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
