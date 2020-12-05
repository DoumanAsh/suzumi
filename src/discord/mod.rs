use serenity::async_trait; //they use this cancer :(
use serenity::client::Context;
use serenity::model::misc::Mentionable;
use serenity::prelude::{TypeMapKey};
use serenity::client::bridge::gateway::{ShardManager};
use serenity::model::prelude::{Ready, Message, Guild, GuildUnavailable, GuildId, Member, ChannelId, PartialGuild, RoleId, Role};

use crate::{game, data};
use crate::assets::Assets;
use crate::db::DbView;
use crate::utils::OptionExt;

use core::fmt;
use std::collections::{HashMap, HashSet};

const CMD_HELP_TXT: &str = include_str!("../../HELP.md");
const MOD_CMD_HELP_TXT: &str = include_str!("../../MOD_HELP.md");

struct ShardManagerTag;
impl TypeMapKey for ShardManagerTag {
    type Value = std::sync::Arc<serenity::prelude::Mutex<ShardManager>>;
}

struct PlayerSendTag;
impl TypeMapKey for PlayerSendTag {
    type Value = player::PlayerSender;
}

mod utils;
mod commands;
mod emoji;
mod player;

#[derive(Clone)]
pub struct Config {
    prefix: char,
}

#[derive(Clone)]
struct Info {
    id: u64,
    owner: u64,
}

#[derive(Clone)]
struct State {
    info: Info,
    db: DbView,
    assets: Assets,
}

//Discord state and handler, which processes incoming messages.
struct Handler {
    state: State,
    mods: tokio::sync::RwLock<HashSet<u64>>,
    config: Config,
}

pub struct HandlerContext<'a> {
    serenity: &'a Context,
    msg: &'a Message,
    text: &'a str,
    is_mod: bool,
}

impl<'a> serenity::http::CacheHttp for HandlerContext<'a> {
    #[inline(always)]
    fn http(&self) -> &serenity::http::client::Http {
        serenity::http::CacheHttp::http(&self.serenity)
    }

    #[inline(always)]
    fn cache(&self) -> Option<&std::sync::Arc<serenity::cache::Cache>> {
        serenity::http::CacheHttp::cache(&self.serenity)
    }
}

impl Handler {
    async fn update_mod_roles(&self, roles: &HashMap<RoleId, Role>) {
        let mut mods = self.mods.write().await;
        for (id, role) in roles {
            if role.name.eq_ignore_ascii_case("Moderator") || role.name.contains("Staff") {
                mods.insert(id.0);
            }
        }
    }

    async fn is_moderator(&self, roles: &Vec<RoleId>) -> bool {
        let mods = self.mods.read().await;
        for role in roles {
            if mods.contains(&role.0) {
                return true;
            }
        }

        false
    }

    async fn handle_chat(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        struct LevelUpCong(String, u8);

        impl fmt::Display for LevelUpCong {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{} Congratulations on level up! Your new level is {}", self.0, self.1)
            }
        }

        let id = ctx.msg.author.id.0;
        let mut user: data::User = match self.state.db.get(id) {
            Ok(user) => user,
            Err(error) => {
                rogu::error!("Unable to get user data: {}", error);
                return Ok(())
            }
        };

        let mut level = game::Level::new(user.exp);
        let result = level.add_for(ctx.msg);
        if result == game::LevelAddResult::Maxed {
            return Ok(())
        }


        user.exp = level.exp;
        let db = self.state.db.clone();
        let _ = tokio::task::spawn_blocking(move || db.put(id, &user)).await;

        let server_id = match ctx.msg.guild_id {
            Some(server) => server.0,
            None => return Ok(()),
        };

        let server_info: data::Server = match self.state.db.get(server_id) {
            Ok(server_info) => server_info,
            Err(error) => {
                rogu::error!("Unable to get server info: {}", error);
                return Ok(());
            }
        };

        if server_info.spam_ch == 0 {
            return Ok(());
        }

        if result == game::LevelAddResult::LevelUp {
            let author = ctx.msg.author.mention();
            let level = LevelUpCong(author, level.level);
            ChannelId(server_info.spam_ch).send_message(&ctx.serenity, |msg| msg.content(level)).await.map(|_| ())
        } else {
            Ok(())
        }
    }

    async fn handle_cmd(&self, ctx: HandlerContext<'_>) -> serenity::Result<()> {
        use commands::*;

        let mut split = ctx.text.split_whitespace();
        //First iteration always has some value, even if there are no whitespaces;
        let cmd = split.next().unwrap_certain();

        match xxhash_rust::xxh3::xxh3_64(cmd.as_bytes()) {
            PING => ctx.msg.reply(ctx, "pong!").await.map(|_| ()),
            HELP => self.handle_help(ctx).await,
            ROLL => self.handle_roll(ctx).await,
            JUDGE => self.handle_judge(ctx, split.collect()).await,
            PLAYER => self.handle_player(ctx, split).await,
            SUGGEST => self.handle_suggest(ctx).await,
            WHOAMI => self.handle_whoami(ctx).await,
            ALLOWANCE => self.handle_allowance(ctx).await,
            SHUTDOWN => self.handle_shutdown(ctx).await,
            CONFIG => self.handle_config(ctx).await,
            SET_WELCOME => self.handle_set_welcome(ctx).await,
            SET_VOICE => self.handle_set_voice(ctx).await,
            SET_DEV => self.handle_set_dev(ctx).await,
            SET_SPAM => self.handle_set_spam(ctx).await,
            _ => ctx.msg.reply(ctx, "Sorry, I do not know such command").await.map(|_| ()),
        }
    }
}

///Discord wrapper
pub struct Discord {
    state: State,
    config: Config,
    token: crate::cli::TokenStr,
}

