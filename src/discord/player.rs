use super::*;

use songbird::Songbird;
use songbird::input::Input as Audio;
use tokio::sync::mpsc;

use std::sync::Arc;

pub type PlayerSender = mpsc::Sender<PlayerCommand>;

pub struct OnTrackFinished(PlayerSender);

#[serenity::async_trait]
impl songbird::events::EventHandler for OnTrackFinished {
    async fn act(&self, _: &songbird::events::EventContext<'_>) -> Option<songbird::events::Event> {
        let _ = self.0.send(PlayerCommand::TrackFinished).await;
        None
    }
}

pub enum PlayerCommand {
    Play(u64, Audio),
    Stop(u64),
    TrackFinished,
    Shutdown,
}

pub struct MusicPlayer {
    db: DbView,
    voice_manager: Arc<Songbird>,
    sender: mpsc::Sender<PlayerCommand>,
    receiver: mpsc::Receiver<PlayerCommand>,
}

impl MusicPlayer {
    pub fn new(db: DbView, voice_manager: Arc<Songbird>) -> (Self, PlayerSender) {
        let (sender, receiver) = tokio::sync::mpsc::channel(64);

        (Self {
            db,
            voice_manager,
            sender: sender.clone(),
            receiver,
        }, sender)
    }

    pub async fn run(mut self) {
        //Reference to currently playing.
        let mut ongoing: Option<(u64, songbird::tracks::TrackHandle)> = None;
        //Back-log to play.
        let mut list = std::collections::VecDeque::new();

        while let Some(cmd) = self.receiver.recv().await {
            match cmd {
                PlayerCommand::Play(server_id, audio) => {
                    if ongoing.is_some() {
                        list.push_back((server_id, audio));
                    } else {
                        rogu::debug!("Start new track on server={}", server_id);
                        let channel_id = loop {
                            match self.db.get::<data::Server>(server_id) {
                                Ok(server) => break server.music_ch,
                                Err(error) => {
                                    rogu::error!("Cannot read music channel id: {}", error);
                                }
                            }
                        };

                        let track = match self.voice_manager.join(server_id, channel_id).await {
                            (handler, Ok(_)) => {
                                let mut handler = handler.lock().await;
                                if !handler.is_deaf() {
                                    let _ = handler.deafen(true).await;
                                }
                                handler.play_source(audio)
                            },
                            (_, Err(error)) => {
                                rogu::error!("Unable to join voice channel: {}", error);
                                continue;
                            },
                        };

                        let end_event = songbird::events::Event::Track(songbird::events::TrackEvent::End);
                        let _ = track.add_event(end_event, OnTrackFinished(self.sender.clone()));

                        ongoing = Some((server_id, track));
                    }
                },
                PlayerCommand::Stop(server_id) => {
                    if let Some((_, ongoing)) = ongoing.take() {
                        if let Err(error) = ongoing.stop() {
                            rogu::warn!("Failed to stop ongoing track: {}", error);
                        }

                        match self.voice_manager.remove(server_id).await {
                            Ok(_) => {
                                rogu::debug!("Left voice channel");
                            },
                            Err(error) => {
                                rogu::warn!("No voice manager for server {}, cannot leave voice channel: {}", server_id, error);
                            },
                        }
                    }

                    if let Some((server_id, audio)) = list.pop_front() {
                        if let Err(error) = self.sender.send(PlayerCommand::Play(server_id, audio)).await {
                            rogu::warn!("Failed to send new player command: {}", error);
                        }
                    }
                },
                PlayerCommand::TrackFinished => {
                    rogu::debug!("Track has been finished");
                    if let Some((server_id, _)) = ongoing.take() {
                        //TODO: Workaround for this buggy piece of shit that deadlocks when you
                        //join the same channel
                        match self.voice_manager.remove(server_id).await {
                            Ok(_) => {
                                rogu::debug!("Left voice channel");
                            },
                            Err(error) => {
                                rogu::warn!("No voice manager for server {}, cannot leave voice channel: {}", server_id, error);
                            },
                        }

                        if let Some((server_id, audio)) = list.pop_front() {
                            if let Err(error) = self.sender.send(PlayerCommand::Play(server_id, audio)).await {
                                rogu::warn!("Failed to send new player command: {}", error);
                            }
                        }
                    }
                },
                PlayerCommand::Shutdown => break,
            }
        }
    }
}
