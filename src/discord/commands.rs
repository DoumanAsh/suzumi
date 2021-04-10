use super::*;
use crate::game::Level;
use crate::utils;

use std::time;
use core::fmt;

///1h
pub const ALLOWANCE_COOL_DOWN: u64 = 60 * 60;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod err {
    pub mod discord {
        pub const NO_USER_INFO: &str = "Cannot find your dossier :(";
        pub const BROKEN_TIME: &str = "My watch is broken, cannot do it now";
    }
}

pub const PING: u64 = xxhash_rust::const_xxh3::xxh3_64(b"ping");
pub const HELP: u64 = xxhash_rust::const_xxh3::xxh3_64(b"help");
pub const ROLL: u64 = xxhash_rust::const_xxh3::xxh3_64(b"roll");
pub const JUDGE: u64 = xxhash_rust::const_xxh3::xxh3_64(b"judge");
pub const WHOAMI: u64 = xxhash_rust::const_xxh3::xxh3_64(b"whoami");
pub const ALLOWANCE: u64 = xxhash_rust::const_xxh3::xxh3_64(b"allowance");
pub const PLAYER: u64 = xxhash_rust::const_xxh3::xxh3_64(b"player");
pub const SUGGEST: u64 = xxhash_rust::const_xxh3::xxh3_64(b"suggest");
pub const SHUTDOWN: u64 = xxhash_rust::const_xxh3::xxh3_64(b"shutdown");
pub const CONFIG: u64 = xxhash_rust::const_xxh3::xxh3_64(b"config");
pub const SET_WELCOME: u64 = xxhash_rust::const_xxh3::xxh3_64(b"set_welcome");
pub const SET_VOICE: u64 = xxhash_rust::const_xxh3::xxh3_64(b"set_voice");
pub const SET_DEV: u64 = xxhash_rust::const_xxh3::xxh3_64(b"set_dev");
pub const SET_SPAM: u64 = xxhash_rust::const_xxh3::xxh3_64(b"set_spam");

//Normally you should prefer to return future, but most of commands are too complicated to avoid
//type erasure, hence hope compiler is able to inline async
impl super::Handler {
    #[inline]
    pub async fn handle_roll(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        //format_args is not thread safe for await
        struct Text(Result<cute_dnd_dice::Roll, cute_dnd_dice::ParseError>);

        impl fmt::Display for Text {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0.as_ref() {
                    Ok(roll) => write!(f, "Roll {}: {}", roll, roll.roll()),
                    Err(error) => write!(f, "Cannot recognize dice: {}", error)
                }
            }
        }

