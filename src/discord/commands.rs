use super::*;
use crate::game::Level;

use std::time;
use core::fmt;

///1h
pub const ALLOWANCE_COOL_DOWN: u64 = 60 * 60;
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

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
pub const SHUTDOWN: u64 = xxhash_rust::const_xxh3::xxh3_64(b"shutdown");
pub const SET_WELCOME: u64 = xxhash_rust::const_xxh3::xxh3_64(b"set_welcome");
pub const CONFIG: u64 = xxhash_rust::const_xxh3::xxh3_64(b"config");

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
}
