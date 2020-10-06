use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

pub struct PickerKey {
    /// Symbolic name of the key
    pub(crate) name: String,
    /// Text on key
    pub(crate) text: String,
    // GTK button
    //TODO: clean up this crap
    pub(crate) gtk: RefCell<Option<gtk::Button>>,
}

pub struct PickerGroup {
    /// Name of the group
    pub(crate) name: String,
    /// Number of keys to show in each row
    pub(crate) cols: i32,
    /// Width of each key in this group
    pub(crate) width: i32,
    /// Name of keys in this group
    pub(crate) keys: Vec<Rc<PickerKey>>,
}

pub struct Picker {
    pub(crate) groups: Vec<PickerGroup>,
    pub(crate) keys: HashMap<String, Rc<PickerKey>>,
}

impl Picker {
    pub fn new() -> Self {
        const DEFAULT_COLS: i32 = 3;

        let mut groups = Vec::new();
        let mut keys = HashMap::new();

        let mut is_group = true;
        let picker_csv = include_str!("../../layouts/picker.csv");
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(picker_csv.as_bytes());
        for record_res in reader.records() {
            let record = record_res.expect("Failed to parse picker.csv");

            let name = record.get(0).unwrap_or("");
            if name.is_empty() {
                is_group = true;
            } else if is_group {
                is_group = false;

                let cols_str = record.get(1).unwrap_or("");
                let cols = match cols_str.parse::<i32>() {
                    Ok(ok) => ok,
                    Err(err) => {
                        eprintln!("Failed to parse column count '{}': {}", cols_str, err);
                        DEFAULT_COLS
                    }
                };

                let width_str = record.get(2).unwrap_or("");
                let width = match width_str.parse::<i32>() {
                    Ok(ok) => ok,
                    Err(err) => {
                        eprintln!("Failed to parse width '{}': {}", width_str, err);
                        1
                    }
                };

                let group = PickerGroup {
                    name: name.to_string(),
                    cols,
                    width,
                    keys: Vec::new(),
                };

                groups.push(group);
            } else {
                let top = record.get(1).unwrap_or("");
                let bottom = record.get(2).unwrap_or("");

                let key = Rc::new(PickerKey {
                    name: name.to_string(),
                    text: if bottom.is_empty() {
                        top.to_string()
                    } else {
                        format!("{}\n{}", top, bottom)
                    },
                    gtk: RefCell::new(None),
                });

                groups.last_mut().map(|group| {
                    group.keys.push(key.clone());
                });

                keys.insert(name.to_string(), key);
            }
        }

        Self { groups, keys }
    }
}