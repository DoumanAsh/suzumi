use super::*;
use crate::game::Level;

use core::fmt;

pub const PING: u64 = xxhash_rust::const_xxh3::xxh3_64(b"ping");
pub const HELP: u64 = xxhash_rust::const_xxh3::xxh3_64(b"help");
pub const ROLL: u64 = xxhash_rust::const_xxh3::xxh3_64(b"roll");
pub const WHOAMI: u64 = xxhash_rust::const_xxh3::xxh3_64(b"whoami");
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
                ctx.msg.reply(ctx, "Cannot find information :(").await
            },
        }?;

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
