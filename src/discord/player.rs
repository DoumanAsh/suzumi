use super::*;

use serenity::voice::AudioSource;
use serenity::client::bridge::voice::ClientVoiceManager;
use tokio::sync::mpsc;

use std::sync::Arc;

pub type PlayerSender = mpsc::Sender<PlayerCommand>;

pub enum PlayerCommand {
    Play(u64, Box<dyn AudioSource>),
    Stop(u64),
    Shutdown,
}

pub struct MusicPlayer {
    db: DbView,
    voice_manager: Arc<serenity::prelude::Mutex<ClientVoiceManager>>,
    receiver: mpsc::Receiver<PlayerCommand>,
}

impl MusicPlayer {
    pub fn new(db: DbView, voice_manager: Arc<serenity::prelude::Mutex<ClientVoiceManager>>) -> (Self, PlayerSender) {
        let (sender, receiver) = tokio::sync::mpsc::channel(64);

        (Self {
            db,
            voice_manager,
            receiver,
        }, sender)
    }

    pub async fn run(mut self) {
        //Reference to currently playing.
        let mut ongoing: Option<serenity::voice::LockedAudio> = None;
        //Back-log to play.
        let mut list = std::collections::VecDeque::new();

        loop {
            let cmd = match self.receiver.try_recv() {
                Ok(cmd) => cmd,
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => match list.pop_front() {
                    Some((server_id, audio)) => PlayerCommand::Play(server_id, audio),
                    None => match self.receiver.recv().await {
                        Some(cmd) => cmd,
                        //Senders are dead
                        None => return,
                    }
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Closed) => return,
            };

            match cmd {
                PlayerCommand::Play(server_id, audio) => {
                    let is_ongoing = match ongoing.as_ref() {
                        Some(audio) => {
                            let audio = audio.lock().await;
                            !audio.finished || audio.playing
                        },
                        None => false,
                    };

                    if is_ongoing {
                        list.push_back((server_id, audio));
                    } else {
                        ongoing = None;
                        let channel_id = loop {
                            match self.db.get::<data::Server>(server_id) {
                                Ok(server) => break server.music_ch,
                                Err(error) => {
                                    rogu::error!("Cannot read music channel id: {}", error);
                                }
                            }
                        };

                        if let Some(handler) = self.voice_manager.lock().await.join(server_id, channel_id) {
                            handler.deafen(true);
                            ongoing = Some(handler.play_returning(audio));
                        } else {
                            rogu::error!("Unable to join voice channel");
                        }
                    }
                },
                PlayerCommand::Stop(server_id) => {
                    let mut voice_manager = self.voice_manager.lock().await;
                    if let Some(handler) = voice_manager.get_mut(server_id) {
                        handler.stop();
                    }

                    ongoing = None;

                    match voice_manager.leave(server_id) {
                        Some(_) => {
                            rogu::debug!("Left voice channel");
                        },
                        None => {
                            rogu::warn!("No voice manager for server {}, cannot leave  voice channel", server_id);
                        },
                    }
                },
                PlayerCommand::Shutdown => break,
            }
        }
    }
}