        let roll = cute_dnd_dice::Roll::from_str(&ctx.text[4..]);
        ctx.msg.reply(ctx, Text(roll)).await?;
        Ok(())
    }

    #[inline]
    pub async fn handle_judge(&self, ctx: HandlerContext<'_>, args: Vec<&'_ str>) -> serenity::Result<()> {
        if args.len() < 2 {
            ctx.msg.reply(&ctx, "You need to give me at least two choices.").await?;
            return Ok(())
        }

        let roll = cute_dnd_dice::Roll::new(1,
                                            core::num::NonZeroU16::new(args.len() as u16).unwrap_certain(),
                                            cute_dnd_dice::Modifier::Plus(0));
        let choice = args[roll.roll() as usize - 1];

        ctx.msg.reply(&ctx, format!("The criminal is {}", choice)).await?;
        Ok(())
    }

    #[inline]
    pub async fn handle_player<'a, T: Iterator<Item=&'a str>>(&self, ctx: HandlerContext<'a>, mut args: T) -> serenity::Result<()> {
        const COST: u32 = 5;
        const START: u64 = xxhash_rust::const_xxh3::xxh3_64(b"start");
        const STOP: u64 = xxhash_rust::const_xxh3::xxh3_64(b"stop");

        let cmd = match args.next() {
            Some(cmd) => cmd,
            None => {
                ctx.msg.reply(&ctx, "Player has following commands: start, stop").await?;
                return Ok(())
            },
        };

        let id = if let Some(id) = ctx.msg.guild_id.as_ref().map(|id| id.0) {
            id
        } else {
            let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
            return Ok(());
        };

        if !self.state.db.get::<data::Server>(id).map(|server| server.music_ch != 0).unwrap_or(false) {
            ctx.msg.reply(&ctx, "Voice channel is not set yet, please do so.").await?;
            return Ok(())
        }

        match xxhash_rust::xxh3::xxh3_64(cmd.as_bytes()) {
            START => match args.next() {
                Some(music) => {
                    let user_id = ctx.msg.author.id.0;
                    let mut user = match self.state.db.get::<data::User>(user_id) {
                        Ok(user) => user,
                        Err(error) => {
                            rogu::error!("Cannot retrieve user info: {}", error);
                            ctx.msg.reply(&ctx, "Cannot access your wallet :(").await?;
                            return Ok(())
                        }
                    };

                    if user.cash < COST {
                        ctx.msg.reply(&ctx, "You do not have enough cash(5¥) for music :P").await?;
                        return Ok(())
                    }

                    user.cash -= COST;

                    let db = self.state.db.clone();
                    let guard = utils::DropGuard::new(move || db.put(user_id, &user), utils::DropAsync);

                    match songbird::ytdl(music).await {
                        Ok(music) => {
                            let mut data = ctx.serenity.data.write().await;
                            if let Some(sender) = data.get_mut::<PlayerSendTag>() {
                                if let Err(error) = sender.send(player::PlayerCommand::Play(id, music)).await {
                                    rogu::error!("Player unexpectedly stopped: {}", error);
                                } else {
                                    let _ = ctx.msg.react(&ctx, emoji::OK).await;
                                    return Ok(());
                                }
                            }
                            let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
                        },
                        Err(_) => {
                            ctx.msg.reply(&ctx, "Cannot download it, is this a youtube link?").await?;
                        },
                    }

                    guard.forget();
                },
                None => {
                    ctx.msg.reply(&ctx, "Play requires something to play, provide link to music").await?;
                }
            },
            STOP => match ctx.is_mod {
                true => {
                    let mut data = ctx.serenity.data.write().await;
                    if let Some(sender) = data.get_mut::<PlayerSendTag>() {
                        if let Err(error) = sender.send(player::PlayerCommand::Stop(id)).await {
                            rogu::error!("Player unexpectedly stopped: {}", error);
                        } else {
                            let _ = ctx.msg.react(&ctx, emoji::OK).await;
                            return Ok(());
                        }
                    }
                    let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
                },
                false => {
                    let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
                }
            },
            _ => {
                ctx.msg.reply(&ctx, "Unknown command, allowed: start, stop").await?;
            },
        }

        Ok(())
    }

    #[inline]
    pub async fn handle_suggest(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        const COST: u32 = 10;

        let id = match ctx.msg.guild_id.as_ref().map(|id| id.0) {
            Some(id) => id,
            None => {
                let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
                return Ok(());
            },
        };

        let channel = match self.state.db.get::<data::Server>(id) {
            Ok(server) if server.dev_ch != 0 => ChannelId(server.dev_ch),
            _ => {
                ctx.msg.reply(&ctx, "Dev channel is not set yet, please ask mods.").await?;
                return Ok(())
            },
        };

        let user_id = ctx.msg.author.id.0;
        let mut user = match self.state.db.get::<data::User>(user_id) {
            Ok(user) => user,
            Err(error) => {
                rogu::error!("Cannot retrieve user info: {}", error);
                ctx.msg.reply(&ctx, "Cannot access your wallet :(").await?;
                return Ok(())
            }
        };

        if user.cash < COST {
            ctx.msg.reply(&ctx, "You do not have enough cash(10¥) to post suggestion").await?;
            return Ok(())
        }

        user.cash -= COST;

        let db = self.state.db.clone();
        let guard = utils::DropGuard::new(move || db.put(user_id, &user), utils::DropAsync);

        let suggestion = &ctx.msg.content[8..];
        let mut author: serenity::builder::CreateEmbedAuthor = Default::default();
        author.name(ctx.msg.author.name.as_str());
        let user_image = if let Some(icon) = ctx.msg.author.avatar.as_ref() {
            let user_image = format!("https://cdn.discordapp.com/avatars/{}/{}.png", ctx.msg.author.id.0, icon);
            author.icon_url(&user_image);
            Some(user_image)
        } else {
            None
        };
        let user_name = ctx.msg.author.name.as_str();

        let result = channel.send_message(&ctx.serenity, move |msg| msg.embed(|m| {
            if let Some(user_image) = user_image {
                m.thumbnail(user_image);
            }

            m.title(format_args!("Suggestion from {}", user_name))
             .set_author(author)
             .description(suggestion)
             .colour(serenity::utils::Colour::DARK_PURPLE)
        })).await;

        if let Err(error) = result {
            rogu::error!("Failed to post suggestion: {}", error);
            ctx.msg.reply(&ctx, "I'm sorry I cannot post your suggestion :(").await?;
            guard.forget();
        } else {
            let _ = ctx.msg.react(&ctx, emoji::OK).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn handle_whoami(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        match self.state.db.get::<data::User>(ctx.msg.author.id.0) {
            Ok(user) => {
                let result = ctx.msg.author.direct_message(&ctx, |m| {
                    m.embed(|m| {
                        let level = Level::new(user.exp);
                        m.title("Profile")
                         .field("Level", level.level, true)
                         .field("Exp", level, true)
                         .field("Cash", user.cash, false)
                         .field("Moderator:", ctx.is_mod, false)
                    })
                }).await;

                if result.is_ok() {
                    let _ = ctx.msg.react(&ctx, emoji::OK).await;
                } else {
                    let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
                }
                result
            },
            Err(error) => {
                rogu::error!("Unable to get user's info: {}", error);
                ctx.msg.reply(ctx, err::discord::NO_USER_INFO).await
            },
        }?;

        Ok(())
    }

    #[inline]
    pub async fn handle_allowance(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        struct Allowance(u32);
        impl fmt::Display for Allowance {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "Your allowance is {}¥", self.0)
            }
        }

        let id = ctx.msg.author.id.0;
        match self.state.db.get::<data::User>(id) {
            Ok(mut user) => match time::SystemTime::UNIX_EPOCH.checked_add(user.last_allowance) {
                Some(before) => {
                    let now = time::SystemTime::now();
                    if let Ok(duration) = now.duration_since(before) {
                        if duration.as_secs() >= ALLOWANCE_COOL_DOWN {
                            let level = game::Level::new(user.exp);
                            let allowance = level.cash();
                            user.cash = user.cash.saturating_add(allowance);
                            user.last_allowance = now.duration_since(time::SystemTime::UNIX_EPOCH).expect("Broken Time");

                            let db = self.state.db.clone();
                            let _ = tokio::task::spawn_blocking(move || db.put(id, &user)).await;

                            ctx.msg.reply(&ctx, Allowance(allowance)).await?;
                            return Ok(())
                        }
                    }
                    //error case can only happen if for some reason now is in past

                    let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
                },
                None => {
                    //well I suppose we're too far in future so fix god damn system time
                    rogu::error!("Time is broken");
                    ctx.msg.reply(ctx, err::discord::BROKEN_TIME).await?;
                }
            },
            Err(error) => {
                rogu::error!("Unable to get user's info: {}", error);
                ctx.msg.reply(ctx, err::discord::NO_USER_INFO).await?;
            },
        }

        Ok(())
    }

    #[inline]
    pub async fn handle_help(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        let result = ctx.msg.author.direct_message(&ctx, |msg| {
            if ctx.is_mod {
                msg.content(format_args!("{}\n{}", CMD_HELP_TXT, MOD_CMD_HELP_TXT))
            } else {
                msg.content(CMD_HELP_TXT)
            }
        }).await;

        match result {
            Ok(_) => {
                let _ = ctx.msg.react(&ctx, emoji::OK).await;
                Ok(())
            },
            Err(err) => Err(err)
        }
    }

    #[inline]
    pub async fn handle_config(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        if ctx.is_mod {
            if let Some(id) = ctx.msg.guild_id.as_ref().map(|id| id.0) {
                if let Ok(server) = self.state.db.get::<data::Server>(id) {
                    ctx.msg.author.direct_message(&ctx, |m| {
                        m.embed(|m| {
                            m.title("Config")
                             .field("Version", VERSION, false)
                             .field("Welcome channel", server.welcome_ch, false)
                             .field("Music channel", server.music_ch, false)
                             .field("Dev channel", server.dev_ch, false)
                             .field("Spam channel", server.spam_ch, false)
                        })
                    }).await?;

                    let _ = ctx.msg.react(&ctx, emoji::OK).await;
                    return Ok(())
                }
            }
        }

        let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
        Ok(())
    }

    #[inline]
    pub async fn handle_shutdown(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        if ctx.is_mod {
            let data = ctx.serenity.data.read().await;
            if let Some(manager) = data.get::<ShardManagerTag>() {
                let _ = ctx.msg.react(&ctx, emoji::OK).await;
                manager.lock().await.shutdown_all().await;
                return Ok(());
            }

            let mut data = ctx.serenity.data.write().await;
            if let Some(sender) = data.get_mut::<PlayerSendTag>() {
                let _ = sender.send(player::PlayerCommand::Shutdown);
            }
        }

        let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
        Ok(())
    }

    #[inline]
    pub async fn handle_set_welcome(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        if ctx.is_mod {
            if let Some(id) = ctx.msg.guild_id.as_ref().map(|id| id.0) {
                if let Ok(mut server) = self.state.db.get::<data::Server>(id) {
                    server.welcome_ch = if server.welcome_ch == ctx.msg.channel_id.0 {
                        0
                    } else {
                        ctx.msg.channel_id.0
                    };

                    let db = self.state.db.clone();
                    let _ = tokio::task::spawn_blocking(move || db.put(id, &server)).await;

                    let _ = ctx.msg.react(&ctx, emoji::OK).await;
                    return Ok(())
                }
            }
        }

        let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
        Ok(())
    }

    #[inline]
    pub async fn handle_set_voice(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        if ctx.is_mod {
            if let Some(guild) = ctx.msg.guild(&ctx.serenity).await {
                let server_id = guild.id.0;
                if let Some(voice_ch) = guild.voice_states.get(&ctx.msg.author.id).and_then(|state| state.channel_id) {
                    if let Ok(mut server) = self.state.db.get::<data::Server>(server_id) {
                        server.music_ch = if server.music_ch == voice_ch.0 {
                            0
                        } else {
                            voice_ch.0
                        };

                        let db = self.state.db.clone();
                        let _ = tokio::task::spawn_blocking(move || db.put(server_id, &server)).await;

                        let _ = ctx.msg.react(&ctx, emoji::OK).await;
                        return Ok(())
                    }
                } else {
                    let _ = ctx.msg.reply(&ctx, "You're not in any channel, please join one before using the command").await;
                }
            }
        }

        let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
        Ok(())
    }

    #[inline]
    pub async fn handle_set_dev(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        if ctx.is_mod {
            if let Some(id) = ctx.msg.guild_id.as_ref().map(|id| id.0) {
                if let Ok(mut server) = self.state.db.get::<data::Server>(id) {
                    server.dev_ch = if server.dev_ch == ctx.msg.channel_id.0 {
                        0
                    } else {
                        ctx.msg.channel_id.0
                    };

                    let db = self.state.db.clone();
                    let _ = tokio::task::spawn_blocking(move || db.put(id, &server)).await;

                    let _ = ctx.msg.react(&ctx, emoji::OK).await;
                    return Ok(())
                }
            }
        }

        let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
        Ok(())
    }

    #[inline]
    pub async fn handle_set_spam(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        if ctx.is_mod {
            if let Some(id) = ctx.msg.guild_id.as_ref().map(|id| id.0) {
                if let Ok(mut server) = self.state.db.get::<data::Server>(id) {
                    server.spam_ch = if server.spam_ch == ctx.msg.channel_id.0 {
                        0
                    } else {
                        ctx.msg.channel_id.0
                    };

                    let db = self.state.db.clone();
                    let _ = tokio::task::spawn_blocking(move || db.put(id, &server)).await;

                    let _ = ctx.msg.react(&ctx, emoji::OK).await;
                    return Ok(())
                }
            }
        }

        let _ = ctx.msg.react(&ctx, emoji::KINSHI).await;
        Ok(())
    }
}
