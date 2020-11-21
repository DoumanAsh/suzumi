use super::*;

impl crate::game::LevelExpModifier for Message {
    fn calculate(&self, level: u8) -> u32 {
        let len = self.content.chars().count();

        if level == 0 {
            1
        } else if len < 10 {
            level as u32
        } else if len < 25 {
            level as u32 * 2
        } else if len < 50 {
            level as u32 * 4
        } else if len < 100 {
            level as u32 * 8
        } else {
            level as u32 * 10
        }
    }
}
