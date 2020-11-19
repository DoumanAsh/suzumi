use crate::data;
use crate::utils::OptionExt;

#[derive(Clone)]
//Namespaces that we use.
//
//Generally `sled::Db` is light-weight, but we do not really need it
//to write into namespaces.
pub struct DbView {
    pub user: sled::Tree,
    pub server: sled::Tree,
}

impl DbView {
    pub fn put<T: Tag>(&self, id: u64, data: &T) {
        let mut retry = 5;

        loop {
            match T::view(self).insert(id.to_be_bytes(), data.serialize().as_ref()) {
                Ok(_) => break,
                Err(error) => match retry {
                    0 => {
                        rogu::error!("Unable to put data for id={} into storage. Error: {}", id, error);
                        break;
                    },
                    _ => {
                        retry -= 1;
                    }
                }
            }
        }
    }

    pub fn get<T: Tag>(&self, id: u64) -> Result<T, sled::Error> {
        match T::view(self).get(id.to_be_bytes()) {
            Ok(Some(result)) => {
                let result: &[u8] = result.as_ref();
                if result.len() != <T as data::Serialize>::SIZE {
                    //TODO: consider using format that would work just fine with extending it (e.g.  json)
                    //but these formats are overhead
                    Ok(T::default())
                } else {
                    let result = result.as_ptr() as *const T::Output;
                    let result = unsafe {
                        result.as_ref().unwrap_certain()
                    };
                    Ok(T::deserialize(result))
                }
            },
            Ok(None) => Ok(T::default()),
            Err(error) => Err(error),
        }
    }
}

trait Tag: data::Deserialize + Default {
    fn view(view: &DbView) -> &sled::Tree;
}

impl Tag for data::User {
    #[inline]
    fn view(view: &DbView) -> &sled::Tree {
        &view.user
    }
}

impl Tag for data::Server {
    #[inline]
    fn view(view: &DbView) -> &sled::Tree {
        &view.server
    }
}

pub struct Db {
    #[allow(unused)]
    db: sled::Db,
    view: DbView,
}

impl Db {
    pub fn open(path: &str) -> Result<Self, sled::Error> {
        let db = sled::Config::new().path(path)
                                    .cache_capacity(128_000)
                                    .mode(sled::Mode::LowSpace)
                                    .flush_every_ms(Some(60_000))
                                    .open()?;

        let user = db.open_tree("user")?;
        let server = db.open_tree("server")?;

        Ok(Self {
            db,
            view: DbView {
                user,
                server,
            },
        })
    }

    #[inline]
    pub fn view(&self) -> DbView {
        self.view.clone()
    }
}
