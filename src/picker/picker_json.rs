use serde::Deserialize;

#[derive(Deserialize)]
pub struct PickerJsonKey {
    pub keysym: String,
    pub label: String,
}

#[derive(Deserialize)]
pub struct PickerJsonGroup {
    pub label: String,
    pub section: String,
    pub cols: u32,
    pub width: i32,
    pub keys: Vec<PickerJsonKey>,
}

pub fn picker_json() -> Vec<PickerJsonGroup> {
    let picker_json = include_str!("../../layouts/picker.json");
    serde_json::from_str(picker_json).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_picker_json() {
        picker_json();
    }
}
