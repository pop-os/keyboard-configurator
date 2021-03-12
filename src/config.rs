use app_dirs2::{app_dir, AppDataType, AppInfo};
use glib::subclass::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer_pretty};
use std::{
    io,
    fs::File,
    path::PathBuf,
};

use crate::DerefCell;

const APP_INFO: AppInfo = AppInfo {
    name: "keyboardconfigurator",
    author: "system76",
};

#[derive(Default, Deserialize, Serialize)]
struct Data {
}

#[derive(Default)]
pub struct ConfigInner {
    path: DerefCell<PathBuf>,
    // XXX mutable
    data: DerefCell<Data>,
}

#[glib::object_subclass]
impl ObjectSubclass for ConfigInner {
    const NAME: &'static str = "S76ConfiguratorConfig";
    type ParentType = glib::Object;
    type Type = Config;
}

impl ObjectImpl for ConfigInner {}

glib::wrapper! {
    pub struct Config(ObjectSubclass<ConfigInner>);
}

impl Config {
    fn open() -> Self {
        let path = app_dir(AppDataType::UserConfig, &APP_INFO, "config.json").unwrap();
        let config: Self = glib::Object::new(&[]).unwrap();
        config.inner().path.set(path);
        //config.inner().data.set(from_reader())
        config
    }

    fn save(&self) -> io::Result<()> {
        let file = File::create(&*self.inner().path)?;
        to_writer_pretty(file, &*self.inner().data)?;
        Ok(())
    }

    fn inner(&self) -> &ConfigInner {
        ConfigInner::from_instance(self)
    }
}