impl Discord {
    #[inline]
    pub async fn new(args: crate::cli::Cli, db: DbView, assets: Assets) -> Result<Self, u8> {
        let http = serenity::http::client::Http::new_with_token(args.token.0.as_str());
        let info = match http.get_current_application_info().await {
            Ok(info) => Info {
                id: info.id.0,
                owner: info.owner.id.0
            },
            Err(error) => {
                eprintln!("Unable to acquire self info. Check token. Error: {}", error);
                return Err(1);
            }
        };

        Ok(Self {
            state: State {
                info,
                db,
                assets,
            },
            config: Config {
                prefix: args.prefix,
            },
            token: args.token.0,
        })
    }

    pub async fn start(self) {
        loop {
            //let voice_manager = serenity::client::bridge::voice::ClientVoiceManager::new(1, self.state.info.id.into());

            let handler = Handler {
                state: self.state.clone(),
                mods: tokio::sync::RwLock::new(HashSet::new()),
                config: self.config.clone(),
            };

            let client = serenity::client::Client::builder(self.token.as_str());

            let mut client = match client.event_handler(handler).await {
                Ok(client) => client,
                Err(error) => {
                    rogu::error!("Unable to connect to discord. Error: {}", error);
                    continue;
                }
            };

            let (player, sender) = player::MusicPlayer::new(self.state.db.clone(), client.voice_manager.clone());
            {
                let mut data = client.data.write().await;
                data.insert::<ShardManagerTag>(client.shard_manager.clone());
                data.insert::<PlayerSendTag>(sender.clone());
            }

            tokio::spawn(player.run());

            loop {
                if let Err(error) = client.start().await {
                    rogu::error!("Client failure. Error: {}", error);
                } else {
                    rogu::info!("Shutting down");
                    return;
                }
            }

            //let _ = sender.send(player::PlayerCommand::Shutdown).await;
        }
    }
}

#[async_trait]
impl serenity::client::EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        //Ignore messages from bots and any non-chatter
        if msg.author.bot || msg.kind != serenity::model::channel::MessageType::Regular {
            return;
        }

        let content = msg.content.trim();

        let result = if msg.guild_id.is_none() {
            let trimmed_cmd = content.trim_start_matches(self.config.prefix);
            //Handle only commands in DM
            if content.len() - trimmed_cmd.len() != 1 {
                return;
            }

            let context = HandlerContext {
                serenity: &ctx,
                msg: &msg,
                text: trimmed_cmd,
                is_mod: false,
            };
            self.handle_cmd(context).await
        } else {
            let trimmed_cmd = content.trim_start_matches(self.config.prefix);

            if content.len() - trimmed_cmd.len() != 1 {
                let context = HandlerContext {
                    serenity: &ctx,
                    msg: &msg,
                    text: content,
                    //we do not care if chat is from moderator or not, at least for now
                    is_mod: false,
                };
                self.handle_chat(context).await
            } else {
                let is_mod = match msg.member(&ctx.http).await {
                    Ok(member) => self.is_moderator(&member.roles).await,
                    Err(error) => {
                        //It is supposed to fail when message is DM, but we handle DM differently
                        rogu::warn!("Member info is unavailable, cannot determine moderator status. Error: {}", error);
                        false
                    }
                };

                let context = HandlerContext {
                    serenity: &ctx,
                    msg: &msg,
                    text: trimmed_cmd,
                    is_mod,
                };
                self.handle_cmd(context).await
            }
        };

        if let Err(error) = result {
            rogu::error!("Failed to deliver message: {}", error);
        }
    }

    async fn cache_ready(&self, ctx: Context, servers: Vec<GuildId>) {
        rogu::info!("Joined {} number of server", servers.len());

        for server in servers {
            let info = match server.to_partial_guild(&ctx.http).await {
                Ok(info) => info,
                Err(error) => {
                    rogu::error!("Failed to get info of server {}. Error: {}", server.0, error);
                    continue;
                }
            };

            self.update_mod_roles(&info.roles).await;
        }
    }

    async fn guild_update(&self, _: Context, _: Option<Guild>, update: PartialGuild) {
        self.update_mod_roles(&update.roles).await;
    }

    async fn guild_delete(&self, _: Context, server: GuildUnavailable, _: Option<Guild>) {
        self.state.db.delete::<data::Server>(server.id.0);
    }

    async fn guild_member_addition(&self, ctx: Context, server: GuildId, member: Member) {
        if member.user.bot {
            return;
        }

        let server_id = server.0;
        let server_info: data::Server = match self.state.db.get(server_id) {
            Ok(server_info) => server_info,
            Err(error) => {
                rogu::error!("Unable to get server info: {}", error);
                return;
            }
        };

        if server_info.welcome_ch == 0 {
            return;
        }

        let name = member.user.name.as_str();
        let mention = member.user.mention();

        let welcome_ch = ChannelId(server_info.welcome_ch);
        let result = match self.state.assets.gen_welcome(name) {
            Some(img) => {
                let mut buffer = Vec::new();
                if let Err(error) = img.write_to(&mut buffer, image::ImageOutputFormat::Png) {
                    rogu::error!("Unexpected error generating image: {}", error);
                    return;
                }

                let attach = serenity::http::AttachmentType::Bytes {
                    data: buffer.as_slice().into(),
                    filename: "welcome.png".to_owned(),
                };

                welcome_ch.send_files(&ctx.http, Some(attach), |msg| msg.content("Welcome")).await
            },
            None => welcome_ch.send_message(&ctx.http, |msg| {
                msg.content(format_args!("{}: Welcome to the server!", mention))
            }).await
        };

        if let Err(error) = result {
            rogu::error!("Unable to post welcome: {}", error);
        }
    }

    async fn ready(&self, _: Context, _: Ready) {
        rogu::debug!("Connected");
    }
}
