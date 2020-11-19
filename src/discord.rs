use serenity::async_trait; //they use this cancer :(
use serenity::model::prelude::{Ready, Message, Guild, GuildUnavailable, GuildId, Member, ChannelId};
use serenity::client::Context;
use serenity::model::misc::Mentionable;

use crate::data;
use crate::assets::Assets;
use crate::db::DbView;

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

struct Handler {
    state: State,
    voice_manager: std::sync::Arc<serenity::prelude::Mutex<serenity::client::bridge::voice::ClientVoiceManager>>
}

///Discord event handler, contains state which is thread safe.
pub struct Discord {
    state: State,
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
            token: args.token.0,
        })
    }

    pub async fn start(self) {
        loop {
            let voice_manager = serenity::client::bridge::voice::ClientVoiceManager::new(1, self.state.info.id.into());

            let handler = Handler {
                state: self.state.clone(),
                voice_manager: std::sync::Arc::new(serenity::prelude::Mutex::new(voice_manager)),
            };

            let mut client = match serenity::client::Client::builder(self.token.as_str()).event_handler(handler).await {
                Ok(client) => client,
                Err(error) => {
                    eprintln!("Unable to connect to discord. Error: {}", error);
                    continue;
                }
            };

            loop {
                if let Err(error) = client.start().await {
                    eprintln!("Client failure. Error: {}", error);
                }
            }
        }
    }
}

#[async_trait]
impl serenity::client::EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
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
